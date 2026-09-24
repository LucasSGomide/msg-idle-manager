//! The window's template children and the wiring that turns a click into a
//! domain intent and the result back into a redraw (architecture rules 8, 12).

use std::cell::{Cell, OnceCell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{
    DEFAULT_MOBILE_VIEWPORT, Effect, Frame, Layout, Liveness, MoveOutcome, PhoneLink, PhoneStatus,
    Preset, PresetCatalogue, ProfileLocator, ProfileRemoval, RemoteIntent, RemoteState, Session,
    SessionId, SlotId, Switch, UpdateChannel, UpdateCheck, UpdateError, UpdateEvent, UpdateInfo,
    UpdatePolicy, UpdateSchedule, UpdateState, VerifiedPackage, Version, Viewport, WorkspaceBook,
    WorkspaceId, WorkspaceList, WorkspaceReadError, ZoomLevel, ZoomMemory, account_name,
    scroll_script, tap_script, workspace_name,
};

use super::shortcut::{Shortcut, repeats_while_held, shortcut_for};
use crate::account_deletion;
use crate::add_game_dialog::{AddGameDialog, Confirmed};
use crate::delete_account_dialog::DeleteAccountDialog;
use crate::message_strip::MessageStrip;
use crate::phone_dialog::PhoneDialog;
use crate::rename_dialog::{NameCheck, RenameDialog};
use crate::save_on_change::Saver;
use crate::session_grid::SessionGrid;
use crate::session_sidebar::{MoveTarget, SessionSidebar};
use crate::start_queue::StartQueue;
use crate::update_notice::UpdateNotice;
use crate::web_engine::{CapturedFrame, EngineCaptureError, EngineView};
use crate::web_view::{AccountSettings, SessionView, background_for};

/// How often the window checks whether the daily update check is due
/// (`UpdateSchedule::next_check_due`) — often enough that a run left open for
/// days never misses the day it falls due, cheap enough that it costs
/// nothing to tick (roadmap item 16 task 07).
const UPDATE_SCHEDULE_TICK_SECS: u64 = 60;

/// What a workspace name window is for — [`Window::present_workspace_name_dialog`]
/// covers both, since only the title, confirm label, starting text and what
/// confirming does differ (item 11 task 06, `FR.15.8`, `FR.17.6`).
enum NameDialogPurpose {
    /// Create a workspace holding exactly these ticked accounts.
    Create(Vec<SessionId>),
    /// Rename this already-existing workspace.
    Rename(WorkspaceId),
}

/// The composite-template backing object for [`super::Window`].
#[derive(Default, CompositeTemplate)]
#[template(resource = "/org/idlemanager/IdleManager/ui/window.ui")]
pub struct Window {
    #[template_child]
    reload_button: TemplateChild<gtk::Button>,
    #[template_child]
    title_label: TemplateChild<gtk::Label>,
    #[template_child]
    main_menu: TemplateChild<gtk::MenuButton>,
    #[template_child]
    add_first_game_button: TemplateChild<gtk::Button>,
    #[template_child]
    layout_single: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    layout_side_by_side: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    layout_grid: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    layout_mobile: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    pager: TemplateChild<gtk::Box>,
    #[template_child]
    page_previous: TemplateChild<gtk::Button>,
    #[template_child]
    page_readout: TemplateChild<gtk::Label>,
    #[template_child]
    page_next: TemplateChild<gtk::Button>,
    #[template_child]
    sidebar_toggle: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    sidebar_revealer: TemplateChild<gtk::Revealer>,
    #[template_child]
    root_box: TemplateChild<gtk::Box>,
    #[template_child]
    content: TemplateChild<gtk::Box>,
    #[template_child]
    empty_state: TemplateChild<gtk::Box>,
    #[template_child]
    first_run_label: TemplateChild<gtk::Label>,
    #[template_child]
    workspace_empty_label: TemplateChild<gtk::Label>,

    /// The window-level message bar under the header bar (design rule 9). Hidden
    /// until a workspace fails to load (task 04) or, later, a save fails
    /// (task 06).
    message_strip: MessageStrip,
    grid: SessionGrid,
    sidebar: SessionSidebar,
    book: RefCell<WorkspaceBook>,
    locator: RefCell<Option<Rc<dyn ProfileLocator>>>,
    /// The debounced save-on-change path (task 06), created once the ports are
    /// attached.
    saver: RefCell<Option<Saver>>,
    /// The game catalogue, read afresh every time the add-game dialog opens so
    /// a file dropped into the presets folder by hand shows up without a
    /// restart.
    catalogue: RefCell<Option<Rc<dyn PresetCatalogue>>>,
    /// One holder per account, owning its network session for the account's
    /// whole life and its view only while it is running. The grid holds its own
    /// reference to the same view; this map is what a later slice asks to stop
    /// or start.
    holders: RefCell<HashMap<SessionId, SessionView>>,
    /// The queue that brings accounts up one at a time (task 05): a restore's
    /// running accounts, or `Start all`'s parked ones. Kept for the window's
    /// whole life and appended to, never replaced — created once, in
    /// `constructed` — since replacing it mid-drain would start two accounts
    /// at once (the fourth Blocker item 14's roadmap doc names).
    start_queue: OnceCell<StartQueue>,
    /// Accounts a `Park all` reached while they were [`Liveness::Starting`]:
    /// no view exists yet to stop, so the id waits here until its page paints
    /// and `finish_starting` parks it then, rather than a second view ever
    /// being built for it (`FR.24.1`).
    park_on_paint: RefCell<HashSet<SessionId>>,
    /// Reads and stores each account's chosen zoom sizes (item 09 task 07).
    /// The shell never learns a file is behind it.
    zoom_memory: RefCell<Option<Rc<dyn ZoomMemory>>>,
    /// One settle timer per account: a zoom gesture rearms that account's
    /// timer, and only when the gestures stop does the settled map get written
    /// once (`FR.12.5`).
    zoom_save_timers: RefCell<HashMap<SessionId, glib::SourceId>>,
    /// Removes one account's profile folder (item 11 task 08). `Arc`, like
    /// `WorkspaceStore` above, since `account_deletion::delete_account` calls
    /// it through `gio::spawn_blocking`.
    removal: RefCell<Option<Arc<dyn ProfileRemoval>>>,
    /// Whether the window is currently minimised (roadmap item 12 task 04,
    /// `FR.1.9`), watched from `realize` through the toplevel surface's own
    /// `state`. Read whenever a view is started, so it opens already marked
    /// to match, and updated by `apply_minimised` on every minimise/restore.
    minimised: Cell<bool>,
    /// The phone's link (roadmap item 13 task 06): every redraw publishes
    /// the book through it and the capture timer publishes frames. `None`
    /// until the ports are attached, or when no link was handed over at all.
    phone_link: RefCell<Option<Arc<dyn PhoneLink>>>,
    /// `win.show-phone`, the main menu's `Phone…` item: opens the phone
    /// dialog. Enabled only once a link exists to open it against. Connect
    /// and disconnect both happen inside the dialog itself (design rule 16),
    /// so the header menu carries no `Un-enrol` item of its own.
    show_phone_action: OnceCell<gio::SimpleAction>,
    /// `win.zoom-in`, the main menu's zoom section (design rule 10). Enabled
    /// only while an account is focused; the section itself is left out of
    /// the menu model entirely otherwise (`update_header`).
    zoom_in_action: OnceCell<gio::SimpleAction>,
    /// `win.zoom-out`, otherwise exactly [`Window::zoom_in_action`].
    zoom_out_action: OnceCell<gio::SimpleAction>,
    /// `win.zoom-reset`, otherwise exactly [`Window::zoom_in_action`].
    zoom_reset_action: OnceCell<gio::SimpleAction>,
    /// Whether the phone is watching right now — between its `Attach` and
    /// its `Leave`.
    phone_attached: Cell<bool>,
    /// The phone's screen as it last reported it, kept across a leave: the
    /// page asks for mobile mode before it attaches, so the mode is entered
    /// at the shape the phone had last time and corrected on the `Attach`
    /// that follows (`FR.3.2`).
    phone_viewport: Cell<Option<Viewport>>,
    /// The account whose live view has been told a phone is watching it, so
    /// the next change of current account can un-tell exactly that one
    /// (`FR.4.3`). Only ever names an account that had a view at the time.
    watched: RefCell<Option<SessionId>>,
    /// The frame timer, alive from `Attach` to `Leave`.
    capture_timer: RefCell<Option<glib::SourceId>>,
    /// A capture asked for and not yet answered: the timer skips its tick
    /// rather than queueing a second snapshot behind a slow one.
    capture_in_flight: Cell<bool>,
    /// When the capture in flight was asked for, so a snapshot the engine
    /// never answers — the phone's picture would silently stop — is named in
    /// the log and asked for again after [`CAPTURE_STALL_SECS`].
    capture_asked_at: Cell<Option<std::time::Instant>>,
    /// The reason the last capture failed, so a persisting failure is logged
    /// once and not twelve times a second.
    capture_failure: RefCell<Option<String>>,
    /// The width of the last frame published, in the engine's pixels. The
    /// phone's taps arrive in that grid, and this is what maps them back to
    /// the page's own CSS pixels.
    frame_width: Cell<Option<u32>>,
    /// Whether a shortcut that must not repeat is currently held down — every
    /// one of them except `Reload` and `Zoom`, the two a user holding the key
    /// down legitimately wants again (`FR.23.3`, `FR.26.6`). GTK 4 exposes no
    /// repeat flag on a key event, so this latch stands in for one (code
    /// standards rule 18): set on the press that runs such a shortcut,
    /// cleared on the next key release of any key, and a press arriving while
    /// it is set is consumed and ignored. Clearing on any release rather than
    /// on one particular keyval is what lets it serve chords whose key is not
    /// `Tab`; it is sound because GDK enables detectable auto-repeat on X11
    /// and synthesises Wayland repeat as presses alone, so a held chord
    /// delivers no release to clear it early.
    chord_held: Cell<bool>,
    /// The account whose live view was last handed the keyboard by
    /// [`Window::follow_focus_with_keyboard`], or `None` when the focused
    /// position has no live view to hand it to.
    ///
    /// This is what makes the grab follow a focus *change* rather than fire
    /// on every [`Window::redraw`]. A redraw happens for reasons that have
    /// nothing to do with focus — a memory reading, another account's
    /// liveness, a phone publish — and grabbing on each of them would keep
    /// yanking the keyboard back from a widget the user deliberately clicked
    /// (`FR.27.1`).
    last_focus_grab: RefCell<Option<SessionId>>,

    /// The bar under the header bar where an update is learned about,
    /// fetched and offered to install (roadmap item 16 task 07, design rule
    /// 29). Hidden until the launch-time or a later check finds something
    /// worth showing.
    update_notice: UpdateNotice,
    /// Checks GitHub, downloads and stages an update. `None` until the ports
    /// are attached.
    update_channel: RefCell<Option<Arc<dyn UpdateChannel>>>,
    /// The state [`Window::drive_update`] last answered — the single source
    /// [`UpdateNotice::render`] draws from.
    update_state: RefCell<UpdateState>,
    /// The pure policy deciding what an update event means; carries only the
    /// version a user last dismissed across calls (architecture rule 9).
    update_policy: RefCell<UpdatePolicy>,
    /// When the last check completed, for [`UpdateSchedule::next_check_due`].
    /// `None` until the first check this run finishes.
    last_update_check_millis: Cell<Option<u64>>,
    /// The version and `What's new` address the last successful check found,
    /// kept so a later `FetchRequested` — reached from `Available` or a named
    /// `Failed` — has something to download; `UpdateState` itself drops
    /// `notes_url` once the state moves past `Available`.
    update_info: RefCell<Option<UpdateInfo>>,
    /// A downloaded package that passed its checks, staged to install once
    /// this process exits. Read by the close handler regardless of whether
    /// the notice is still showing — a dismissed `Ready` still installs
    /// (design's `**States**` bullet).
    verified_package: RefCell<Option<VerifiedPackage>>,
    /// Whether `apply_on_exit` should relaunch the application once this
    /// process exits — set only by `Restart now`; an ordinary quit while
    /// `Ready` applies the update without relaunching.
    relaunch_after_update: Cell<bool>,
    /// The version this build is running, read once when the update channel
    /// is attached — the main menu's own insensitive `Idle Manager <version>`
    /// item names it on every redraw without asking the channel again.
    running_version: Cell<Option<Version>>,
}

impl std::fmt::Debug for Window {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Window").finish_non_exhaustive()
    }
}

