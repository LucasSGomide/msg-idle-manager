//! The window's template children and the wiring that turns a click into a
//! domain intent and the result back into a redraw (architecture rules 8, 12).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;
use webkit6::prelude::WebViewExt;
use webkit6::{LoadEvent, WebView};

use idle_manager_core::{
    Layout, Liveness, Preset, PresetCatalogue, ProfileLocator, Session, SessionBook, SessionId,
    Workspace, WorkspaceReadError, ZoomLevel, ZoomMemory,
};

use crate::add_game_dialog::{AddGameDialog, Confirmed};
use crate::message_strip::MessageStrip;
use crate::save_on_change::Saver;
use crate::session_grid::SessionGrid;
use crate::session_sidebar::SessionSidebar;
use crate::start_queue::StartQueue;
use crate::web_view::SessionView;

/// The composite-template backing object for [`super::Window`].
#[derive(Default, CompositeTemplate)]
#[template(resource = "/org/idlemanager/IdleManager/ui/window.ui")]
pub struct Window {
    #[template_child]
    add_game_button: TemplateChild<gtk::Button>,
    #[template_child]
    reload_button: TemplateChild<gtk::Button>,
    #[template_child]
    add_first_game_button: TemplateChild<gtk::Button>,
    #[template_child]
    layout_single: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    layout_side_by_side: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    layout_grid: TemplateChild<gtk::ToggleButton>,
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

    /// The window-level message bar under the header bar (design rule 9). Hidden
    /// until a workspace fails to load (task 04) or, later, a save fails
    /// (task 06).
    message_strip: MessageStrip,
    grid: SessionGrid,
    sidebar: SessionSidebar,
    book: RefCell<SessionBook>,
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
    /// The queue that brings restored accounts up one at a time, alive only
    /// while a restore is draining (task 05).
    start_queue: RefCell<Option<StartQueue>>,
    /// Reads and stores each account's chosen zoom sizes (item 09 task 07).
    /// The shell never learns a file is behind it.
    zoom_memory: RefCell<Option<Rc<dyn ZoomMemory>>>,
    /// One settle timer per account: a zoom gesture rearms that account's
    /// timer, and only when the gestures stop does the settled map get written
    /// once (`FR.12.5`).
    zoom_save_timers: RefCell<HashMap<SessionId, glib::SourceId>>,
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

        self.grid.set_hexpand(true);
        self.grid.set_vexpand(true);
        self.content.append(&self.grid);

