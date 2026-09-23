//! The sidebar's template children and the tree plumbing: a `ListView` over a
//! `NoSelection` wrapping a `TreeListModel`, whose root is one
//! [`super::workspace_row::WorkspaceRow`] per workspace and whose children are
//! that workspace's [`Row`]s — one factory binding each flat position to
//! either layout (architecture rules 8, 12).

use std::cell::{Cell, OnceCell, RefCell};
use std::sync::{Arc, Once};

use gio::prelude::ActionMapExt;
use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::pango;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{
    Destinations, MemoryProbe, Session, SessionId, Visibility, WorkspaceBook, WorkspaceId,
};

use super::MoveTarget;
use super::row::{Row, status_label};
use super::workspace_row::WorkspaceRow;
use crate::memory_footer::MemoryFooter;

/// The status keys that share the mark stack's `"dot"` page — a filled dot
/// for `current`/`visible`, a ring for `background` (design rule 1). Cleared
/// and re-applied on every bind because the list recycles row widgets; the
/// other three keys (`parked`, `starting`, `queued`) pick a different stack
/// page instead of one of these classes, so they need no class of their own
/// here.
const DOT_CLASSES: [&str; 3] = ["status-current", "status-visible", "status-background"];

/// A status key's mark-stack page name — which of the leading mark's shapes
/// this state draws, matching the stack pages [`build_row_widgets`] builds
/// (design rule 1, `FR.25.3`).
fn mark_page(status: &str) -> &'static str {
    match status {
        "parked" => "parked",
        "starting" => "starting",
        "queued" => "queued",
        _ => "dot",
    }
}

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

/// A handler run with an account's id when its row menu's `Rename…` item is
/// chosen. The menu carries only the id; the window opens the dialog and the
/// book decides whether a typed name is stored (architecture rule 8).
type RenameHandler = Box<dyn Fn(SessionId)>;

/// A handler run with an account's id when its row menu's `Delete account…`
/// item is chosen. The menu carries only the id; the window opens the
/// confirmation window and decides everything else (architecture rule 8,
/// item 11 task 08).
type DeleteHandler = Box<dyn Fn(SessionId)>;

/// A handler run with a workspace's id and the state a person expanded or
/// collapsed its heading to.
type ExpansionHandler = Box<dyn Fn(WorkspaceId, bool)>;

/// A handler run with a workspace's id when its heading menu's `Rename…` item
/// is chosen. The menu carries only the id; the window opens the shared name
/// window and the book decides whether a typed name is stored (architecture
/// rule 8).
type WorkspaceRenameHandler = Box<dyn Fn(WorkspaceId)>;

/// A handler run with a workspace's id when its heading menu's `Remove
/// workspace` item is chosen.
type WorkspaceRemoveHandler = Box<dyn Fn(WorkspaceId)>;

/// A handler run with a workspace's id when its heading menu's `Park all`
/// item is chosen. The menu carries only the id; the window asks the book
/// which accounts that touches and what each one's transition is
/// (architecture rule 8).
type ParkAllHandler = Box<dyn Fn(WorkspaceId)>;

/// A handler run with a workspace's id when its heading menu's `Start all`
/// item is chosen. Otherwise exactly [`ParkAllHandler`].
type StartAllHandler = Box<dyn Fn(WorkspaceId)>;

/// A handler run with an account's id and the chosen destination when its
/// row's `Move to ▸` submenu picks one (design rule 23).
type MoveHandler = Box<dyn Fn(SessionId, MoveTarget)>;

/// A handler run when the sidebar's own `+ Add account` button is pressed
/// (design rule 22).
type AddAccountHandler = Box<dyn Fn()>;

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
/// The action a row's `Rename…` item is bound to, namespaced under
/// [`ROW_ACTION_GROUP`] in the menu's detailed action name. Stateless — it
/// carries only the account's id, like [`PARKING_ACTION`] — and, unlike
/// [`PARKING_ACTION`], never disabled: a rename is offered in every liveness
/// (`FR.13.1`).
const RENAME_ACTION: &str = "rename";
/// The action a row's `Delete account…` item is bound to, namespaced under
/// [`ROW_ACTION_GROUP`] in the menu's detailed action name. Stateless, like
/// [`RENAME_ACTION`], and never disabled — offered for an account in any
/// state or workspace (`FR.21.1`).
const DELETE_ACTION: &str = "delete";

/// The action group a workspace heading's ⋯ menu button installs its actions
/// under — local to that button, like [`ROW_ACTION_GROUP`] is to an account
/// row's.
const HEADING_ACTION_GROUP: &str = "heading";
/// The action a heading's `Rename…` item is bound to, namespaced under
/// [`HEADING_ACTION_GROUP`].
const WORKSPACE_RENAME_ACTION: &str = "rename";
/// The action a heading's `Remove workspace` item is bound to, namespaced
/// under [`HEADING_ACTION_GROUP`].
const WORKSPACE_REMOVE_ACTION: &str = "remove";
/// The action a heading's `Park all` item is bound to, namespaced under
/// [`HEADING_ACTION_GROUP`]. Stateless — it carries only the workspace's id;
/// the window decides which accounts that touches and what each becomes
/// (architecture rule 8). Its `enabled` follows [`WorkspaceRow::can_park_all`]
/// (design rule 2).
const PARK_ALL_ACTION: &str = "park-all";
/// The action a heading's `Start all` item is bound to, namespaced under
/// [`HEADING_ACTION_GROUP`]. Otherwise exactly [`PARK_ALL_ACTION`], following
/// [`WorkspaceRow::can_start_all`] instead.
const START_ALL_ACTION: &str = "start-all";

