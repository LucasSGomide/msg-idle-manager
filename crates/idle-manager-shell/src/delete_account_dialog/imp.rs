//! The dialog's template children and the wiring between its three pages
//! (architecture rule 12).

use std::cell::{Cell, RefCell};
use std::path::Path;

use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

/// The page name [`gtk::Stack`] shows while asking for confirmation.
const CONFIRM_PAGE: &str = "confirm";
/// The page name shown while the deletion sequence runs — no buttons, and
/// the one page that refuses to close (`FR.21.9`).
const WORKING_PAGE: &str = "working";
/// The page name shown when the sequence fails.
const FAILED_PAGE: &str = "failed";

/// A handler run with no arguments when the user confirms or retries.
type ActionHandler = Box<dyn Fn()>;

/// The composite-template backing object for [`super::DeleteAccountDialog`].
#[derive(Default, CompositeTemplate)]
#[template(resource = "/org/idlemanager/IdleManager/ui/delete-account-dialog.ui")]
pub struct DeleteAccountDialog {
    #[template_child]
    stages: TemplateChild<gtk::Stack>,
    #[template_child]
    confirm_sentence: TemplateChild<gtk::Label>,
    #[template_child]
    cancel_button: TemplateChild<gtk::Button>,
    #[template_child]
    delete_button: TemplateChild<gtk::Button>,
    #[template_child]
    working_label: TemplateChild<gtk::Label>,
    #[template_child]
    failed_sentence: TemplateChild<gtk::Label>,
    #[template_child]
    reason_label: TemplateChild<gtk::Label>,
    #[template_child]
    folder_label: TemplateChild<gtk::Label>,
    #[template_child]
    retry_button: TemplateChild<gtk::Button>,
    #[template_child]
    close_button: TemplateChild<gtk::Button>,

    /// Set only by [`DeleteAccountDialog::close_on_success`], so a
    /// programmatic close on success bypasses the `working`-page guard below
    /// that exists to stop a *person* from closing the window mid-deletion —
    /// without this, that guard would block the app's own close too, since
    /// `gtk_window_close` raises the same `close-request` either way and the
    /// dialog is still on `working` right up to the moment success closes it.
    allow_close: Cell<bool>,

    pub(super) on_confirmed: RefCell<Option<ActionHandler>>,
    pub(super) on_retry: RefCell<Option<ActionHandler>>,
    pub(super) on_closed_after_failure: RefCell<Option<ActionHandler>>,
}

impl std::fmt::Debug for DeleteAccountDialog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeleteAccountDialog")
            .finish_non_exhaustive()
    }
}

#[glib::object_subclass]
impl ObjectSubclass for DeleteAccountDialog {
    const NAME: &'static str = "IdleManagerDeleteAccountDialog";
    type Type = super::DeleteAccountDialog;
    type ParentType = gtk::Window;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for DeleteAccountDialog {
    fn constructed(&self) {
        self.parent_constructed();

        let obj = self.obj();

        let dialog = obj.downgrade();
        self.cancel_button.connect_clicked(move |_| {
            if let Some(dialog) = dialog.upgrade() {
                dialog.close();
            }
        });

        let dialog = obj.downgrade();
        self.delete_button.connect_clicked(move |_| {
            if let Some(dialog) = dialog.upgrade() {
                dialog.imp().confirm();
            }
        });

        let dialog = obj.downgrade();
        self.retry_button.connect_clicked(move |_| {
            if let Some(dialog) = dialog.upgrade() {
                dialog.imp().retry();
            }
        });

        let dialog = obj.downgrade();
        self.close_button.connect_clicked(move |_| {
            if let Some(dialog) = dialog.upgrade() {
                dialog.close();
            }
        });

        // The one gate for every way this window can end: the `working` page
        // accepts no close at all — not the close button, not Escape below —
        // because by the time it shows, website data has already been wiped
        // and there is no half-finished state worth preserving (`FR.21.9`,
        // "Behind the spinner"). Closing from `failed` counts as `Close`
        // exactly once, whether a person clicked the button or the window
        // itself; closing from `confirm` (nothing has happened yet) reports
        // nothing.
        let dialog = obj.downgrade();
        self.obj().connect_close_request(move |_| {
            let Some(dialog) = dialog.upgrade() else {
                return glib::Propagation::Proceed;
            };
            let imp = dialog.imp();
            if imp.allow_close.get() {
                return glib::Propagation::Proceed;
            }
            match imp.stages.visible_child_name().as_deref() {
                Some(WORKING_PAGE) => glib::Propagation::Stop,
                Some(FAILED_PAGE) => {
                    if let Some(handler) = imp.on_closed_after_failure.borrow().as_ref() {
                        handler();
                    }
                    glib::Propagation::Proceed
                }
                _ => glib::Propagation::Proceed,
            }
        });

        let escape = gtk::EventControllerKey::new();
        let dialog = obj.downgrade();
        escape.connect_key_pressed(move |_, key, _, _| {
            if key != gdk::Key::Escape {
                return glib::Propagation::Proceed;
            }
            let Some(dialog) = dialog.upgrade() else {
                return glib::Propagation::Proceed;
            };
            if dialog.imp().stages.visible_child_name().as_deref() == Some(WORKING_PAGE) {
                // Swallowed, not just left to `close-request`: Escape must do
                // nothing at all while deletion is under way (`FR.21.9`).
                return glib::Propagation::Stop;
            }
            dialog.close();
            glib::Propagation::Stop
        });
        obj.add_controller(escape);
    }
}

impl WidgetImpl for DeleteAccountDialog {}
impl WindowImpl for DeleteAccountDialog {}

impl DeleteAccountDialog {
    /// Fills the `confirm` page's sentence with `name` and shows it.
    pub(super) fn configure(&self, name: &str) {
        self.confirm_sentence.set_label(&format!(
            "Delete {name}? Its logins and saved game data will be removed from this computer. This cannot be undone."
        ));
        self.obj().set_deletable(true);
        self.stages.set_visible_child_name(CONFIRM_PAGE);
    }

    fn confirm(&self) {
        if let Some(handler) = self.on_confirmed.borrow().as_ref() {
            handler();
        }
    }

    fn retry(&self) {
        if let Some(handler) = self.on_retry.borrow().as_ref() {
            handler();
        }
    }

    pub(super) fn show_working(&self, name: &str) {
        self.working_label.set_label(&format!("Deleting {name}…"));
        // No window-manager close affordance while this page shows
        // (`FR.21.9`); `close-request` and the Escape controller above are
        // the two paths that actually enforce it headless, where no window
        // manager renders the decoration this property would otherwise hide.
        self.obj().set_deletable(false);
        self.stages.set_visible_child_name(WORKING_PAGE);
    }

    /// Closes the window because the deletion actually succeeded — the one
    /// close the `working`-page guard in `constructed` must never block.
    pub(super) fn close_on_success(&self) {
        self.allow_close.set(true);
        self.obj().close();
    }

    pub(super) fn show_failed(&self, name: &str, reason: &str, folder: &Path) {
        self.failed_sentence
            .set_label(&format!("Couldn't finish deleting {name}."));
        self.reason_label.set_label(reason);
        self.folder_label.set_label(&folder.display().to_string());
        self.obj().set_deletable(true);
        self.stages.set_visible_child_name(FAILED_PAGE);
    }
}