        let window = self.obj().downgrade();
        self.grid.connect_slot_focused(move |slot| {
            if let Some(window) = window.upgrade() {
                let imp = window.imp();
                imp.book.borrow_mut().set_focused(slot);
                // The sidebar marks the focused-slot row as current, so a focus
                // change made in the grid has to reach it too.
                imp.redraw();
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

        // The parked slot's own Start button routes through the same intent, so
        // the row and the panel never run two starts (architecture rule 8).
        let window = self.obj().downgrade();
        self.grid.connect_start_requested(move |id| {
            if let Some(window) = window.upgrade() {
                window.imp().toggle_parking(&id);
            }
        });

        // The wheel half of the zoom gesture (task 05): it acts on the account
        // the pointer is over, and lands in the same window method as the
        // keyboard half so the domain is the only thing that decides what a
        // step means (architecture rule 8). A notch up (negative delta) is a
        // step in.
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

        for button in [self.add_game_button.get(), self.add_first_game_button.get()] {
            let window = self.obj().downgrade();
            button.connect_clicked(move |_| {
                if let Some(window) = window.upgrade() {
                    window.imp().present_add_game_dialog();
                }
            });
        }

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

        // The zoom gesture (item 09) joins this one controller rather than
        // adding a second, so one place decides what a keypress means. Capture
        // phase because `FR.11.1` says neither the reload nor the zoom keys are
        // gated on a web view holding keyboard focus — a game that binds them
        // on its own canvas must not swallow them first.
        let key_controller = gtk::EventControllerKey::new();
        key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        let grid = self.grid.clone();
        let window = self.obj().downgrade();
        key_controller.connect_key_pressed(move |_, key, _, modifiers| {
            let ctrl = modifiers.contains(gdk::ModifierType::CONTROL_MASK);

            if key == gdk::Key::F5 || (ctrl && key == gdk::Key::r) {
                grid.reload_focused();
                return glib::Propagation::Stop;
            }

            if ctrl && let Some(step) = zoom_step_for(key) {
                if let Some(window) = window.upgrade() {
                    window.imp().zoom_focused_account(step);
                }
                return glib::Propagation::Stop;
            }

            glib::Propagation::Proceed
        });
        self.obj().add_controller(key_controller);

        self.connect_layout_toggle(&self.layout_single, Layout::Single);
        self.connect_layout_toggle(&self.layout_side_by_side, Layout::SideBySide);
        self.connect_layout_toggle(&self.layout_grid, Layout::Grid);

        // The one place a save is allowed to be waited on: a change made a
        // moment before quitting has nothing else to trigger its write, so
        // closing finishes any pending one first (task 06).
        let window = self.obj().downgrade();
        self.obj().connect_close_request(move |_| {
            if let Some(window) = window.upgrade()
                && let Some(saver) = window.imp().saver.borrow().as_ref()
            {
                saver.flush();
            }
            glib::Propagation::Proceed
        });

        self.redraw();
    }
}

impl WidgetImpl for Window {}
impl WindowImpl for Window {}
impl ApplicationWindowImpl for Window {}

impl Window {
    pub(super) fn attach_ports(
        &self,
        ports: super::WindowPorts,
        read_outcome: Result<Option<Workspace>, WorkspaceReadError>,
    ) {
        self.locator.replace(Some(ports.locator));
        self.catalogue.replace(Some(ports.catalogue));
        self.zoom_memory.replace(Some(ports.zoom_memory));
        self.saver
            .replace(Some(Saver::new(ports.store, self.message_strip.clone())));
        self.apply_read_outcome(read_outcome);
    }

    /// Asks for the workspace to be saved after an action changed it. Cheap —
    /// it only rearms a timer — so a caller in doubt asks (task 06). A no-op
    /// before the ports are attached.
    fn request_save(&self) {
        if let Some(saver) = self.saver.borrow().as_ref() {
            saver.request(self.book.borrow().workspace());
        }
    }

    /// Acts on what the composition root read before the window was built: a
    /// saved workspace to restore, no file (a first run), or a failure. A
    /// failure is never flattened into "no file" — it is logged in fields and
    /// shown in the strip, never dropped (code standards rules 14, 15).
    fn apply_read_outcome(&self, outcome: Result<Option<Workspace>, WorkspaceReadError>) {
        match outcome {
            Ok(None) => {
                tracing::info!("no saved workspace; opening a first run");
            }
            Ok(Some(workspace)) => self.restore_workspace(workspace),
            Err(error) => {
                tracing::warn!(error = %error, "the saved workspace could not be read");
                self.message_strip.show(&describe_read_error(&error));
            }
        }
    }

    /// Rebuilds the book from `workspace`, prepares each account's profile
    /// directories, hands the grid a dormant holder at each saved placement,
    /// and redraws once — list, arrangement and every slot — before anything
    /// loads. An account whose profile cannot be prepared is logged and skipped,
    /// not the whole restore abandoned. The start queue (task 05) brings the
    /// running accounts up one at a time afterwards.
    fn restore_workspace(&self, workspace: Workspace) {
        let Some(locator) = self.locator.borrow().clone() else {
            tracing::error!("no profile locator attached; cannot restore the workspace");
            return;
        };

        *self.book.borrow_mut() = SessionBook::restore(workspace);

        // Install every account's chosen sizes onto the book before any holder
        // is built, so a relaunched account is right before it is ever drawn
        // and the start queue does not have to correct it (`FR.12.7`).
        if let Some(memory) = self.zoom_memory.borrow().clone() {
            let ids: Vec<SessionId> = self
                .book
                .borrow()
                .sessions()
                .iter()
                .map(|session| session.id().clone())
                .collect();
            for id in ids {
                let remembered = memory.read(&id);
                self.book.borrow_mut().restore_zoom(&id, remembered);
            }
        }

        // Each dormant holder opens at the size resolved for the arrangement
        // being restored, not the game-file baseline (`FR.12.2`): a switch
        // later re-resolves, but the first draw is already right.
        let layout = self.book.borrow().layout();
        let accounts: Vec<(SessionId, String, String, ZoomLevel, Option<String>)> = self
            .book
            .borrow()
            .sessions()
            .iter()
            .map(|session| {
                (
                    session.id().clone(),
                    session.display_name().to_owned(),
                    session.start_address().to_owned(),
                    session.zoom_for(layout),
                    session.browser_identity().map(str::to_owned),
                )
            })
            .collect();

        for (id, name, address, zoom, identity) in accounts {
            let directories = match locator.locate(&id) {
                Ok(directories) => directories,
                Err(error) => {
                    tracing::error!(session = %id, %error, "could not prepare the profile directories; skipping this account");
                    continue;
                }
            };

            let holder =
                SessionView::dormant(&id, &directories, &address, zoom, identity.as_deref());
            self.grid.add_dormant_session(&id, &name);
            self.holders.borrow_mut().insert(id, holder);
        }

        self.redraw();

        // The header's layout toggles start on "1" from the template; the
        // restored book may be arranged for another layout, and "that
        // arrangement selected" is part of a truthful restore. Done after
        // `redraw` and outside any `book` borrow, since flipping a toggle runs
        // its handler synchronously.
        let layout = self.book.borrow().layout();
        self.select_layout_toggle(layout);

        tracing::info!(accounts = self.holders.borrow().len(), "workspace restored");

        // The arrangement is drawn; now bring the running accounts back one at
        // a time (task 05). Nothing queued means nothing to do.
        let order = self.book.borrow().start_order();
        if !order.is_empty() {
            self.start_queue
                .replace(Some(StartQueue::begin(self.obj().downgrade(), order)));
        }
    }

    /// Sets the header's layout toggle group to `layout` without the user
    /// touching it — for a restore, where the book's layout is set directly.
    fn select_layout_toggle(&self, layout: Layout) {
        let toggle = match layout {
            Layout::Single => &self.layout_single,
            Layout::SideBySide => &self.layout_side_by_side,
            Layout::Grid => &self.layout_grid,
        };
        toggle.set_active(true);
    }

    fn connect_layout_toggle(&self, toggle: &gtk::ToggleButton, layout: Layout) {
        let window = self.obj().downgrade();
        toggle.connect_toggled(move |toggle| {
            if !toggle.is_active() {
                return;
            }
            if let Some(window) = window.upgrade() {
                let imp = window.imp();
                imp.book.borrow_mut().set_layout(layout);
                imp.redraw();
                imp.snap_all_zoom();
                imp.request_save();
            }
        });
    }

    /// After an arrangement switch, redraw every account at the size it last
    /// chose for the arrangement now in force, falling back to the game file's
    /// size. No readout is shown — nobody asked for a size change, the
    /// arrangement did (`FR.11.6`). Every account with a holder, not only the
    /// visible ones: an off-grid or parked account is then already the right
    /// size the moment it is next brought into a place, with no second code
    /// path and no visible correction (`FR.11.7`). Nothing is written — a
    /// switch consumes chosen sizes and never records one (`FR.12.4`).
    fn snap_all_zoom(&self) {
        let layout = self.book.borrow().layout();
        let resolved: Vec<(SessionId, ZoomLevel)> = self
            .book
            .borrow()
            .sessions()
            .iter()
            .map(|session| (session.id().clone(), session.zoom_for(layout)))
            .collect();

        let mut holders = self.holders.borrow_mut();
        for (id, zoom) in resolved {
            if let Some(holder) = holders.get_mut(&id) {
                holder.set_zoom(zoom);
            }
        }
    }

    fn present_add_game_dialog(&self) {
        let Some(catalogue) = self.catalogue.borrow().clone() else {
            tracing::error!("no preset catalogue attached; cannot open the add-game dialog");
            return;
        };

        let dialog = AddGameDialog::new(catalogue.as_ref());
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
                } => window
                    .imp()
                    .create_account_from_preset(preset, account_name),
                Confirmed::Custom { name, address } => window.imp().create_account(name, address),
            }
        });

        dialog.present();
    }

    /// Adds an account from a typed name and address, then builds its view.
    fn create_account(&self, name: &str, address: &str) {
        let id = self.book.borrow_mut().add(name, address);
        self.realise_account(&id);
    }

    /// Adds an account for `preset`'s game under `account_name` — the book
    /// copies the game's address, zoom, browser identity and keep-awake default
    /// onto it — then builds its view.
    fn create_account_from_preset(&self, preset: &Preset, account_name: &str) {
        let id = self.book.borrow_mut().add_from_preset(account_name, preset);
        self.realise_account(&id);
    }

    /// Prepares the profile directories for a just-added account and hands its
    /// first view to the grid. Reads the account's name, address, zoom and
    /// browser identity off the book, never off a preset kept on the side —
    /// which is what makes the account independent of the file it came from.
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
            self.book.borrow_mut().restore_zoom(id, remembered);
        }

        let layout = self.book.borrow().layout();
        let Some((name, address, zoom, identity)) =
            self.book.borrow().sessions().iter().find_map(|session| {
                (session.id() == id).then(|| {
                    (
                        session.display_name().to_owned(),
                        session.start_address().to_owned(),
                        session.zoom_for(layout),
                        session.browser_identity().map(str::to_owned),
                    )
                })
            })
        else {
            tracing::error!(session = %id, "the account is not in the book");
            return;
        };

        let directories = match locator.locate(id) {
            Ok(directories) => directories,
            Err(error) => {
                tracing::error!(session = %id, %error, "could not prepare the profile directories");
                return;
            }
        };

        let holder = SessionView::new(id, &directories, &address, zoom, identity.as_deref());
        let Some(view) = holder.view() else {
            tracing::error!(session = %id, "the new account's view was not built");
            return;
        };
        self.grid.add_session(id, &name, view);
        self.holders.borrow_mut().insert(id.clone(), holder);
        self.redraw();
        self.request_save();
    }

    fn focus_session(&self, id: &SessionId) {
        self.book.borrow_mut().focus_session(id);
        self.redraw();
        self.request_save();
    }

    /// The park/start button was pressed. The direction is the book's to
    /// decide from the account's current liveness — the row carries none
    /// (architecture rule 8).
    fn toggle_parking(&self, id: &SessionId) {
        let liveness = self
            .book
            .borrow()
            .sessions()
            .iter()
            .find(|session| session.id() == id)
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
        if let Some(holder) = self.holders.borrow_mut().get_mut(id) {
            holder.stop();
        }
        self.grid.release_view(id);
        self.redraw();
    }

    /// Starts a parked or queued account, in the order the roadmap item's second
    /// diagram fixes: unpark in the book (which returns `Starting`, since no page
    /// has painted), redraw so the row shows it and the button goes insensitive,
    /// then build a new view against the kept network session and hand it to the
    /// grid (architecture rule 8). Returns the new view, or `None` when the
    /// account has no holder to start. The start queue (task 05) is the only
    /// caller that reads the return.
    pub(crate) fn start_session(&self, id: &SessionId) -> Option<WebView> {
        self.book.borrow_mut().unpark(id);
        self.redraw();

        let view = {
            let mut holders = self.holders.borrow_mut();
            let Some(holder) = holders.get_mut(id) else {
                tracing::error!(session = %id, "no holder to start");
                return None;
            };
            holder.start().clone()
        };

        self.grid.attach_view(id, &view);

        // The starting interval ends at the new view's first commit — the same
        // signal the grid uses to drop the cover. Which load event counts as
        // "first paint" is the roadmap item's fourth blocker and has to be
        // checked against a real game.
        let window = self.obj().downgrade();
        let owned_id = id.clone();
        view.connect_load_changed(move |_, event| {
            if !matches!(event, LoadEvent::Committed | LoadEvent::Finished) {
                return;
            }
            if let Some(window) = window.upgrade() {
                window.imp().finish_starting(&owned_id);
            }
        });

        Some(view)
    }

    /// Whether the book still reports `id` as [`Liveness::Queued`]. The start
    /// queue asks before each turn, so a state that changed while it was
    /// draining is never overwritten by a start nobody asked for.
    pub(crate) fn is_queued(&self, id: &SessionId) -> bool {
        self.book
            .borrow()
            .sessions()
            .iter()
            .any(|session| session.id() == id && session.liveness() == Liveness::Queued)
    }

    /// The shell reports a started account's first paint: end its starting
    /// interval in the book and redraw. A no-op unless the account is actually
    /// starting, so a later navigation's load event does not churn the sidebar.
    fn finish_starting(&self, id: &SessionId) {
        let starting = self
            .book
            .borrow()
            .sessions()
            .iter()
            .find(|session| session.id() == id)
            .map(Session::liveness)
            == Some(Liveness::Starting);
        if !starting {
            return;
        }

        self.book.borrow_mut().mark_started(id);
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
            holder.set_keep_awake(value).cloned()
        };

        // Parked: nothing to reload now. `SessionView::start` applies the
        // remembered value the next time a view is built for this account.
        let Some(view) = view else {
            return;
        };

        // Same signal, same reason as `start_session`: the starting interval
        // this toggle opened ends at the reload's first paint.
        let window = self.obj().downgrade();
        let id = id.clone();
        view.connect_load_changed(move |_, event| {
            if !matches!(event, LoadEvent::Committed | LoadEvent::Finished) {
                return;
            }
            if let Some(window) = window.upgrade() {
                window.imp().finish_starting(&id);
            }
        });
    }

    /// A window-level zoom gesture from the keyboard: step the account in the
    /// focused slot and show the figure over that place. A no-op when the grid
    /// is empty or the focused slot holds nothing — the book returns `None` and
    /// nothing is drawn (`FR.11.7`, item 09 wireframe).
    fn zoom_focused_account(&self, step: ZoomStep) {
        let Some(id) = self
            .book
            .borrow()
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
    fn apply_zoom_step(&self, id: &SessionId, step: ZoomStep) {
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
            .sessions()
            .iter()
            .find(|session| session.id() == id)
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
        self.sidebar.sync(&book);

        let empty = book.sessions().is_empty();
        self.empty_state.set_visible(empty);
        self.grid.set_visible(!empty);
    }
}

