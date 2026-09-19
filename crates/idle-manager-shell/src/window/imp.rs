//! The window's template children and the wiring that turns a click into a
//! domain intent and the result back into a redraw (architecture rules 8, 12).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{
    Layout, Liveness, MoveOutcome, Preset, PresetCatalogue, ProfileLocator, ProfileRemoval,
    Session, SessionId, SlotId, WorkspaceBook, WorkspaceId, WorkspaceList, WorkspaceReadError,
    ZoomLevel, ZoomMemory, account_name, workspace_name,
};

use crate::account_deletion;
use crate::add_game_dialog::{AddGameDialog, Confirmed};
use crate::delete_account_dialog::DeleteAccountDialog;
use crate::message_strip::MessageStrip;
use crate::rename_dialog::{NameCheck, RenameDialog};
use crate::save_on_change::Saver;
use crate::session_grid::SessionGrid;
use crate::session_sidebar::{MoveTarget, SessionSidebar};
use crate::start_queue::StartQueue;
use crate::web_engine::EngineView;
use crate::web_view::{AccountSettings, SessionView, background_for};

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
    /// Removes one account's profile folder (item 11 task 08). `Arc`, like
    /// `WorkspaceStore` above, since `account_deletion::delete_account` calls
    /// it through `gio::spawn_blocking`.
    removal: RefCell<Option<Arc<dyn ProfileRemoval>>>,
    /// Whether the window is currently minimised (roadmap item 12 task 04,
    /// `FR.1.9`), watched from `realize` through the toplevel surface's own
    /// `state`. Read whenever a view is started, so it opens already marked
    /// to match, and updated by `apply_minimised` on every minimise/restore.
    minimised: Cell<bool>,
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
                imp.book.borrow_mut().active_mut().set_focused(slot);
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
        let window = self.obj().downgrade();
        key_controller.connect_key_pressed(move |_, key, _, modifiers| {
            if let Some(window) = window.upgrade()
                && window.imp().handle_shortcut_key(key, modifiers)
            {
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
        self.apply_read_outcome(read_outcome);
    }

    /// Asks for the workspace to be saved after an action changed it. Cheap —
    /// it only rearms a timer — so a caller in doubt asks (task 06). A no-op
    /// before the ports are attached.
    fn request_save(&self) {
        if let Some(saver) = self.saver.borrow().as_ref() {
            saver.request(self.book.borrow().saved());
        }
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

    /// A layout toggle was pressed. Returns early when it already names the
    /// shown workspace's layout — reached after
    /// [`Window::focus_session`] flips the toggle to match an incoming
    /// workspace, so that never triggers a second arrangement or a second
    /// save (`FR.16.4`).
    fn connect_layout_toggle(&self, toggle: &gtk::ToggleButton, layout: Layout) {
        let window = self.obj().downgrade();
        toggle.connect_toggled(move |toggle| {
            if !toggle.is_active() {
                return;
            }
            if let Some(window) = window.upgrade() {
                let imp = window.imp();
                if imp.book.borrow().active().layout() == layout {
                    return;
                }
                imp.book.borrow_mut().set_layout(layout);
                imp.redraw();
                imp.snap_zoom_for_active();
                imp.request_save();
            }
        });
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

    /// A "New workspace" window confirmed `name` for the ticked `ids`. Only
    /// on success does it leave selection mode, redraw and save; a refusal is
    /// logged and changes nothing — the dialog's own name check and
    /// `Destinations::can_create` mean this is not expected in practice (code
    /// standards rule 1).
    fn create_workspace_from_ticked(&self, name: &str, ids: &[SessionId]) {
        // See `move_ticked`'s comment: bound to a `let` so the `RefMut`
        // temporary drops before `redraw` below borrows the book again.
        let result = self.book.borrow_mut().create_workspace(name, ids);
        match result {
            Ok(_) => {
                self.sidebar.end_selection();
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
    /// timer (`FR.12.5`) — then prunes it from the sidebar's ticked set,
    /// closes `dialog`, redraws and saves.
    fn finish_account_deletion(&self, id: &SessionId, dialog: &DeleteAccountDialog) {
        self.book.borrow_mut().remove_account(id);
        if let Some(timer) = self.zoom_save_timers.borrow_mut().remove(id) {
            timer.remove();
        }
        self.sidebar.forget_ticked(id);
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
        self.sidebar.connect_move_requested(move |ids, target| {
            if let Some(window) = window.upgrade() {
                window.imp().move_ticked(&ids, target);
            }
        });

        let window = self.obj().downgrade();
        self.sidebar.connect_selection_changed(move || {
            if let Some(window) = window.upgrade() {
                window.imp().redraw();
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
    }

    /// A heading was expanded or collapsed. Stored and saved; nothing else
    /// changes, so this never redraws (`FR.16.2`).
    fn set_expanded(&self, workspace: &WorkspaceId, expanded: bool) {
        self.book.borrow_mut().set_expanded(workspace, expanded);
        self.request_save();
    }

    /// `Move to…` chose `target` for the ticked `ids`. An existing workspace
    /// moves them straight away — only on success does it leave selection
    /// mode and clear the ticks, redraw and save; no liveness changes and the
    /// shown workspace stays shown (`FR.17.8`). A refusal is logged and
    /// changes nothing; the menu never offers a destination the book would
    /// refuse, so this is not expected to be reached in practice (code
    /// standards rule 1). `MoveTarget::New` opens the "New workspace" window
    /// instead — selection mode and the ticks survive until that window
    /// itself confirms or is cancelled (`FR.17.7`).
    fn move_ticked(&self, ids: &[SessionId], target: MoveTarget) {
        let workspace = match target {
            MoveTarget::Existing(workspace) => workspace,
            MoveTarget::New => {
                self.present_workspace_name_dialog(NameDialogPurpose::Create(ids.to_vec()));
                return;
            }
        };
        // Bound to a `let` rather than matched directly on the `borrow_mut()`
        // call: a `match` scrutinee's temporaries live to the end of the
        // whole expression, so matching the `RefMut` in place would keep
        // `book` borrowed through the `Ok` arm below and panic on `redraw`'s
        // own borrow (measured 2026-09-14, the same hazard `sync`'s own
        // guard already fixed for the sidebar's tree).
        let result = self.book.borrow_mut().move_accounts(ids, &workspace);
        match result {
            Ok(()) => {
                self.sidebar.end_selection();
                self.redraw();
                self.request_save();
            }
            Err(refusal) => {
                tracing::error!(?refusal, %workspace, "could not move the ticked accounts");
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
    fn focus_session(&self, id: &SessionId) {
        let switch = self.book.borrow_mut().focus_account(id);
        if let Some(switch) = switch {
            let layout = self.book.borrow().active().layout();
            self.select_layout_toggle(layout);
            self.snap_zoom_for_active();
            tracing::debug!(from = %switch.from(), to = %switch.to(), "switched the shown workspace");
        }
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
    pub(crate) fn start_session(&self, id: &SessionId) -> Option<EngineView> {
        self.book.borrow_mut().unpark(id);
        self.redraw();

        let view = {
            let mut holders = self.holders.borrow_mut();
            let Some(holder) = holders.get_mut(id) else {
                tracing::error!(session = %id, "no holder to start");
                return None;
            };
            holder.start(self.minimised.get()).clone()
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
    /// sidebar. Searched wherever the account's workspace is, since the start
    /// queue may be starting one in a hidden workspace.
    fn finish_starting(&self, id: &SessionId) {
        if liveness_anywhere(&self.book.borrow(), id) != Some(Liveness::Starting) {
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
    }

    /// Runs the window's own keyboard shortcuts for `key` under `modifiers`:
    /// F5 / `Ctrl`+R reloads the focused view, and `Ctrl` plus a zoom key
    /// steps the focused account's zoom (item 09). Returns whether the key
    /// was consumed.
    ///
    /// Both the GTK key controller wired in `constructed` (Linux, and
    /// Windows whenever a GTK widget — not a game's `WebView2` child window —
    /// holds focus) and, on Windows, `EngineHost`'s `AcceleratorKeyPressed`
    /// subscription (roadmap item 12 task 03) call this one function, so the
    /// two engines can never disagree about what a shortcut does. `pub(crate)`
    /// for that second caller in `web_engine/webview2`, across the module
    /// boundary but inside the one crate (architecture rules 8, 12).
    pub(crate) fn handle_shortcut_key(&self, key: gdk::Key, modifiers: gdk::ModifierType) -> bool {
        let ctrl = modifiers.contains(gdk::ModifierType::CONTROL_MASK);

        if key == gdk::Key::F5 || (ctrl && key == gdk::Key::r) {
            self.grid.reload_focused();
            return true;
        }

        if ctrl && let Some(step) = zoom_step_for(key) {
            self.zoom_focused_account(step);
            return true;
        }

        false
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
        self.sidebar.sync(&book);
        self.sidebar
            .set_move_destinations(book.destinations(self.sidebar.ticked_count()));

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
#[derive(Debug, Clone, Copy)]
pub(crate) enum ZoomStep {
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
