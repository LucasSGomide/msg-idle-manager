//! The sidebar's template children and the tree plumbing: a `ListView` over a
//! `NoSelection` wrapping a `TreeListModel`, whose root is one
//! [`super::workspace_row::WorkspaceRow`] per workspace and whose children are
//! that workspace's [`Row`]s — one factory binding each flat position to
//! either layout (architecture rules 8, 12).

use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashSet;
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

/// A handler run with the ticked ids and the chosen destination when `Move
/// to…` picks an existing workspace.
type MoveHandler = Box<dyn Fn(Vec<SessionId>, MoveTarget)>;

/// A handler run whenever the sidebar's own selection state changes — the
/// mode toggled, or a tick toggled — telling the window a redraw is needed to
/// carry the change to the row widgets (`FR.17.4`).
type SelectionChangedHandler = Box<dyn Fn()>;

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
    select_toggle: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    selection_bar: TemplateChild<gtk::Box>,
    #[template_child]
    ticked_label: TemplateChild<gtk::Label>,
    #[template_child]
    move_to_button: TemplateChild<gtk::MenuButton>,

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
    /// Whether a click ticks a row instead of switching to it (item 11 task
    /// 05, `FR.17.1`). `pub(super)`: `SessionSidebar::is_selecting` (item 14
    /// task 04) reads it so the window's navigation shortcuts can tell
    /// whether the sidebar owns the screen right now (`FR.23.4`).
    pub(super) is_selecting: Cell<bool>,
    /// The ticked account ids — a set of ids, not the list's own positional
    /// selection, since a collapsed heading leaves the model and would take a
    /// positional selection with it (`FR.17.4`).
    ticked: RefCell<HashSet<SessionId>>,
    /// What the window last reported as fitting the current ticked count,
    /// supplied through [`SessionSidebar::set_move_destinations`] and
    /// materialised into the `Move to…` menu when it opens.
    move_destinations: RefCell<Destinations>,
    pub(super) on_activated: RefCell<Option<ActivateHandler>>,
    pub(super) on_parking_toggled: RefCell<Option<ParkingHandler>>,
    pub(super) on_keep_awake_toggled: RefCell<Option<KeepAwakeHandler>>,
    pub(super) on_rename_requested: RefCell<Option<RenameHandler>>,
    pub(super) on_delete_requested: RefCell<Option<DeleteHandler>>,
    pub(super) on_expansion_toggled: RefCell<Option<ExpansionHandler>>,
    pub(super) on_move_requested: RefCell<Option<MoveHandler>>,
    pub(super) on_selection_changed: RefCell<Option<SelectionChangedHandler>>,
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
            if imp.is_selecting.get() {
                imp.toggle_tick(&id);
            } else if let Some(handler) = imp.on_activated.borrow().as_ref() {
                handler(id);
            }
        });

        let sidebar = self.obj().downgrade();
        self.select_toggle.connect_toggled(move |toggle| {
            let Some(sidebar) = sidebar.upgrade() else {
                return;
            };
            sidebar.imp().set_selecting(toggle.is_active());
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
                    Row::new(session, visibility, current)
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

    /// How many accounts are currently ticked — what the window asks
    /// [`idle_manager_core::WorkspaceBook::destinations`] with before calling
    /// [`SessionSidebar::set_move_destinations`] back.
    pub(super) fn ticked_count(&self) -> usize {
        self.ticked.borrow().len()
    }

    /// Records what the window found fits the current ticked count, and
    /// rebuilds the `Move to…` menu from it. Cheap — a handful of menu items —
    /// so calling it on every redraw, whether or not the menu is open, costs
    /// nothing and keeps it never stale by the time it does open.
    pub(super) fn set_move_destinations(&self, destinations: Destinations) {
        self.move_destinations.replace(destinations);
        self.rebuild_move_menu();
    }

    /// Leaves selection mode and clears every tick. Called by the window only
    /// after a move or a create has actually been applied — a cancelled name
    /// window calls nothing, so the mode and every tick survive it
    /// (`FR.17.7`).
    pub(super) fn end_selection(&self) {
        self.select_toggle.set_active(false);
    }

    fn set_selecting(&self, selecting: bool) {
        self.is_selecting.set(selecting);
        self.select_toggle
            .set_label(if selecting { "Done" } else { "Select" });
        if !selecting {
            self.ticked.borrow_mut().clear();
        }
        self.selection_bar.set_visible(selecting);
        self.update_selection_bar();
        self.notify_selection_changed();
    }

    /// Drops `id` from the ticked set, without touching selection mode.
    /// Called once an account is actually gone, so a stale id never lingers
    /// in the set (item 11 task 08). A no-op if `id` was not ticked.
    pub(super) fn forget_ticked(&self, id: &SessionId) {
        if self.ticked.borrow_mut().remove(id) {
            self.update_selection_bar();
            self.notify_selection_changed();
        }
    }

    fn toggle_tick(&self, id: &SessionId) {
        let mut ticked = self.ticked.borrow_mut();
        if !ticked.remove(id) {
            ticked.insert(id.clone());
        }
        drop(ticked);
        self.update_selection_bar();
        self.notify_selection_changed();
    }

    fn update_selection_bar(&self) {
        let count = self.ticked.borrow().len();
        self.ticked_label.set_label(&format!("{count} ticked"));
        self.move_to_button.set_sensitive(count > 0);
    }

    fn notify_selection_changed(&self) {
        if let Some(handler) = self.on_selection_changed.borrow().as_ref() {
            handler();
        }
    }

    /// Rebuilds the `Move to…` popover from the last
    /// [`SessionSidebar::set_move_destinations`] answer: one item per offered
    /// workspace, choosing it through a single parameterised action so the
    /// menu never needs one action per workspace, then a section break and
    /// `New workspace…` (design rule 7), insensitive while
    /// [`idle_manager_core::Destinations::can_create`] says no room exists for
    /// a brand new workspace to hold the ticked count.
    fn rebuild_move_menu(&self) {
        let destinations = self.move_destinations.borrow();

        let menu = gio::Menu::new();
        for workspace in destinations.workspaces() {
            let item = gio::MenuItem::new(Some(workspace.name()), None);
            item.set_action_and_target_value(
                Some(&format!("{MOVE_ACTION_GROUP}.{CHOOSE_ACTION}")),
                Some(&workspace.id().as_str().to_variant()),
            );
            menu.append_item(&item);
        }
        let new_workspace_section = gio::Menu::new();
        new_workspace_section.append(
            Some("New workspace…"),
            Some(&format!("{MOVE_ACTION_GROUP}.{NEW_WORKSPACE_ACTION}")),
        );
        menu.append_section(None, &new_workspace_section);
        self.move_to_button.set_menu_model(Some(&menu));

        let choose = gio::SimpleAction::new(CHOOSE_ACTION, Some(glib::VariantTy::STRING));
        let sidebar = self.obj().downgrade();
        choose.connect_activate(move |_, target| {
            let Some(sidebar) = sidebar.upgrade() else {
                return;
            };
            let Some(workspace_id) = target.and_then(glib::Variant::get::<String>) else {
                return;
            };
            let imp = sidebar.imp();
            let ids: Vec<SessionId> = imp.ticked.borrow().iter().cloned().collect();
            if let Some(handler) = imp.on_move_requested.borrow().as_ref() {
                handler(ids, MoveTarget::Existing(WorkspaceId::new(workspace_id)));
            }
        });

        let new_workspace = gio::SimpleAction::new(NEW_WORKSPACE_ACTION, None);
        new_workspace.set_enabled(destinations.can_create());
        let sidebar = self.obj().downgrade();
        new_workspace.connect_activate(move |_, _| {
            let Some(sidebar) = sidebar.upgrade() else {
                return;
            };
            let imp = sidebar.imp();
            let ids: Vec<SessionId> = imp.ticked.borrow().iter().cloned().collect();
            if let Some(handler) = imp.on_move_requested.borrow().as_ref() {
                handler(ids, MoveTarget::New);
            }
        });

        let group = gio::SimpleActionGroup::new();
        group.add_action(&choose);
        group.add_action(&new_workspace);
        self.move_to_button
            .insert_action_group(MOVE_ACTION_GROUP, Some(&group));
    }
}

/// The action group the `Move to…` button's menu items are inserted under.
const MOVE_ACTION_GROUP: &str = "move-to";
/// The single parameterised action every `Move to…` menu item activates, the
/// chosen workspace's id as its string target.
const CHOOSE_ACTION: &str = "choose";
/// The action the `Move to…` menu's `New workspace…` item is bound to.
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
/// heading's name alone, or an account's name, dot, keep-awake mark and ⋯
/// menu — one `GtkTreeExpander` wrapping a superset box, since `::setup` runs
/// before the bound row's type is known
/// (`docs/research/gtk4-drag-and-accordion.md:90`). The dot's class, its hover
/// text and the menu item's label all come from the row's status key and
/// liveness, derived once in [`super::row`], so items 03 and 08 extend that
/// and never this.
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
        let Some(tick) = row.first_child().and_downcast::<gtk::CheckButton>() else {
            return;
        };
        let Some(name) = tick.next_sibling().and_downcast::<gtk::Label>() else {
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
        let widgets = RowWidgets {
            tick,
            name,
            dot,
            keep_awake_mark,
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

/// The five widgets every row's box holds, leading tick box first then
/// trailing order, found once per bind and handed to whichever branch binds
/// them — kept under clippy's argument-count budget as one value.
struct RowWidgets {
    tick: gtk::CheckButton,
    name: gtk::Label,
    dot: gtk::Widget,
    keep_awake_mark: gtk::Label,
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
/// (item 14 task 07), and only outside selection mode, so a click there
/// cannot change the list mid-select (item 11 task 06).
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
    widgets.tick.set_visible(false);
    widgets.dot.set_visible(false);
    widgets.keep_awake_mark.set_visible(false);

    let is_selecting = sidebar
        .upgrade()
        .is_some_and(|sidebar| sidebar.imp().is_selecting.get());
    widgets.settings.set_visible(!is_selecting);
    bind_heading_menu(&widgets.settings, &workspace, sidebar);
}

/// Binds the account layout: the name markup, the status dot's class, hover
/// text and accessible label, the keep-awake mark, and the ⋯ menu — or, for
/// the dim "No accounts" placeholder, the name alone with everything else
/// hidden and the row not activatable.
fn bind_account(
    item: &gtk::ListItem,
    account: &Row,
    widgets: &RowWidgets,
    sidebar: &glib::WeakRef<super::SessionSidebar>,
) {
    item.set_activatable(!account.is_placeholder());
    widgets.name.set_label(&account.name_markup());

    if account.is_placeholder() {
        widgets.tick.set_visible(false);
        widgets.dot.set_visible(false);
        widgets.keep_awake_mark.set_visible(false);
        widgets.settings.set_visible(false);
        return;
    }

    bind_tick(&widgets.tick, account, sidebar);

    widgets.dot.set_visible(true);
    let key = account.status();
    for class in STATUS_CLASSES {
        widgets.dot.remove_css_class(class);
    }
    widgets.dot.add_css_class(&format!("status-{key}"));
    // The dot is the only *visible* state signal now (design rule 1); the
    // state word still names it for a pointer and a screen reader, from the
    // same key that picked the class above.
    let state = status_label(&key);
    widgets.dot.set_tooltip_text(Some(state));
    widgets
        .dot
        .update_property(&[gtk::accessible::Property::Label(state)]);
    // Cleared and re-applied on every bind, like the dot's classes above: the
    // list recycles this label across accounts, so a mark left set from a
    // previous bind must not survive onto one with the flag off.
    let mark = account.keep_awake_mark();
    widgets.keep_awake_mark.set_label(&mark);
    widgets.keep_awake_mark.set_visible(!mark.is_empty());
    widgets.settings.set_visible(true);
    bind_row_menu(&widgets.settings, account, sidebar);
}

/// Binds the leading tick box: visible only while selecting, checked from the
/// sidebar's own ticked set. It carries no click handler of its own — it is
/// `can-target: false` (set once in [`build_row_widgets`]), a pure display
/// that lets a click land on the row underneath, so ticking it and ticking
/// anywhere else on the row are the same one path through
/// `list_view`'s own `activate` (`FR.17.1`). That also sidesteps the
/// recycled-widget hazard a real per-bind `toggled` connection would have: no
/// handler to disconnect, and `set_active` here fires nothing to misfire.
fn bind_tick(
    tick: &gtk::CheckButton,
    account: &Row,
    sidebar: &glib::WeakRef<super::SessionSidebar>,
) {
    let Some(sidebar) = sidebar.upgrade() else {
        return;
    };
    let imp = sidebar.imp();
    let selecting = imp.is_selecting.get();
    tick.set_visible(selecting);
    if selecting {
        let id = SessionId::new(account.id());
        tick.set_active(imp.ticked.borrow().contains(&id));
    }
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

/// Builds one row's widget tree — name, status dot, keep-awake mark and the ⋯
/// menu button, in that trailing order — wired to no signals and bound to no
/// item yet. A heading only ever shows the name; [`bind_heading`] and the
/// account branch of [`row_factory`]'s bind closure decide which widgets show.
/// The menu's model and action group are built per bind in [`bind_row_menu`],
/// since the Park/Start item's label inverts with the bound account. Split
/// out to keep the factory's closures under clippy's line budget: building the
/// tree is one level of abstraction, binding it is another (code standards
/// rule 6).
fn build_row_widgets() -> gtk::Box {
    // Hidden outside selection mode; shown and its checked state set only for
    // an account row, in [`bind_tick`] (design rule 6 — paid for by the
    // 200 px width, not by the name giving way). `can-target: false` makes it
    // a pure display: a click meant for it reaches the row underneath
    // instead, so ticking the box and ticking anywhere else on the row are
    // the same path through `list_view`'s own `activate`.
    let tick = gtk::CheckButton::builder()
        .valign(gtk::Align::Center)
        .can_target(false)
        .focusable(false)
        .build();
    tick.set_visible(false);

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
    row.append(&tick);
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

fn bind_row_menu(
    settings: &gtk::MenuButton,
    data: &Row,
    sidebar: &glib::WeakRef<super::SessionSidebar>,
) {
    let id = SessionId::new(data.id());
    let delete_id = id.clone();

    let action_label = data.action_label();
    let menu = gio::Menu::new();
    // The Park/Start item names its own chord beside the label, since a menu
    // item has nowhere to hover — the accelerator for the direction this row
    // currently offers, which is the one `Ctrl`+`P` / `Ctrl`+`S` pair member
    // that is not inert on it (`FR.25.4`, design rules 19 and 20).
    menu.append_item(&accelerated_item(
        action_label.as_str(),
        &format!("{ROW_ACTION_GROUP}.{PARKING_ACTION}"),
        data.action_accelerator().as_str(),
    ));
    menu.append(
        Some("Keep running when hidden"),
        Some(&format!("{ROW_ACTION_GROUP}.{KEEP_AWAKE_ACTION}")),
    );
    menu.append(
        Some("Rename…"),
        Some(&format!("{ROW_ACTION_GROUP}.{RENAME_ACTION}")),
    );
    menu.append(
        Some("Delete account…"),
        Some(&format!("{ROW_ACTION_GROUP}.{DELETE_ACTION}")),
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
    let rename_id = id.clone();
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

    let rename = gio::SimpleAction::new(RENAME_ACTION, None);
    // Always sensitive: a rename is offered in every liveness, including
    // while an account is starting, unlike `PARKING_ACTION` above
    // (`FR.13.1`).
    let rename_sidebar = sidebar.clone();
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
    delete.connect_activate(move |_, _| {
        let Some(sidebar) = delete_sidebar.upgrade() else {
            return;
        };
        if let Some(handler) = sidebar.imp().on_delete_requested.borrow().as_ref() {
            handler(delete_id.clone());
        }
    });

    let group = gio::SimpleActionGroup::new();
    group.add_action(&parking);
    group.add_action(&keep_awake);
    group.add_action(&rename);
    group.add_action(&delete);
    settings.insert_action_group(ROW_ACTION_GROUP, Some(&group));
}

/// Rebuilds `settings`'s menu model and action group from `workspace`'s
/// current identity and its two `can_*` flags — `bind_row_menu`'s shape
/// applied to a heading, two sections: `Park all` and `Start all` first, the
/// deliberate actions (design rule 5), each `enabled` following
/// [`WorkspaceRow::can_park_all`] / [`WorkspaceRow::can_start_all`] exactly as
/// a row's own Park/Start item follows its account's state (design rule 2);
/// then, for a named workspace, `Rename…` and `Remove workspace`. `Ungrouped`
/// gets the first section alone — it has no name to rename and cannot be
/// removed (item 14 task 07). Rebuilt on every bind for the same reason
/// `bind_row_menu` is (the list recycles this `MenuButton`).
fn bind_heading_menu(
    settings: &gtk::MenuButton,
    workspace: &WorkspaceRow,
    sidebar: &glib::WeakRef<super::SessionSidebar>,
) {
    let id = WorkspaceId::new(workspace.id());
    let is_ungrouped = id.is_ungrouped();

    let menu = gio::Menu::new();
    let actions = gio::Menu::new();
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
    menu.append_section(None, &actions);

    if !is_ungrouped {
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
    settings.set_menu_model(Some(&menu));

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
                handler(id.clone());
            }
        });

        group.add_action(&rename);
        group.add_action(&remove);
    }

    settings.insert_action_group(HEADING_ACTION_GROUP, Some(&group));
}
