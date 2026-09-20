//! The index down the leading edge of the window: one row per account, in the
//! order they were added, whether or not each one currently holds a slot.
//!
//! It renders from [`SessionBook`] state and, on row activation, reports the
//! account's id back through [`SessionSidebar::connect_row_activated`]; it
//! decides nothing itself (architecture rule 8).

mod imp;
mod row;
mod workspace_row;

use std::sync::Arc;

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{Destinations, MemoryProbe, SessionId, WorkspaceBook, WorkspaceId};

/// The destination a `Move to…` choice named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveTarget {
    /// Move into this already-existing workspace.
    Existing(WorkspaceId),
    /// The escape hatch: create a new workspace holding exactly the ticked
    /// accounts (item 11 task 06, `FR.17.6`).
    New,
}

glib::wrapper! {
    /// The account list widget, built from `ui/session-sidebar.ui`. Build one
    /// with [`SessionSidebar::new`], feed it [`SessionSidebar::sync`] whenever
    /// the book changes, and install a handler with
    /// [`SessionSidebar::connect_row_activated`].
    pub struct SessionSidebar(ObjectSubclass<imp::SessionSidebar>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl SessionSidebar {
    /// Builds an empty sidebar showing its empty-state line.
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    /// Starts the memory footer sampling `probe`. Call once, when the ports are
    /// attached; the footer runs each sample off the main context (architecture
    /// rule 10).
    pub fn start_memory_sampling(&self, probe: Arc<dyn MemoryProbe>) {
        self.imp().start_memory_sampling(probe);
    }

    /// Redraws the whole tree from `book`: one heading per workspace, in
    /// sidebar order, each with its accounts nested beneath — placement from
    /// [`idle_manager_core::WorkspaceBook::placement`], so an account in a
    /// workspace that is not shown reads `background` regardless of the slot
    /// it still holds there. Each heading's expansion is re-applied from
    /// `book` without re-emitting [`SessionSidebar::connect_expansion_toggled`].
    /// Shows the empty-state line in place of the tree only when no workspace
    /// holds any account at all.
    pub fn sync(&self, book: &WorkspaceBook) {
        self.imp().sync(book);
    }

    /// Registers `handler` to run with an account's id when its row is
    /// activated. Replaces any previous handler.
    pub fn connect_row_activated(&self, handler: impl Fn(SessionId) + 'static) {
        self.imp().on_activated.replace(Some(Box::new(handler)));
    }

    /// Registers `handler` to run with an account's id when its row's
    /// park/start button is pressed. The window decides which direction the
    /// press means from the account's current liveness; the row carries no
    /// state of its own (architecture rule 8). Replaces any previous handler.
    pub fn connect_parking_toggled(&self, handler: impl Fn(SessionId) + 'static) {
        self.imp()
            .on_parking_toggled
            .replace(Some(Box::new(handler)));
    }

    /// Registers `handler` to run with an account's id and the value asked
    /// for when its row menu's "Keep running when hidden" item is chosen.
    /// The menu reports only the intent it was given; whether the flag
    /// actually changes, and what the shell does in response, is the
    /// window's to decide (architecture rule 8). Replaces any previous
    /// handler.
    pub fn connect_keep_awake_toggled(&self, handler: impl Fn(SessionId, bool) + 'static) {
        self.imp()
            .on_keep_awake_toggled
            .replace(Some(Box::new(handler)));
    }

    /// Registers `handler` to run with an account's id when its row menu's
    /// `Rename…` item is chosen. The row decides nothing else: the window
    /// opens the rename dialog and the book decides whether a typed name is
    /// stored (architecture rule 8). Replaces any previous handler.
    pub fn connect_rename_requested(&self, handler: impl Fn(SessionId) + 'static) {
        self.imp()
            .on_rename_requested
            .replace(Some(Box::new(handler)));
    }

    /// Registers `handler` to run with a workspace's id and the state its
    /// heading was expanded to when a person clicks a heading or its expander
    /// arrow. Never runs while [`SessionSidebar::sync`] is re-applying
    /// `book`'s own remembered state (`FR.16.2`) — only a person's own click
    /// is a request to store one. Replaces any previous handler.
    pub fn connect_expansion_toggled(&self, handler: impl Fn(WorkspaceId, bool) + 'static) {
        self.imp()
            .on_expansion_toggled
            .replace(Some(Box::new(handler)));
    }

    /// How many accounts are currently ticked. The window asks
    /// [`idle_manager_core::WorkspaceBook::destinations`] with this on every
    /// redraw and hands the answer back through
    /// [`SessionSidebar::set_move_destinations`], so the `Move to…` menu is
    /// never stale by the time it opens (`FR.17.2`).
    #[must_use]
    pub fn ticked_count(&self) -> usize {
        self.imp().ticked_count()
    }

    /// Supplies what the window found fits the current ticked count. The
    /// sidebar never decides which workspaces have room itself.
    pub fn set_move_destinations(&self, destinations: Destinations) {
        self.imp().set_move_destinations(destinations);
    }

    /// Leaves selection mode and clears every tick. Call only after a move or
    /// a create has actually been applied — never after a cancelled name
    /// window, so a mistyped name never costs the selection (`FR.17.7`).
    pub fn end_selection(&self) {
        self.imp().end_selection();
    }

    /// Registers `handler` to run with the ticked ids and the chosen
    /// destination when `Move to…` picks one. The sidebar decides nothing
    /// about whether the move can happen; the window asks the book
    /// (architecture rule 8). Replaces any previous handler.
    pub fn connect_move_requested(&self, handler: impl Fn(Vec<SessionId>, MoveTarget) + 'static) {
        self.imp()
            .on_move_requested
            .replace(Some(Box::new(handler)));
    }

    /// Registers `handler` to run whenever the sidebar's own selection state
    /// changes — the mode toggled, or a tick toggled — so the window knows to
    /// redraw and carry the change to the row widgets. Replaces any previous
    /// handler.
    pub fn connect_selection_changed(&self, handler: impl Fn() + 'static) {
        self.imp()
            .on_selection_changed
            .replace(Some(Box::new(handler)));
    }

    /// Registers `handler` to run with a workspace's id when its heading
    /// menu's `Rename…` item is chosen. The row decides nothing else: the
    /// window opens the shared name window and the book decides whether a
    /// typed name is stored (architecture rule 8). Replaces any previous
    /// handler.
    pub fn connect_workspace_rename_requested(&self, handler: impl Fn(WorkspaceId) + 'static) {
        self.imp()
            .on_workspace_rename_requested
            .replace(Some(Box::new(handler)));
    }

    /// Registers `handler` to run with a workspace's id when its heading
    /// menu's `Remove workspace` item is chosen. Replaces any previous
    /// handler.
    pub fn connect_workspace_remove_requested(&self, handler: impl Fn(WorkspaceId) + 'static) {
        self.imp()
            .on_workspace_remove_requested
            .replace(Some(Box::new(handler)));
    }

    /// Registers `handler` to run with a workspace's id when its heading
    /// menu's `Park all` item is chosen. The row decides nothing else: the
    /// window asks the book which accounts that touches and what to do with
    /// each one (architecture rule 8, `FR.24.1`). Replaces any previous
    /// handler.
    pub fn connect_park_all_requested(&self, handler: impl Fn(WorkspaceId) + 'static) {
        self.imp()
            .on_park_all_requested
            .replace(Some(Box::new(handler)));
    }

    /// Registers `handler` to run with a workspace's id when its heading
    /// menu's `Start all` item is chosen. Otherwise exactly
    /// [`SessionSidebar::connect_park_all_requested`] (`FR.24.2`). Replaces
    /// any previous handler.
    pub fn connect_start_all_requested(&self, handler: impl Fn(WorkspaceId) + 'static) {
        self.imp()
            .on_start_all_requested
            .replace(Some(Box::new(handler)));
    }

    /// Registers `handler` to run with an account's id when its row menu's
    /// `Delete account…` item is chosen. The row decides nothing else: the
    /// window opens the confirmation window and the deletion sequence is its
    /// own (architecture rule 8, item 11 task 08). Replaces any previous
    /// handler.
    pub fn connect_delete_requested(&self, handler: impl Fn(SessionId) + 'static) {
        self.imp()
            .on_delete_requested
            .replace(Some(Box::new(handler)));
    }

    /// Drops `id` from the ticked set, without changing selection mode
    /// itself. Called once an account is actually gone — a delete — so a
    /// stale id in the set never survives it (item 11 task 08). A no-op if
    /// `id` was not ticked.
    pub fn forget_ticked(&self, id: &SessionId) {
        self.imp().forget_ticked(id);
    }

    /// Whether a click ticks a row instead of switching to it right now
    /// (item 11 task 05). The window's navigation shortcuts read this before
    /// acting: while the owner is choosing accounts to move, `Shift`+`Tab`
    /// and `Ctrl`+`Tab` must change nothing on screen (`FR.23.4`).
    #[must_use]
    pub(crate) fn is_selecting(&self) -> bool {
        self.imp().is_selecting.get()
    }
}

impl Default for SessionSidebar {
    fn default() -> Self {
        Self::new()
    }
}
