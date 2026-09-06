//! The sidebar's template children and the list plumbing: a `ListView` over a
//! `NoSelection` wrapping a `ListStore` of [`Row`], with a factory that binds
//! each row to a name label and a status marker (architecture rules 8, 12).

use std::cell::{OnceCell, RefCell};
use std::sync::Once;

use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::pango;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{SessionBook, SessionId, Visibility};

use super::row::{Row, status_label};

/// The status-dot keys `sidebar.css` styles, one class each. Cleared and
/// re-applied on every bind because the list recycles row widgets.
const STATUS_CLASSES: [&str; 3] = ["status-current", "status-visible", "status-background"];

/// A handler run with the activated account's id when the user clicks a row.
type ActivateHandler = Box<dyn Fn(SessionId)>;

/// The composite-template backing object for [`super::SessionSidebar`].
#[derive(Default, CompositeTemplate)]
#[template(resource = "/org/idlemanager/IdleManager/ui/session-sidebar.ui")]
pub struct SessionSidebar {
    #[template_child]
    scroller: TemplateChild<gtk::ScrolledWindow>,
    #[template_child]
    list_view: TemplateChild<gtk::ListView>,
    #[template_child]
    empty_label: TemplateChild<gtk::Label>,
    #[template_child]
    footer: TemplateChild<gtk::Box>,

    store: OnceCell<gio::ListStore>,
    pub(super) on_activated: RefCell<Option<ActivateHandler>>,
}

impl std::fmt::Debug for SessionSidebar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionSidebar").finish_non_exhaustive()
    }
}

#[glib::object_subclass]
impl ObjectSubclass for SessionSidebar {
    const NAME: &'static str = "IdleManagerSessionSidebar";
    type Type = super::SessionSidebar;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for SessionSidebar {
    fn constructed(&self) {
        self.parent_constructed();

        install_styles();

        let store = gio::ListStore::new::<Row>();
        // NoSelection, not SingleSelection: the current row is the one whose
        // account holds the focused slot, drawn from domain state on every
        // redraw — a selection highlight would drift from it (single-click
        // activation also selects on hover) and say nothing the markup doesn't.
        let selection = gtk::NoSelection::new(Some(store.clone()));

        self.list_view.set_model(Some(&selection));
        self.list_view.set_factory(Some(&row_factory()));

        let sidebar = self.obj().downgrade();
        self.list_view.connect_activate(move |list_view, position| {
            let Some(sidebar) = sidebar.upgrade() else {
                return;
            };
            let Some(row) = list_view
                .model()
                .and_then(|model| model.item(position))
                .and_downcast::<Row>()
            else {
                return;
            };
            if let Some(handler) = sidebar.imp().on_activated.borrow().as_ref() {
                handler(SessionId::new(row.id()));
            }
        });

        self.store
            .set(store)
            .expect("the store is set once, here in constructed");
    }
}

impl WidgetImpl for SessionSidebar {}
impl BoxImpl for SessionSidebar {}

impl SessionSidebar {
    pub(super) fn sync(&self, book: &SessionBook) {
        let store = self.store.get().expect("store set in constructed");
        let focused = book.focused();

        store.remove_all();
        for session in book.sessions() {
            let current = session.visibility() == Visibility::InSlot(focused);
            store.append(&Row::new(
                session.id(),
                session.display_name(),
                session.visibility(),
                current,
            ));
        }

        let empty = book.sessions().is_empty();
        self.scroller.set_visible(!empty);
        self.empty_label.set_visible(empty);
        // The footer stays hidden and empty in this slice; item 05 fills it with
        // the memory readout, and reserving the slot now avoids reopening this
        // template then.
        self.footer.set_visible(false);
    }
}

/// Installs `sidebar.css` on the default display once. The provider is
/// display-global, so the [`Once`] keeps a second sidebar from stacking it.
fn install_styles() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let Some(display) = gdk::Display::default() else {
            return;
        };
        let provider = gtk::CssProvider::new();
        provider.load_from_resource("/org/idlemanager/IdleManager/css/sidebar.css");
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });
}

/// Builds the factory that turns each [`Row`] into a name label and a trailing
/// status marker — a word and a coloured dot. The word and the dot's class both
/// come from the row's status key, derived once in [`super::row`], so items 03
/// and 08 extend that and never this.
fn row_factory() -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();

    factory.connect_setup(|_, item| {
        let item = item
            .downcast_ref::<gtk::ListItem>()
            .expect("a list item factory is handed ListItems");

        let name = gtk::Label::builder()
            .use_markup(true)
            .xalign(0.0)
            .hexpand(true)
            .ellipsize(pango::EllipsizeMode::End)
            .build();

        let status = gtk::Label::builder().xalign(1.0).build();
        status.add_css_class("sidebar-status");

        let dot = gtk::Box::builder()
            .width_request(10)
            .height_request(10)
            .valign(gtk::Align::Center)
            .build();
        dot.add_css_class("status-dot");

        let row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .build();
        row.append(&name);
        row.append(&status);
        row.append(&dot);

        item.set_child(Some(&row));
    });

    factory.connect_bind(|_, item| {
        let item = item
            .downcast_ref::<gtk::ListItem>()
            .expect("a list item factory is handed ListItems");
        let Some(data) = item.item().and_downcast::<Row>() else {
            return;
        };
        let Some(row) = item.child().and_downcast::<gtk::Box>() else {
            return;
        };
        let Some(name) = row.first_child().and_downcast::<gtk::Label>() else {
            return;
        };
        let Some(status) = name.next_sibling().and_downcast::<gtk::Label>() else {
            return;
        };
        let Some(dot) = row.last_child() else {
            return;
        };

        let key = data.status();
        name.set_label(&data.name_markup());
        status.set_label(status_label(&key));
        for class in STATUS_CLASSES {
            dot.remove_css_class(class);
        }
        dot.add_css_class(&format!("status-{key}"));
    });

    factory
}
