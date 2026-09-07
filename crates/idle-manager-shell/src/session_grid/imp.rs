//! The grid's private state, geometry and drawing (architecture rule 12).

// Widget pixel geometry: dimensions and slot counts are small non-negative
// integers and sub-pixel precision is irrelevant, so the numeric casts in the
// layout and drawing math below are all safe (code-standards rule 27).
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::cell::{Cell, RefCell};

use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::glib;
use gtk::graphene;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{Layout, Liveness, SessionBook, SessionId, SlotId, Visibility};
use webkit6::prelude::*;
use webkit6::{LoadEvent, WebView};

use crate::slot_placeholder::SlotPlaceholder;

/// A handler run when the user clicks a slot to focus it.
type SlotFocusHandler = Box<dyn Fn(SlotId)>;

/// A handler run with an account's id when its slot placeholder's button is
/// pressed. Same intent the sidebar row's button sends (task 05).
type StartHandler = Box<dyn Fn(SessionId)>;

/// One session's view and where it currently sits.
struct SlotEntry {
    id: SessionId,
    overlay: gtk::Overlay,
    /// The name cover drawn under the view until the page paints.
    cover: gtk::Box,
    /// The parked-account panel, an overlay kept for the slot's whole life and
    /// shown only while the account is parked or starting in this slot.
    placeholder: SlotPlaceholder,
    placement: Visibility,
}

/// The composite-template backing object for [`super::SessionGrid`].
#[derive(Default, CompositeTemplate)]
#[template(resource = "/org/idlemanager/IdleManager/ui/session-grid.ui")]
pub struct SessionGrid {
    slots: RefCell<Vec<SlotEntry>>,
    layout: Cell<Layout>,
    focused: Cell<usize>,
    pub(super) on_slot_focused: RefCell<Option<SlotFocusHandler>>,
    pub(super) on_start_requested: RefCell<Option<StartHandler>>,
}

#[glib::object_subclass]
impl ObjectSubclass for SessionGrid {
    const NAME: &'static str = "IdleManagerSessionGrid";
    type Type = super::SessionGrid;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
        klass.set_layout_manager_type::<super::SlotLayout>();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl std::fmt::Debug for SessionGrid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionGrid").finish_non_exhaustive()
    }
}

impl ObjectImpl for SessionGrid {
    fn constructed(&self) {
        self.parent_constructed();

        let obj = self.obj();
        // An off-grid child is allocated outside these bounds; clipping is what
        // keeps it invisible without unrealising it.
        obj.set_overflow(gtk::Overflow::Hidden);

        let click = gtk::GestureClick::new();
        // Capture phase: a slot is filled by a web view that would otherwise
        // consume the press, so the grid has to see it on the way down. The
        // gesture never claims the event, so the click still reaches the page.
        click.set_propagation_phase(gtk::PropagationPhase::Capture);
        let grid = obj.downgrade();
        click.connect_pressed(move |_, _, x, y| {
            if let Some(grid) = grid.upgrade() {
                grid.imp().focus_slot_at(x, y);
            }
        });
        obj.add_controller(click);
    }

    fn dispose(&self) {
        for entry in self.slots.borrow().iter() {
            entry.overlay.unparent();
        }
    }
}

impl WidgetImpl for SessionGrid {
    fn snapshot(&self, snapshot: &gtk::Snapshot) {
        let obj = self.obj();

        let mut child = obj.first_child();
        while let Some(current) = child {
            obj.snapshot_child(&current, snapshot);
            child = current.next_sibling();
        }

        self.draw_slot_lines(snapshot);
    }
}

impl SessionGrid {
    /// Registers a brand-new account with its first running view already in
    /// its slot. The cover carries the name until the page paints; the
    /// placeholder is built hidden.
    pub(super) fn add_session(&self, id: &SessionId, display_name: &str, view: &WebView) {
        self.register_slot(id, display_name, Some(view));
    }

    /// Registers a restored account with no view yet (item 07 task 04). The
    /// placeholder shows immediately — its line and button come from the next
    /// `sync` reading the account's liveness — and the start queue attaches a
    /// view later with [`SessionGrid::attach_view`].
    pub(super) fn add_dormant_session(&self, id: &SessionId, display_name: &str) {
        self.register_slot(id, display_name, None);
    }

