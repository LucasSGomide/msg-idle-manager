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

use idle_manager_core::{Layout, SessionBook, SessionId, SlotId, Visibility};
use webkit6::prelude::*;
use webkit6::{LoadEvent, WebView};

/// A handler run when the user clicks a slot to focus it.
type SlotFocusHandler = Box<dyn Fn(SlotId)>;

/// One session's view and where it currently sits.
struct SlotEntry {
    id: SessionId,
    overlay: gtk::Overlay,
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
    pub(super) fn add_session(&self, id: &SessionId, display_name: &str, view: &WebView) {
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(view));

        let cover = build_cover(display_name);
        overlay.add_overlay(&cover);

        // The name shows on the window's own background until the page commits
        // its first bytes, then the live page covers it.
        view.connect_load_changed(move |_, event| {
            if matches!(event, LoadEvent::Committed | LoadEvent::Finished) {
                cover.set_visible(false);
            }
        });

        overlay.set_parent(&*self.obj());

        self.slots.borrow_mut().push(SlotEntry {
            id: id.clone(),
            overlay,
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

    pub(super) fn sync(&self, book: &SessionBook) {
        self.layout.set(book.layout());
        self.focused.set(book.focused().index());

        for entry in self.slots.borrow_mut().iter_mut() {
            if let Some(session) = book.sessions().iter().find(|s| s.id() == &entry.id) {
                entry.placement = session.visibility();
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
