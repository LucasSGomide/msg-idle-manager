//! The dialog's template children and its keyboard and sensitivity wiring
//! (architecture rule 12).

use std::cell::RefCell;

use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::account_name;

/// A handler run with the typed name when the user confirms.
type ConfirmHandler = Box<dyn Fn(&str)>;

/// The composite-template backing object for [`super::RenameDialog`].
#[derive(Default, CompositeTemplate)]
#[template(resource = "/org/idlemanager/IdleManager/ui/rename-dialog.ui")]
pub struct RenameDialog {
    #[template_child]
    name_entry: TemplateChild<gtk::Entry>,
    #[template_child]
    rename_button: TemplateChild<gtk::Button>,
    #[template_child]
    cancel_button: TemplateChild<gtk::Button>,

    pub(super) on_confirmed: RefCell<Option<ConfirmHandler>>,
}

impl std::fmt::Debug for RenameDialog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenameDialog").finish_non_exhaustive()
    }
}

#[glib::object_subclass]
impl ObjectSubclass for RenameDialog {
    const NAME: &'static str = "IdleManagerRenameDialog";
    type Type = super::RenameDialog;
    type ParentType = gtk::Window;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for RenameDialog {
    fn constructed(&self) {
        self.parent_constructed();

        let obj = self.obj();

        let dialog = obj.downgrade();
        self.name_entry.connect_changed(move |_| {
            if let Some(dialog) = dialog.upgrade() {
                dialog.imp().refresh_rename_sensitivity();
            }
        });

        let dialog = obj.downgrade();
        self.rename_button.connect_clicked(move |_| {
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
    }
}

impl WidgetImpl for RenameDialog {}
impl WindowImpl for RenameDialog {}

impl RenameDialog {
    /// Fills the field with `name`, selects all of it and puts the keyboard
    /// focus there, then derives `Rename`'s starting sensitivity from it —
    /// the same trimmed-non-empty check the add-game dialog runs
    /// (`account_name`, `FR.13.3`).
    pub(super) fn set_current_name(&self, name: &str) {
        self.name_entry.set_text(name);
        self.name_entry.select_region(0, -1);
        self.name_entry.grab_focus();
        self.refresh_rename_sensitivity();
    }

    fn refresh_rename_sensitivity(&self) {
        let ready = account_name(&self.name_entry.text()).is_some();
        self.rename_button.set_sensitive(ready);
    }

    fn confirm(&self) {
        let Some(name) = account_name(&self.name_entry.text()) else {
            return;
        };

        if let Some(handler) = self.on_confirmed.borrow().as_ref() {
            handler(&name);
        }
        self.obj().close();
    }
}