#[glib::object_subclass]
impl ObjectSubclass for Window {
    const NAME: &'static str = "IdleManagerWindow";
    type Type = super::Window;
    type ParentType = gtk::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for Window {
    fn constructed(&self) {
        self.parent_constructed();

        // The strip spans the sidebar and the grid, directly under the header
        // bar (design rule 9).
        self.root_box.prepend(&self.message_strip);
        self.install_update_notice();

        self.grid.set_hexpand(true);
        self.grid.set_vexpand(true);
        self.content.append(&self.grid);

        install_styles();
        arm_debug_minimise(&self.obj());
        arm_debug_layout(&self.obj());
        self.register_menu_actions();
        self.init_start_queue();

        let window = self.obj().downgrade();
        self.grid.connect_slot_focused(move |slot| {
            if let Some(window) = window.upgrade() {
                let imp = window.imp();
                let focused = imp.book.borrow_mut().active_mut().focus_slot(slot);
                // A click on an empty trailing slot leaves the focus where it
                // was (architecture rule 8) — the sidebar marks the
                // focused-slot row as current, so a real focus change has to
                // reach it too, but a no-op redraws nothing.
                if focused {
                    imp.redraw();
                }
            }
        });

        // A GtkRevealer unrealises its child when folded. session_grid.rs must
        // not do that — WebKit throttles a view it believes hidden and an idle
        // game loses progress — but the sidebar holds only labels, so dropping
        // and rebuilding them on each fold costs nothing (code standards
        // rule 18).
        self.sidebar_revealer.set_child(Some(&self.sidebar));
        self.sidebar_toggle
            .bind_property("active", &*self.sidebar_revealer, "reveal-child")
            .sync_create()
            .build();

        self.wire_sidebar_signals();

        // The parked slot's own Start button routes through the same intent, so
        // the row and the panel never run two starts (architecture rule 8).
        let window = self.obj().downgrade();
        self.grid.connect_start_requested(move |id| {
            if let Some(window) = window.upgrade() {
                window.imp().toggle_parking(&id);
            }
        });

        self.wire_account_dropped();

        // The wheel half of the zoom gesture (task 05): it acts on the account
        // the pointer is over, and only while that is also the focused place —
        // the grid gates it, so the click is what arms the wheel. It lands in
        // the same window method as the keyboard half so the domain is the only
        // thing that decides what a step means (architecture rule 8). A notch up
        // (negative delta) is a step in.
        let window = self.obj().downgrade();
        self.grid.connect_zoom_scrolled(move |id, delta_y| {
            let Some(window) = window.upgrade() else {
                return;
            };
            let step = match delta_y.partial_cmp(&0.0) {
                Some(std::cmp::Ordering::Less) => ZoomStep::In,
                Some(std::cmp::Ordering::Greater) => ZoomStep::Out,
                _ => return,
            };
            window.imp().apply_zoom_step(&id, step);
        });

        let window = self.obj().downgrade();
        self.add_first_game_button.connect_clicked(move |_| {
            if let Some(window) = window.upgrade() {
                window.imp().present_add_game_dialog();
            }
        });

        // `+ Add account`, the sidebar's own leading control (design rule 22)
        // — no longer a header-bar button.
        let window = self.obj().downgrade();
        self.sidebar.connect_add_account_requested(move || {
            if let Some(window) = window.upgrade() {
                window.imp().present_add_game_dialog();
            }
        });

        // Reload the focused view: a game's own page has no chrome, and a login
        // that half-completes needs a way back to a clean load. The button is
        // the visible affordance; F5 / Ctrl+R on a capture-phase controller so a
        // page that binds those keys on its canvas does not swallow them first.
        let window = self.obj().downgrade();
        self.reload_button.connect_clicked(move |_| {
            if let Some(window) = window.upgrade() {
                window.imp().grid.reload_focused();
            }
        });

        // The zoom gesture (item 09) and, now, the two navigation keys (item
        // 14) join this one controller rather than adding a second, so one
        // place decides what a keypress means. Capture phase because
        // `FR.11.1` says neither the reload nor the zoom keys are gated on a
        // web view holding keyboard focus, and `FR.23.3` asks the same of
        // `Shift`+`Tab` and `Ctrl`+`Tab` — a game that binds any of them on
        // its own canvas must not swallow them first.
        let key_controller = gtk::EventControllerKey::new();
        key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        let window = self.obj().downgrade();
        key_controller.connect_key_pressed(move |_, key, _, modifiers| {
            if let Some(window) = window.upgrade()
                && window.imp().handle_shortcut_key(key, modifiers)
            {
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        // GTK 4 exposes no repeat flag on a key event (code standards rule
        // 18): `chord_held` is the substitute, and this release clears it —
        // any release, not one particular keyval, since the chords this
        // guards are spread across `B`, `P`, `S`, the digits and `Tab`, and a
        // held chord delivers no release of its own to race with. The release
        // itself is left to propagate — WebKitGTK re-queues an unhandled
        // press but never sees the one stopped in capture, so the page
        // receives a release with no matching press and ignores it.
        let window = self.obj().downgrade();
        key_controller.connect_key_released(move |_, _key, _, _modifiers| {
            if let Some(window) = window.upgrade() {
                window.imp().chord_held.set(false);
            }
        });
        self.obj().add_controller(key_controller);

        // A dialog, or another application, takes the keyboard from the
        // window and the page loses it with them; nothing redraws when one
        // closes, so the hand-over has to be re-made from the window's own
        // activation rather than waiting for the next focus change
        // (`FR.27.1`, `FR.27.3`).
        self.obj().connect_is_active_notify(|window| {
            window.imp().follow_window_activation();
        });

        self.connect_layout_toggle(&self.layout_single, Layout::Single);
        self.connect_layout_toggle(&self.layout_side_by_side, Layout::SideBySide);
        self.connect_layout_toggle(&self.layout_grid, Layout::Grid);
        self.connect_layout_toggle(&self.layout_mobile, Layout::Mobile);

        self.wire_pager();

        // The one place a save, and now a staged update, are allowed to be
        // waited on: a change made a moment before quitting has nothing else
        // to trigger its write, and `Restart now` needs the swap to have
        // actually happened before the relaunch it asked for (task 06;
        // roadmap item 16 task 07).
        let window = self.obj().downgrade();
        self.obj().connect_close_request(move |_| {
            if let Some(window) = window.upgrade() {
                window.imp().flush_and_apply_on_close();
            }
            glib::Propagation::Proceed
        });

        self.redraw();
    }
}

impl WidgetImpl for Window {
    /// Watches the toplevel surface's own `state` for `MINIMIZED`, the one
    /// place both engines learn the window was minimised or restored
    /// (roadmap item 12 task 04, `FR.1.9`) — only a realized toplevel has a
    /// `GdkSurface` to watch, so this is the first point it can be wired, and
    /// it is wired before any account can have started.
    fn realize(&self) {
        self.parent_realize();

        let Some(surface) = self.obj().surface() else {
            return;
        };
        let Ok(toplevel) = surface.dynamic_cast::<gdk::Toplevel>() else {
            return;
        };

        let window = self.obj().downgrade();
        toplevel.connect_state_notify(move |toplevel| {
            if let Some(window) = window.upgrade() {
                let minimised = toplevel.state().contains(gdk::ToplevelState::MINIMIZED);
                window.imp().apply_minimised(minimised);
            }
        });
    }
}
impl WindowImpl for Window {}
impl ApplicationWindowImpl for Window {}

impl Window {
    pub(super) fn attach_ports(
        &self,
        ports: super::WindowPorts,
        read_outcome: Result<Option<WorkspaceList>, WorkspaceReadError>,
    ) {
        self.locator.replace(Some(ports.locator));
        self.catalogue.replace(Some(ports.catalogue));
        self.zoom_memory.replace(Some(ports.zoom_memory));
        self.removal.replace(Some(ports.removal));
        self.sidebar.start_memory_sampling(ports.probe);
        self.saver
            .replace(Some(Saver::new(ports.store, self.message_strip.clone())));
        if let Some(phone) = ports.phone {
            self.phone_link.replace(Some(phone.link));
            self.spawn_intent_loop(phone.intents);
            arm_debug_enrol(&self.obj());
        }
        // A phone enrolled in an earlier run is enrolled now; the menu must
        // say so before anything else redraws.
        self.refresh_phone_actions();
        self.apply_read_outcome(read_outcome);
        // A first run restores nothing and so redraws nothing; the phone
        // still deserves a truthful, if empty, first snapshot.
        self.publish_state();

        // The running version, read once here rather than through
        // `gio::spawn_blocking`: it is a local, in-memory read (the manager
        // built at start-up already knows it), the same reason
        // `store.read()` above happens synchronously before the window is
        // shown rather than on a worker thread.
        self.running_version
            .set(Some(ports.update.current_version()));
        // A read failure takes the strip over an apply failure: a workspace
        // that would not load is the more urgent of the two, and the strip
        // holds only one line at a time (design rule 9).
        if !self.message_strip.is_visible()
            && let Some(reason) = ports.update.last_apply_failure()
        {
            self.message_strip.show(&as_shown_sentence(&reason));
        }
        self.update_channel.replace(Some(ports.update));
        // The launch-time check (`**Entry**`): runs once here, now that the
        // channel exists, rather than from `constructed` — the ports are not
        // attached yet at that point (`saver` and `phone_link` wait the same
        // way).
        self.drive_update(UpdateEvent::CheckRequested { manual: false });
    }

    /// Flushes any pending workspace save and, if an update passed its
    /// checks while the window was open, arranges for it to install once
    /// this process exits — the window's close handler, run whichever way
    /// the window closes: `Restart now`'s own `close()` or an ordinary quit
    /// (task 06; roadmap item 16 task 07). A failed swap is logged and the
    /// close proceeds regardless (code standards rule 14) — the next launch
    /// names it in the message strip instead (`last_apply_failure`).
    fn flush_and_apply_on_close(&self) {
        if let Some(saver) = self.saver.borrow().as_ref() {
            saver.flush();
        }
        if let Some(package) = self.verified_package.borrow().as_ref()
            && let Some(channel) = self.update_channel.borrow().as_ref()
            && let Err(error) = channel.apply_on_exit(package, self.relaunch_after_update.get())
        {
            tracing::error!(%error, "the staged update could not be applied on exit");
        }
    }

    /// Places the update notice directly beneath the message strip, so the
    /// two stack in that order whenever both have something to say (design
    /// rule 9's own strip first, this bar's new pattern beneath it), wires
    /// its three presses, and arms the daily-check schedule.
    fn install_update_notice(&self) {
        self.root_box
            .insert_child_after(&self.update_notice, Some(&self.message_strip));
        self.wire_update_notice();
        self.arm_update_schedule();
    }

    /// Wires the notice's three presses straight to [`Window::drive_update`]
    /// (architecture rule 8): the window decides nothing about updates, only
    /// forwards the press and redraws from whatever the policy answers.
    fn wire_update_notice(&self) {
        let window = self.obj().downgrade();
        self.update_notice.connect_fetch_requested(move || {
            if let Some(window) = window.upgrade() {
                window.imp().drive_update(UpdateEvent::FetchRequested);
            }
        });

        let window = self.obj().downgrade();
        self.update_notice.connect_restart_requested(move || {
            if let Some(window) = window.upgrade() {
                // The normal close, so the arrangement is saved first
                // (`connect_close_request`); the swap and the relaunch only
                // happen once the process has actually exited.
                window.imp().relaunch_after_update.set(true);
                window.close();
            }
        });

        let window = self.obj().downgrade();
        self.update_notice.connect_dismissed(move || {
            if let Some(window) = window.upgrade() {
                window.imp().drive_update(UpdateEvent::Dismissed);
            }
        });
    }

    /// Ticks once a minute for the rest of the window's life, running a
    /// fresh automatic check whenever [`UpdateSchedule::next_check_due`]
    /// says the daily one is due. Harmless before the channel is attached —
    /// [`Window::drive_update`] answers no effect without one.
    fn arm_update_schedule(&self) {
        let window = self.obj().downgrade();
        glib::timeout_add_local(Duration::from_secs(UPDATE_SCHEDULE_TICK_SECS), move || {
            let Some(window) = window.upgrade() else {
                return glib::ControlFlow::Break;
            };
            let imp = window.imp();
            if UpdateSchedule::next_check_due(imp.last_update_check_millis.get(), now_millis()) {
                imp.drive_update(UpdateEvent::CheckRequested { manual: false });
            }
            glib::ControlFlow::Continue
        });
    }

    /// Applies `event` to the update policy, redraws the notice from
    /// whatever state comes back, and runs the one effect the policy
    /// answers, if any — every network call and file check on a worker
    /// thread, coming back to the GTK thread with a result (architecture
    /// rule 10). The one place the shell decides anything about updates is
    /// this dispatch; the policy decides what the event means.
    fn drive_update(&self, event: UpdateEvent) {
        let previous = self.update_state.borrow().clone();
        let (next_state, effect) = self.update_policy.borrow_mut().apply(previous, event);

        // `UpdateState` carries `notes_url` only on `Available` — kept here
        // so a later download or a retry still has it once the state moves
        // on (see `update_info`'s own doc comment).
        if let UpdateState::Available {
            version,
            ref notes_url,
        } = next_state
        {
            self.update_info.replace(Some(UpdateInfo {
                version,
                notes_url: notes_url.clone(),
            }));
        }

        self.update_state.replace(next_state.clone());
        self.update_notice.render(&next_state);

        let Some(channel) = self.update_channel.borrow().clone() else {
            return;
        };
        match effect {
            Some(Effect::RunCheck) => self.run_update_check(channel),
            Some(Effect::Download) => self.start_update_download(channel),
            None => {}
        }
    }

    /// Runs [`UpdateChannel::check`] on a worker thread and turns its answer
    /// into the matching [`UpdateEvent`] back on the GTK thread. An
    /// automatic failure is logged, never shown (`**States**`'s "one warning
    /// line in the log"); a manual one always answers on screen because
    /// [`UpdatePolicy::apply`] carries `manual` through to `CheckFailed`.
    fn run_update_check(&self, channel: Arc<dyn UpdateChannel>) {
        let window = self.obj().downgrade();
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(move || channel.check()).await;
            let Some(window) = window.upgrade() else {
                return;
            };
            let imp = window.imp();
            imp.last_update_check_millis.set(Some(now_millis()));
            match result {
                Ok(Ok(UpdateCheck::Available(info))) => {
                    imp.drive_update(UpdateEvent::Found {
                        version: info.version,
                        notes_url: info.notes_url,
                    });
                }
                Ok(Ok(UpdateCheck::UpToDate)) => {
                    let current = imp.running_version.get().unwrap_or(Version::new(0, 0, 0));
                    imp.drive_update(UpdateEvent::NothingNewer { current });
                }
                Ok(Err(error)) => {
                    tracing::warn!(%error, "the update check failed");
                    imp.drive_update(UpdateEvent::CheckFailed {
                        reason: describe_check_error(&error),
                    });
                }
                Err(_) => {
                    tracing::error!("the update check task panicked");
                    imp.drive_update(UpdateEvent::CheckFailed {
                        reason: describe_check_error(&UpdateError::Offline {
                            reason: "the check task panicked".to_string(),
                        }),
                    });
                }
            }
        });
    }

    /// Downloads the cached [`Window::update_info`] on a worker thread,
    /// forwarding every percentage back to the GTK thread as it arrives
    /// (architecture rule 10), then turns the final answer into `Verified`
    /// or `Rejected`. A no-op, logging an error, if nothing is cached — not
    /// expected, since the policy only answers [`Effect::Download`] from
    /// `Available` or a named `Failed`, both of which cache it first.
    fn start_update_download(&self, channel: Arc<dyn UpdateChannel>) {
        let Some(info) = self.update_info.borrow().clone() else {
            tracing::error!("a download was requested with no update info cached");
            return;
        };

        let (progress_tx, progress_rx) = async_channel::unbounded::<u8>();

        let window = self.obj().downgrade();
        glib::spawn_future_local(async move {
            while let Ok(percent) = progress_rx.recv().await {
                let Some(window) = window.upgrade() else {
                    break;
                };
                window.imp().drive_update(UpdateEvent::Progress(percent));
            }
        });

        let window = self.obj().downgrade();
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(move || {
                let progress = move |percent: u8| {
                    // The receiving end closes once this whole closure
                    // returns; a send past that point is simply the last,
                    // unread percentage and not an error.
                    let _ = progress_tx.send_blocking(percent);
                };
                channel.download(&info, &progress)
            })
            .await;

            let Some(window) = window.upgrade() else {
                return;
            };
            let imp = window.imp();
            match result {
                Ok(Ok(package)) => {
                    imp.verified_package.replace(Some(package));
                    imp.drive_update(UpdateEvent::Verified);
                }
                Ok(Err(error)) => {
                    imp.drive_update(UpdateEvent::Rejected {
                        reason: describe_download_error(&error),
                    });
                }
                Err(_) => {
                    tracing::error!("the update download task panicked");
                    imp.drive_update(UpdateEvent::Rejected {
                        reason: describe_download_error(&UpdateError::Io {
                            reason: "the download task panicked".to_string(),
                        }),
                    });
                }
            }
        });
    }

    /// Loops over the phone's intents on the GTK main context, one
    /// [`Window::apply_remote_intent`] per message (architecture rule 10).
    /// Ends when the channel closes — the sender dropped, which is how a
    /// server that never started hands over nothing — or the window is gone.
    fn spawn_intent_loop(&self, intents: async_channel::Receiver<RemoteIntent>) {
        let window = self.obj().downgrade();
        glib::spawn_future_local(async move {
            while let Ok(intent) = intents.recv().await {
                let Some(window) = window.upgrade() else {
                    break;
                };
                window.imp().apply_remote_intent(intent);
            }
            tracing::debug!("the phone's intent channel closed; no more remote intents");
        });
    }

    /// Turns one message from the phone into the same call a click on the
    /// desktop makes, and nothing else (architecture rule 8): the closed set
    /// of [`RemoteIntent`] is the whole of what a phone may do (`FR.6.3`).
    /// `Park` and `Start` apply only where the row menu's item would; a tap
    /// or a scroll reaches only the current account's live page, and only
    /// while mobile mode is on, since that is the page the phone is looking
    /// at.
    fn apply_remote_intent(&self, intent: RemoteIntent) {
        tracing::debug!(?intent, "remote intent received");
        match intent {
            RemoteIntent::Attach { viewport } => self.attach_phone(viewport),
            RemoteIntent::Leave => self.detach_phone(),
            RemoteIntent::ChooseAccount(id) => self.focus_session(&id),
            RemoteIntent::Park(id) => {
                let liveness = liveness_anywhere(&self.book.borrow(), &id);
                if park_applies(liveness) {
                    self.park_session(&id);
                    self.request_save();
                }
            }
            RemoteIntent::Start(id) => {
                let liveness = liveness_anywhere(&self.book.borrow(), &id);
                if start_applies(liveness) {
                    self.start_session(&id);
                    self.request_save();
                }
            }
            RemoteIntent::SetMobileMode(true) => {
                let already_on = self.book.borrow().is_mobile_mode();
                if !already_on {
                    let viewport = self.phone_viewport.get().unwrap_or(DEFAULT_MOBILE_VIEWPORT);
                    self.enter_mobile_mode(viewport);
                }
            }
            RemoteIntent::SetMobileMode(false) => {
                let on = self.book.borrow().is_mobile_mode();
                if on {
                    self.leave_mobile_mode(None);
                }
            }
            RemoteIntent::Tap { x, y } => {
                let (x, y) = self.frame_scale().point(x, y);
                self.run_in_current_page(&tap_script(x, y));
            }
            RemoteIntent::Scroll { x, y, dx, dy } => {
                let scale = self.frame_scale();
                let (x, y) = scale.point(x, y);
                let (dx, dy) = scale.scroll_delta(dx, dy);
                self.run_in_current_page(&scroll_script(x, y, dx, dy));
            }
        }
    }