    fn register_slot(&self, id: &SessionId, display_name: &str, view: Option<&WebView>) {
        let overlay = gtk::Overlay::new();

        let cover = build_cover(display_name);
        overlay.add_overlay(&cover);

        let placeholder = SlotPlaceholder::new();
        placeholder.set_name(display_name);
        placeholder.set_button_label("Start");
        overlay.add_overlay(&placeholder);

        let grid = self.obj().downgrade();
        let session = id.clone();
        placeholder.connect_start_requested(move || {
            let Some(grid) = grid.upgrade() else {
                return;
            };
            if let Some(handler) = grid.imp().on_start_requested.borrow().as_ref() {
                handler(session.clone());
            }
        });

        match view {
            Some(view) => {
                overlay.set_child(Some(view));
                hide_cover_once_painted(view, &cover);
                placeholder.set_state_text("Parked");
                placeholder.set_visible(false);
            }
            None => {
                // A restored account: the cover is for a loading view it does
                // not have yet, and the placeholder's line and visibility come
                // from the next `sync` reading its liveness.
                cover.set_visible(false);
            }
        }

        overlay.set_parent(&*self.obj());

        self.slots.borrow_mut().push(SlotEntry {
            id: id.clone(),
            overlay,
            cover,
            placeholder,
            placement: Visibility::OffGrid,
        });

        self.obj().queue_allocate();
    }

    /// Reloads the web view sitting in the focused slot. A no-op when that slot
    /// is empty or its session is off-grid — there is nothing on screen to
    /// reload. The header-bar button and the `F5` / `Ctrl`+`R` accelerators are
    /// the only callers.
    pub(super) fn reload_focused(&self) {
        let focused = Visibility::InSlot(SlotId::new(self.focused.get()));
        let slots = self.slots.borrow();
        let Some(entry) = slots.iter().find(|entry| entry.placement == focused) else {
            return;
        };
        if let Some(view) = entry
            .overlay
            .child()
            .and_then(|child| child.downcast::<WebView>().ok())
        {
            tracing::debug!(session = %entry.id, "reloading the focused view");
            view.reload();
        }
    }

    /// Drops the parked account's view out of its slot overlay and shows the
    /// placeholder panel in its place. The `SlotEntry` keeps its placement, so
    /// the slot stays the account's and switching layouts still moves it. A
    /// no-op for an account the grid has no entry for.
    pub(super) fn release_view(&self, id: &SessionId) {
        let slots = self.slots.borrow();
        let Some(entry) = slots.iter().find(|entry| &entry.id == id) else {
            return;
        };
        entry.overlay.set_child(None::<&gtk::Widget>);
        entry.cover.set_visible(false);
        entry.placeholder.set_state_text("Parked");
        entry.placeholder.set_button_label("Start");
        entry.placeholder.set_button_sensitive(true);
        entry.placeholder.set_visible(true);
        tracing::debug!(session = %id, "parked: showing the slot placeholder");
    }

    /// Puts a freshly started account's view back into the slot it still holds.
    /// The placeholder stays up, now reading `Starting` with its button
    /// disabled, and covers the blank loading view until the page paints. The
    /// `SlotEntry` and its placement are unchanged. A no-op for an account the
    /// grid has no entry for.
    pub(super) fn attach_view(&self, id: &SessionId, view: &WebView) {
        let slots = self.slots.borrow();
        let Some(entry) = slots.iter().find(|entry| &entry.id == id) else {
            return;
        };
        entry.overlay.set_child(Some(view));
        entry.cover.set_visible(false);
        entry.placeholder.set_state_text("Starting");
        entry.placeholder.set_button_sensitive(false);

        let placeholder = entry.placeholder.clone();
        view.connect_load_changed(move |_, event| {
            if matches!(event, LoadEvent::Committed | LoadEvent::Finished) {
                placeholder.set_visible(false);
            }
        });
        tracing::debug!(session = %id, "starting: view attached behind the placeholder");
        self.obj().queue_allocate();
    }

    pub(super) fn sync(&self, book: &SessionBook) {
        self.layout.set(book.layout());
        self.focused.set(book.focused().index());

        for entry in self.slots.borrow_mut().iter_mut() {
            if let Some(session) = book.sessions().iter().find(|s| s.id() == &entry.id) {
                entry.placement = session.visibility();
                apply_placeholder(&entry.placeholder, placeholder_panel(session.liveness()));
            }
        }

        let obj = self.obj();
        obj.queue_allocate();
        obj.queue_draw();
    }

