//! The placeholder panel's template children and the property bindings that
//! push its three settable strings onto the widgets (architecture rule 12).

use std::cell::{Cell, RefCell};

use gtk::CompositeTemplate;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

/// A handler run when the panel's button is pressed. The panel carries no
/// account id; the grid, which built it, knows which slot it stands in and
/// names the account to the window (architecture rule 8).
type StartHandler = Box<dyn Fn()>;

/// The composite-template backing object for [`super::SlotPlaceholder`].
#[derive(Default, CompositeTemplate, glib::Properties)]
#[template(resource = "/org/idlemanager/IdleManager/ui/slot-placeholder.ui")]
#[properties(wrapper_type = super::SlotPlaceholder)]
pub struct SlotPlaceholder {
    #[template_child]
    name_label: TemplateChild<gtk::Label>,
    #[template_child]
    state_label: TemplateChild<gtk::Label>,
    #[template_child]
    action_button: TemplateChild<gtk::Button>,

    /// The account's display name, shown large at the top of the panel.
    #[property(get, set)]
    name: RefCell<String>,
    /// The line of state text under the name — `Parked`, `Starting`, or item
    /// 08's failure wording. A property, not markup, so item 08 reuses this
    /// widget.
    #[property(get, set)]
    state_text: RefCell<String>,
    /// The button's label — `Start` here, a different action for item 08.
    #[property(get, set)]
    button_label: RefCell<String>,
    /// Whether the button is pressable. `false` while the account is starting.
    #[property(get, set, default = true)]
    button_sensitive: Cell<bool>,
    /// Whether the button is shown at all. `false` for a queued account (item
    /// 07): the start queue owns the order, so there is nothing useful to press
    /// while it drains. Defaults visible, so every present caller is unchanged.
    #[property(get, set, default = true)]
    button_visible: Cell<bool>,

    pub(super) on_start_requested: RefCell<Option<StartHandler>>,
}

impl std::fmt::Debug for SlotPlaceholder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SlotPlaceholder").finish_non_exhaustive()
    }
}

#[glib::object_subclass]
impl ObjectSubclass for SlotPlaceholder {
    const NAME: &'static str = "IdleManagerSlotPlaceholder";
    type Type = super::SlotPlaceholder;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

#[glib::derived_properties]
impl ObjectImpl for SlotPlaceholder {
    fn constructed(&self) {
        self.parent_constructed();

        let obj = self.obj();
        obj.bind_property("name", &*self.name_label, "label")
            .sync_create()
            .build();
        obj.bind_property("state-text", &*self.state_label, "label")
            .sync_create()
            .build();
        obj.bind_property("button-label", &*self.action_button, "label")
            .sync_create()
            .build();
        obj.bind_property("button-sensitive", &*self.action_button, "sensitive")
            .sync_create()
            .build();
        obj.bind_property("button-visible", &*self.action_button, "visible")
            .sync_create()
            .build();

        let placeholder = obj.downgrade();
        self.action_button.connect_clicked(move |_| {
            let Some(placeholder) = placeholder.upgrade() else {
                return;
            };
            if let Some(handler) = placeholder.imp().on_start_requested.borrow().as_ref() {
                handler();
            }
        });
    }
}

impl WidgetImpl for SlotPlaceholder {}
impl BoxImpl for SlotPlaceholder {}