    /// The phone started watching with a screen of `viewport`: the one
    /// mobile slot takes that shape when the mode is on (`FR.3.2`), the
    /// current account's view is told it is watched, and the frame timer
    /// starts. The redraw at the end reallocates the slot and publishes the
    /// new viewport.
    fn attach_phone(&self, viewport: Viewport) {
        self.phone_attached.set(true);
        self.phone_viewport.set(Some(viewport));
        {
            let mut book = self.book.borrow_mut();
            if book.is_mobile_mode() && book.mobile_viewport() != Some(viewport) {
                book.set_mobile_viewport(viewport);
            }
        }
        tracing::info!(
            width = viewport.width,
            height = viewport.height,
            "the phone is watching"
        );
        self.sync_watched();
        self.start_capture_timer();
        self.redraw();
    }

    /// The phone stopped watching: no more frames, and the watched view goes
    /// back to the behaviour `background_for` dictates for it (`FR.4.3`).
    /// The current account is left as it is, so coming back resumes there.
    fn detach_phone(&self) {
        self.stop_capture_timer();
        self.phone_attached.set(false);
        self.sync_watched();
        tracing::info!("the phone stopped watching");
    }

    /// The account a phone is looking at right now: the current one while a
    /// phone is attached, none otherwise. [`Window::sync_watched`] tells
    /// that view; [`Window::start_session`] builds a view for it armed from
    /// the start.
    fn watched_account(&self) -> Option<SessionId> {
        if self.phone_attached.get() {
            current_account(&self.book.borrow()).map(|(id, _)| id)
        } else {
            None
        }
    }

    /// Whether a phone is looking at `id` right now.
    fn phone_watches(&self, id: &SessionId) -> bool {
        self.watched_account().as_ref() == Some(id)
    }

    /// Makes the current account's view the one watched view while the phone
    /// is attached, and no view watched otherwise: whichever view was told
    /// before and is not wanted now is un-told first (`FR.4.3`). Records
    /// only a view that exists, so an account parked at the time is told
    /// when its view is next built ([`Window::finish_starting`]).
    fn sync_watched(&self) {
        let wanted = self.watched_account();
        let previous = self.watched.borrow().clone();
        if previous == wanted {
            return;
        }

        let minimised = self.minimised.get();
        let holders = self.holders.borrow();
        if let Some(id) = &previous
            && let Some(holder) = holders.get(id)
        {
            holder.set_watched(false, minimised);
        }
        let applied = wanted.filter(|id| {
            holders
                .get(id)
                .is_some_and(|holder| holder.view().is_some())
        });
        if let Some(id) = &applied
            && let Some(holder) = holders.get(id)
        {
            holder.set_watched(true, minimised);
        }
        drop(holders);
        self.watched.replace(applied);
    }

    /// Runs `source` in the current account's live page — the phone's tap or
    /// scroll. Ignored while mobile mode is off, since the phone is then not
    /// looking at any page, and when the current account has no view.
    fn run_in_current_page(&self, source: &str) {
        let (mobile_mode, current) = {
            let book = self.book.borrow();
            (book.is_mobile_mode(), current_account(&book))
        };
        if !mobile_mode {
            tracing::debug!("phone gesture ignored; mobile mode is off");
            return;
        }
        let Some((id, _)) = current else {
            return;
        };
        if let Some(holder) = self.holders.borrow().get(&id) {
            tracing::debug!(session = %id, "phone gesture delivered to the page");
            holder.run_script(source);
        }
    }

    /// The mapping from the last frame's pixel grid, where the phone's
    /// gestures are measured, to the page's CSS pixels.
    fn frame_scale(&self) -> FrameScale {
        let viewport = self
            .book
            .borrow()
            .mobile_viewport()
            .unwrap_or(DEFAULT_MOBILE_VIEWPORT);
        FrameScale::between(self.frame_width.get(), viewport.width)
    }

    /// Arms the frame timer if it is not already running: every
    /// [`FRAME_INTERVAL_MILLIS`] one [`Window::capture_tick`].
    fn start_capture_timer(&self) {
        if self.capture_timer.borrow().is_some() {
            return;
        }
        let window = self.obj().downgrade();
        let timer =
            glib::timeout_add_local(Duration::from_millis(FRAME_INTERVAL_MILLIS), move || {
                let Some(window) = window.upgrade() else {
                    return glib::ControlFlow::Break;
                };
                window.imp().capture_tick();
                glib::ControlFlow::Continue
            });
        self.capture_timer.replace(Some(timer));
    }

    fn stop_capture_timer(&self) {
        if let Some(timer) = self.capture_timer.borrow_mut().take() {
            timer.remove();
        }
    }

    /// One tick of the frame timer: when [`capture_gate`] allows, asks the
    /// current account's view for a picture and hands the answer to
    /// [`Window::finish_capture`]. Only that one view is ever photographed
    /// (`FR.4.2`).
    fn capture_tick(&self) {
        let (mobile_mode, current) = {
            let book = self.book.borrow();
            (book.is_mobile_mode(), current_account(&book))
        };
        if self.capture_in_flight.get()
            && let Some(asked) = self.capture_asked_at.get()
            && asked.elapsed() >= Duration::from_secs(CAPTURE_STALL_SECS)
        {
            // The engine owes an answer it may never give — a web process
            // that stopped, or a page the compositor no longer drives. The
            // stall is named once and a new capture asked for; an answer
            // that arrives late still publishes, just out of order.
            tracing::warn!(
                stalled_secs = asked.elapsed().as_secs(),
                "a frame capture has not answered; asking again"
            );
            self.capture_in_flight.set(false);
        }
        if !capture_gate(
            self.phone_attached.get(),
            mobile_mode,
            current.as_ref().map(|(_, liveness)| *liveness),
            self.capture_in_flight.get(),
        ) {
            return;
        }
        let Some((id, _)) = current else {
            return;
        };
        let holders = self.holders.borrow();
        let Some(holder) = holders.get(&id) else {
            return;
        };

        self.capture_in_flight.set(true);
        self.capture_asked_at.set(Some(std::time::Instant::now()));
        let window = self.obj().downgrade();
        holder.capture_frame(move |outcome| {
            if let Some(window) = window.upgrade() {
                window.imp().finish_capture(outcome);
            }
        });
    }

    /// A capture answered: a picture goes to the phone through the link,
    /// which encodes and forwards it off the main context; a failure is
    /// logged once per distinct reason, and the next tick tries again.
    fn finish_capture(&self, outcome: Result<CapturedFrame, EngineCaptureError>) {
        self.capture_in_flight.set(false);
        self.capture_asked_at.set(None);
        match outcome {
            Ok(captured) => {
                let frame = frame_for_phone(captured);
                if let Some(width) = frame_width(&frame) {
                    self.frame_width.set(Some(width));
                }
                self.capture_failure.replace(None);
                if let Some(link) = self.phone_link.borrow().as_ref() {
                    link.publish_frame(frame);
                }
            }
            Err(error) => {
                let reason = error.to_string();
                if self.capture_failure.borrow().as_deref() != Some(reason.as_str()) {
                    tracing::warn!(%reason, "frame capture failed; frames pause until it succeeds");
                    self.capture_failure.replace(Some(reason));
                }
            }
        }
    }

    /// Sends the phone the book as it stands, so every change the sidebar
    /// sees, the phone sees (`FR.2.1`). A no-op before a link is attached.
    fn publish_state(&self) {
        if let Some(link) = self.phone_link.borrow().as_ref() {
            link.publish_state(&RemoteState::from_book(&self.book.borrow()));
        }
    }

    /// Asks for the workspace to be saved after an action changed it. Cheap —
    /// it only rearms a timer — so a caller in doubt asks (task 06). A no-op
    /// before the ports are attached.
    fn request_save(&self) {
        if let Some(saver) = self.saver.borrow().as_ref() {
            saver.request(self.book.borrow().saved());
        }
    }

    /// Builds the one [`StartQueue`] this window keeps for its whole life,
    /// called once from `constructed`. Kept as its own method, like
    /// `register_phone_actions`, to keep `constructed` under clippy's line
    /// budget.
    fn init_start_queue(&self) {
        if self
            .start_queue
            .set(StartQueue::new(self.obj().downgrade()))
            .is_err()
        {
            tracing::error!("the start queue was constructed twice");
        }
    }

    /// The one [`StartQueue`] this window keeps for its whole life.
    ///
    /// # Panics
    ///
    /// Never: [`Window::constructed`] sets it before any other method can run.
    fn start_queue(&self) -> &StartQueue {
        self.start_queue
            .get()
            .expect("start_queue set in constructed")
    }

    /// Acts on what the composition root read before the window was built: a
    /// saved workspace list to restore, no file (a first run), or a failure. A
    /// failure is never flattened into "no file" — it is logged in fields and
    /// shown in the strip, never dropped (code standards rules 14, 15).
    fn apply_read_outcome(&self, outcome: Result<Option<WorkspaceList>, WorkspaceReadError>) {
        match outcome {
            Ok(None) => {
                tracing::info!("no saved workspace; opening a first run");
            }
            Ok(Some(workspaces)) => self.restore_workspace(workspaces),
            Err(error) => {
                tracing::warn!(error = %error, "the saved workspace could not be read");
                self.message_strip.show(&describe_read_error(&error));
            }
        }
    }