/// The workspace heading name label's hover text (`FR.25.1`) — a tooltip is
/// not a mark (design rule 13), so this is the only thing a heading gains
/// beyond its plain name.
const NEXT_WORKSPACE_TOOLTIP: &str = "Next workspace (Ctrl+Tab)";

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
    #[template_child]
    add_account_button: TemplateChild<gtk::Button>,

    /// The memory readout pinned to the foot of the column (item 05 task 04).
    /// Sampling starts when the window attaches its ports.
    memory_footer: MemoryFooter,

    /// The tree's root: one [`WorkspaceRow`] per workspace, in sidebar order.
    root: OnceCell<gio::ListStore>,
    /// The flat model the list view actually renders — kept so
    /// [`SessionSidebar::sync`] can walk every position and re-apply each
    /// heading's remembered expansion (`FR.16.2`).
    tree_model: OnceCell<gtk::TreeListModel>,
    /// Set for the duration of [`SessionSidebar::sync`]'s re-application walk,
    /// so the `notify::expanded` handler it drives through does not report a
    /// click nobody made.
    applying_expansion: Cell<bool>,
    /// The workspace `sync` last found shown, as a plain id string —
    /// `bind_heading_menu` reads it on every bind so only that heading's
    /// `Park all` / `Start all` carry an accelerator (design rule 27).
    shown_workspace: RefCell<Option<String>>,
    /// What the window last reported as fitting a single account's move,
    /// supplied through [`SessionSidebar::set_move_destinations`] and
    /// materialised into each row's `Move to ▸` submenu on its next bind
    /// (design rule 23).
    move_destinations: RefCell<Destinations>,
    pub(super) on_activated: RefCell<Option<ActivateHandler>>,
    pub(super) on_parking_toggled: RefCell<Option<ParkingHandler>>,
    pub(super) on_keep_awake_toggled: RefCell<Option<KeepAwakeHandler>>,
    pub(super) on_rename_requested: RefCell<Option<RenameHandler>>,
    pub(super) on_delete_requested: RefCell<Option<DeleteHandler>>,
    pub(super) on_expansion_toggled: RefCell<Option<ExpansionHandler>>,
    pub(super) on_move_requested: RefCell<Option<MoveHandler>>,
    pub(super) on_add_account_requested: RefCell<Option<AddAccountHandler>>,
    pub(super) on_workspace_rename_requested: RefCell<Option<WorkspaceRenameHandler>>,
    pub(super) on_workspace_remove_requested: RefCell<Option<WorkspaceRemoveHandler>>,
    pub(super) on_park_all_requested: RefCell<Option<ParkAllHandler>>,
    pub(super) on_start_all_requested: RefCell<Option<StartAllHandler>>,
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

        let root = gio::ListStore::new::<WorkspaceRow>();
        // `passthrough = false`, `autoexpand = false`: `false` passthrough is
        // what makes the model hand out a `GtkTreeListRow` for
        // `gtk_tree_expander_set_list_row`, and every heading starts collapsed
        // until `sync` re-applies what was remembered
        // (`docs/research/gtk4-drag-and-accordion.md:90`).
        let tree_model = gtk::TreeListModel::new(root.clone(), false, false, create_model_func);
        // NoSelection, not SingleSelection: the current row is the one whose
        // account holds the focused slot, drawn from domain state on every
        // redraw — a selection highlight would drift from it (single-click
        // activation also selects on hover) and say nothing the markup doesn't.
        let selection = gtk::NoSelection::new(Some(tree_model.clone()));

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
                .and_downcast::<gtk::TreeListRow>()
                .and_then(|tree_row| tree_row.item())
                .and_downcast::<Row>()
            else {
                return;
            };
            let id = SessionId::new(row.id());
            let imp = sidebar.imp();
            if let Some(handler) = imp.on_activated.borrow().as_ref() {
                handler(id);
            }
        });

        let sidebar = self.obj().downgrade();
        self.add_account_button.connect_clicked(move |_| {
            let Some(sidebar) = sidebar.upgrade() else {
                return;
            };
            if let Some(handler) = sidebar.imp().on_add_account_requested.borrow().as_ref() {
                handler();
            }
        });

        self.footer.append(&self.memory_footer);

        self.root
            .set(root)
            .expect("the root store is set once, here in constructed");
        self.tree_model
            .set(tree_model)
            .expect("the tree model is set once, here in constructed");
    }
}

impl WidgetImpl for SessionSidebar {}
impl BoxImpl for SessionSidebar {}

