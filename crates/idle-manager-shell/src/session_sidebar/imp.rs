//! The sidebar's template children and the list plumbing: a `ListView` over a
//! `NoSelection` wrapping a `ListStore` of [`Row`], with a factory that binds
//! each row to a name label, a coloured status dot and a ⋯ menu (architecture
//! rules 8, 12).

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
const STATUS_CLASSES: [&str; 6] = [
    "status-current",
    "status-visible",
    "status-background",
    "status-parked",
    "status-starting",
    "status-queued",
];

/// A handler run with the activated account's id when the user clicks a row.
type ActivateHandler = Box<dyn Fn(SessionId)>;

/// A handler run with an account's id when its row menu's Park/Start item is
/// chosen. The direction — park or start — is the window's to decide from the
/// account's current liveness, not the row's (architecture rule 8).
type ParkingHandler = Box<dyn Fn(SessionId)>;

/// A handler run with an account's id and the value asked for when its row
/// menu's "Keep running when hidden" item is chosen. The menu reports only the
/// intent; whether anything actually changes is the book's to decide
/// (architecture rule 8).
type KeepAwakeHandler = Box<dyn Fn(SessionId, bool)>;

/// The name the per-row menu's action group is inserted under. Local to the
/// row's own `MenuButton`, distinct from any application- or window-scoped
/// `win`/`app` prefix.
const ROW_ACTION_GROUP: &str = "row";
/// The action a row's Park/Start item is bound to, namespaced under
/// [`ROW_ACTION_GROUP`] in the menu's detailed action name. Stateless — it
/// carries only the account's id; the window reads liveness for direction
/// (architecture rule 8).
const PARKING_ACTION: &str = "parking";
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

/// Builds the factory that turns each [`Row`] into a name label, a coloured
/// status dot, a keep-awake indication and a ⋯ menu carrying Park/Start and the
/// keep-awake setting. The dot's class, its hover text and the menu item's
/// label all come from the row's status key and liveness, derived once in
/// [`super::row`], so items 03 and 08 extend that and never this.
fn row_factory(sidebar: &super::SessionSidebar) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();

    factory.connect_setup(|_, item| {
        let item = item
            .downcast_ref::<gtk::ListItem>()
            .expect("a list item factory is handed ListItems");
        item.set_child(Some(&build_row_widgets()));
    });

    let sidebar = sidebar.downgrade();
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
        let Some(dot) = name.next_sibling() else {
            return;
        };
        let Some(keep_awake_mark) = dot.next_sibling().and_downcast::<gtk::Label>() else {
            return;
        };
        let Some(settings) = keep_awake_mark
            .next_sibling()
            .and_downcast::<gtk::MenuButton>()
        else {
            return;
        };

        let key = data.status();
        name.set_label(&data.name_markup());
        for class in STATUS_CLASSES {
            dot.remove_css_class(class);
        }
        dot.add_css_class(&format!("status-{key}"));
        // The dot is the only *visible* state signal now (design rule 1); the
        // state word still names it for a pointer and a screen reader, from the
        // same key that picked the class above.
        let state = status_label(&key);
        dot.set_tooltip_text(Some(state));
        dot.update_property(&[gtk::accessible::Property::Label(state)]);
        // Cleared and re-applied on every bind, like the dot's classes above:
        // the list recycles this label across accounts, so a mark left set
        // from a previous bind must not survive onto one with the flag off.
        let mark = data.keep_awake_mark();
        keep_awake_mark.set_label(&mark);
        keep_awake_mark.set_visible(!mark.is_empty());
        bind_row_menu(&settings, &data, &sidebar);
    });

    factory
}