    /// Rebuilds the book from `workspaces`, prepares every account's profile
    /// directories in every workspace — not only the one shown — hands the
    /// grid a dormant holder at each saved placement, and redraws once — list,
    /// arrangement and every slot — before anything loads. An account whose
    /// profile cannot be prepared is logged and skipped, not the whole restore
    /// abandoned. The start queue (task 05) brings the running accounts up one
    /// at a time afterwards, the shown workspace's first (`FR.18.4`).
    fn restore_workspace(&self, workspaces: WorkspaceList) {
        let Some(locator) = self.locator.borrow().clone() else {
            tracing::error!("no profile locator attached; cannot restore the workspace");
            return;
        };

        *self.book.borrow_mut() = WorkspaceBook::restore(workspaces);

        // Install every account's chosen sizes onto the book before any holder
        // is built, so a relaunched account is right before it is ever drawn
        // and the start queue does not have to correct it (`FR.12.7`). Every
        // workspace, not only the one shown — a hidden workspace's accounts
        // still run and must open at the right size the first time they are
        // shown.
        if let Some(memory) = self.zoom_memory.borrow().clone() {
            let ids: Vec<SessionId> = self
                .book
                .borrow()
                .workspaces()
                .flat_map(|workspace| workspace.book().sessions())
                .map(|session| session.id().clone())
                .collect();
            let mut book = self.book.borrow_mut();
            for id in ids {
                let remembered = memory.read(&id);
                if let Some(session_book) = book
                    .workspaces_mut()
                    .find(|session_book| session_book.session(&id).is_some())
                {
                    session_book.restore_zoom(&id, remembered);
                }
            }
        }

        // Each dormant holder opens at the size resolved for its own
        // workspace's arrangement, not the game-file baseline (`FR.12.2`) —
        // every workspace may have a different layout, so each account is
        // resolved against the one it actually belongs to.
        let accounts: Vec<(SessionId, String, String, ZoomLevel, Option<String>, bool)> = self
            .book
            .borrow()
            .workspaces()
            .flat_map(|workspace| {
                let layout = workspace.book().layout();
                workspace
                    .book()
                    .sessions()
                    .iter()
                    .map(move |session| {
                        (
                            session.id().clone(),
                            session.display_name().to_owned(),
                            session.start_address().to_owned(),
                            session.zoom_for(layout),
                            session.browser_identity().map(str::to_owned),
                            session.is_webgl_enabled(),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        for (id, name, address, zoom, identity, webgl_enabled) in accounts {
            let directories = match locator.locate(&id) {
                Ok(directories) => directories,
                Err(error) => {
                    tracing::error!(session = %id, %error, "could not prepare the profile directories; skipping this account");
                    continue;
                }
            };

            let holder = SessionView::dormant(
                &id,
                &directories,
                &address,
                AccountSettings {
                    zoom,
                    identity,
                    webgl_enabled,
                },
            );
            self.grid.add_dormant_session(&id, &name);
            self.holders.borrow_mut().insert(id, holder);
        }

        self.redraw();

        // The header's layout toggles start on "1" from the template; the
        // shown workspace may be arranged for another layout, and "that
        // arrangement selected" is part of a truthful restore. Done after
        // `redraw` and outside any `book` borrow, since flipping a toggle runs
        // its handler synchronously.
        let layout = self.book.borrow().active().layout();
        self.select_layout_toggle(layout);

        tracing::info!(accounts = self.holders.borrow().len(), "workspace restored");

        // The arrangement is drawn; now bring the running accounts back one at
        // a time (task 05), the shown workspace's first. Nothing queued means
        // nothing to do.
        let order = self.book.borrow().start_order();
        self.start_queue().enqueue(order);
    }

    /// Sets the header's layout toggle group to `layout` without the user
    /// touching it — for a restore, where the book's layout is set directly,
    /// and for a workspace switch into or out of mobile mode, where the book
    /// already holds `Mobile` and the `Phone` toggle has to follow.
    fn select_layout_toggle(&self, layout: Layout) {
        let toggle = match layout {
            Layout::Single => &self.layout_single,
            Layout::SideBySide => &self.layout_side_by_side,
            Layout::Grid => &self.layout_grid,
            Layout::Mobile => &self.layout_mobile,
        };
        toggle.set_active(true);
    }

    /// A layout toggle was pressed: the book is asked for `layout` through
    /// [`Window::choose_layout`].
    fn connect_layout_toggle(&self, toggle: &gtk::ToggleButton, layout: Layout) {
        let window = self.obj().downgrade();
        toggle.connect_toggled(move |toggle| {
            if !toggle.is_active() {
                return;
            }
            if let Some(window) = window.upgrade() {
                window.imp().choose_layout(layout);
            }
        });
    }

    /// Arranges the shown workspace for `layout`. Returns early when it
    /// already names the shown workspace's layout — reached after
    /// [`Window::focus_session`] flips the toggle to match an incoming
    /// workspace, so that never triggers a second arrangement or a second
    /// save (`FR.16.4`) — and after [`Window::enter_mobile_mode`] or
    /// [`Window::leave_mobile_mode`] flip it for a phone's request, for the
    /// same reason. `Mobile` enters mobile mode at the attached phone's own
    /// screen size, or the default when no phone is watching; `1`, `2` or
    /// `4` while the mode is on leaves it first, which puts the arrangement
    /// from before back, then switches only if the chosen layout differs from
    /// the restored one (Remote Access `FR.3.5`).
    fn choose_layout(&self, layout: Layout) {
        if self.book.borrow().active().layout() == layout {
            return;
        }
        match layout {
            Layout::Mobile => {
                let viewport = match self.phone_viewport.get() {
                    Some(viewport) if self.phone_attached.get() => viewport,
                    _ => DEFAULT_MOBILE_VIEWPORT,
                };
                self.enter_mobile_mode(viewport);
            }
            Layout::Single | Layout::SideBySide | Layout::Grid => {
                self.leave_mobile_mode(Some(layout));
            }
        }
    }

    /// Switches mobile mode on with the one slot shaped as `viewport` —
    /// pressed on the desktop or asked for by the phone, one path (`FR.3.1`).
    /// The `Phone` toggle follows the book; when the toggle itself was what
    /// was pressed, selecting it again is a no-op.
    fn enter_mobile_mode(&self, viewport: Viewport) {
        self.book.borrow_mut().enter_mobile_mode(viewport);
        self.select_layout_toggle(Layout::Mobile);
        self.finish_layout_change();
    }

    /// Switches mobile mode off, putting back the arrangement from before,
    /// then switches to `then` if it names a layout other than the restored
    /// one (`FR.3.5`). The toggles follow the book, and the handler they run
    /// finds nothing left to change.
    fn leave_mobile_mode(&self, then: Option<Layout>) {
        {
            let mut book = self.book.borrow_mut();
            book.leave_mobile_mode();
            if let Some(layout) = then
                && book.active().layout() != layout
            {
                book.set_layout(layout);
            }
        }
        let layout = self.book.borrow().active().layout();
        self.select_layout_toggle(layout);
        self.finish_layout_change();
    }

    /// What every arrangement change ends with: the grid redrawn, every
    /// account snapped to its size for the layout now in force, and a save
    /// requested.
    fn finish_layout_change(&self) {
        self.redraw();
        self.snap_zoom_for_active();
        self.request_save();
    }

    /// After an arrangement switch, redraw every account in the **shown**
    /// workspace at the size it last chose for the arrangement now in force,
    /// falling back to the game file's size. No readout is shown — nobody
    /// asked for a size change, the arrangement did (`FR.11.6`). Every
    /// account with a holder, not only the visible ones: an off-grid or
    /// parked account is then already the right size the moment it is next
    /// brought into a place, with no second code path and no visible
    /// correction (`FR.11.7`). Nothing is written — a switch consumes chosen
    /// sizes and never records one (`FR.12.4`).
    fn snap_zoom_for_active(&self) {
        let book = self.book.borrow();
        let layout = book.active().layout();
        let resolved: Vec<(SessionId, ZoomLevel)> = book
            .active()
            .sessions()
            .iter()
            .map(|session| (session.id().clone(), session.zoom_for(layout)))
            .collect();
        drop(book);

        let mut holders = self.holders.borrow_mut();
        for (id, zoom) in resolved {
            if let Some(holder) = holders.get_mut(&id) {
                holder.set_zoom(zoom);
            }
        }
    }

    /// Opens the add-game dialog, its `Workspace` field filled from
    /// [`WorkspaceBook::destinations`] for one more account, defaulting to
    /// the shown workspace when it has room and to `Ungrouped` otherwise
    /// (`FR.17.5`).
    fn present_add_game_dialog(&self) {
        let Some(catalogue) = self.catalogue.borrow().clone() else {
            tracing::error!("no preset catalogue attached; cannot open the add-game dialog");
            return;
        };

        let book = self.book.borrow();
        let destinations = book.destinations(1);
        let shown = book.active_id().clone();
        drop(book);
        let default = if destinations
            .workspaces()
            .iter()
            .any(|workspace| workspace.id() == &shown)
        {
            shown
        } else {
            WorkspaceId::ungrouped()
        };

        let dialog = AddGameDialog::new(catalogue.as_ref(), &destinations, &default);
        dialog.set_transient_for(Some(&*self.obj()));

        let window = self.obj().downgrade();
        dialog.connect_confirmed(move |confirmed| {
            let Some(window) = window.upgrade() else {
                return;
            };
            // The dialog reports what the user chose; the book decides what it
            // means (architecture rule 8).
            match confirmed {
                Confirmed::Preset {
                    preset,
                    account_name,
                    workspace,
                } => window
                    .imp()
                    .create_account_from_preset(preset, account_name, workspace),
                Confirmed::Custom {
                    name,
                    address,
                    workspace,
                } => window.imp().create_account(name, address, workspace),
            }
        });

        dialog.present();
    }

    /// Opens the rename dialog for `id`, transient for the main window and
    /// pre-filled with its current name (`present_add_game_dialog`'s pattern).
    /// A no-op, logged, if the book has no account under `id`.
    fn present_rename_dialog(&self, id: &SessionId) {
        let Some(current_name) = self
            .book
            .borrow()
            .active()
            .session(id)
            .map(|session| session.display_name().to_owned())
        else {
            tracing::error!(session = %id, "no account to rename");
            return;
        };

        // `account_name`'s rule never answers `Taken` — two accounts may
        // share a name (`SessionBook::add` already allows it).
        let dialog =
            RenameDialog::new(
                "Rename account",
                "Rename",
                &current_name,
                |text| match account_name(text) {
                    Some(_) => NameCheck::Ok,
                    None => NameCheck::Empty,
                },
            );
        dialog.set_transient_for(Some(&*self.obj()));

        let window = self.obj().downgrade();
        let id = id.clone();
        dialog.connect_confirmed(move |name| {
            let Some(window) = window.upgrade() else {
                return;
            };
            window.imp().rename_account(&id, name);
        });

        dialog.present();
    }

    /// Applies a confirmed rename. Only when the book actually stores the
    /// name — never for an unknown id or one that trims to empty — does it
    /// redraw and save; it never touches a `SessionView` holder, so nothing
    /// reloads, a running game keeps running and a parked one stays parked
    /// (architecture rule 8).
    fn rename_account(&self, id: &SessionId, name: &str) {
        let renamed = self.book.borrow_mut().rename(id, name);
        if renamed {
            self.redraw();
            self.request_save();
        }
    }

    /// Opens the shared name window for `purpose`: creating a workspace from
    /// the ticked accounts, or renaming one that exists —
    /// `present_rename_dialog`'s pattern, with the name check run against
    /// [`WorkspaceBook::name_conflict`] on every keystroke so the confirm
    /// button and the dim "taken" line always track the book's own rule
    /// (`FR.15.8`). A no-op, logged, if `purpose` names a workspace the book
    /// no longer holds.
    fn present_workspace_name_dialog(&self, purpose: NameDialogPurpose) {
        let title = match &purpose {
            NameDialogPurpose::Create(_) => "New workspace",
            NameDialogPurpose::Rename(_) => "Rename workspace",
        };
        let confirm_label = match &purpose {
            NameDialogPurpose::Create(_) => "Create",
            NameDialogPurpose::Rename(_) => "Rename",
        };
        let current_name = match &purpose {
            NameDialogPurpose::Create(_) => String::new(),
            NameDialogPurpose::Rename(id) => {
                let Some(name) = self
                    .book
                    .borrow()
                    .workspaces()
                    .find(|workspace| workspace.id() == id)
                    .map(|workspace| workspace.name().to_owned())
                else {
                    tracing::error!(workspace = %id, "no workspace to rename");
                    return;
                };
                name
            }
        };
        let excluding = match &purpose {
            NameDialogPurpose::Create(_) => None,
            NameDialogPurpose::Rename(id) => Some(id.clone()),
        };

        let window = self.obj().downgrade();
        let check = move |text: &str| {
            let Some(window) = window.upgrade() else {
                return NameCheck::Empty;
            };
            let Some(trimmed) = workspace_name(text) else {
                return NameCheck::Empty;
            };
            if window
                .imp()
                .book
                .borrow()
                .name_conflict(&trimmed, excluding.as_ref())
            {
                NameCheck::Taken
            } else {
                NameCheck::Ok
            }
        };

        let dialog = RenameDialog::new(title, confirm_label, &current_name, check);
        dialog.set_transient_for(Some(&*self.obj()));

        let window = self.obj().downgrade();
        dialog.connect_confirmed(move |name| {
            let Some(window) = window.upgrade() else {
                return;
            };
            match &purpose {
                NameDialogPurpose::Create(ids) => {
                    window.imp().create_workspace_from_ticked(name, ids);
                }
                NameDialogPurpose::Rename(id) => {
                    window.imp().apply_workspace_rename(id, name);
                }
            }
        });

        dialog.present();
    }

    /// A "New workspace" window confirmed `name` for `ids` — a single
    /// account, moved there from its row's `Move to ▸` submenu (design
    /// rule 23). Only on success does it redraw and save; a refusal is
    /// logged and changes nothing — the dialog's own name check and
    /// `Destinations::can_create` mean this is not expected in practice (code
    /// standards rule 1).
    fn create_workspace_from_ticked(&self, name: &str, ids: &[SessionId]) {
        // See `move_account`'s comment: bound to a `let` so the `RefMut`
        // temporary drops before `redraw` below borrows the book again.
        let result = self.book.borrow_mut().create_workspace(name, ids);
        match result {
            Ok(_) => {
                self.redraw();
                self.request_save();
            }
            Err(refusal) => {
                tracing::error!(?refusal, "could not create the workspace");
            }
        }
    }

    /// A "Rename workspace" window confirmed `name` for `id`. Only on success
    /// does it redraw and save.
    fn apply_workspace_rename(&self, id: &WorkspaceId, name: &str) {
        let result = self.book.borrow_mut().rename_workspace(id, name);
        match result {
            Ok(()) => {
                self.redraw();
                self.request_save();
            }
            Err(refusal) => {
                tracing::error!(?refusal, workspace = %id, "could not rename the workspace");
            }
        }
    }

    /// `Remove workspace` was chosen for `id`. On success, when `id` was the
    /// shown workspace, runs what a switch runs — `select_layout_toggle` for
    /// `Ungrouped`'s layout and `snap_zoom_for_active` — then always redraws
    /// and saves. Never touches a `SessionView` holder or the disk: the
    /// accounts keep running exactly as they were, just regrouped under
    /// `Ungrouped` (`FR.15.4`, `FR.15.11`, `FR.21.8`).
    fn remove_workspace(&self, id: &WorkspaceId) {
        let was_active = self.book.borrow().active_id() == id;
        // See `move_ticked`'s comment: bound to a `let` so the `RefMut`
        // temporary drops before `select_layout_toggle`/`redraw` below borrow
        // the book again.
        let result = self.book.borrow_mut().remove_workspace(id);
        match result {
            Ok(()) => {
                if was_active {
                    let layout = self.book.borrow().active().layout();
                    self.select_layout_toggle(layout);
                    self.snap_zoom_for_active();
                }
                self.redraw();
                self.request_save();
            }
            Err(refusal) => {
                tracing::error!(?refusal, workspace = %id, "could not remove the workspace");
            }
        }
    }

    /// Opens the delete-account confirmation for `id`, transient for the main
    /// window (`present_rename_dialog`'s pattern). Searched across every
    /// workspace, since deletion is offered for an account in any of them
    /// (`FR.21.1`). A no-op, logged, if the book has no account under `id`.
    fn present_delete_dialog(&self, id: &SessionId) {
        let Some(name) = self.book.borrow().workspaces().find_map(|workspace| {
            workspace
                .book()
                .session(id)
                .map(|session| session.display_name().to_owned())
        }) else {
            tracing::error!(session = %id, "no account to delete");
            return;
        };

        let dialog = DeleteAccountDialog::new(&name);
        dialog.set_transient_for(Some(&*self.obj()));

        let window = self.obj().downgrade();
        let confirm_id = id.clone();
        let confirm_dialog = dialog.clone();
        dialog.connect_confirmed(move || {
            if let Some(window) = window.upgrade() {
                window
                    .imp()
                    .run_account_deletion(&confirm_id, confirm_dialog.clone());
            }
        });

        let window = self.obj().downgrade();
        let retry_id = id.clone();
        let retry_dialog = dialog.clone();
        dialog.connect_retry(move || {
            if let Some(window) = window.upgrade() {
                window
                    .imp()
                    .run_account_deletion(&retry_id, retry_dialog.clone());
            }
        });

        let window = self.obj().downgrade();
        let close_id = id.clone();
        dialog.connect_closed_after_failure(move || {
            if let Some(window) = window.upgrade() {
                window.imp().restore_after_failed_delete(&close_id);
            }
        });

        dialog.present();
    }

    /// Runs (or re-runs, for `Retry`) the fixed deletion sequence for `id`
    /// (`FR.21.10`), reporting progress and outcome through `dialog`. Parks
    /// the account first — idempotent, so a retry costs nothing extra — then
    /// hands whichever holder `id` still has, if any, to
    /// [`account_deletion::delete_account`], which skips the steps that need
    /// one when an earlier attempt already ran them. Only on success does it
    /// forget the account everywhere; a failure shows the dialog's `failed`
    /// page with the reason and the account's own folder path. A no-op,
    /// logged, if the removal port was never attached.
    fn run_account_deletion(&self, id: &SessionId, dialog: DeleteAccountDialog) {
        let Some(removal) = self.removal.borrow().clone() else {
            tracing::error!("no profile removal port attached; cannot delete an account");
            return;
        };

        let name = self.book.borrow().workspaces().find_map(|workspace| {
            workspace
                .book()
                .session(id)
                .map(|session| session.display_name().to_owned())
        });
        let name = name.unwrap_or_default();
        dialog.show_working(&name);

        // Idempotent (`SessionBook::park`'s own doc): safe to run again on
        // every retry.
        self.book.borrow_mut().park(id);
        self.redraw();
        self.request_save();

        let holder = self.holders.borrow_mut().remove(id);
        let grid = self.grid.clone();
        let window = self.obj().downgrade();
        let task_id = id.clone();

        glib::spawn_future_local(async move {
            let outcome =
                account_deletion::delete_account(task_id.clone(), holder, grid, removal.clone())
                    .await;

            let Some(window) = window.upgrade() else {
                return;
            };
            match outcome {
                Ok(()) => window.imp().finish_account_deletion(&task_id, &dialog),
                Err(error) => {
                    let folder = removal.folder(&task_id);
                    dialog.show_failed(&name, &error.reason, &folder);
                }
            }
        });
    }

    /// A deletion attempt for `id` actually removed the folder. Forgets the
    /// account everywhere: the book (`FR.21.5`) and any pending zoom-save
    /// timer (`FR.12.5`) — then closes `dialog`, redraws and saves.
    fn finish_account_deletion(&self, id: &SessionId, dialog: &DeleteAccountDialog) {
        self.book.borrow_mut().remove_account(id);
        if let Some(timer) = self.zoom_save_timers.borrow_mut().remove(id) {
            timer.remove();
        }
        dialog.close_on_success();
        self.redraw();
        self.request_save();
    }

    /// `Close` was pressed after a failed deletion attempt. By the time
    /// `failed` shows, the attempt has always already cleared `id`'s data and
    /// torn down its holder and grid entry (`account_deletion::delete_account`
    /// runs that step before the folder removal that failed) — this rebuilds
    /// a fresh dormant holder and grid placeholder for it, exactly as a
    /// restore does for any other parked account (`restore_workspace`'s
    /// pattern), so the row reads `parked` and the account is ready to be
    /// deleted again later. A no-op if a holder is somehow still present, or
    /// if the book no longer has the account.
    fn restore_after_failed_delete(&self, id: &SessionId) {
        if self.holders.borrow().contains_key(id) {
            return;
        }
        let Some(locator) = self.locator.borrow().clone() else {
            tracing::error!("no profile locator attached; cannot rebuild the parked account");
            return;
        };

        let book = self.book.borrow();
        let Some(workspace) = book
            .workspaces()
            .find(|workspace| workspace.book().session(id).is_some())
        else {
            return;
        };
        let layout = workspace.book().layout();
        let session = workspace.book().session(id).expect("just found above");
        let (name, address, zoom, identity, webgl_enabled) = (
            session.display_name().to_owned(),
            session.start_address().to_owned(),
            session.zoom_for(layout),
            session.browser_identity().map(str::to_owned),
            session.is_webgl_enabled(),
        );
        drop(book);

        let directories = match locator.locate(id) {
            Ok(directories) => directories,
            Err(error) => {
                tracing::error!(session = %id, %error, "could not prepare the profile directories");
                return;
            }
        };

        let holder = SessionView::dormant(
            id,
            &directories,
            &address,
            AccountSettings {
                zoom,
                identity,
                webgl_enabled,
            },
        );
        self.grid.add_dormant_session(id, &name);
        self.holders.borrow_mut().insert(id.clone(), holder);
        self.redraw();
    }

    /// The pager's two arrows: each mirrors Shift+Tab's `NextAccount` arm in
    /// `run_shortcut`, the book turns the page — and wraps, its own doing —
    /// then the window redraws and saves. Extracted from `constructed` only
    /// to keep it under the house line limit; no logic beyond calling the
    /// two book methods lives here (code standards rule 6).
    fn wire_pager(&self) {
        let window = self.obj().downgrade();
        self.page_previous.connect_clicked(move |_| {
            if let Some(window) = window.upgrade() {
                let imp = window.imp();
                imp.book.borrow_mut().previous_page();
                imp.sync_watched();
                imp.redraw();
                imp.request_save();
            }
        });

        let window = self.obj().downgrade();
        self.page_next.connect_clicked(move |_| {
            if let Some(window) = window.upgrade() {
                let imp = window.imp();
                imp.book.borrow_mut().next_page();
                imp.sync_watched();
                imp.redraw();
                imp.request_save();
            }
        });
    }

    /// The sidebar row's own intents: focusing, parking, keep-awake and
    /// rename. Extracted from `constructed` only to keep it under the
    /// house line limit — each closure still just forwards the id to the
    /// matching `Window` method, which is where the domain call happens
    /// (architecture rule 8).
    fn wire_sidebar_signals(&self) {
        let window = self.obj().downgrade();
        self.sidebar.connect_row_activated(move |id| {
            if let Some(window) = window.upgrade() {
                window.imp().focus_session(&id);
            }
        });

        let window = self.obj().downgrade();
        self.sidebar.connect_parking_toggled(move |id| {
            if let Some(window) = window.upgrade() {
                window.imp().toggle_parking(&id);
            }
        });

        let window = self.obj().downgrade();
        self.sidebar.connect_keep_awake_toggled(move |id, value| {
            if let Some(window) = window.upgrade() {
                window.imp().toggle_keep_awake(&id, value);
            }
        });

        let window = self.obj().downgrade();
        self.sidebar.connect_rename_requested(move |id| {
            if let Some(window) = window.upgrade() {
                window.imp().present_rename_dialog(&id);
            }
        });

        let window = self.obj().downgrade();
        self.sidebar
            .connect_expansion_toggled(move |workspace, expanded| {
                if let Some(window) = window.upgrade() {
                    window.imp().set_expanded(&workspace, expanded);
                }
            });

        let window = self.obj().downgrade();
        self.sidebar.connect_move_requested(move |id, target| {
            if let Some(window) = window.upgrade() {
                window.imp().move_account(&id, target);
            }
        });

        let window = self.obj().downgrade();
        self.sidebar.connect_workspace_rename_requested(move |id| {
            if let Some(window) = window.upgrade() {
                window
                    .imp()
                    .present_workspace_name_dialog(NameDialogPurpose::Rename(id));
            }
        });

        let window = self.obj().downgrade();
        self.sidebar.connect_workspace_remove_requested(move |id| {
            if let Some(window) = window.upgrade() {
                window.imp().remove_workspace(&id);
            }
        });

        let window = self.obj().downgrade();
        self.sidebar.connect_delete_requested(move |id| {
            if let Some(window) = window.upgrade() {
                window.imp().present_delete_dialog(&id);
            }
        });

        let window = self.obj().downgrade();
        self.sidebar.connect_park_all_requested(move |workspace| {
            if let Some(window) = window.upgrade() {
                window.imp().park_all(&workspace);
            }
        });

        let window = self.obj().downgrade();
        self.sidebar.connect_start_all_requested(move |workspace| {
            if let Some(window) = window.upgrade() {
                window.imp().start_all(&workspace);
            }
        });
    }

    /// A heading was expanded or collapsed. Stored and saved; nothing else
    /// changes, so this never redraws (`FR.16.2`).
    fn set_expanded(&self, workspace: &WorkspaceId, expanded: bool) {
        self.book.borrow_mut().set_expanded(workspace, expanded);
        self.request_save();
    }

    /// A row's `Move to ▸` submenu chose `target` for `id` (design rule 23).
    /// An existing workspace moves it straight away — only on success does it
    /// redraw and save; no liveness changes and the shown workspace stays
    /// shown (`FR.17.8`). A refusal is logged and changes nothing; the menu
    /// never offers a destination the book would refuse, so this is not
    /// expected to be reached in practice (code standards rule 1).
    /// `MoveTarget::New` opens the "New workspace" window instead.
    fn move_account(&self, id: &SessionId, target: MoveTarget) {
        let workspace = match target {
            MoveTarget::Existing(workspace) => workspace,
            MoveTarget::New => {
                self.present_workspace_name_dialog(NameDialogPurpose::Create(vec![id.clone()]));
                return;
            }
        };
        // Bound to a `let` rather than matched directly on the `borrow_mut()`
        // call: a `match` scrutinee's temporaries live to the end of the
        // whole expression, so matching the `RefMut` in place would keep
        // `book` borrowed through the `Ok` arm below and panic on `redraw`'s
        // own borrow (measured 2026-09-14, the same hazard `sync`'s own
        // guard already fixed for the sidebar's tree).
        let result = self
            .book
            .borrow_mut()
            .move_accounts(std::slice::from_ref(id), &workspace);
        match result {
            Ok(()) => {
                self.redraw();
                self.request_save();
            }
            Err(refusal) => {
                tracing::error!(?refusal, %workspace, session = %id, "could not move the account");
            }
        }
    }

    /// The drop half of the drag (item 10 task 05): the grid only reports
    /// which account landed on which slot, so the book is the only thing
    /// that decides whether that is a swap, a fill or nothing.
    fn wire_account_dropped(&self) {
        let window = self.obj().downgrade();
        self.grid.connect_account_dropped(move |id, slot| {
            if let Some(window) = window.upgrade() {
                window.imp().drop_account(&id, slot);
            }
        });
    }

    /// Applies a dropped account (item 10 task 05). On `Swapped` or `Filled`
    /// — a real move — it redraws, which moves the places and rebuilds the
    /// sidebar in the book's new order, then saves. On `Unchanged` — the
    /// source place, a slot outside the current layout, or an unknown or
    /// off-grid id — it does nothing at all: no redraw, no save. It never
    /// touches a `SessionView` holder or a zoom, so no page reloads, starts,
    /// stops or resizes (`FR.14.8`).
    fn drop_account(&self, id: &SessionId, slot: SlotId) {
        let outcome = self.book.borrow_mut().active_mut().move_to_slot(id, slot);
        match outcome {
            MoveOutcome::Swapped { .. } | MoveOutcome::Filled => {
                self.redraw();
                self.request_save();
            }
            MoveOutcome::Unchanged => {}
        }
    }

    /// Adds an account from a typed name and address into `workspace`, then
    /// builds its view. Never switches which workspace is shown, so an
    /// account added into a hidden workspace starts running out of sight
    /// (`FR.17.9`). A refusal — `workspace` is named and already full — is
    /// logged and creates nothing; it cannot arise from the add-game window
    /// today, since its `Workspace` field never offers a full workspace, but
    /// the book's answer is trusted over an assumption about the caller
    /// (code standards rule 1).
    fn create_account(&self, name: &str, address: &str, workspace: &WorkspaceId) {
        // See `move_ticked`'s comment: bound to a `let` so the `RefMut`
        // temporary drops before `realise_account` below borrows the book
        // again.
        let result = self.book.borrow_mut().add(workspace, name, address);
        match result {
            Ok(id) => self.realise_account(&id),
            Err(refusal) => {
                tracing::error!(?refusal, %workspace, "could not add the account");
            }
        }
    }

    /// Adds an account for `preset`'s game under `account_name` into
    /// `workspace` — the book copies the game's address, zoom, browser
    /// identity and keep-awake default onto it — then builds its view.
    /// Otherwise exactly [`Window::create_account`].
    fn create_account_from_preset(
        &self,
        preset: &Preset,
        account_name: &str,
        workspace: &WorkspaceId,
    ) {
        // See `move_ticked`'s comment: bound to a `let` so the `RefMut`
        // temporary drops before `realise_account` below borrows the book
        // again.
        let result = self
            .book
            .borrow_mut()
            .add_from_preset(workspace, account_name, preset);
        match result {
            Ok(id) => self.realise_account(&id),
            Err(refusal) => {
                tracing::error!(?refusal, %workspace, "could not add the account");
            }
        }
    }

    /// Prepares the profile directories for a just-added account and hands its
    /// first view to the grid. Reads the account's name, address, zoom and
    /// browser identity off the book, never off a preset kept on the side —
    /// which is what makes the account independent of the file it came from.
    /// Resolves the zoom against the layout of whichever workspace the
    /// account actually joined, not the shown one, so it appears at the
    /// right size the first time that workspace is shown (item 11 task 07).
    fn realise_account(&self, id: &SessionId) {
        let Some(locator) = self.locator.borrow().clone() else {
            tracing::error!("no profile locator attached; cannot create an account");
            return;
        };

        // Install the account's chosen sizes onto the book before resolving the
        // size to open at, so the page is never drawn at the baseline and then
        // jumps to the remembered size (`FR.12.7`). A fresh account has no
        // stored sizes and reads back an empty map.
        if let Some(memory) = self.zoom_memory.borrow().clone() {
            let remembered = memory.read(id);
            let mut book = self.book.borrow_mut();
            if let Some(session_book) = book
                .workspaces_mut()
                .find(|book| book.session(id).is_some())
            {
                session_book.restore_zoom(id, remembered);
            }
        }

        let book = self.book.borrow();
        let Some(workspace) = book
            .workspaces()
            .find(|workspace| workspace.book().session(id).is_some())
        else {
            tracing::error!(session = %id, "the account is not in the book");
            return;
        };
        let layout = workspace.book().layout();
        let session = workspace.book().session(id).expect("just found above");
        let (name, address, zoom, identity, webgl_enabled) = (
            session.display_name().to_owned(),
            session.start_address().to_owned(),
            session.zoom_for(layout),
            session.browser_identity().map(str::to_owned),
            session.is_webgl_enabled(),
        );
        drop(book);

        let directories = match locator.locate(id) {
            Ok(directories) => directories,
            Err(error) => {
                tracing::error!(session = %id, %error, "could not prepare the profile directories");
                return;
            }
        };

        let holder = SessionView::new(
            id,
            &directories,
            &address,
            AccountSettings {
                zoom,
                identity,
                webgl_enabled,
            },
            self.minimised.get(),
        );
        let Some(view) = holder.view() else {
            tracing::error!(session = %id, "the new account's view was not built");
            return;
        };
        self.grid.add_session(id, &name, view);
        self.holders.borrow_mut().insert(id.clone(), holder);
        self.redraw();
        self.request_save();
    }

    /// Focuses `id`. When it belongs to a workspace that is not shown, the
    /// book switches to it first — the header's layout buttons are flipped to
    /// its arrangement and its accounts are snapped to their sizes for it,
    /// without treating the flip as a second layout switch of its own
    /// (`connect_layout_toggle`'s own guard), and no view is built, destroyed,
    /// stopped or reloaded (`FR.16.3`).
    /// While a phone is attached, being watched moves with the current
    /// account: the previous one is un-told before the next frame tick
    /// (`FR.4.3`).
    fn focus_session(&self, id: &SessionId) {
        let switch = self.book.borrow_mut().focus_account(id);
        if let Some(switch) = &switch {
            self.after_workspace_switch(switch);
        }
        self.sync_watched();
        self.redraw();
        self.request_save();
    }

    /// What a switch to another shown workspace always does beyond focusing
    /// the account: flip the header's layout toggles to match its
    /// arrangement and snap its accounts to their sizes for it, without
    /// treating the flip as a second layout switch of its own
    /// (`connect_layout_toggle`'s own guard, `FR.16.3`). Both
    /// [`Window::focus_session`] and [`Window::run_shortcut`]'s
    /// `NextWorkspace` arm land here, so a click on a sidebar name and
    /// `Ctrl`+`Tab` can never disagree about what switching a workspace does.
    fn after_workspace_switch(&self, switch: &Switch) {
        let layout = self.book.borrow().active().layout();
        self.select_layout_toggle(layout);
        self.snap_zoom_for_active();
        tracing::debug!(from = %switch.from(), to = %switch.to(), "switched the shown workspace");
    }

    /// The park/start button was pressed. The direction is the book's to
    /// decide from the account's current liveness — the row carries none
    /// (architecture rule 8).
    fn toggle_parking(&self, id: &SessionId) {
        let liveness = self
            .book
            .borrow()
            .active()
            .session(id)
            .map(Session::liveness);

        match liveness {
            Some(Liveness::Live) => {
                self.park_session(id);
                self.request_save();
            }
            Some(Liveness::Parked) => {
                self.start_session(id);
                self.request_save();
            }
            // Starting and Queued both keep the row's Start item insensitive, so
            // a press that still arrives is a stale event; during a restore the
            // start queue owns a queued account's turn. An unknown id has
            // nothing to toggle.
            Some(Liveness::Starting | Liveness::Queued) | None => {}
        }
    }

    /// Parks a running account, in the order the roadmap item's first diagram
    /// fixes: the domain records the park, then the engine is made to match
    /// (architecture rule 8). The account keeps its slot; the slot falls back
    /// to the name cover until it is started again.
    fn park_session(&self, id: &SessionId) {
        self.book.borrow_mut().park(id);
        self.stop_view(id);
        self.redraw();
    }

    /// Stops `id`'s running view without touching the book: the holder itself,
    /// the `watched` mark if it named this account, and the grid's own
    /// reference. The book-side park and the redraw are each caller's own —
    /// shared by [`Window::park_session`]'s `Live` handling and `park_all`'s
    /// per-account loop, which redraws once for the whole workspace instead
    /// of once per account (`FR.24.1`).
    fn stop_view(&self, id: &SessionId) {
        if let Some(holder) = self.holders.borrow_mut().get_mut(id) {
            holder.stop();
        }
        // The view that was told it is watched is gone with the stop; the
        // one built by the next start is told afresh at its first paint.
        let was_watched = self.watched.borrow().as_ref() == Some(id);
        if was_watched {
            self.watched.replace(None);
        }
        self.grid.release_view(id);
    }

    /// The heading's `Park all`: parks every account in `workspace` that is
    /// not already parked, no confirmation, because `Start all` undoes it
    /// (`FR.24.1`). A running account is stopped at once; a queued one simply
    /// leaves the queue — [`StartQueue`] already skips an id the book no
    /// longer reports queued; a starting one has no view yet to stop, so its
    /// id waits in `park_on_paint` until its page paints and `finish_starting`
    /// parks it then, so no second view is ever built for it. One redraw and
    /// one save for the whole workspace, not once per account.
    fn park_all(&self, workspace: &WorkspaceId) {
        let touched = self.book.borrow_mut().park_all(workspace);
        if touched.is_empty() {
            return;
        }

        for (id, before) in touched {
            match before {
                Liveness::Live => self.stop_view(&id),
                Liveness::Starting => {
                    self.park_on_paint.borrow_mut().insert(id);
                }
                Liveness::Queued | Liveness::Parked => {}
            }
        }

        self.redraw();
        self.request_save();
    }

    /// The heading's `Start all`: queues every parked account in `workspace`,
    /// in workspace order, and hands them to the one start queue the window
    /// keeps for its whole life — appended, never replacing it, so a
    /// still-draining restore is never started twice over (`FR.24.2`,
    /// `FR.8.2`).
    fn start_all(&self, workspace: &WorkspaceId) {
        let ids = self.book.borrow_mut().queue_parked(workspace);
        if ids.is_empty() {
            return;
        }

        self.redraw();
        self.request_save();
        self.start_queue().enqueue(ids);
    }

    /// Starts a parked or queued account, in the order the roadmap item's second
    /// diagram fixes: unpark in the book (which returns `Starting`, since no page
    /// has painted), redraw so the row shows it and the button goes insensitive,
    /// then build a new view against the kept network session and hand it to the
    /// grid (architecture rule 8). Returns the new view, or `None` when the
    /// account has no holder to start. The start queue (task 05) is the only
    /// caller that reads the return.
    pub(crate) fn start_session(&self, id: &SessionId) -> Option<EngineView> {
        self.book.borrow_mut().unpark(id);
        self.redraw();

        let view = {
            let mut holders = self.holders.borrow_mut();
            let Some(holder) = holders.get_mut(id) else {
                tracing::error!(session = %id, "no holder to start");
                return None;
            };
            holder
                .start(self.minimised.get(), self.phone_watches(id))
                .clone()
        };

        self.grid.attach_view(id, &view);

        // The starting interval ends at the new view's first paint — the same
        // signal the grid uses to drop the cover.
        let window = self.obj().downgrade();
        let owned_id = id.clone();
        view.connect_painted(move || {
            if let Some(window) = window.upgrade() {
                window.imp().finish_starting(&owned_id);
            }
        });

        Some(view)
    }

    /// Whether the book still reports `id` as [`Liveness::Queued`], wherever
    /// its workspace is — the start queue also brings back accounts in a
    /// hidden workspace (`FR.18.4`). The start queue asks before each turn, so
    /// a state that changed while it was draining is never overwritten by a
    /// start nobody asked for.
    pub(crate) fn is_queued(&self, id: &SessionId) -> bool {
        liveness_anywhere(&self.book.borrow(), id) == Some(Liveness::Queued)
    }

    /// The shell reports a started account's first paint: end its starting
    /// interval in the book and redraw. A no-op unless the account is actually
    /// starting, so a later navigation's load event does not churn the
    /// sidebar — *unless* `park_all` reached this id while it was starting, in
    /// which case its liveness is already `Parked` and `park_on_paint` is what
    /// carries the "park it the moment it paints" intent instead (`FR.24.1`).
    /// Searched wherever the account's workspace is, since the start queue may
    /// be starting one in a hidden workspace.
    ///
    /// A page that has just painted is a fresh one — a new view after a
    /// start, or a reload — so if this is the watched account it is told
    /// again, and if the phone is waiting on an account that had no view
    /// until now, it is told for the first time.
    fn finish_starting(&self, id: &SessionId) {
        let pending_park = self.park_on_paint.borrow_mut().remove(id);
        let was_starting = liveness_anywhere(&self.book.borrow(), id) == Some(Liveness::Starting);
        if !was_starting && !pending_park {
            return;
        }

        self.book.borrow_mut().mark_started(id);

        if pending_park {
            self.park_session(id);
            return;
        }

        let is_watched = self.watched.borrow().as_ref() == Some(id);
        if is_watched {
            if let Some(holder) = self.holders.borrow().get(id) {
                holder.set_watched(true, self.minimised.get());
            }
        } else {
            self.sync_watched();
        }
        self.redraw();
    }

    /// The row menu's "Keep running when hidden" item was chosen, in the
    /// order the roadmap item's first diagram fixes: the domain records the
    /// flag, then the engine is made to match (architecture rule 8). A no-op
    /// if the account already held `value` — the book reports nothing
    /// changed, and a reload would be spent on nothing.
    fn toggle_keep_awake(&self, id: &SessionId, value: bool) {
        let changed = self.book.borrow_mut().set_keep_awake(id, value);
        if !changed {
            return;
        }
        // A live account moved to `Starting` by the set above; a parked one
        // did not, so this only shows the reloading marker where it applies.
        self.redraw();
        self.request_save();

        let view = {
            let mut holders = self.holders.borrow_mut();
            let Some(holder) = holders.get_mut(id) else {
                tracing::error!(session = %id, "no holder to apply keep-awake to");
                return;
            };
            holder.set_keep_awake(value, self.minimised.get()).cloned()
        };

        // Parked: nothing to reload now. `SessionView::start` applies the
        // remembered value the next time a view is built for this account.
        let Some(view) = view else {
            return;
        };

        // Same signal, same reason as `start_session`: the starting interval
        // this toggle opened ends at the reload's first paint.
        // `EngineView::connect_painted` self-disconnects once it fires, so
        // arming it again here on the same live view — on top of whatever
        // `start_session` or an earlier toggle already armed — never
        // accumulates a permanently-connected closure per toggle.
        let window = self.obj().downgrade();
        let owned_id = id.clone();
        view.connect_painted(move || {
            if let Some(window) = window.upgrade() {
                window.imp().finish_starting(&owned_id);
            }
        });
    }

    /// The window was minimised or restored (roadmap item 12 task 04,
    /// `FR.1.9`): every live account whose keep-awake is off is marked
    /// invisible to its engine, and every live account is marked visible
    /// again once the window is no longer minimised — `background_for`
    /// decides which, reading each account's keep-awake straight off the
    /// book rather than a copy kept on the holder (architecture rule 8).
    /// Applying this again for a state that has not actually changed would
    /// reissue the platform call for nothing, so a repeat is a no-op.
    fn apply_minimised(&self, minimised: bool) {
        if self.minimised.replace(minimised) == minimised {
            return;
        }

        let book = self.book.borrow();
        for (id, holder) in self.holders.borrow().iter() {
            let Some(view) = holder.view() else {
                continue;
            };
            let keep_awake = book
                .workspaces()
                .find_map(|workspace| workspace.book().session(id))
                .is_some_and(Session::is_kept_awake);
            view.set_background(background_for(minimised, keep_awake));
        }
        drop(book);

        // The watched view is the exception: a phone is looking at it, so a
        // minimise never puts it in the background (`FR.4.3`).
        if let Some(id) = self.watched.borrow().as_ref()
            && let Some(holder) = self.holders.borrow().get(id)
        {
            holder.set_watched(true, minimised);
        }
    }

    /// Decides what `key` under `modifiers` means with [`shortcut_for`] and,
    /// for a match, runs it with [`Window::run_shortcut`]. Returns whether
    /// the key was consumed.
    ///
    /// Both the GTK key controller wired in `constructed` (Linux, and
    /// Windows whenever a GTK widget — not a game's `WebView2` child window —
    /// holds focus) and, on Windows, `EngineHost`'s `AcceleratorKeyPressed`
    /// subscription (roadmap item 12 task 03) call this one function, so the
    /// two engines can never disagree about what a shortcut does. `pub(crate)`
    /// for that second caller in `web_engine/webview2`, across the module
    /// boundary but inside the one crate (architecture rules 8, 12).
    ///
    /// Every shortcut but `Reload` and `Zoom` runs once while held, not on
    /// every repeat event the platform delivers: GTK 4 exposes no repeat flag
    /// on a key event, so `chord_held` latches on the press that runs one and
    /// the next key release (wired in `constructed`) clears it (code
    /// standards rule 18, `FR.23.3`, `FR.26.6`). Reload and zoom are excluded
    /// deliberately — holding `Ctrl`+`-` to zoom out several steps is the
    /// gesture, not a misfire. The key is consumed either way.
    pub(crate) fn handle_shortcut_key(&self, key: gdk::Key, modifiers: gdk::ModifierType) -> bool {
        let Some(shortcut) = shortcut_for(key, modifiers) else {
            return false;
        };

        if repeats_while_held(shortcut) {
            self.run_shortcut(shortcut);
            return true;
        }

        if self.chord_held.replace(true) {
            return true;
        }

        self.run_shortcut(shortcut);
        true
    }

    /// Runs `shortcut`, as [`handle_shortcut_key`](Window::handle_shortcut_key)
    /// decided it. `Reload` and `Zoom` act exactly as before this table
    /// existed. The shown workspace being empty, or holding only one account
    /// or one other workspace, makes the navigation arms a no-op by the
    /// book's own answer, with nothing extra to check here.
    ///
    /// The window's own controls are driven through the control the mouse
    /// presses, never past it (`FR.26.1`, `FR.26.2`): a chord sets a toggle's
    /// active state and the toggle's own handler does the rest, so a control
    /// and the thing it controls can never drift apart, and the chord gets
    /// every consequence of a click — the save, the zoom snap, leaving mobile
    /// mode — without restating any of them.
    pub(crate) fn run_shortcut(&self, shortcut: Shortcut) {
        match shortcut {
            Shortcut::Reload => self.grid.reload_focused(),
            Shortcut::Zoom(step) => {
                // Zoom is locked in `Mobile` (Remote Access `FR.3.2`): the key
                // is still the window's, so it is consumed, but it changes
                // nothing and flashes no readout (design rule 10).
                if !self.is_mobile_layout_shown() {
                    self.zoom_focused_account(step);
                }
            }
            Shortcut::NextAccount => {
                self.book.borrow_mut().focus_next_account();
                self.sync_watched();
                self.redraw();
                self.request_save();
            }
            Shortcut::NextWorkspace => {
                let switch = self.book.borrow_mut().focus_next_workspace();
                if let Some(switch) = switch {
                    self.after_workspace_switch(&switch);
                    self.sync_watched();
                    self.redraw();
                    self.request_save();
                }
            }
            Shortcut::ToggleSidebar => {
                // The button, never the revealer: `constructed` binds
                // `sidebar_toggle`'s `active` to the revealer's
                // `reveal-child`, so driving the button is the whole of the
                // chord and the two can never disagree about which way the
                // sidebar sits (`FR.26.1`).
                self.sidebar_toggle
                    .set_active(!self.sidebar_toggle.is_active());
            }
            Shortcut::Arrange(layout) => {
                // The toggle, never `choose_layout` directly: selecting it
                // fires its own `toggled` handler, which is the path a click
                // takes — leaving mobile mode first and ending in
                // `finish_layout_change` (`FR.26.2`, architecture rule 8).
                // The chord for the arrangement already showing sets an
                // already-active toggle, which emits nothing, so it is a
                // no-op for free.
                self.select_layout_toggle(layout);
            }
            Shortcut::ParkFocused => self.park_or_start_focused(Liveness::Live),
            Shortcut::StartFocused => self.park_or_start_focused(Liveness::Parked),
            Shortcut::ParkWorkspace => {
                let workspace = self.book.borrow().active_id().clone();
                self.park_all(&workspace);
            }
            Shortcut::StartWorkspace => {
                let workspace = self.book.borrow().active_id().clone();
                self.start_all(&workspace);
            }
            Shortcut::AddAccount => self.present_add_game_dialog(),
            Shortcut::RenameFocused => {
                let focused = self
                    .book
                    .borrow()
                    .active()
                    .focused_session()
                    .map(|session| session.id().clone());
                if let Some(id) = focused {
                    self.present_rename_dialog(&id);
                }
            }
            // The buttons, never `next_page`/`previous_page` directly — the
            // window's own controls are driven through the control a click
            // presses, never past it, exactly like `ToggleSidebar` and
            // `Arrange` above.
            Shortcut::NextPage => self.page_next.emit_clicked(),
            Shortcut::PreviousPage => self.page_previous.emit_clicked(),
        }
    }

    /// The act behind `Ctrl`+`P` and `Ctrl`+`S`: run the focused account
    /// through the row menu's own [`Window::toggle_parking`], but only when
    /// its liveness is `from` — the state the chord's one direction starts
    /// from (`Liveness::Live` to park, `Liveness::Parked` to start).
    ///
    /// That guard is what makes each chord idempotent rather than a flip
    /// (`FR.26.3`): `toggle_parking` moves an account to whichever liveness
    /// it is not, which is right for a menu item whose label already read the
    /// state, and wrong for a key that carries no label. `Starting` and
    /// `Queued` match neither direction and fall through to silence, exactly
    /// as the row's own item is greyed in those states (design rule 2); so
    /// does an empty workspace, which has no focused account at all
    /// (`FR.26.6`).
    fn park_or_start_focused(&self, from: Liveness) {
        // Bound to a `let` so the borrow ends here: `toggle_parking` borrows
        // the book again on the next line (code standards rule 12).
        let focused = self
            .book
            .borrow()
            .active()
            .focused_session()
            .map(|session| (session.id().clone(), session.liveness()));

        if let Some((id, liveness)) = focused
            && liveness == from
        {
            self.toggle_parking(&id);
        }
    }

    /// Whether the shown workspace is arranged for [`Layout::Mobile`], where
    /// every zoom gesture is a no-op.
    fn is_mobile_layout_shown(&self) -> bool {
        self.book.borrow().active().layout() == Layout::Mobile
    }

    /// A window-level zoom gesture from the keyboard: step the account in the
    /// focused slot and show the figure over that place. A no-op when the grid
    /// is empty or the focused slot holds nothing — the book returns `None` and
    /// nothing is drawn (`FR.11.7`, item 09 wireframe).
    fn zoom_focused_account(&self, step: ZoomStep) {
        let Some(id) = self
            .book
            .borrow()
            .active()
            .focused_session()
            .map(|session| session.id().clone())
        else {
            return;
        };
        self.apply_zoom_step(&id, step);
    }

    /// Applies `step` to `id`: the book decides what a step means and records it
    /// against the current arrangement (architecture rule 8), the account's
    /// holder is resized in place with no reload, and the figure is shown over
    /// its place. Both the keyboard branch here and task 05's wheel branch land
    /// here, so the two can never disagree. A no-op for an id the book does not
    /// hold.
    ///
    /// `pub(crate)`: on Windows, `web_engine/webview2.rs`'s ipc handler routes
    /// the bridge script's wheel-zoom message here too (roadmap item 12 task
    /// 03) — a third caller, same reasoning as `handle_shortcut_key`, so
    /// Linux's wheel gesture and Windows' can never disagree either.
    pub(crate) fn apply_zoom_step(&self, id: &SessionId, step: ZoomStep) {
        // The book already answers `None` for every step in `Mobile`; the
        // guard here says so at the call site, so no reader has to trace it
        // through the domain to see that no readout can flash (design rule
        // 10).
        if self.is_mobile_layout_shown() {
            return;
        }
        let resolved = match step {
            ZoomStep::In => self.book.borrow_mut().zoom_in(id),
            ZoomStep::Out => self.book.borrow_mut().zoom_out(id),
            ZoomStep::Reset => self.book.borrow_mut().reset_zoom(id),
        };
        let Some(zoom) = resolved else {
            return;
        };

        if let Some(holder) = self.holders.borrow_mut().get_mut(id) {
            holder.set_zoom(zoom);
        }
        self.grid
            .flash_zoom_readout(id, &format!("{:.0}%", zoom.multiplier() * 100.0));
        self.schedule_zoom_save(id);
    }

    /// Restarts `id`'s settle timer. When the gestures stop, the settled
    /// remembered map is read off the book and written once (`FR.12.5`) — a
    /// burst of twenty wheel notches costs one write, not twenty. Nothing else
    /// writes `state.toml` (`FR.12.4`).
    fn schedule_zoom_save(&self, id: &SessionId) {
        if let Some(timer) = self.zoom_save_timers.borrow_mut().remove(id) {
            timer.remove();
        }

        let window = self.obj().downgrade();
        let account = id.clone();
        let timer = glib::timeout_add_local_once(
            Duration::from_millis(ZOOM_SAVE_SETTLE_MILLIS),
            move || {
                if let Some(window) = window.upgrade() {
                    window.imp().zoom_save_timers.borrow_mut().remove(&account);
                    window.imp().write_zoom_now(&account);
                }
            },
        );
        self.zoom_save_timers.borrow_mut().insert(id.clone(), timer);
    }

    /// Writes `id`'s settled remembered map through the port now. A failure is
    /// logged with the account id and a reason and otherwise ignored — the size
    /// is still right on screen, it just will not survive a restart (code
    /// standards rules 14, 15).
    fn write_zoom_now(&self, id: &SessionId) {
        let Some(memory) = self.zoom_memory.borrow().clone() else {
            return;
        };
        let Some(remembered) = self
            .book
            .borrow()
            .active()
            .session(id)
            .map(|session| session.remembered_zoom().clone())
        else {
            return;
        };

        if let Err(error) = memory.write(id, &remembered) {
            tracing::warn!(session = %id, reason = %error, "could not persist the remembered zoom");
        }
    }

    fn redraw(&self) {
        let book = self.book.borrow();
        self.grid.sync(&book);
        // Every row's own `Move to ▸` submenu moves exactly one account
        // (design rule 23), so the destinations it offers are always for a
        // single account — recomputed here rather than per row-bind, since
        // it depends on nothing about which row is asking. Set before
        // `sidebar.sync` below, whose row factory reads it while building
        // each row's menu.
        self.sidebar.set_move_destinations(book.destinations(1));
        self.sidebar.sync(&book);

        self.update_header(&book);

        // Two distinct empty states (`FR.15.10`): no account anywhere is the
        // existing first-run message and its button; the shown workspace
        // alone being empty, while another workspace holds accounts, is the
        // quiet "No games in this workspace" line with no button.
        let shown_empty = book.active().sessions().is_empty();
        let no_accounts_anywhere = book
            .workspaces()
            .all(|workspace| workspace.book().sessions().is_empty());
        self.empty_state.set_visible(shown_empty);
        self.first_run_label.set_visible(no_accounts_anywhere);
        self.add_first_game_button.set_visible(no_accounts_anywhere);
        self.workspace_empty_label
            .set_visible(shown_empty && !no_accounts_anywhere);
        self.grid.set_visible(!shown_empty);

        // The pager reads two numbers off the shown workspace and shows
        // itself only while they mean something (`FR.22.5`); the wrap on a
        // turn is the book's own, not this widget's (code standards rule 6).
        let page = book.active().page();
        let pages = book.active().page_count();
        self.page_readout
            .set_text(&format!("{}/{}", page + 1, pages));
        self.pager.set_visible(pages > 1);

        // Every change the sidebar sees, the phone sees (`FR.2.1`).
        if let Some(link) = self.phone_link.borrow().as_ref() {
            let state = RemoteState::from_book(&book);
            tracing::debug!(
                mobile_mode = state.mobile_mode,
                current = ?state.current,
                "state published to the phone"
            );
            link.publish_state(&state);
        }
        self.refresh_phone_actions();

        // Explicitly, because the next call borrows the book again (code
        // standards rule 12) — and last, because it is the only part of a
        // redraw that touches something outside this window's own widgets.
        drop(book);
        self.follow_focus_with_keyboard();
    }

    /// Redraws everything in the header bar that names the focused account
    /// (design rule 26): the title, reload's tooltip, and the main menu's
    /// zoom section. `None` for every one of them when nothing is focused —
    /// the title goes empty, reload greys out and the zoom section is left
    /// out of the menu entirely (never shown greyed, matching design rule 18's
    /// reasoning for the pager).
    fn update_header(&self, book: &WorkspaceBook) {
        let focused = book.active().focused_session();
        let workspace_name = book
            .workspaces()
            .find(|workspace| workspace.id() == book.active_id())
            .map(|workspace| workspace.name().to_owned())
            .unwrap_or_default();

        if let Some(session) = focused {
            let name = session.display_name();
            self.title_label
                .set_label(&format!("{name} · {workspace_name}"));
            self.reload_button
                .set_tooltip_text(Some(&format!("Reload {name} (F5)")));
            self.reload_button.set_sensitive(true);
        } else {
            self.title_label.set_label("");
            self.reload_button
                .set_tooltip_text(Some("Nothing to reload"));
            self.reload_button.set_sensitive(false);
        }

        let zoom_label = focused.map(|session| {
            let percent = (session.zoom_for(book.active().layout()).multiplier() * 100.0).round();
            format!("Zoom — {} · {percent:.0}%", session.display_name())
        });
        self.main_menu.set_menu_model(Some(&build_main_menu(
            zoom_label.as_deref(),
            self.running_version.get(),
        )));
        for action in [
            &self.zoom_in_action,
            &self.zoom_out_action,
            &self.zoom_reset_action,
        ] {
            if let Some(action) = action.get() {
                action.set_enabled(focused.is_some());
            }
        }
    }

    /// The window gained or lost the keyboard as a whole: a dialog opened or
    /// closed over it, or the user moved to another application and back.
    ///
    /// Losing it forgets the hand-over: whatever took the window's focus owns
    /// the keyboard now, and the view drops its own the moment the toplevel
    /// does, so the record of having handed it over is stale and would
    /// otherwise block the next grab (see `last_focus_grab`). Gaining it
    /// makes the hand-over again, which is what puts the keyboard back in the
    /// focused game after a dialog closes — no redraw follows a dialog
    /// closing, so without this the page would stay deaf until the focused
    /// account next changed (`FR.27.1`, `FR.27.3`).
    fn follow_window_activation(&self) {
        if self.obj().is_active() {
            self.follow_focus_with_keyboard();
        } else {
            self.last_focus_grab.replace(None);
        }
    }

    /// Hands the focused account's live view the keyboard, so its page is
    /// live to typing the moment its outline appears (`FR.27.1`).
    ///
    /// Called from the end of [`Window::redraw`] rather than from each of the
    /// routes that can change the focused account — a sidebar row,
    /// `Shift`+`Tab`, `Ctrl`+`Tab`, a pager arrow, a click on a slot, an
    /// arrangement change, a drag, a workspace switch, an account added, a
    /// restore, a phone choosing one. Every one of them already ends in a
    /// redraw, so one hook covers all of them, and covers a twelfth route
    /// written later without it having to remember this.
    ///
    /// Three things stop it, in order:
    ///
    /// - No focused account, or no live view for it — parked, queued,
    ///   starting, or an empty trailing slot on a part-empty last page. The
    ///   keyboard is left exactly where it is; nothing reaches sideways for
    ///   another slot's view (`FR.27.2`). `last_focus_grab` is cleared, which
    ///   is what makes the grab happen later, when *that* account's own view
    ///   goes live — so starting the focused account hands it the keyboard at
    ///   its first paint.
    /// - This account already has it, by `last_focus_grab`. See that field.
    /// - Something else is being typed into (`FR.27.3`). A modal dialog is
    ///   its own toplevel, so [`gtk::Window::is_active`] on this window
    ///   covers the rename dialog and the add-game form with one check; the
    ///   [`gtk::Editable`] test covers an entry inside this window, of which
    ///   there is none today and may be one tomorrow (code standards rule 1
    ///   — trust the check, not the absence). The redesign (roadmap item 11)
    ///   removed the sidebar's third blocker, its selection mode, along with
    ///   the mode itself — `FR.23.4` and `FR.26.6`'s selection-mode clauses
    ///   are obsolete with it.
    fn follow_focus_with_keyboard(&self) {
        let focused = self
            .book
            .borrow()
            .active()
            .focused_session()
            .map(|session| session.id().clone());

        let Some(id) = focused else {
            self.last_focus_grab.replace(None);
            return;
        };

        // A short borrow of its own: `grab_focus` below runs toolkit code
        // that can reach back into this window, and a `RefCell` held across
        // it would be a panic rather than a bug report (code standards rules
        // 12, 18).
        let has_live_view = self
            .holders
            .borrow()
            .get(&id)
            .is_some_and(|holder| holder.view().is_some());
        if !has_live_view {
            self.last_focus_grab.replace(None);
            return;
        }

        if self.last_focus_grab.borrow().as_ref() == Some(&id) {
            return;
        }

        // `RootExt::focus`, spelled out because `GtkWindowExt` offers the
        // same name for the same property and the compiler cannot pick.
        let focus_widget = gtk::prelude::RootExt::focus(&*self.obj());
        let active = self.obj().is_active();
        let editing = focus_widget.is_some_and(|widget| widget.is::<gtk::Editable>());
        if !active || editing {
            tracing::debug!(
                session = %id, active, editing,
                "the focused account's view was not handed the keyboard"
            );
            return;
        }

        let view = self
            .holders
            .borrow()
            .get(&id)
            .and_then(|holder| holder.view().cloned());
        if let Some(view) = view {
            view.grab_focus();
            self.last_focus_grab.replace(Some(id));
        }
    }

    /// Registers the main menu's actions: `Phone…` (item 13 task 07), the
    /// zoom section's three steps (design rule 10), and `Check for updates`
    /// (roadmap item 16 task 07). `show-phone` starts disabled — nothing can
    /// be opened until a link is attached; the zoom actions start disabled
    /// too, since nothing is focused before the first restore.
    /// `check-for-updates` starts enabled and stays that way: it always
    /// answers on screen, even offline (`**Entry**`).
    fn register_menu_actions(&self) {
        let show_phone = gio::SimpleAction::new("show-phone", None);
        show_phone.set_enabled(false);
        let window = self.obj().downgrade();
        show_phone.connect_activate(move |_, _| {
            if let Some(window) = window.upgrade() {
                window.imp().present_phone_dialog();
            }
        });
        self.obj().add_action(&show_phone);
        if self.show_phone_action.set(show_phone).is_err() {
            tracing::error!("the show-phone action was registered twice");
        }

        self.register_zoom_action("zoom-in", ZoomStep::In);
        self.register_zoom_action("zoom-out", ZoomStep::Out);
        self.register_zoom_action("zoom-reset", ZoomStep::Reset);

        let check_for_updates = gio::SimpleAction::new("check-for-updates", None);
        let window = self.obj().downgrade();
        check_for_updates.connect_activate(move |_, _| {
            if let Some(window) = window.upgrade() {
                window
                    .imp()
                    .drive_update(UpdateEvent::CheckRequested { manual: true });
            }
        });
        self.obj().add_action(&check_for_updates);
    }

    /// Registers one of the main menu's zoom actions under `name`, applying
    /// `step` to the focused account when activated. A no-op, quietly, if
    /// nothing is focused — the section is left out of the menu whenever that
    /// is true (`update_header`), so this is not expected to be reached.
    fn register_zoom_action(&self, name: &'static str, step: ZoomStep) {
        let action = gio::SimpleAction::new(name, None);
        action.set_enabled(false);
        let window = self.obj().downgrade();
        action.connect_activate(move |_, _| {
            let Some(window) = window.upgrade() else {
                return;
            };
            let imp = window.imp();
            let focused = imp
                .book
                .borrow()
                .active()
                .focused_session()
                .map(|session| session.id().clone());
            if let Some(id) = focused {
                imp.apply_zoom_step(&id, step);
            }
        });
        self.obj().add_action(&action);
        let cell = match name {
            "zoom-in" => &self.zoom_in_action,
            "zoom-out" => &self.zoom_out_action,
            _ => &self.zoom_reset_action,
        };
        if cell.set(action).is_err() {
            tracing::error!(action = name, "a zoom action was registered twice");
        }
    }

    /// Asks the link where the phone stands and enables `Phone…` from the
    /// answer — it needs only a link to open; a desktop that is not
    /// listening still opens the dialog, which says why inside it (design
    /// rule 8).
    fn refresh_phone_actions(&self) {
        let status = self
            .phone_link
            .borrow()
            .as_ref()
            .map(|link| link.phone_status());
        self.apply_phone_status(status.as_ref());
    }

    /// Enables `Phone…` from `status`; `None` is no link at all.
    fn apply_phone_status(&self, status: Option<&PhoneStatus>) {
        if let Some(action) = self.show_phone_action.get() {
            action.set_enabled(status.is_some());
        }
    }

    /// Opens the phone dialog over the window. Its one-second poll reports
    /// each change of status back here, so a scan made while it is open
    /// enables the dialog's own `Disconnect` without waiting for a redraw.
    fn present_phone_dialog(&self) {
        let Some(link) = self.phone_link.borrow().clone() else {
            return;
        };
        let dialog = PhoneDialog::new(link);
        dialog.set_transient_for(Some(&*self.obj()));

        let window = self.obj().downgrade();
        dialog.connect_status_changed(move |status| {
            if let Some(window) = window.upgrade() {
                window.imp().apply_phone_status(Some(status));
            }
        });

        dialog.present();
    }
}

/// Installs `window.css` on the default display once — the arrangement
/// toggles' pictures (design rule 24) — the same guarded-`Once` shape
/// `session_sidebar/imp.rs` and `session_grid/imp.rs` already use for their
/// own stylesheets.
fn install_styles() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let Some(display) = gdk::Display::default() else {
            return;
        };
        let provider = gtk::CssProvider::new();
        provider.load_from_resource("/org/idlemanager/IdleManager/css/window.css");
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });
}

/// Builds the main menu's model from scratch: the zoom section — its label
/// naming the focused account and its zoom, left out of the menu entirely
/// when `zoom_label` is `None` rather than shown greyed (design rule 18's
/// reasoning) — then `Phone…`, the running version beside `Check for
/// updates`, and `Keyboard Shortcuts`. Rebuilt on every redraw, like a
/// sidebar row's own menu, since the zoom section's label changes with the
/// focused account (design rule 1's re-derive-on-bind pattern).
/// `running_version` is `None` only for the redraw before the update channel
/// is attached, and names the item `Idle Manager` alone until then.
fn build_main_menu(zoom_label: Option<&str>, running_version: Option<Version>) -> gio::Menu {
    let menu = gio::Menu::new();

    if let Some(label) = zoom_label {
        let zoom = gio::Menu::new();
        zoom.append_item(&accelerated_menu_item(
            "Zoom in",
            "win.zoom-in",
            "<Control>plus",
        ));
        zoom.append_item(&accelerated_menu_item(
            "Zoom out",
            "win.zoom-out",
            "<Control>minus",
        ));
        zoom.append_item(&accelerated_menu_item(
            "Reset zoom",
            "win.zoom-reset",
            "<Control>0",
        ));
        menu.append_section(Some(label), &zoom);
    }

    let phone = gio::Menu::new();
    phone.append(Some("Phone…"), Some("win.show-phone"));
    menu.append_section(None, &phone);

    // The version item is bound to no action, so `GtkPopoverMenu` draws it
    // insensitive on its own (design rule 23's same reasoning for `Move to
    // ▸`'s own containing-workspace row).
    let updates = gio::Menu::new();
    let version_label = match running_version {
        Some(version) => format!("Idle Manager {version}"),
        None => "Idle Manager".to_string(),
    };
    updates.append_item(&gio::MenuItem::new(Some(&version_label), None));
    updates.append(Some("Check for updates"), Some("win.check-for-updates"));
    menu.append_section(None, &updates);

    let shortcuts = gio::Menu::new();
    shortcuts.append_item(&accelerated_menu_item(
        "Keyboard Shortcuts",
        "win.show-help-overlay",
        "<Control>question",
    ));
    menu.append_section(None, &shortcuts);

    menu
}

/// A menu item that names its keyboard chord beside its label — the main
/// menu's own copy of `session_sidebar/imp.rs`'s `accelerated_item`, since
/// nothing importable sits between the two crate-private modules for a
/// three-line helper to live in (design rule 19, `FR.25.4`).
fn accelerated_menu_item(label: &str, action: &str, accelerator: &str) -> gio::MenuItem {
    let item = gio::MenuItem::new(Some(label), Some(action));
    item.set_attribute_value("accel", Some(&accelerator.to_variant()));
    item
}

/// The account in the shown workspace's focused slot — the one a watching
/// phone sees — with its liveness, or `None` when that slot holds nothing.
fn current_account(book: &WorkspaceBook) -> Option<(SessionId, Liveness)> {
    book.active()
        .focused_session()
        .map(|session| (session.id().clone(), session.liveness()))
}

/// Whether a phone's `Park` applies to an account in this state: only a
/// running one has a rendering process to stop, exactly where the row
/// menu's item is offered (architecture rule 8).
fn park_applies(liveness: Option<Liveness>) -> bool {
    liveness == Some(Liveness::Live)
}

/// Whether a phone's `Start` applies to an account in this state: only a
/// parked one. A starting or queued account is already on its way up, and
/// during a restore the start queue owns a queued account's turn.
fn start_applies(liveness: Option<Liveness>) -> bool {
    liveness == Some(Liveness::Parked)
}

/// Whether the frame timer photographs the current view on this tick: a
/// phone is attached, mobile mode is on so the phone is looking at a page,
/// the current account (`current`, `None` for an empty slot) is live so
/// there is a page, and the last snapshot has answered (`FR.4.2`).
fn capture_gate(
    attached: bool,
    mobile_mode: bool,
    current: Option<Liveness>,
    in_flight: bool,
) -> bool {
    attached && mobile_mode && current == Some(Liveness::Live) && !in_flight
}

/// How often the current view is photographed for the phone, in
/// milliseconds: about twelve pictures a second (code standards rule 5).
const FRAME_INTERVAL_MILLIS: u64 = 80;

/// How long a capture may go unanswered before it is called stalled, logged
/// and asked for again: many times the frame interval, so a slow snapshot on
/// a busy page is never mistaken for a dead one.
const CAPTURE_STALL_SECS: u64 = 5;

/// The mapping from a frame's pixel grid to the page's CSS pixels. The phone
/// measures its taps on the picture it was sent, whose width is the engine's
/// — on a high-density desktop the scale factor times the viewport the page
/// is laid out for — while `tap_script` and `scroll_script` speak in client
/// coordinates. Before the first frame the two grids are taken as equal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FrameScale {
    frame_width: u32,
    viewport_width: u32,
}

impl FrameScale {
    /// The scale between a frame `frame_width` pixels wide and a viewport
    /// `viewport_width` CSS pixels wide; the identity when no frame has
    /// been captured yet or either width is zero.
    fn between(frame_width: Option<u32>, viewport_width: u32) -> Self {
        match frame_width {
            Some(frame_width) if frame_width > 0 && viewport_width > 0 => Self {
                frame_width,
                viewport_width,
            },
            _ => Self {
                frame_width: 1,
                viewport_width: 1,
            },
        }
    }

