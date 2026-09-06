//! The dialog's template children and its keyboard and sensitivity wiring
//! (architecture rule 12).

use std::cell::RefCell;

use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

/// A handler run with the entered name and address when the user confirms.
type ConfirmHandler = Box<dyn Fn(&str, &str)>;

/// The composite-template backing object for [`super::AddGameDialog`].
#[derive(Default, CompositeTemplate)]
#[template(resource = "/org/idlemanager/IdleManager/ui/add-game-dialog.ui")]
pub struct AddGameDialog {
    #[template_child]
    name_entry: TemplateChild<gtk::Entry>,
    #[template_child]
    address_entry: TemplateChild<gtk::Entry>,
    #[template_child]
    add_button: TemplateChild<gtk::Button>,
    #[template_child]
    cancel_button: TemplateChild<gtk::Button>,
    pub(super) on_confirmed: RefCell<Option<ConfirmHandler>>,
}

impl std::fmt::Debug for AddGameDialog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AddGameDialog").finish_non_exhaustive()
    }
}

#[glib::object_subclass]
impl ObjectSubclass for AddGameDialog {
    const NAME: &'static str = "IdleManagerAddGameDialog";
    type Type = super::AddGameDialog;
    type ParentType = gtk::Window;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for AddGameDialog {
    fn constructed(&self) {
        self.parent_constructed();

        let obj = self.obj();

        for entry in [self.name_entry.get(), self.address_entry.get()] {
            let dialog = obj.downgrade();
            entry.connect_changed(move |_| {
                if let Some(dialog) = dialog.upgrade() {
                    dialog.imp().refresh_add_sensitivity();
                }
            });
        }

        let dialog = obj.downgrade();
        self.add_button.connect_clicked(move |_| {
            if let Some(dialog) = dialog.upgrade() {
                dialog.imp().confirm();
            }
        });

        let dialog = obj.downgrade();
        self.cancel_button.connect_clicked(move |_| {
            if let Some(dialog) = dialog.upgrade() {
                dialog.close();
            }
        });

        let escape = gtk::EventControllerKey::new();
        let dialog = obj.downgrade();
        escape.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                if let Some(dialog) = dialog.upgrade() {
                    dialog.close();
                }
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        obj.add_controller(escape);

        self.name_entry.grab_focus();
    }
}

impl WidgetImpl for AddGameDialog {}
impl WindowImpl for AddGameDialog {}

impl AddGameDialog {
    fn refresh_add_sensitivity(&self) {
        let filled = !self.name_entry.text().trim().is_empty()
            && !self.address_entry.text().trim().is_empty();
        self.add_button.set_sensitive(filled);
    }

    fn confirm(&self) {
        let name = self.name_entry.text();
        let address = self.address_entry.text();
        if let Some(handler) = self.on_confirmed.borrow().as_ref() {
            handler(name.trim(), address.trim());
        }
        self.obj().close();
    }
}