impl SessionSidebar {
    pub(super) fn sync(&self, book: &WorkspaceBook) {
        // Guards the whole rebuild, not just the reapply walk below: removing
        // an expanded row while tearing down the old tree — the very first
        // line here, on every sync after the first — collapses it as a side
        // effect, which fires the same `notify::expanded` a person's own
        // click would (`FR.16.2`). Measured 2026-09-14: an unguarded removal
        // reached `Window::set_expanded` while `Window::redraw` still held the
        // book borrowed, panicking on the re-entrant `borrow_mut`.
        self.applying_expansion.set(true);
        self.shown_workspace
            .replace(Some(book.active_id().as_str().to_owned()));

        let root = self.root.get().expect("root set in constructed");
        root.remove_all();

        let current_id = book.active().focused_session().map(Session::id);
        for workspace in book.workspaces() {
            let rows: Vec<Row> = workspace
                .book()
                .sessions()
                .iter()
                .map(|session| {
                    let visibility = book.placement(session.id()).unwrap_or(Visibility::OffGrid);
                    let current = current_id == Some(session.id());
                    Row::new(
                        session,
                        visibility,
                        current,
                        workspace.id().as_str(),
                        workspace.name(),
                    )
                })
                .collect();
            root.append(&WorkspaceRow::new(
                workspace.id(),
                workspace.name(),
                rows,
                book.can_park_all(workspace.id()),
                book.can_start_all(workspace.id()),
            ));
        }

        self.reapply_expansion(book);
        self.applying_expansion.set(false);

        // Zero accounts anywhere, not just the shown workspace: with any
        // account at all the tree always has something to show, even if only
        // Ungrouped's own heading and its "No accounts" leaf.
        let empty = book
            .workspaces()
            .all(|workspace| workspace.book().sessions().is_empty());
        self.scroller.set_visible(!empty);
        self.empty_label.set_visible(empty);
        // The footer's figures count every live account across every workspace,
        // not just the ones on screen (`FR.18.2`).
        self.footer.set_visible(!empty);
        self.memory_footer
            .set_running_count(book.live_session_count());
    }

    /// Walks the flat model's top-level rows and sets each one's `expanded`
    /// from `book`. The tree destroys a collapsed heading's own child model
    /// (`docs/research/gtk4-drag-and-accordion.md:113`), so this has to run
    /// after every rebuild, not once. Relies on the caller already holding
    /// `applying_expansion` for the `notify::expanded` handler
    /// [`row_factory`] wires up (`FR.16.2`).
    fn reapply_expansion(&self, book: &WorkspaceBook) {
        let Some(tree_model) = self.tree_model.get() else {
            return;
        };

        let mut position = 0;
        while let Some(item) = tree_model.item(position) {
            if let Some(tree_row) = item.downcast_ref::<gtk::TreeListRow>()
                && tree_row.depth() == 0
                && let Some(workspace_row) = tree_row.item().and_downcast::<WorkspaceRow>()
            {
                let expanded = book
                    .workspaces()
                    .find(|workspace| workspace.id().as_str() == workspace_row.id())
                    .is_some_and(|workspace| workspace.is_expanded());
                tree_row.set_expanded(expanded);
            }
            position += 1;
        }
    }

    pub(super) fn start_memory_sampling(&self, probe: Arc<dyn MemoryProbe>) {
        self.memory_footer.start_sampling(probe);
    }

    /// Records what the window found fits a single account's move — read by
    /// each row's own `Move to ▸` submenu the next time it binds
    /// (`bind_row_menu`). Cheap to call on every redraw, whether or not any
    /// row's menu is open, so it is never stale by the time one does open.
    pub(super) fn set_move_destinations(&self, destinations: Destinations) {
        self.move_destinations.replace(destinations);
    }
}

/// The parameterised action every `Move to ▸` item but the last activates,
/// namespaced under [`ROW_ACTION_GROUP`] like every other item in a row's
/// menu, the chosen workspace's id as its string target (design rule 23).
const MOVE_ACTION: &str = "move-to";
/// The action `Move to ▸`'s trailing `New workspace…` item is bound to,
/// namespaced under [`ROW_ACTION_GROUP`].
const NEW_WORKSPACE_ACTION: &str = "new-workspace";

/// The function [`gtk::TreeListModel`] calls to find a node's children: a
/// [`WorkspaceRow`]'s own pre-built store, `None` for a leaf [`Row`]
/// (`docs/research/gtk4-drag-and-accordion.md:90`).
fn create_model_func(item: &glib::Object) -> Option<gio::ListModel> {
    item.downcast_ref::<WorkspaceRow>()
        .map(|workspace| workspace.children().upcast::<gio::ListModel>())
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

/// Builds the factory that turns each flat position into either layout: a
/// heading's name alone, or an account's focus bar, mark, name, keep-awake
/// mark and ⋯ menu — one `GtkTreeExpander` wrapping a superset box, since
/// `::setup` runs before the bound row's type is known
/// (`docs/research/gtk4-drag-and-accordion.md:90`). The mark's shape and
/// class, its hover text and the menu item's label all come from the row's
/// status key and liveness, derived once in [`super::row`], so items 03 and
/// 08 extend that and never this.
fn row_factory(sidebar: &super::SessionSidebar) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();

    factory.connect_setup(|_, item| {
        let item = item
            .downcast_ref::<gtk::ListItem>()
            .expect("a list item factory is handed ListItems");
        let expander = gtk::TreeExpander::new();
        expander.set_indent_for_depth(true);
        expander.set_child(Some(&build_row_widgets()));
        item.set_child(Some(&expander));
    });

    let sidebar = sidebar.downgrade();
    factory.connect_bind(move |_, item| {
        let item = item
            .downcast_ref::<gtk::ListItem>()
            .expect("a list item factory is handed ListItems");
        let Some(tree_row) = item.item().and_downcast::<gtk::TreeListRow>() else {
            return;
        };
        let Some(expander) = item.child().and_downcast::<gtk::TreeExpander>() else {
            return;
        };
        expander.set_list_row(Some(&tree_row));

        let Some(data) = tree_row.item() else {
            return;
        };
        let Some(row) = expander.child().and_downcast::<gtk::Box>() else {
            return;
        };
        let Some(focus_bar) = row.first_child() else {
            return;
        };
        let Some(mark_stack) = focus_bar.next_sibling().and_downcast::<gtk::Stack>() else {
            return;
        };
        let Some(name) = mark_stack.next_sibling().and_downcast::<gtk::Label>() else {
            return;
        };
        let Some(settings) = name.next_sibling().and_downcast::<gtk::MenuButton>() else {
            return;
        };
        let widgets = RowWidgets {
            focus_bar,
            mark_stack,
            name,
            settings,
        };

        if let Some(workspace) = data.downcast_ref::<WorkspaceRow>() {
            bind_heading(item, &expander, &widgets, &sidebar);
            wire_expansion_toggled(&tree_row, workspace, &sidebar);
            return;
        }

        let Some(account) = data.downcast_ref::<Row>() else {
            return;
        };
        expander.set_hide_expander(true);
        bind_account(item, account, &widgets, &sidebar);
    });

    factory
}