    /// A point of the frame in CSS pixels, rounded to the nearest.
    fn point(self, x: u32, y: u32) -> (u32, u32) {
        (self.length(x), self.length(y))
    }

    /// A finger's movement across the frame as the wheel distance
    /// `scroll_script` adds to the page's scroll position: scaled to CSS
    /// pixels and negated, because a finger moving up drags the content up,
    /// which is a scroll down.
    fn scroll_delta(self, dx: i32, dy: i32) -> (i32, i32) {
        (-self.signed_length(dx), -self.signed_length(dy))
    }

    fn length(self, frame_pixels: u32) -> u32 {
        let scaled = u64::from(frame_pixels) * u64::from(self.viewport_width);
        let rounded = (scaled + u64::from(self.frame_width) / 2) / u64::from(self.frame_width);
        u32::try_from(rounded).unwrap_or(u32::MAX)
    }

    fn signed_length(self, frame_pixels: i32) -> i32 {
        let frame_width = i64::from(self.frame_width);
        let scaled = i64::from(frame_pixels) * i64::from(self.viewport_width);
        let rounded = (scaled + scaled.signum() * (frame_width / 2)) / frame_width;
        i32::try_from(rounded).unwrap_or(0)
    }
}

/// The engine's picture as the phone link carries it: the same two shapes,
/// named on the core's side so the link never learns the engine seam's type.
fn frame_for_phone(captured: CapturedFrame) -> Frame {
    match captured {
        CapturedFrame::Rgba {
            width,
            height,
            stride,
            bytes,
        } => Frame::Rgba {
            width,
            height,
            stride,
            bytes,
        },
        CapturedFrame::Jpeg(bytes) => Frame::Jpeg(bytes),
    }
}

/// The frame's width in its own pixels, known without decoding only for raw
/// pixels; a ready-made JPEG's is read by the link, not here.
fn frame_width(frame: &Frame) -> Option<u32> {
    match frame {
        Frame::Rgba { width, .. } => Some(*width),
        Frame::Jpeg(_) => None,
    }
}

/// `id`'s liveness, searched across every workspace `book` holds — the one
/// place a caller needs an account's state regardless of which workspace it
/// belongs to, since [`WorkspaceBook`] does not yet forward every
/// [`Session`] query itself. `None` for an id no workspace holds.
fn liveness_anywhere(book: &WorkspaceBook, id: &SessionId) -> Option<Liveness> {
    book.workspaces()
        .find_map(|workspace| workspace.book().session(id))
        .map(Session::liveness)
}

/// How long after the last zoom gesture the settled size is written, so a
/// burst of wheel notches collapses into one write (`FR.12.5`, code standards
/// rule 5). Each gesture restarts the account's timer.
const ZOOM_SAVE_SETTLE_MILLIS: u64 = 500;

/// Which way a zoom gesture steps.
// `pub(crate)`: `web_engine/webview2.rs`'s ipc handler constructs this too
// (roadmap item 12 task 03), across the module boundary but inside the one
// crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZoomStep {
    /// One step larger.
    In,
    /// One step smaller.
    Out,
    /// Back to the game file's size, forgetting the current arrangement's
    /// chosen size.
    Reset,
}

