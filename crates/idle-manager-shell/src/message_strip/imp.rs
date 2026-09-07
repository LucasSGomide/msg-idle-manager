//! The message strip's template children and the close-button wiring
//! (architecture rule 12).

use std::cell::RefCell;

use gtk::CompositeTemplate;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

/// The composite-template backing object for [`super::MessageStrip`].
#[derive(Default, CompositeTemplate, glib::Properties)]
#[template(resource = "/org/idlemanager/IdleManager/ui/message-strip.ui")]
#[properties(wrapper_type = super::MessageStrip)]
pub struct MessageStrip {
    #[template_child]
    message_label: TemplateChild<gtk::Label>,
    #[template_child]
    close_button: TemplateChild<gtk::Button>,

    /// The single line the strip shows. Empty while the strip has nothing to
    /// say — the strip is hidden then, so a caller sets this and shows the
    /// widget together through [`super::MessageStrip::show`].
    #[property(get, set)]
    message: RefCell<String>,
}

impl std::fmt::Debug for MessageStrip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessageStrip").finish_non_exhaustive()
    }
}

#[glib::object_subclass]
impl ObjectSubclass for MessageStrip {
    const NAME: &'static str = "IdleManagerMessageStrip";
    type Type = super::MessageStrip;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

#[glib::derived_properties]
impl ObjectImpl for MessageStrip {
    fn constructed(&self) {
        self.parent_constructed();

        let obj = self.obj();
        obj.bind_property("message", &*self.message_label, "label")
            .sync_create()
            .build();

        // Dismissed for good: the window below is already whatever it should be
        // (a first run, or the session carrying on), so hiding the strip is the
        // whole action (design rule 9).
        let strip = obj.downgrade();
        self.close_button.connect_clicked(move |_| {
            if let Some(strip) = strip.upgrade() {
                strip.set_visible(false);
            }
        });
    }
}

impl WidgetImpl for MessageStrip {}
impl BoxImpl for MessageStrip {}