/// How long after the last zoom gesture the settled size is written, so a
/// burst of wheel notches collapses into one write (`FR.12.5`, code standards
/// rule 5). Each gesture restarts the account's timer.
const ZOOM_SAVE_SETTLE_MILLIS: u64 = 500;

/// Which way a zoom gesture steps.
#[derive(Debug, Clone, Copy)]
pub(super) enum ZoomStep {
    /// One step larger.
    In,
    /// One step smaller.
    Out,
    /// Back to the game file's size, forgetting the current arrangement's
    /// chosen size.
    Reset,
}

/// The zoom step a key names under the control modifier, or `None` for any
/// other key.
///
/// Plus, equals and keypad-add all mean "in" because which one a keyboard
/// delivers for `Ctrl`+`+` depends on its layout; minus and keypad-subtract
/// mean "out"; zero and keypad-zero mean reset. The exact set the keyboard
/// under test delivers is recorded in this item's `test-script.md` (code
/// standards rule 18).
fn zoom_step_for(key: gdk::Key) -> Option<ZoomStep> {
    match key {
        gdk::Key::plus | gdk::Key::equal | gdk::Key::KP_Add => Some(ZoomStep::In),
        gdk::Key::minus | gdk::Key::KP_Subtract => Some(ZoomStep::Out),
        gdk::Key::_0 | gdk::Key::KP_0 => Some(ZoomStep::Reset),
        _ => None,
    }
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