/// Builds one row's widget tree — name, status dot, keep-awake mark and the ⋯
/// menu button, in that trailing order — wired to no signals and bound to no
/// [`Row`] yet. The menu's model and action group are built per bind in
/// [`bind_row_menu`], since the Park/Start item's label inverts with the bound
/// account. Split out to keep the factory's closures under clippy's line
/// budget: building the tree is one level of abstraction, binding it is another
/// (code standards rule 6).
fn build_row_widgets() -> gtk::Box {
    let name = gtk::Label::builder()
        .use_markup(true)
        .xalign(0.0)
        .hexpand(true)
        .ellipsize(pango::EllipsizeMode::End)
        .build();

    // A bare coloured dot, no word beside it: the colour is the only visible
    // state signal (design rule 1). The `Img` role plus the accessible label
    // set on every bind keep the state reachable to a screen reader.
    let dot = gtk::Box::builder()
        .width_request(10)
        .height_request(10)
        .valign(gtk::Align::Center)
        .accessible_role(gtk::AccessibleRole::Img)
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

    let settings = gtk::MenuButton::builder()
        .valign(gtk::Align::Center)
        .icon_name("view-more-symbolic")
        .build();
    settings.add_css_class("flat");

    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .build();
    row.append(&name);
    row.append(&dot);
    row.append(&keep_awake_mark);
    row.append(&settings);

    row
}

/// Rebuilds `settings`'s menu model and action group from `data`'s current
/// standing and installs both under [`ROW_ACTION_GROUP`], replacing whatever
/// the widget held before.
///
/// Both are rebuilt on every bind, not reused: the list recycles this
/// `MenuButton` across accounts, the Park/Start item's label inverts with the
/// bound account's liveness (design rule 2), and `GtkWidget` exposes no way to
/// read an already-inserted action group back out to update it in place — so a
/// fresh [`gio::Menu`] and a fresh [`gio::SimpleActionGroup`], both seeded with
/// the account now bound, are the only avenue `set_menu_model` /
/// `insert_action_group` offer, and match design rule 1's re-derive-on-bind
/// pattern (architecture rules 8, 10).
///
/// Neither action decides anything. `parking` carries only the account's id and
/// leaves park-or-start to the window, read from liveness (architecture
/// rule 8); it is disabled — the item shown, greyed — while the account is
/// starting, so the start cannot be triggered twice. The `keep-awake` action's
/// `change-state` handler reports the requested value as an intent and never
/// calls [`gio::SimpleAction::set_state`] itself, so the checkbox only moves
/// once the book's answer comes back through [`SessionSidebar::sync`] and this
/// function runs again.
fn bind_row_menu(
    settings: &gtk::MenuButton,
    data: &Row,
    sidebar: &glib::WeakRef<super::SessionSidebar>,
) {
    let id = SessionId::new(data.id());

    let action_label = data.action_label();
    let menu = gio::Menu::new();
    menu.append(
        Some(action_label.as_str()),
        Some(&format!("{ROW_ACTION_GROUP}.{PARKING_ACTION}")),
    );
    menu.append(
        Some("Keep running when hidden"),
        Some(&format!("{ROW_ACTION_GROUP}.{KEEP_AWAKE_ACTION}")),
    );
    settings.set_menu_model(Some(&menu));

    let parking = gio::SimpleAction::new(PARKING_ACTION, None);
    parking.set_enabled(data.action_sensitive());
    let parking_sidebar = sidebar.clone();
    let parking_id = id.clone();
    parking.connect_activate(move |_, _| {
        let Some(sidebar) = parking_sidebar.upgrade() else {
            return;
        };
        if let Some(handler) = sidebar.imp().on_parking_toggled.borrow().as_ref() {
            handler(parking_id.clone());
        }
    });

    let keep_awake = gio::SimpleAction::new_stateful(
        KEEP_AWAKE_ACTION,
        None,
        &data.is_kept_awake().to_variant(),
    );
    let keep_awake_sidebar = sidebar.clone();
    keep_awake.connect_change_state(move |_, requested| {
        let Some(requested) = requested.and_then(glib::Variant::get::<bool>) else {
            return;
        };
        let Some(sidebar) = keep_awake_sidebar.upgrade() else {
            return;
        };
        if let Some(handler) = sidebar.imp().on_keep_awake_toggled.borrow().as_ref() {
            handler(id.clone(), requested);
        }
    });

    let group = gio::SimpleActionGroup::new();
    group.add_action(&parking);
    group.add_action(&keep_awake);
    settings.insert_action_group(ROW_ACTION_GROUP, Some(&group));
}
