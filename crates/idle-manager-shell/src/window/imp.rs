//! The window's template children and the wiring that turns a click into a
//! domain intent and the result back into a redraw (architecture rules 8, 12).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;
use webkit6::LoadEvent;
use webkit6::prelude::WebViewExt;

use idle_manager_core::{Layout, Liveness, ProfileLocator, Session, SessionBook, SessionId};

use crate::add_game_dialog::AddGameDialog;
use crate::session_grid::SessionGrid;
use crate::session_sidebar::SessionSidebar;
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
    content: TemplateChild<gtk::Box>,
    #[template_child]
    empty_state: TemplateChild<gtk::Box>,

    grid: SessionGrid,
    sidebar: SessionSidebar,
    book: RefCell<SessionBook>,
    locator: RefCell<Option<Rc<dyn ProfileLocator>>>,
    /// One holder per account, owning its network session for the account's
    /// whole life and its view only while it is running. The grid holds its own
    /// reference to the same view; this map is what a later slice asks to stop
    /// or start.
    holders: RefCell<HashMap<SessionId, SessionView>>,
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

        // The parked slot's own Start button routes through the same intent, so
        // the row and the panel never run two starts (architecture rule 8).
        let window = self.obj().downgrade();
        self.grid.connect_start_requested(move |id| {
            if let Some(window) = window.upgrade() {
                window.imp().toggle_parking(&id);
            }
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

        let reload_keys = gtk::EventControllerKey::new();
        reload_keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let grid = self.grid.clone();
        reload_keys.connect_key_pressed(move |_, key, _, modifiers| {
            let ctrl_r = key == gdk::Key::r && modifiers.contains(gdk::ModifierType::CONTROL_MASK);
            if key == gdk::Key::F5 || ctrl_r {
                grid.reload_focused();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        self.obj().add_controller(reload_keys);

        self.connect_layout_toggle(&self.layout_single, Layout::Single);
        self.connect_layout_toggle(&self.layout_side_by_side, Layout::SideBySide);
        self.connect_layout_toggle(&self.layout_grid, Layout::Grid);

        self.redraw();
    }
}

impl WidgetImpl for Window {}
impl WindowImpl for Window {}
impl ApplicationWindowImpl for Window {}

impl Window {
    pub(super) fn attach_locator(&self, locator: Rc<dyn ProfileLocator>) {
        self.locator.replace(Some(locator));
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
            }
        });
    }

    fn present_add_game_dialog(&self) {
        let dialog = AddGameDialog::new();
        dialog.set_transient_for(Some(&*self.obj()));

        let window = self.obj().downgrade();
        dialog.connect_confirmed(move |name, address| {
            if let Some(window) = window.upgrade() {
                window.imp().create_account(name, address);
            }
        });

        dialog.present();
    }

    fn create_account(&self, name: &str, address: &str) {
        let Some(locator) = self.locator.borrow().clone() else {
            tracing::error!("no profile locator attached; cannot create an account");
            return;
        };

        let id = self.book.borrow_mut().add(name, address);

        let directories = match locator.locate(&id) {
            Ok(directories) => directories,
            Err(error) => {
                tracing::error!(session = %id, %error, "could not prepare the profile directories");
                return;
            }
        };

        let holder = SessionView::new(&directories.data, &directories.cache, address);
        let Some(view) = holder.view() else {
            tracing::error!(session = %id, "the new account's view was not built");
            return;
        };
        self.grid.add_session(&id, name, view);
        self.holders.borrow_mut().insert(id, holder);
        self.redraw();
    }

    fn focus_session(&self, id: &SessionId) {
        self.book.borrow_mut().focus_session(id);
        self.redraw();
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
            Some(Liveness::Live) => self.park_session(id),
            Some(Liveness::Parked) => self.start_session(id),
            // Starting keeps its button insensitive, so a press that still
            // arrives is a stale event; an unknown id has nothing to toggle.
            Some(Liveness::Starting) | None => {}
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

    /// Starts a parked account, in the order the roadmap item's second diagram
    /// fixes: unpark in the book (which returns `Starting`, since no page has
    /// painted), redraw so the row shows it and the button goes insensitive,
    /// then build a new view against the kept network session and hand it to
    /// the grid (architecture rule 8).
    fn start_session(&self, id: &SessionId) {
        self.book.borrow_mut().unpark(id);
        self.redraw();

        let view = {
            let mut holders = self.holders.borrow_mut();
            let Some(holder) = holders.get_mut(id) else {
                tracing::error!(session = %id, "no holder to start");
                return;
            };
            holder.start().clone()
        };

        self.grid.attach_view(id, &view);

        // The starting interval ends at the new view's first commit — the same
        // signal the grid uses to drop the cover. Which load event counts as
        // "first paint" is the roadmap item's fourth blocker and has to be
        // checked against a real game.
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

    fn redraw(&self) {
        let book = self.book.borrow();
        self.grid.sync(&book);
        self.sidebar.sync(&book);

        let empty = book.sessions().is_empty();
        self.empty_state.set_visible(empty);
        self.grid.set_visible(!empty);
    }
}
