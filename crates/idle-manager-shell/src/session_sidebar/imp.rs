//! The sidebar's template children and the list plumbing: a `ListView` over a
//! `NoSelection` wrapping a `ListStore` of [`Row`], with a factory that binds
//! each row to a name label and a status marker (architecture rules 8, 12).

use std::cell::{OnceCell, RefCell};
use std::sync::Once;

use gio::prelude::ActionMapExt;
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
const STATUS_CLASSES: [&str; 5] = [
    "status-current",
    "status-visible",
    "status-background",
    "status-parked",
    "status-starting",
];

/// A handler run with the activated account's id when the user clicks a row.
type ActivateHandler = Box<dyn Fn(SessionId)>;

/// A handler run with an account's id when its row's park/start button is
/// pressed. The direction — park or start — is the window's to decide from the
/// account's current liveness, not the row's (architecture rule 8).
type ParkingHandler = Box<dyn Fn(SessionId)>;

/// A handler run with an account's id and the value asked for when its row
/// menu's "Keep running when hidden" item is chosen. The menu reports only the
/// intent; whether anything actually changes is the book's to decide
/// (architecture rule 8).
type KeepAwakeHandler = Box<dyn Fn(SessionId, bool)>;

/// The name the per-row settings menu's action group is inserted under. Local
/// to the row's own `MenuButton`, distinct from any application- or
/// window-scoped `win`/`app` prefix.
const ROW_ACTION_GROUP: &str = "row";
/// The stateful action a row's "Keep running when hidden" item is bound to,
/// namespaced under [`ROW_ACTION_GROUP`] in the menu's detailed action name.
const KEEP_AWAKE_ACTION: &str = "keep-awake";

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
    pub(super) on_parking_toggled: RefCell<Option<ParkingHandler>>,
    pub(super) on_keep_awake_toggled: RefCell<Option<KeepAwakeHandler>>,
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
        self.list_view.set_factory(Some(&row_factory(&self.obj())));

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
            store.append(&Row::new(session, current));
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

/// Builds the factory that turns each [`Row`] into a name label, a trailing
/// status marker — a word and a coloured dot — a keep-awake indication, a
/// park/start button and a settings menu button. The word and the dot's class
/// both come from the row's status key, derived once in [`super::row`], so
/// items 03 and 08 extend that and never this.
fn row_factory(sidebar: &super::SessionSidebar) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();

    let sidebar = sidebar.downgrade();
    let bind_sidebar = sidebar.clone();
    factory.connect_setup(move |_, item| {
        let item = item
            .downcast_ref::<gtk::ListItem>()
            .expect("a list item factory is handed ListItems");

        let (row, action) = build_row_widgets();
        item.set_child(Some(&row));

        // One handler for the life of the recycled widget: it reads whichever
        // Row is bound at click time, so the list recycling a row never needs
        // the handler reconnected (code standards rule 18).
        let sidebar = sidebar.clone();
        let item = item.downgrade();
        action.connect_clicked(move |_| {
            let Some(item) = item.upgrade() else {
                return;
            };
            let Some(data) = item.item().and_downcast::<Row>() else {
                return;
            };
            let Some(sidebar) = sidebar.upgrade() else {
                return;
            };
            if let Some(handler) = sidebar.imp().on_parking_toggled.borrow().as_ref() {
                handler(SessionId::new(data.id()));
            }
        });
    });

    factory.connect_bind(move |_, item| {
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
        let Some(dot) = status.next_sibling() else {
            return;
        };
        let Some(keep_awake_mark) = dot.next_sibling().and_downcast::<gtk::Label>() else {
            return;
        };
        let Some(action) = keep_awake_mark.next_sibling().and_downcast::<gtk::Button>() else {
            return;
        };
        let Some(settings) = action.next_sibling().and_downcast::<gtk::MenuButton>() else {
            return;
        };

        let key = data.status();
        name.set_label(&data.name_markup());
        status.set_label(status_label(&key));
        for class in STATUS_CLASSES {
            dot.remove_css_class(class);
        }
        dot.add_css_class(&format!("status-{key}"));
        // Cleared and re-applied on every bind, like the dot's classes above:
        // the list recycles this label across accounts, so a mark left set
        // from a previous bind must not survive onto one with the flag off.
        let mark = data.keep_awake_mark();
        keep_awake_mark.set_label(&mark);
        keep_awake_mark.set_visible(!mark.is_empty());
        action.set_label(&data.action_label());
        bind_keep_awake_action(&settings, &data, &bind_sidebar);
        action.set_sensitive(data.action_sensitive());
    });

    factory
}