/// The one line the message strip shows for a workspace that would not load:
/// the parse failure names where the file was kept so the user can go and look;
/// an unreachable location has nothing to point at.
fn describe_read_error(error: &WorkspaceReadError) -> String {
    match error {
        WorkspaceReadError::Unreadable { kept, .. } => format!(
            "The saved workspace could not be read. It was kept aside at {} and the window below is a first run.",
            kept.display()
        ),
        WorkspaceReadError::Inaccessible { reason } => {
            format!(
                "The saved workspace could not be read: {reason}. The window below is a first run."
            )
        }
    }
}

/// The notice's line for a check that failed — manual, since an automatic
/// one is only ever logged (`run_update_check`). `error`'s own `Display` is a
/// lowercase fragment ready to embed, exactly like [`describe_read_error`]'s
/// own `reason` fields.
fn describe_check_error(error: &UpdateError) -> String {
    format!("Could not check for updates: {error}.")
}

/// The notice's line for a download or a verification that failed. A
/// rejected package names no reason of its own — the wireframe's own fixed
/// sentence, never "the update could not be verified: <detail>", since the
/// detail is a checksum or a signature mismatch the user cannot act on;
/// every other failure names `error`.
fn describe_download_error(error: &UpdateError) -> String {
    match error {
        UpdateError::Rejected { .. } => {
            "The update could not be verified and was discarded.".to_string()
        }
        other => format!("The download failed: {other}."),
    }
}

