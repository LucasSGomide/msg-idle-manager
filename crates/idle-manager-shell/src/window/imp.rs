//! The window's template children and the wiring that turns a click into a
//! domain intent and the result back into a redraw (architecture rules 8, 12).

use std::cell::RefCell;
use std::rc::Rc;

use gtk::CompositeTemplate;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{Layout, ProfileLocator, SessionBook};

use crate::add_game_dialog::AddGameDialog;
use crate::session_grid::SessionGrid;
use crate::web_view;

/// The composite-template backing object for [`super::Window`].
#[derive(Default, CompositeTemplate)]
#[template(resource = "/org/idlemanager/IdleManager/ui/window.ui")]
pub struct Window {
    #[template_child]
    add_game_button: TemplateChild<gtk::Button>,
    #[template_child]
    add_first_game_button: TemplateChild<gtk::Button>,
    #[template_child]
    layout_single: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    layout_side_by_side: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    layout_grid: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    content: TemplateChild<gtk::Box>,
    #[template_child]
    empty_state: TemplateChild<gtk::Box>,

    grid: SessionGrid,
    book: RefCell<SessionBook>,
    locator: RefCell<Option<Rc<dyn ProfileLocator>>>,
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
                window.imp().book.borrow_mut().set_focused(slot);
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

        let view = web_view::build(&directories.data, &directories.cache, address);
        self.grid.add_session(&id, name, &view);
        self.redraw();
    }

    fn redraw(&self) {
        let book = self.book.borrow();
        self.grid.sync(&book);

        let empty = book.sessions().is_empty();
        self.empty_state.set_visible(empty);
        self.grid.set_visible(!empty);
    }
}