/// The four widgets every row's box holds, leading edge first then trailing
/// order, found once per bind and handed to whichever branch binds them —
/// kept under clippy's argument-count budget as one value.
struct RowWidgets {
    focus_bar: gtk::Widget,
    mark_stack: gtk::Stack,
    name: gtk::Label,
    settings: gtk::MenuButton,
}

/// Binds the heading layout: the plain name label alone, every account-only
/// widget hidden, and no expander suppression — a heading always has at least
/// one child, the placeholder included, so its arrow is always meaningful.
/// `ListItem:activatable` false keeps a heading click to expand only
/// (`docs/research/gtk4-drag-and-accordion.md:113`). The name label carries
/// the `Next workspace (Ctrl+Tab)` hover text (`FR.25.1`) — a heading itself
/// stays undecorated (design rule 13), so the tooltip is the only thing it
/// gains. The ⋯ menu button is shown for every heading, `Ungrouped` included
/// (item 14 task 07). Its ⋯ button stays in the tree, opacity-hidden by
/// `sidebar.css` until hover, focus or its own open popover reveals it
/// (design rule 21) — never `set_visible(false)`, which would take it out of
/// tab order too.
fn bind_heading(
    item: &gtk::ListItem,
    expander: &gtk::TreeExpander,
    widgets: &RowWidgets,
    sidebar: &glib::WeakRef<super::SessionSidebar>,
) {
    let Some(workspace) = expander
        .list_row()
        .and_then(|row| row.item())
        .and_downcast::<WorkspaceRow>()
    else {
        return;
    };

    expander.set_hide_expander(false);
    item.set_activatable(false);
    widgets
        .name
        .set_label(&glib::markup_escape_text(&workspace.name()));
    widgets.name.set_tooltip_text(Some(NEXT_WORKSPACE_TOOLTIP));
    widgets.focus_bar.set_visible(false);
    widgets.mark_stack.set_visible(false);
    widgets.settings.set_visible(true);

    let shown = sidebar.upgrade().is_some_and(|sidebar| {
        sidebar.imp().shown_workspace.borrow().as_deref() == Some(workspace.id().as_str())
    });
    bind_heading_menu(&widgets.settings, &workspace, shown, sidebar);
}

/// Binds the account layout: the name markup, the leading mark's shape,
/// class, hover text and accessible label, the focus bar, and the ⋯ menu —
/// or, for the dim "No accounts" placeholder, the name alone with
/// everything else hidden and the row not activatable and not targetable by
/// right-click or `Shift`+`F10` (design rule 21).
fn bind_account(
    item: &gtk::ListItem,
    account: &Row,
    widgets: &RowWidgets,
    sidebar: &glib::WeakRef<super::SessionSidebar>,
) {
    item.set_activatable(!account.is_placeholder());
    widgets.name.set_label(&account.name_markup());

    if account.is_placeholder() {
        widgets.focus_bar.set_visible(false);
        widgets.mark_stack.set_visible(false);
        widgets.settings.set_visible(false);
        return;
    }

    let key = account.status();
    // The focus bar keys on `is-current`, not `status == "current"`: a
    // parked or starting focused account never reads `"current"` (liveness
    // outranks visibility in `status_key`), but it can still hold the
    // window's focused slot (design rule 26; measured 2026-09-22).
    widgets.focus_bar.set_visible(account.is_current());

    widgets.mark_stack.set_visible(true);
    let page = mark_page(&key);
    widgets.mark_stack.set_visible_child_name(page);
    if page == "dot" {
        let dot = widgets
            .mark_stack
            .child_by_name("dot")
            .expect("built in build_row_widgets");
        for class in DOT_CLASSES {
            dot.remove_css_class(class);
        }
        dot.add_css_class(&format!("status-{key}"));
    }
    // The mark is the only *visible* state signal now (design rule 1); the
    // state word still names it for a pointer and a screen reader, from the
    // same key that picked the shape above. Set on the stack, which is what a
    // pointer actually hovers and what carries the accessible role.
    let state = status_label(&key);
    widgets.mark_stack.set_tooltip_text(Some(state));
    widgets
        .mark_stack
        .update_property(&[gtk::accessible::Property::Label(state)]);

    widgets.settings.set_visible(true);
    // Design rule 27: only the focused account's own menu shows the
    // Park/Start and Rename accelerators — `is-current`, not `status`, for
    // the same reason the focus bar above reads it.
    bind_row_menu(&widgets.settings, account, account.is_current(), sidebar);
}