/// Turns `reason` — a lowercase, unpunctuated fragment, exactly like
/// [`UpdateChannel::last_apply_failure`]'s own contract — into the sentence
/// the message strip shows.
fn as_shown_sentence(reason: &str) -> String {
    let mut sentence = String::with_capacity(reason.len() + 1);
    let mut chars = reason.chars();
    if let Some(first) = chars.next() {
        sentence.extend(first.to_uppercase());
    }
    sentence.push_str(chars.as_str());
    if !sentence.ends_with('.') {
        sentence.push('.');
    }
    sentence
}

/// The current time in milliseconds since the Unix epoch, for
/// [`UpdateSchedule::next_check_due`] — from `glib::real_time`'s
/// microseconds rather than [`std::time::SystemTime`], since the window
/// already runs on a `glib` main loop and this keeps the one clock source.
fn now_millis() -> u64 {
    u64::try_from(glib::real_time()).unwrap_or(0) / 1_000
}

/// Setting this in the environment to a number of seconds makes the window
/// minimise itself that long after it is built (roadmap item 13 task 02's
/// minimised-snapshot measurement). A headless X server has no window manager
/// to iconify through, and a hand-run test cannot time a minimise against the
/// frame dump, so the window does it to itself. Off unless set; a value that
/// is not a number is ignored with a warning.
const DEBUG_MINIMISE_ENV: &str = "IDLE_MANAGER_MINIMISE_AFTER_SECS";