    /// Allocates every child. Called by [`super::SlotLayout`].
    pub(super) fn allocate_slots(&self, width: i32, height: i32) {
        let layout = self.layout.get();
        let (columns, rows) = grid_dimensions(layout);
        let slot_width = width / i32::try_from(columns).unwrap_or(1);
        let slot_height = height / i32::try_from(rows).unwrap_or(1);

        for entry in self.slots.borrow().iter() {
            let allocation = match entry.placement {
                Visibility::InSlot(slot) => {
                    let index = slot.index();
                    let column = i32::try_from(index % columns).unwrap_or(0);
                    let row = i32::try_from(index / columns).unwrap_or(0);
                    gdk::Rectangle::new(
                        column * slot_width,
                        row * slot_height,
                        slot_width,
                        slot_height,
                    )
                }
                Visibility::OffGrid => {
                    // Outside our own bounds and clipped by
                    // `set_overflow(Hidden)`: still realised, mapped and
                    // allocated, so WebKit keeps it running. A `GtkStack` would
                    // unrealise it, and WebKit throttles a view it believes
                    // hidden — for an idle game that is lost progress
                    // (code-standards rule 18).
                    gdk::Rectangle::new(
                        width + slot_width.max(1),
                        height + slot_height.max(1),
                        slot_width.max(1),
                        slot_height.max(1),
                    )
                }
            };
            entry.overlay.size_allocate(&allocation, -1);
        }
    }

    fn focus_slot_at(&self, x: f64, y: f64) {
        let obj = self.obj();
        let (width, height) = (obj.width(), obj.height());
        if width <= 0 || height <= 0 {
            return;
        }

        let layout = self.layout.get();
        let (columns, rows) = grid_dimensions(layout);
        let column = ((x * columns as f64 / f64::from(width)) as usize).min(columns - 1);
        let row = ((y * rows as f64 / f64::from(height)) as usize).min(rows - 1);
        let index = row * columns + column;
        if index >= layout.slot_count() {
            return;
        }

        self.focused.set(index);
        if let Some(handler) = self.on_slot_focused.borrow().as_ref() {
            handler(SlotId::new(index));
        }
        obj.queue_draw();
    }

    fn draw_slot_lines(&self, snapshot: &gtk::Snapshot) {
        if self.slots.borrow().is_empty() {
            return;
        }

        let obj = self.obj();
        let (width, height) = (obj.width() as f32, obj.height() as f32);
        let layout = self.layout.get();
        let (columns, rows) = grid_dimensions(layout);

        let base = obj.color();
        let hairline = gdk::RGBA::new(base.red(), base.green(), base.blue(), 0.15);
        let marker = gdk::RGBA::new(base.red(), base.green(), base.blue(), 0.55);

        for column in 1..columns {
            let x = width * column as f32 / columns as f32;
            snapshot.append_color(&hairline, &graphene::Rect::new(x - 0.5, 0.0, 1.0, height));
        }
        for row in 1..rows {
            let y = height * row as f32 / rows as f32;
            snapshot.append_color(&hairline, &graphene::Rect::new(0.0, y - 0.5, width, 1.0));
        }

        let focused = self.focused.get();
        if focused < layout.slot_count() {
            let slot_width = width / columns as f32;
            let slot_height = height / rows as f32;
            let column = (focused % columns) as f32;
            let row = (focused / columns) as f32;
            let (x, y) = (column * slot_width, row * slot_height);
            let thickness = 2.0;
            snapshot.append_color(&marker, &graphene::Rect::new(x, y, slot_width, thickness));
            snapshot.append_color(
                &marker,
                &graphene::Rect::new(x, y + slot_height - thickness, slot_width, thickness),
            );
            snapshot.append_color(&marker, &graphene::Rect::new(x, y, thickness, slot_height));
            snapshot.append_color(
                &marker,
                &graphene::Rect::new(x + slot_width - thickness, y, thickness, slot_height),
            );
        }
    }
}

/// Columns and rows for each layout: one cell, two side by side, or two by two.
fn grid_dimensions(layout: Layout) -> (usize, usize) {
    match layout {
        Layout::Single => (1, 1),
        Layout::SideBySide => (2, 1),
        Layout::Grid => (2, 2),
    }
}

