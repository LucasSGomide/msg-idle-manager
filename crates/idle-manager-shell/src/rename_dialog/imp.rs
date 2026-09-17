//! The dialog's template children and its keyboard and sensitivity wiring
//! (architecture rule 12).

use std::cell::RefCell;

use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use super::NameCheck;

/// A handler run with the typed name when the user confirms.
type ConfirmHandler = Box<dyn Fn(&str)>;

/// The check a confirm attempt runs against the current field text, supplied
/// by the caller: `account_name` for renaming an account, which never answers
/// [`NameCheck::Taken`], or a closure over `WorkspaceBook`'s own name rule for
/// naming a workspace.
type NameChecker = Box<dyn Fn(&str) -> NameCheck>;

/// The composite-template backing object for [`super::RenameDialog`].
#[derive(CompositeTemplate)]
#[template(resource = "/org/idlemanager/IdleManager/ui/rename-dialog.ui")]
pub struct RenameDialog {
    #[template_child]
    name_entry: TemplateChild<gtk::Entry>,
    #[template_child]
    taken_label: TemplateChild<gtk::Label>,
    #[template_child]
    rename_button: TemplateChild<gtk::Button>,
    #[template_child]
    cancel_button: TemplateChild<gtk::Button>,

    check: RefCell<NameChecker>,
    pub(super) on_confirmed: RefCell<Option<ConfirmHandler>>,
}

impl Default for RenameDialog {
    fn default() -> Self {
        Self {
            name_entry: TemplateChild::default(),
            taken_label: TemplateChild::default(),
            rename_button: TemplateChild::default(),
            cancel_button: TemplateChild::default(),
            check: RefCell::new(Box::new(|_| NameCheck::Ok)),
            on_confirmed: RefCell::new(None),
        }
    }
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
                dialog.imp().refresh_from_check();
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
    /// Sets the window's title and confirm button label, fills the field with
    /// `current_name`, selects all of it and puts the keyboard focus there,
    /// installs `check`, then derives the confirm button's starting
    /// sensitivity and the dim "taken" line from it.
    pub(super) fn configure(
        &self,
        title: &str,
        confirm_label: &str,
        current_name: &str,
        check: NameChecker,
    ) {
        self.obj().set_title(Some(title));
        self.rename_button.set_label(confirm_label);
        drop(self.check.replace(check));

        self.name_entry.set_text(current_name);
        self.name_entry.select_region(0, -1);
        self.name_entry.grab_focus();
        self.refresh_from_check();
    }

    fn refresh_from_check(&self) {
        let text = self.name_entry.text();
        let result = (self.check.borrow())(&text);
        self.rename_button
            .set_sensitive(matches!(result, NameCheck::Ok));
        self.taken_label
            .set_visible(matches!(result, NameCheck::Taken));
    }

    fn confirm(&self) {
        let text = self.name_entry.text();
        if !matches!((self.check.borrow())(&text), NameCheck::Ok) {
            return;
        }
        let trimmed = text.trim().to_owned();

        if let Some(handler) = self.on_confirmed.borrow().as_ref() {
            handler(&trimmed);
        }
        self.obj().close();
    }
}