/// Reports a person's own expand or collapse of `workspace`'s heading through
/// [`SessionSidebar::connect_expansion_toggled`], skipping a change made by
/// [`SessionSidebar::sync`]'s own re-application walk
/// (`applying_expansion`, `FR.16.2`).
fn wire_expansion_toggled(
    tree_row: &gtk::TreeListRow,
    workspace: &WorkspaceRow,
    sidebar: &glib::WeakRef<super::SessionSidebar>,
) {
    let id = WorkspaceId::new(workspace.id());
    let sidebar = sidebar.clone();
    tree_row.connect_notify_local(Some("expanded"), move |row, _| {
        let Some(sidebar) = sidebar.upgrade() else {
            return;
        };
        if sidebar.imp().applying_expansion.get() {
            return;
        }
        if let Some(handler) = sidebar.imp().on_expansion_toggled.borrow().as_ref() {
            handler(id.clone(), row.is_expanded());
        }
    });
}

/// Builds one row's widget tree — a focus bar, the leading status mark, the
/// name, the keep-awake icon and the ⋯ menu button, in that order — wired for
/// hover, right-click and `Shift`+`F10` / `Menu` (design rule 21) but bound to
/// no item yet. A heading only ever shows the name; [`bind_heading`] and the
/// account branch of [`row_factory`]'s bind closure decide which widgets show.
/// The menu's model and action group are built per bind in [`bind_row_menu`],
/// since the Park/Start item's label inverts with the bound account. Split
/// out to keep the factory's closures under clippy's line budget: building the
/// tree is one level of abstraction, binding it is another (code standards
/// rule 6).
fn build_row_widgets() -> gtk::Box {
    // The focused row's 3 px bar (design rule 26), shown only for the
    // `current` account by [`bind_account`]. `sidebar.css` colours it in the
    // theme's own selection colour.
    let focus_bar = gtk::Box::builder().valign(gtk::Align::Fill).build();
    focus_bar.add_css_class("focus-bar");
    focus_bar.set_visible(false);

    let name = gtk::Label::builder()
        .use_markup(true)
        .xalign(0.0)
        .hexpand(true)
        .ellipsize(pango::EllipsizeMode::End)
        .build();

    // The leading mark: one shape per state for `parked`/`starting`/`queued`
    // (design rule 1); `current`/`visible`/`background` share the plain dot,
    // coloured only (owner feedback 2026-09-22 dropped `background`'s ring in
    // favour of its original filled dot). A `GtkStack` so exactly one of the
    // four pages shows at a time without hiding and showing four separate
    // siblings by hand. Every page is sized to match the dot's 10 px, so the
    // mark never grows or shrinks the row when its state changes. The `Img`
    // role plus the accessible label set on every bind keep the state
    // reachable to a screen reader.
    let mark_stack = gtk::Stack::builder()
        .valign(gtk::Align::Center)
        .accessible_role(gtk::AccessibleRole::Img)
        .build();
    mark_stack.add_css_class("status-mark");

    // 4 px, not the icons' own 10: a solid CSS circle fills its box edge to
    // edge, while a symbolic icon's glyph sits inside a small margin baked
    // into the icon itself, so the two never read as the same size at equal
    // declared dimensions. Settled here after two rounds of owner feedback
    // (10 → 8 → 4 px, 2026-09-22).
    let dot = gtk::Box::builder()
        .width_request(4)
        .height_request(4)
        .build();
    dot.add_css_class("status-dot");
    mark_stack.add_named(&dot, Some("dot"));

    let pause = gtk::Image::from_icon_name("media-playback-pause-symbolic");
    pause.set_pixel_size(10);
    pause.add_css_class("mark-parked");
    mark_stack.add_named(&pause, Some("parked"));

    let queued = gtk::Image::from_icon_name("document-open-recent-symbolic");
    queued.set_pixel_size(10);
    queued.add_css_class("mark-queued");
    mark_stack.add_named(&queued, Some("queued"));

    let spinner = gtk::Spinner::builder()
        .width_request(10)
        .height_request(10)
        .spinning(true)
        .build();
    spinner.add_css_class("mark-starting");
    mark_stack.add_named(&spinner, Some("starting"));

    // Always in the tree and always visible — `sidebar.css` opacity-hides it
    // until the row is hovered, keyboard-focused or its own popover is open
    // (design rule 21). Never `set_visible(false)` outside the placeholder
    // row, which would also drop it from tab order.
    let settings = gtk::MenuButton::builder()
        .valign(gtk::Align::Center)
        .icon_name("view-more-symbolic")
        .build();
    settings.add_css_class("flat");
    settings.add_css_class("row-menu-button");

    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .build();
    row.add_css_class("sidebar-row");
    row.append(&focus_bar);
    row.append(&mark_stack);
    row.append(&name);
    row.append(&settings);

    wire_row_menu_gestures(&row, &settings);

    row
}