/// What a slot's placeholder panel reads for an account in `liveness`, or
/// `None` when the account is live and its view fills the slot.
///
/// Derived during `sync` — not only from the imperative `release_view` /
/// `attach_view` calls — so a slot drawn straight from a restored book reads
/// correctly before any view exists (design rule 4, architecture rule 8). A
/// queued account gets the line and no button at all: the start queue owns the
/// order and there is nothing useful to press while it drains.
fn placeholder_panel(liveness: Liveness) -> Option<PlaceholderPanel> {
    match liveness {
        Liveness::Parked => Some(PlaceholderPanel {
            state_text: "Parked",
            button_visible: true,
            button_sensitive: true,
        }),
        Liveness::Queued => Some(PlaceholderPanel {
            state_text: "Queued",
            button_visible: false,
            button_sensitive: false,
        }),
        Liveness::Starting => Some(PlaceholderPanel {
            state_text: "Starting",
            button_visible: true,
            button_sensitive: false,
        }),
        Liveness::Live => None,
    }
}

/// The three placeholder facts `placeholder_panel` derives from a liveness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlaceholderPanel {
    state_text: &'static str,
    button_visible: bool,
    button_sensitive: bool,
}

/// Pushes `panel` onto `placeholder`, or hides the panel when `panel` is
/// `None`. The `Start` label is constant here; item 08's failure panel is the
/// same widget with a different one.
fn apply_placeholder(placeholder: &SlotPlaceholder, panel: Option<PlaceholderPanel>) {
    let Some(panel) = panel else {
        placeholder.set_visible(false);
        return;
    };
    placeholder.set_state_text(panel.state_text);
    placeholder.set_button_label("Start");
    placeholder.set_button_visible(panel.button_visible);
    placeholder.set_button_sensitive(panel.button_sensitive);
    placeholder.set_visible(true);
}

/// Hides `cover` the first time `view` commits a page — the name shows on the
/// window's own background until then, and the live page covers it after.
fn hide_cover_once_painted(view: &WebView, cover: &gtk::Box) {
    let cover = cover.clone();
    view.connect_load_changed(move |_, event| {
        if matches!(event, LoadEvent::Committed | LoadEvent::Finished) {
            cover.set_visible(false);
        }
    });
}

/// An opaque cover carrying the account's name, shown until the page paints.
fn build_cover(display_name: &str) -> gtk::Box {
    let cover = gtk::Box::new(gtk::Orientation::Vertical, 0);
    cover.add_css_class("background");

    let label = gtk::Label::new(None);
    label.set_markup(&format!(
        "<span size='xx-large'>{}</span>",
        glib::markup_escape_text(display_name)
    ));
    label.set_halign(gtk::Align::Center);
    label.set_valign(gtk::Align::Center);
    label.set_hexpand(true);
    label.set_vexpand(true);
    cover.append(&label);

    cover
}

/// The layout manager [`super::SessionGrid`] installs on itself.
#[derive(Debug, Default)]
pub struct SlotLayout;

#[glib::object_subclass]
impl ObjectSubclass for SlotLayout {
    const NAME: &'static str = "IdleManagerSlotLayout";
    type Type = super::SlotLayout;
    type ParentType = gtk::LayoutManager;
}

impl ObjectImpl for SlotLayout {}

impl LayoutManagerImpl for SlotLayout {
    fn measure(
        &self,
        _widget: &gtk::Widget,
        _orientation: gtk::Orientation,
        _for_size: i32,
    ) -> (i32, i32, i32, i32) {
        // The grid takes whatever space it is given and divides it; it asks for
        // nothing of its own.
        (0, 0, -1, -1)
    }

    fn allocate(&self, widget: &gtk::Widget, width: i32, height: i32, _baseline: i32) {
        let Some(grid) = widget.downcast_ref::<super::SessionGrid>() else {
            return;
        };
        grid.imp().allocate_slots(width, height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_queued_account_gets_the_queued_line_and_no_button() {
        let panel = placeholder_panel(Liveness::Queued).expect("a queued account shows a panel");

        assert_eq!((panel.state_text, panel.button_visible), ("Queued", false));
    }

    #[test]
    fn a_parked_accounts_panel_is_unchanged_by_the_queued_state() {
        let panel = placeholder_panel(Liveness::Parked).expect("a parked account shows a panel");

        assert_eq!(
            (
                panel.state_text,
                panel.button_visible,
                panel.button_sensitive
            ),
            ("Parked", true, true),
        );
    }

    #[test]
    fn a_live_account_shows_no_panel() {
        assert_eq!(placeholder_panel(Liveness::Live), None);
    }
}