/// Arms [`DEBUG_MINIMISE_ENV`]'s timer when the variable is set.
fn arm_debug_minimise(window: &super::Window) {
    let Some(raw) = std::env::var_os(DEBUG_MINIMISE_ENV) else {
        return;
    };
    let Some(secs) = raw.to_str().and_then(|value| value.parse::<u64>().ok()) else {
        tracing::warn!(
            variable = DEBUG_MINIMISE_ENV,
            "not a number of seconds; ignored"
        );
        return;
    };
    let weak = window.downgrade();
    glib::timeout_add_local_once(Duration::from_secs(secs), move || {
        if let Some(window) = weak.upgrade() {
            tracing::info!(after_secs = secs, "debug switch: minimising the window");
            window.minimize();
        }
    });
}

/// Setting this in the environment to one of `single`, `side-by-side`,
/// `grid` or `mobile` makes the window select that layout toggle
/// [`DEBUG_LAYOUT_DELAY_SECS`] after it is built (roadmap item 13 task 03),
/// so a headless run can exercise the `Mobile` path — the grid's allocation
/// log line and the file `saved()` writes meanwhile — with nobody there to
/// press `Phone`. Off unless set; an unknown word is ignored with a warning.
const DEBUG_LAYOUT_ENV: &str = "IDLE_MANAGER_DEBUG_LAYOUT";

/// How long after the window is built [`DEBUG_LAYOUT_ENV`]'s toggle is
/// selected — long enough for the restore and the first starts to have
/// happened, so the switch acts on a populated grid.
const DEBUG_LAYOUT_DELAY_SECS: u64 = 5;

/// Arms [`DEBUG_LAYOUT_ENV`]'s timer when the variable is set.
fn arm_debug_layout(window: &super::Window) {
    let Some(raw) = std::env::var_os(DEBUG_LAYOUT_ENV) else {
        return;
    };
    let Some(layout) = raw.to_str().and_then(layout_named) else {
        tracing::warn!(
            variable = DEBUG_LAYOUT_ENV,
            "not one of single, side-by-side, grid or mobile; ignored"
        );
        return;
    };
    let weak = window.downgrade();
    glib::timeout_add_local_once(Duration::from_secs(DEBUG_LAYOUT_DELAY_SECS), move || {
        if let Some(window) = weak.upgrade() {
            tracing::info!(?layout, "debug switch: selecting the layout toggle");
            window.imp().select_layout_toggle(layout);
        }
    });
}

/// The layout [`DEBUG_LAYOUT_ENV`] names, spelt as the session file spells
/// it, or `None` for any other word.
fn layout_named(name: &str) -> Option<Layout> {
    match name {
        "single" => Some(Layout::Single),
        "side-by-side" => Some(Layout::SideBySide),
        "grid" => Some(Layout::Grid),
        "mobile" => Some(Layout::Mobile),
        _ => None,
    }
}

/// Setting this in the environment to any value makes the window begin an
/// enrolment [`DEBUG_ENROL_DELAY_SECS`] after its ports attach and log the
/// offered address (roadmap item 13 task 06), so a headless run can enrol a
/// scripted phone without a screen to read the phone dialog's code from.
/// Off unless set.
const DEBUG_ENROL_ENV: &str = "IDLE_MANAGER_DEBUG_ENROL";

/// How long after the ports attach [`DEBUG_ENROL_ENV`]'s enrolment begins —
/// long enough for the server's listening line to have been logged first.
const DEBUG_ENROL_DELAY_SECS: u64 = 3;

/// Arms [`DEBUG_ENROL_ENV`]'s timer when the variable is set.
fn arm_debug_enrol(window: &super::Window) {
    if std::env::var_os(DEBUG_ENROL_ENV).is_none() {
        return;
    }
    let weak = window.downgrade();
    glib::timeout_add_local_once(Duration::from_secs(DEBUG_ENROL_DELAY_SECS), move || {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let Some(link) = window.imp().phone_link.borrow().clone() else {
            tracing::warn!(variable = DEBUG_ENROL_ENV, "no phone link to enrol through");
            return;
        };
        let offer = link.begin_enrolment();
        tracing::info!(
            address = %offer.address,
            expires_in_secs = offer.expires_in_secs,
            "debug switch: enrolment offered"
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn park_applies_only_to_a_live_account() {
        let outcomes = [
            park_applies(Some(Liveness::Live)),
            park_applies(Some(Liveness::Parked)),
            park_applies(Some(Liveness::Starting)),
            park_applies(Some(Liveness::Queued)),
            park_applies(None),
        ];

        assert_eq!(outcomes, [true, false, false, false, false]);
    }

    #[test]
    fn start_applies_only_to_a_parked_account() {
        let outcomes = [
            start_applies(Some(Liveness::Live)),
            start_applies(Some(Liveness::Parked)),
            start_applies(Some(Liveness::Starting)),
            start_applies(Some(Liveness::Queued)),
            start_applies(None),
        ];

        assert_eq!(outcomes, [false, true, false, false, false]);
    }

    #[test]
    fn the_capture_gate_opens_only_when_attached_in_mobile_mode_on_a_live_view_with_none_in_flight()
    {
        let livenesses = [
            None,
            Some(Liveness::Live),
            Some(Liveness::Parked),
            Some(Liveness::Starting),
            Some(Liveness::Queued),
        ];

        let open: Vec<(bool, bool, Option<Liveness>, bool)> = (0..8u8)
            .flat_map(|bits| {
                livenesses
                    .into_iter()
                    .map(move |current| (bits & 1 != 0, bits & 2 != 0, current, bits & 4 != 0))
            })
            .filter(|(attached, mobile_mode, current, in_flight)| {
                capture_gate(*attached, *mobile_mode, *current, *in_flight)
            })
            .collect();

        assert_eq!(open, vec![(true, true, Some(Liveness::Live), false)]);
    }

    #[test]
    fn a_hidpi_frame_maps_a_tap_back_to_css_pixels() {
        let scale = FrameScale::between(Some(824), 412);

        assert_eq!(scale.point(200, 150), (100, 75));
    }

    #[test]
    fn before_the_first_frame_the_grids_are_taken_as_equal() {
        let scale = FrameScale::between(None, 412);

        assert_eq!(scale.point(200, 150), (200, 150));
    }

    #[test]
    fn a_finger_moving_up_scrolls_the_content_down() {
        let scale = FrameScale::between(Some(412), 412);

        assert_eq!(scale.scroll_delta(6, -40), (-6, 40));
    }

    #[test]
    fn a_scroll_on_a_hidpi_frame_is_halved_rounded_away_from_zero_and_negated() {
        let scale = FrameScale::between(Some(824), 412);

        assert_eq!(scale.scroll_delta(-30, 41), (15, -21));
    }

    #[test]
    fn raw_pixels_keep_their_size_and_stride_on_the_way_to_the_phone() {
        let frame = frame_for_phone(CapturedFrame::Rgba {
            width: 2,
            height: 1,
            stride: 12,
            bytes: vec![0; 12],
        });

        assert_eq!(
            (frame_width(&frame), frame),
            (
                Some(2),
                Frame::Rgba {
                    width: 2,
                    height: 1,
                    stride: 12,
                    bytes: vec![0; 12],
                }
            )
        );
    }

    #[test]
    fn a_ready_made_jpeg_passes_through_with_no_width_of_its_own() {
        let frame = frame_for_phone(CapturedFrame::Jpeg(vec![0xFF, 0xD8]));

        assert_eq!(
            (frame_width(&frame), frame),
            (None, Frame::Jpeg(vec![0xFF, 0xD8]))
        );
    }
}