/// Right-click and `Shift`+`F10` / `Menu` anywhere on `row` open `settings`'s
/// own popover — the same menu the ⋯ button opens (design rule 21). Neither
/// is automatic on a `GtkListView` row.
fn wire_row_menu_gestures(row: &gtk::Box, settings: &gtk::MenuButton) {
    let click = gtk::GestureClick::new();
    click.set_button(gdk::BUTTON_SECONDARY);
    let popup_target = settings.downgrade();
    click.connect_pressed(move |gesture, _, _, _| {
        gesture.set_state(gtk::EventSequenceState::Claimed);
        if let Some(settings) = popup_target.upgrade() {
            settings.popup();
        }
    });
    row.add_controller(click);

    let key = gtk::EventControllerKey::new();
    let popup_target = settings.downgrade();
    key.connect_key_pressed(move |_, keyval, _, modifiers| {
        let is_menu_key = keyval == gdk::Key::Menu;
        let is_shift_f10 =
            keyval == gdk::Key::F10 && modifiers.contains(gdk::ModifierType::SHIFT_MASK);
        if !is_menu_key && !is_shift_f10 {
            return glib::Propagation::Proceed;
        }
        if let Some(settings) = popup_target.upgrade() {
            settings.popup();
        }
        glib::Propagation::Stop
    });
    row.add_controller(key);
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
/// A menu item that names its keyboard chord beside its label.
///
/// `GtkPopoverMenu` draws the `accel` attribute at the item's trailing edge,
/// in `gtk::accelerator_parse` syntax — `<Control><Shift>p` and the like. It
/// is the menu's equivalent of the tooltip a button uses to name its key, and
/// design rule 19 asks for one wherever a control mirrors a chord
/// (`FR.25.4`).
fn accelerated_item(label: &str, action: &str, accelerator: &str) -> gio::MenuItem {
    let item = gio::MenuItem::new(Some(label), Some(action));
    item.set_attribute_value("accel", Some(&accelerator.to_variant()));
    item
}

/// Rebuilds `settings`'s menu model and action group from `data`'s current
/// standing: `Park`/`Start` and `Keep running when hidden` in one section,
/// `Move to ▸` and `Rename…` in a second, `Delete account…` set apart in a
/// third (2.5's wireframe). `is_focused` is whether `data` is the sidebar's
/// `current` row — only there do the Park/Start and `Rename…` items carry
/// their accelerator (design rule 27); every other row's menu shows the same
/// items with no chord beside them.
fn bind_row_menu(
    settings: &gtk::MenuButton,
    data: &Row,
    is_focused: bool,
    sidebar: &glib::WeakRef<super::SessionSidebar>,
) {
    let destinations = sidebar
        .upgrade()
        .map(|sidebar| sidebar.imp().move_destinations.borrow().clone())
        .unwrap_or_default();

    settings.set_menu_model(Some(&build_row_menu_model(data, is_focused, &destinations)));
    settings.insert_action_group(
        ROW_ACTION_GROUP,
        Some(&build_row_action_group(data, &destinations, sidebar)),
    );
}

/// Builds a row's menu model: `Park`/`Start` and `Keep running when hidden`
/// in one section, `Move to ▸` and `Rename…` in a second, `Delete account…`
/// set apart in a third (2.5's wireframe). `is_focused` is whether `data` is
/// the sidebar's `current` row — only there do the Park/Start and `Rename…`
/// items carry their accelerator (design rule 27); every other row's menu
/// shows the same items with no chord beside them. Split from
/// [`bind_row_menu`] to keep both halves under clippy's line budget (code
/// standards rule 6): building the model is one level of abstraction, wiring
/// the actions it names is another.
fn build_row_menu_model(data: &Row, is_focused: bool, destinations: &Destinations) -> gio::Menu {
    let menu = gio::Menu::new();

    let action_label = data.action_label();
    let parking_accelerator = is_focused.then(|| data.action_accelerator());
    let actions_section = gio::Menu::new();
    match parking_accelerator {
        // The Park/Start item names its own chord beside the label, since a
        // menu item has nowhere to hover — the accelerator for the direction
        // this row currently offers, which is the one `Ctrl`+`P` / `Ctrl`+`S`
        // pair member that is not inert on it (`FR.25.4`, design rules 19,
        // 20, 27).
        Some(accelerator) => actions_section.append_item(&accelerated_item(
            action_label.as_str(),
            &format!("{ROW_ACTION_GROUP}.{PARKING_ACTION}"),
            accelerator.as_str(),
        )),
        None => actions_section.append(
            Some(action_label.as_str()),
            Some(&format!("{ROW_ACTION_GROUP}.{PARKING_ACTION}")),
        ),
    }
    actions_section.append(
        Some("Keep running when hidden"),
        Some(&format!("{ROW_ACTION_GROUP}.{KEEP_AWAKE_ACTION}")),
    );
    menu.append_section(None, &actions_section);

    let move_section = gio::Menu::new();
    let move_to_submenu =
        build_move_submenu(destinations, &data.workspace_id(), &data.workspace_name());
    move_section.append_submenu(Some("Move to"), &move_to_submenu);
    if is_focused {
        move_section.append_item(&accelerated_item(
            "Rename…",
            &format!("{ROW_ACTION_GROUP}.{RENAME_ACTION}"),
            "F2",
        ));
    } else {
        move_section.append(
            Some("Rename…"),
            Some(&format!("{ROW_ACTION_GROUP}.{RENAME_ACTION}")),
        );
    }
    menu.append_section(None, &move_section);

    let delete_section = gio::Menu::new();
    delete_section.append(
        Some("Delete account…"),
        Some(&format!("{ROW_ACTION_GROUP}.{DELETE_ACTION}")),
    );
    menu.append_section(None, &delete_section);

    menu
}

/// Builds the action group [`build_row_menu_model`]'s items activate: park,
/// keep-awake, rename, delete, and `Move to ▸`'s two actions. Neither
/// `parking` nor `move_to`/`new_workspace` decides anything itself — each
/// carries only the account's id (and, for `move_to`, the chosen
/// workspace's), leaving the transition to the window (architecture
/// rule 8). The `keep-awake` action's `change-state` handler reports the
/// requested value as an intent and never calls
/// [`gio::SimpleAction::set_state`] itself, so the checkbox only moves once
/// the book's answer comes back through [`SessionSidebar::sync`] and this
/// runs again.
fn build_row_action_group(
    data: &Row,
    destinations: &Destinations,
    sidebar: &glib::WeakRef<super::SessionSidebar>,
) -> gio::SimpleActionGroup {
    let id = SessionId::new(data.id());

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
    let keep_awake_id = id.clone();
    keep_awake.connect_change_state(move |_, requested| {
        let Some(requested) = requested.and_then(glib::Variant::get::<bool>) else {
            return;
        };
        let Some(sidebar) = keep_awake_sidebar.upgrade() else {
            return;
        };
        if let Some(handler) = sidebar.imp().on_keep_awake_toggled.borrow().as_ref() {
            handler(keep_awake_id.clone(), requested);
        }
    });

    let rename = gio::SimpleAction::new(RENAME_ACTION, None);
    // Always sensitive: a rename is offered in every liveness, including
    // while an account is starting, unlike `PARKING_ACTION` above
    // (`FR.13.1`).
    let rename_sidebar = sidebar.clone();
    let rename_id = id.clone();
    rename.connect_activate(move |_, _| {
        let Some(sidebar) = rename_sidebar.upgrade() else {
            return;
        };
        if let Some(handler) = sidebar.imp().on_rename_requested.borrow().as_ref() {
            handler(rename_id.clone());
        }
    });

    let delete = gio::SimpleAction::new(DELETE_ACTION, None);
    // Always sensitive, like `RENAME_ACTION` — offered for an account in any
    // state or workspace (`FR.21.1`).
    let delete_sidebar = sidebar.clone();
    let delete_id = id.clone();
    delete.connect_activate(move |_, _| {
        let Some(sidebar) = delete_sidebar.upgrade() else {
            return;
        };
        if let Some(handler) = sidebar.imp().on_delete_requested.borrow().as_ref() {
            handler(delete_id.clone());
        }
    });

    // `Move to ▸`'s items (design rule 23): `MOVE_ACTION` picks an existing
    // workspace by its id, targeted the same way the old sidebar-wide
    // `Move to…` button's `choose` action was; `NEW_WORKSPACE_ACTION` opens
    // the "New workspace" window for this one account.
    let move_to = gio::SimpleAction::new(MOVE_ACTION, Some(glib::VariantTy::STRING));
    let move_sidebar = sidebar.clone();
    let move_id = id.clone();
    move_to.connect_activate(move |_, target| {
        let Some(sidebar) = move_sidebar.upgrade() else {
            return;
        };
        let Some(workspace_id) = target.and_then(glib::Variant::get::<String>) else {
            return;
        };
        if let Some(handler) = sidebar.imp().on_move_requested.borrow().as_ref() {
            handler(
                move_id.clone(),
                MoveTarget::Existing(WorkspaceId::new(workspace_id)),
            );
        }
    });

    let new_workspace = gio::SimpleAction::new(NEW_WORKSPACE_ACTION, None);
    new_workspace.set_enabled(destinations.can_create());
    let new_workspace_sidebar = sidebar.clone();
    new_workspace.connect_activate(move |_, _| {
        let Some(sidebar) = new_workspace_sidebar.upgrade() else {
            return;
        };
        if let Some(handler) = sidebar.imp().on_move_requested.borrow().as_ref() {
            handler(id.clone(), MoveTarget::New);
        }
    });

    let group = gio::SimpleActionGroup::new();
    group.add_action(&parking);
    group.add_action(&keep_awake);
    group.add_action(&rename);
    group.add_action(&delete);
    group.add_action(&move_to);
    group.add_action(&new_workspace);
    group
}

/// The `Move to ▸` submenu for an account currently in `own_workspace_id`
/// (named `own_workspace_name`): that workspace first, always, shown
/// insensitive and marked "(here)" — an item with no bound action, which
/// `GtkPopoverMenu` already draws disabled, so the reader can see where the
/// account already is without it being a live choice, and shown even when a
/// full workspace drops out of `destinations` itself (`WorkspaceBook::destinations`'s
/// `count` check already counts this account once) — then every other
/// destination `destinations` offers, then a section break and `New
/// workspace…` (design rule 7), insensitive when
/// [`idle_manager_core::Destinations::can_create`] says no room exists
/// (design rule 23). A pure builder over its inputs, so it needs no sidebar
/// or window access of its own (code standards rule 6).
fn build_move_submenu(
    destinations: &Destinations,
    own_workspace_id: &str,
    own_workspace_name: &str,
) -> gio::Menu {
    let submenu = gio::Menu::new();
    submenu.append(Some(&format!("{own_workspace_name} (here)")), None);
    for workspace in destinations.workspaces() {
        if workspace.id().as_str() == own_workspace_id {
            continue;
        }
        let item = gio::MenuItem::new(Some(workspace.name()), None);
        item.set_action_and_target_value(
            Some(&format!("{ROW_ACTION_GROUP}.{MOVE_ACTION}")),
            Some(&workspace.id().as_str().to_variant()),
        );
        submenu.append_item(&item);
    }

    let new_workspace_section = gio::Menu::new();
    new_workspace_section.append(
        Some("New workspace…"),
        Some(&format!("{ROW_ACTION_GROUP}.{NEW_WORKSPACE_ACTION}")),
    );
    submenu.append_section(None, &new_workspace_section);

    submenu
}

/// Rebuilds `settings`'s menu model and action group from `workspace`'s
/// current identity and its two `can_*` flags — `bind_row_menu`'s shape
/// applied to a heading, two sections: `Park all` and `Start all` first, the
/// deliberate actions (design rule 5), each `enabled` following
/// [`WorkspaceRow::can_park_all`] / [`WorkspaceRow::can_start_all`] exactly as
/// a row's own Park/Start item follows its account's state (design rule 2);
/// then, for a named workspace, `Rename…` and `Remove workspace`. `Ungrouped`
/// gets the first section alone — it has no name to rename and cannot be
/// removed (item 14 task 07). `shown` is whether `workspace` is the one on
/// screen — only there do `Park all` / `Start all` carry their accelerator
/// (design rule 27); elsewhere the same two items show with no chord.
/// Rebuilt on every bind for the same reason `bind_row_menu` is (the list
/// recycles this `MenuButton`).
fn bind_heading_menu(
    settings: &gtk::MenuButton,
    workspace: &WorkspaceRow,
    shown: bool,
    sidebar: &glib::WeakRef<super::SessionSidebar>,
) {
    let id = WorkspaceId::new(workspace.id());

    settings.set_menu_model(Some(&build_heading_menu_model(&id, shown)));
    settings.insert_action_group(
        HEADING_ACTION_GROUP,
        Some(&build_heading_action_group(workspace, &id, sidebar)),
    );
}

/// Builds a heading's menu model: `Park all` and `Start all` first, the
/// deliberate actions (design rule 5), carrying their accelerator only when
/// `shown` says this heading is the one on screen (design rule 27); then, for
/// a named workspace, `Rename…` and `Remove workspace`. `Ungrouped` gets the
/// first section alone — it has no name to rename and cannot be removed
/// (item 14 task 07). Split from [`bind_heading_menu`] the same way
/// [`build_row_menu_model`] is from `bind_row_menu` (code standards rule 6).
fn build_heading_menu_model(id: &WorkspaceId, shown: bool) -> gio::Menu {
    let menu = gio::Menu::new();
    let actions = gio::Menu::new();
    if shown {
        actions.append_item(&accelerated_item(
            "Park all",
            &format!("{HEADING_ACTION_GROUP}.{PARK_ALL_ACTION}"),
            "<Control><Shift>p",
        ));
        actions.append_item(&accelerated_item(
            "Start all",
            &format!("{HEADING_ACTION_GROUP}.{START_ALL_ACTION}"),
            "<Control><Shift>s",
        ));
    } else {
        actions.append(
            Some("Park all"),
            Some(&format!("{HEADING_ACTION_GROUP}.{PARK_ALL_ACTION}")),
        );
        actions.append(
            Some("Start all"),
            Some(&format!("{HEADING_ACTION_GROUP}.{START_ALL_ACTION}")),
        );
    }
    menu.append_section(None, &actions);

    if !id.is_ungrouped() {
        let settings_section = gio::Menu::new();
        settings_section.append(
            Some("Rename…"),
            Some(&format!("{HEADING_ACTION_GROUP}.{WORKSPACE_RENAME_ACTION}")),
        );
        settings_section.append(
            Some("Remove workspace"),
            Some(&format!("{HEADING_ACTION_GROUP}.{WORKSPACE_REMOVE_ACTION}")),
        );
        menu.append_section(None, &settings_section);
    }
    menu
}

/// Builds the action group [`build_heading_menu_model`]'s items activate.
fn build_heading_action_group(
    workspace: &WorkspaceRow,
    id: &WorkspaceId,
    sidebar: &glib::WeakRef<super::SessionSidebar>,
) -> gio::SimpleActionGroup {
    let is_ungrouped = id.is_ungrouped();

    let park_all = gio::SimpleAction::new(PARK_ALL_ACTION, None);
    park_all.set_enabled(workspace.can_park_all());
    let park_all_sidebar = sidebar.clone();
    let park_all_id = id.clone();
    park_all.connect_activate(move |_, _| {
        let Some(sidebar) = park_all_sidebar.upgrade() else {
            return;
        };
        if let Some(handler) = sidebar.imp().on_park_all_requested.borrow().as_ref() {
            handler(park_all_id.clone());
        }
    });

    let start_all = gio::SimpleAction::new(START_ALL_ACTION, None);
    start_all.set_enabled(workspace.can_start_all());
    let start_all_sidebar = sidebar.clone();
    let start_all_id = id.clone();
    start_all.connect_activate(move |_, _| {
        let Some(sidebar) = start_all_sidebar.upgrade() else {
            return;
        };
        if let Some(handler) = sidebar.imp().on_start_all_requested.borrow().as_ref() {
            handler(start_all_id.clone());
        }
    });

    let group = gio::SimpleActionGroup::new();
    group.add_action(&park_all);
    group.add_action(&start_all);

    if !is_ungrouped {
        let rename = gio::SimpleAction::new(WORKSPACE_RENAME_ACTION, None);
        let rename_sidebar = sidebar.clone();
        let rename_id = id.clone();
        rename.connect_activate(move |_, _| {
            let Some(sidebar) = rename_sidebar.upgrade() else {
                return;
            };
            if let Some(handler) = sidebar
                .imp()
                .on_workspace_rename_requested
                .borrow()
                .as_ref()
            {
                handler(rename_id.clone());
            }
        });

        let remove = gio::SimpleAction::new(WORKSPACE_REMOVE_ACTION, None);
        let remove_sidebar = sidebar.clone();
        let remove_id = id.clone();
        remove.connect_activate(move |_, _| {
            let Some(sidebar) = remove_sidebar.upgrade() else {
                return;
            };
            if let Some(handler) = sidebar
                .imp()
                .on_workspace_remove_requested
                .borrow()
                .as_ref()
            {
                handler(remove_id.clone());
            }
        });

        group.add_action(&rename);
        group.add_action(&remove);
    }

    group
}