/// Builds one row's widget tree — name, status word, status dot, keep-awake
/// mark, park/start button and settings menu button, in that trailing order —
/// wired to no signals and bound to no [`Row`] yet. Returns the row's
/// container and the action button, the only child [`row_factory`]'s setup
/// closure needs a handle to. Split out to keep that closure under clippy's
/// line budget: building the tree is one level of abstraction, wiring its
/// signals is another (code standards rule 6).
fn build_row_widgets() -> (gtk::Box, gtk::Button) {
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

    // Hidden by default: shown only on a bind where the bound account's flag
    // is on, so a recycled row never shows a stale mark left by whichever
    // account it held before (code standards rule 18).
    let keep_awake_mark = gtk::Label::builder()
        .valign(gtk::Align::Center)
        .visible(false)
        .build();
    keep_awake_mark.add_css_class("keep-awake-mark");

    let action = gtk::Button::builder().valign(gtk::Align::Center).build();
    action.add_css_class("flat");

    // The menu's content never varies between rows or binds — only the
    // action behind "row.keep-awake" does, rebuilt on every bind below — so
    // it is built once here rather than on every bind.
    let settings_menu = gio::Menu::new();
    settings_menu.append(
        Some("Keep running when hidden"),
        Some(&format!("{ROW_ACTION_GROUP}.{KEEP_AWAKE_ACTION}")),
    );
    let settings = gtk::MenuButton::builder()
        .valign(gtk::Align::Center)
        .icon_name("view-more-symbolic")
        .menu_model(&settings_menu)
        .build();
    settings.add_css_class("flat");

    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .build();
    row.append(&name);
    row.append(&status);
    row.append(&dot);
    row.append(&keep_awake_mark);
    row.append(&action);
    row.append(&settings);

    (row, action)
}

/// Rebuilds `settings`'s action group from `data`'s current flag and installs
/// it under [`ROW_ACTION_GROUP`], replacing whatever the widget held before.
///
/// A fresh [`gio::SimpleAction`] on every bind, not a reused one: the list
/// recycles this `MenuButton` across accounts, and `GtkWidget` exposes no way
/// to read an already-inserted action group back out to update it in place, so
/// building a new one — seeded with the account now bound, per design rule 1's
/// pattern of re-deriving state on every bind rather than reaching for the
/// book — is the only avenue `insert_action_group` offers (architecture
/// rules 8, 10).
///
/// The action's `change-state` handler reports the requested value as an
/// intent and never calls [`gio::SimpleAction::set_state`] itself: the menu
/// decides nothing, so the checkbox only moves once the book's answer comes
/// back around through [`SessionSidebar::sync`] and this function runs again
/// (architecture rule 8).
fn bind_keep_awake_action(
    settings: &gtk::MenuButton,
    data: &Row,
    sidebar: &glib::WeakRef<super::SessionSidebar>,
) {
    let action = gio::SimpleAction::new_stateful(
        KEEP_AWAKE_ACTION,
        None,
        &data.is_kept_awake().to_variant(),
    );

    let id = SessionId::new(data.id());
    let sidebar = sidebar.clone();
    action.connect_change_state(move |_, requested| {
        let Some(requested) = requested.and_then(glib::Variant::get::<bool>) else {
            return;
        };
        let Some(sidebar) = sidebar.upgrade() else {
            return;
        };
        if let Some(handler) = sidebar.imp().on_keep_awake_toggled.borrow().as_ref() {
            handler(id.clone(), requested);
        }
    });

    let group = gio::SimpleActionGroup::new();
    group.add_action(&action);
    settings.insert_action_group(ROW_ACTION_GROUP, Some(&group));
}
