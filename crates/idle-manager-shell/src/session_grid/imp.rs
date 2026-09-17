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
use std::rc::Rc;
use std::sync::Once;
use std::time::Duration;

use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::glib;
use gtk::graphene;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{Layout, Liveness, SessionId, SlotId, Visibility, WorkspaceBook};
use webkit6::prelude::*;
use webkit6::{LoadEvent, WebView};

use crate::slot_placeholder::SlotPlaceholder;

/// A handler run when the user clicks a slot to focus it.
type SlotFocusHandler = Box<dyn Fn(SlotId)>;

/// A handler run with an account's id when its slot placeholder's button is
/// pressed. Same intent the sidebar row's button sends (task 05).
type StartHandler = Box<dyn Fn(SessionId)>;

/// A handler run with an account's id and the wheel's vertical delta when
/// `Ctrl` and the wheel turn over that account's place. Negative is a notch
/// up; the window decides what a notch means (architecture rule 8).
type ScrollZoomHandler = Box<dyn Fn(SessionId, f64)>;

/// A handler run with an account's id and the slot it was dropped on. The
/// grid decides nothing about what a drop means — swap, fill or no-op is the
/// book's call (architecture rule 8, task 05).
type AccountDroppedHandler = Box<dyn Fn(SessionId, SlotId)>;

/// How long the zoom readout stays up after the last gesture before it fades,
/// leaving nothing behind (`FR.11.6`). A run of gestures rearms it, so one
/// figure keeps updating rather than a queue forming.
const ZOOM_READOUT_FADE_MILLIS: u64 = 1000;

/// Opacity of the drop-target tint over a place under a drag (task 05's new
/// pattern; code-standards rule 5). Translucent enough that a game's page or
/// a parked panel underneath still reads through it.
const DROP_HIGHLIGHT_ALPHA: f32 = 0.18;

/// The transient percentage figure over one place, plus the fade timer that is
/// cancelled and rearmed on every gesture (item 09's new pattern). Held in an
/// [`Rc`] so the timer callback can reach the timer slot to clear it and never
/// double-remove a source that already fired.
struct Readout {
    label: gtk::Label,
    timer: RefCell<Option<glib::SourceId>>,
}

/// One session's view and where it currently sits.
struct SlotEntry {
    id: SessionId,
    overlay: gtk::Overlay,
    /// The name cover drawn under the view until the page paints.
    cover: gtk::Box,
    /// The cover's name label, kept so [`SessionGrid::sync`] can refresh it
    /// from the book on every pass — a rename would otherwise never reach it,
    /// since it was set only once, at registration (`FR.13.4`).
    cover_label: gtk::Label,
    /// The parked-account panel, an overlay kept for the slot's whole life and
    /// shown only while the account is parked or starting in this slot.
    placeholder: SlotPlaceholder,
    /// The transient zoom figure, a third overlay layer over the same stack
    /// that carries the cover and the panel — hidden until a gesture.
    readout: Rc<Readout>,
    /// The drag grip (item 10 task 04's new pattern): shown while the pointer
    /// hovers this place, hidden while a drag from it is under way, and
    /// forced hidden by [`SessionGrid::sync`] for an off-grid entry or in the
    /// `Single` layout, where there is nowhere to drop an account.
    grip: gtk::Image,
    placement: Visibility,
}

/// The composite-template backing object for [`super::SessionGrid`].
#[derive(Default, CompositeTemplate)]
#[template(resource = "/org/idlemanager/IdleManager/ui/session-grid.ui")]
pub struct SessionGrid {
    slots: RefCell<Vec<SlotEntry>>,
    layout: Cell<Layout>,
    focused: Cell<usize>,
    /// The slot under the pointer during a drag, tinted by [`Self::snapshot`]
    /// until the pointer leaves the grid or the drag ends (task 05). Not the
    /// same state as `focused`: grabbing or dropping never moves the current
    /// marker (`docs/design.md` rule 1).
    hovered_slot: Cell<Option<usize>>,
    pub(super) on_slot_focused: RefCell<Option<SlotFocusHandler>>,
    pub(super) on_start_requested: RefCell<Option<StartHandler>>,
    pub(super) on_zoom_scrolled: RefCell<Option<ScrollZoomHandler>>,
    pub(super) on_account_dropped: RefCell<Option<AccountDroppedHandler>>,
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

        install_styles();

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

        // Tracks the pointer during a drag so the hovered place can be
        // tinted. Its own pointer test covers the whole widget including its
        // children (`FR.14.6`), which is what lets it track a drag across a
        // place filled by a web page. It accepts no drops itself, so it never
        // competes with the drop target below.
        let drop_motion = gtk::DropControllerMotion::new();
        let motion_grid = obj.downgrade();
        drop_motion.connect_motion(move |_, x, y| {
            if let Some(grid) = motion_grid.upgrade() {
                grid.imp().set_hovered_slot_at(x, y);
            }
        });
        let leave_grid = obj.downgrade();
        drop_motion.connect_leave(move |_| {
            if let Some(grid) = leave_grid.upgrade() {
                grid.imp().clear_hovered_slot();
            }
        });
        obj.add_controller(drop_motion);

        // One drop target for the whole grid, not one per place, so an empty
        // place — which has no overlay of its own — accepts a drop too.
        // Capture phase: measured to be required, not a precaution
        // (roadmap item's Technical References) — in the bubble phase
        // `WebKitWebViewBase`'s own drop target on the page underneath
        // consumes the drop first and the grid never receives it.
        let drop_target =
            gtk::DropTarget::new(super::DraggedAccount::static_type(), gdk::DragAction::MOVE);
        drop_target.set_propagation_phase(gtk::PropagationPhase::Capture);
        let drop_grid = obj.downgrade();
        drop_target.connect_drop(move |_target, value, x, y| {
            let Some(grid) = drop_grid.upgrade() else {
                return false;
            };
            let imp = grid.imp();
            // The drop ends the drag over this pointer whether or not it
            // resolves to a slot, so the highlight always clears here rather
            // than waiting on the motion controller's own `leave` — a drop at
            // the same point the pointer already sat at is not a pointer
            // motion and would otherwise leave the tint stuck.
            imp.clear_hovered_slot();
            let Some(slot) = imp.slot_at(x, y) else {
                return false;
            };
            let Ok(payload) = value.get::<super::DraggedAccount>() else {
                return false;
            };
            if let Some(handler) = imp.on_account_dropped.borrow().as_ref() {
                handler(payload.session_id().clone(), slot);
            }
            true
        });
        obj.add_controller(drop_target);
    }

    fn dispose(&self) {
        for entry in self.slots.borrow().iter() {
            if let Some(timer) = entry.readout.timer.borrow_mut().take() {
                timer.remove();
            }
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

        self.draw_drop_highlight(snapshot);
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

        let (cover, cover_label) = build_cover(display_name);
        overlay.add_overlay(&cover);

        let placeholder = SlotPlaceholder::new();
        placeholder.set_name(display_name);
        placeholder.set_button_label("Start");
        overlay.add_overlay(&placeholder);

        let readout = build_readout();
        overlay.add_overlay(&readout.label);

        let grip_handle = build_grip();
        overlay.add_overlay(&grip_handle);
        self.wire_grip(&overlay, &grip_handle, id, display_name);

        // The wheel-zoom controller goes on the overlay, not the view: the
        // overlay lives for the account's whole life while the view is
        // destroyed and rebuilt on every park and start, and the controller
        // must survive that. Capture phase for the same reason the click
        // gesture uses it — the web view would otherwise consume the event on
        // the way down. Whether a capture-phase scroll controller here actually
        // sees a wheel event bound for the WebKitGTK view underneath (WebKit
        // scrolls in its own process) is verified in this item's
        // test-script.md; if the view wins, this moves onto the view and
        // `SessionView::start` re-attaches it on every rebuild (code standards
        // rule 18).
        // `DISCRETE` alongside `VERTICAL` because a step is a notch, not a
        // distance: without it the controller reports the smooth deltas the
        // device sends, and a high-resolution wheel or a touchpad sends several
        // fractional-delta events per physical notch — each one a full step, so
        // one notch compounded into two or three and the size moved by an
        // amount that varied with the device. `DISCRETE` makes GTK accumulate
        // those deltas and emit one ±1 delta per notch (code standards rule 18).
        let scroll = gtk::EventControllerScroll::new(
            gtk::EventControllerScrollFlags::VERTICAL | gtk::EventControllerScrollFlags::DISCRETE,
        );
        scroll.set_propagation_phase(gtk::PropagationPhase::Capture);
        let scroll_grid = self.obj().downgrade();
        let scroll_session = id.clone();
        scroll.connect_scroll(move |controller, _dx, dy| {
            let ctrl_held = controller.current_event().is_some_and(|event| {
                event
                    .modifier_state()
                    .contains(gdk::ModifierType::CONTROL_MASK)
            });
            if !ctrl_held {
                return glib::Propagation::Proceed;
            }
            let Some(grid) = scroll_grid.upgrade() else {
                return glib::Propagation::Proceed;
            };
            // `FR.11.8`: the gesture is gated on the click — it acts on the
            // place under the pointer, but only once that place is the focused
            // one, narrowing `FR.11.3`. Passing the
            // pointer over a game is not a choice to resize it, and a wheel that
            // resized whatever it crossed changed sizes nobody was looking at.
            if !grid.imp().is_focused_session(&scroll_session) {
                return glib::Propagation::Proceed;
            }
            if let Some(handler) = grid.imp().on_zoom_scrolled.borrow().as_ref() {
                handler(scroll_session.clone(), dy);
            }
            glib::Propagation::Stop
        });
        overlay.add_controller(scroll);

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
            cover_label,
            placeholder,
            readout,
            grip: grip_handle,
            placement: Visibility::OffGrid,
        });

        self.obj().queue_allocate();
    }

    /// Wires `handle` — the overlay's grip — to show on hover and to start a
    /// drag past the threshold, per the recipe measured in the roadmap item's
    /// Technical References.
    fn wire_grip(&self, overlay: &gtk::Overlay, handle: &gtk::Image, id: &SessionId, name: &str) {
        // Shows the grip while the pointer is anywhere inside the place —
        // over the game's page or the parked panel alike — and hides it on
        // leave (`FR.14.1`). Gated on layout here rather than only in `sync`:
        // `Single` never shows a grip at all, even for the one place that is
        // on screen, so there is nowhere to move an account (design's grip
        // rule this slice owes; wireframe "one-place arrangement").
        let hover = gtk::EventControllerMotion::new();
        let enter_owner = self.obj().downgrade();
        let enter_handle = handle.clone();
        hover.connect_enter(move |_, _, _| {
            if let Some(owner) = enter_owner.upgrade()
                && owner.imp().layout.get() != Layout::Single
            {
                enter_handle.set_visible(true);
            }
        });
        let leave_handle = handle.clone();
        hover.connect_leave(move |_| {
            leave_handle.set_visible(false);
        });
        overlay.add_controller(hover.clone());

        // The grip recipe from `docs/research/gtk4-drag-and-accordion.md:35`:
        // a claiming `GestureClick` grouped with a `DragSource`. Claiming
        // denies the sequence to any ancestor gesture; grouping is what keeps
        // the claim from also denying the drag source on this same widget.
        // The grid's own click-to-focus gesture does not depend on this claim
        // — it stays in the capture phase and instead skips focusing when a
        // pick lands on `slot-grip` (`focus_slot_at`) — but the claim and
        // group are still the documented, measured recipe (`FR.14.3`,
        // Technical References).
        let claim = gtk::GestureClick::new();
        claim.connect_pressed(|gesture, _n_press, _x, _y| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
        });

        let source = gtk::DragSource::new();
        source.set_actions(gdk::DragAction::MOVE);
        let payload = super::DraggedAccount::new(id.clone());
        source.set_content(Some(&gdk::ContentProvider::for_value(&payload.to_value())));

        // A chip carrying the account's name, not a `WidgetPaintable` of the
        // place: a quarter-window paintable would hide the very places being
        // dropped on. The grip hides for the drag's duration — `drag-begin`
        // fires the overlay's motion `leave` too, but hiding it directly here
        // keeps the grip's own state authoritative.
        let chip_text = name.to_owned();
        let begin_handle = handle.clone();
        source.connect_drag_begin(move |_source, drag| {
            let chip = gtk::Label::new(Some(&chip_text));
            chip.add_css_class("drag-chip");
            gtk::DragIcon::for_drag(drag).set_child(Some(&chip));
            begin_handle.set_visible(false);
        });
        // Restores the grip once the drag ends — dropped, cancelled, or
        // Escaped — if the pointer is still over this place; a plain
        // press-and-release never reaches this handler at all, since a drag
        // only starts past the threshold. Also clears the drop highlight: a
        // completed drop already clears it in the drop target's own handler,
        // but Escape and a release outside the grid are cancelled by GTK
        // before that handler ever runs, so this is the only place left to
        // catch them.
        let end_hover = hover;
        let end_owner = self.obj().downgrade();
        let end_handle = handle.clone();
        source.connect_drag_end(move |_source, _drag, _delete| {
            let Some(owner) = end_owner.upgrade() else {
                return;
            };
            owner.imp().clear_hovered_slot();
            if end_hover.contains_pointer() && owner.imp().layout.get() != Layout::Single {
                end_handle.set_visible(true);
            }
        });

        claim.group_with(&source);
        handle.add_controller(claim);
        handle.add_controller(source);
    }

    /// Whether `id` is the account sitting in the focused slot. False for an
    /// account the grid has no entry for, and for one whose entry is off-grid —
    /// neither can be the place the last click landed on.
    pub(super) fn is_focused_session(&self, id: &SessionId) -> bool {
        let focused = Visibility::InSlot(SlotId::new(self.focused.get()));
        self.slots
            .borrow()
            .iter()
            .any(|entry| &entry.id == id && entry.placement == focused)
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

    /// Removes `id`'s entry entirely: cancels its readout's fade timer if one
    /// is armed, unparents its overlay from the grid, and drops it from
    /// `slots`. Called only once the account's profile folder is actually
    /// gone (`FR.21.5`, item 11 task 08) — the grid decides nothing about
    /// when that is safe, the deletion sequence does. A no-op for an account
    /// the grid has no entry for.
    pub(super) fn remove_session(&self, id: &SessionId) {
        let mut slots = self.slots.borrow_mut();
        let Some(index) = slots.iter().position(|entry| &entry.id == id) else {
            return;
        };
        let entry = slots.remove(index);
        drop(slots);

        if let Some(timer) = entry.readout.timer.borrow_mut().take() {
            timer.remove();
        }
        entry.overlay.unparent();
        self.obj().queue_allocate();
    }

    /// Shows `figure` over `id`'s place, updating whatever is already there,
    /// and cancels and rearms that place's fade timer so a run of gestures
    /// shows one figure rather than a queue (`FR.11.6`). A no-op for an account
    /// with no place in the grid.
    pub(super) fn flash_zoom_readout(&self, id: &SessionId, figure: &str) {
        let slots = self.slots.borrow();
        let Some(entry) = slots.iter().find(|entry| &entry.id == id) else {
            return;
        };
        let readout = Rc::clone(&entry.readout);

        if let Some(timer) = readout.timer.borrow_mut().take() {
            timer.remove();
        }
        readout.label.set_text(figure);
        readout.label.set_visible(true);

        let armed = Rc::downgrade(&readout);
        let timer = glib::timeout_add_local_once(
            Duration::from_millis(ZOOM_READOUT_FADE_MILLIS),
            move || {
                if let Some(readout) = armed.upgrade() {
                    readout.timer.borrow_mut().take();
                    readout.label.set_visible(false);
                }
            },
        );
        *readout.timer.borrow_mut() = Some(timer);
    }

    pub(super) fn sync(&self, book: &WorkspaceBook) {
        self.layout.set(book.active().layout());
        self.focused.set(book.active().focused().index());

        let single = self.layout.get() == Layout::Single;
        for entry in self.slots.borrow_mut().iter_mut() {
            if let Some(session) = book
                .workspaces()
                .find_map(|workspace| workspace.book().session(&entry.id))
            {
                // The cover and the placeholder both follow the book's name on
                // every pass, the same way the placeholder's other fields
                // already do — a rename would otherwise never reach either,
                // since both were set once, at registration (`FR.13.4`).
                let name = session.display_name();
                entry.cover_label.set_markup(&cover_markup(name));
                entry.placeholder.set_name(name);
                apply_placeholder(&entry.placeholder, placeholder_panel(session.liveness()));
            }
            // An account not in the shown workspace is off-grid explicitly —
            // today an entry the book cannot place at all would otherwise keep
            // its stale placement, which with several workspaces would leave a
            // hidden one's view drawn in a place (`FR.18.1`).
            entry.placement = book.placement(&entry.id).unwrap_or(Visibility::OffGrid);
            // There is nowhere to drop an account off-grid or in `Single`, so
            // the grip never shows there — a hidden widget is never picked,
            // so a hidden grip cannot take a press either.
            if single || entry.placement == Visibility::OffGrid {
                entry.grip.set_visible(false);
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

        // A press on the grip must not make its place active (`FR.14.3`).
        // The grid's own click gesture stays in the capture phase — a web
        // page underneath would otherwise consume the click — so it cannot
        // rely on the grip's own claim, which runs later in the target/bubble
        // phases. It checks what is under the pointer instead: a pick that
        // lands on `slot-grip` focuses nothing. Measured on X11 and Wayland
        // (roadmap item's Technical References).
        if let Some(picked) = obj.pick(x, y, gtk::PickFlags::DEFAULT)
            && picked.has_css_class("slot-grip")
        {
            return;
        }

        let Some(slot) = self.slot_at(x, y) else {
            return;
        };

        self.focused.set(slot.index());
        if let Some(handler) = self.on_slot_focused.borrow().as_ref() {
            handler(slot);
        }
        obj.queue_draw();
    }

    /// The slot the point `(x, y)` in the grid's own coordinates falls in, or
    /// `None` outside the grid's own bounds or past however many slots the
    /// current layout actually has. Shared by click-to-focus and the drop
    /// target (task 05) — one place decides where a point on the grid lands.
    fn slot_at(&self, x: f64, y: f64) -> Option<SlotId> {
        let obj = self.obj();
        let (width, height) = (obj.width(), obj.height());
        if width <= 0 || height <= 0 {
            return None;
        }

        let layout = self.layout.get();
        let (columns, rows) = grid_dimensions(layout);
        let column = ((x * columns as f64 / f64::from(width)) as usize).min(columns - 1);
        let row = ((y * rows as f64 / f64::from(height)) as usize).min(rows - 1);
        let index = row * columns + column;
        if index >= layout.slot_count() {
            return None;
        }

        Some(SlotId::new(index))
    }

    /// Records the slot under `(x, y)` as the drop target's drag highlight and
    /// queues a redraw when it actually changed slot. Outside every slot —
    /// there is no slot at that point — clears the highlight the same way
    /// [`Self::clear_hovered_slot`] does.
    fn set_hovered_slot_at(&self, x: f64, y: f64) {
        let slot = self.slot_at(x, y).map(SlotId::index);
        if self.hovered_slot.replace(slot) != slot {
            self.obj().queue_draw();
        }
    }

    /// Clears the drag highlight when the pointer leaves the grid entirely.
    fn clear_hovered_slot(&self) {
        if self.hovered_slot.replace(None).is_some() {
            self.obj().queue_draw();
        }
    }

    /// Tints whichever slot [`Self::hovered_slot`] names, including the
    /// source place and an empty one — a drop is valid on either — drawn
    /// after the children and before [`Self::draw_slot_lines`] so the hairline
    /// and focus outline still read on top of it. A no-op with no drag under
    /// way.
    fn draw_drop_highlight(&self, snapshot: &gtk::Snapshot) {
        let Some(index) = self.hovered_slot.get() else {
            return;
        };

        let obj = self.obj();
        let (width, height) = (obj.width() as f32, obj.height() as f32);
        let layout = self.layout.get();
        if index >= layout.slot_count() {
            return;
        }
        let (columns, rows) = grid_dimensions(layout);
        let slot_width = width / columns as f32;
        let slot_height = height / rows as f32;
        let column = (index % columns) as f32;
        let row = (index / columns) as f32;

        let base = obj.color();
        let tint = gdk::RGBA::new(base.red(), base.green(), base.blue(), DROP_HIGHLIGHT_ALPHA);
        snapshot.append_color(
            &tint,
            &graphene::Rect::new(
                column * slot_width,
                row * slot_height,
                slot_width,
                slot_height,
            ),
        );
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

/// The transient zoom figure for one place: a short label on its own opaque
/// ground, centred horizontally and low in the place so it never covers what
/// the reader is adjusting (item 09 wireframe). Hidden until a gesture.
fn build_readout() -> Rc<Readout> {
    let label = gtk::Label::new(None);
    label.add_css_class("zoom-readout");
    label.set_halign(gtk::Align::Center);
    label.set_valign(gtk::Align::End);
    label.set_margin_bottom(24);
    label.set_visible(false);

    Rc::new(Readout {
        label,
        timer: RefCell::new(None),
    })
}

/// The drag grip for a place's top-right corner (item 10 task 04's new
/// pattern): the overlay child itself, not wrapped in a positioning box —
/// wrapping it would need `can-target = false` on the wrapper or it would
/// take every click in that corner away from the game underneath
/// (`FR.14.4`). Hidden until the pointer hovers the place.
fn build_grip() -> gtk::Image {
    let grip = gtk::Image::from_icon_name("list-drag-handle-symbolic");
    grip.add_css_class("slot-grip");
    grip.set_halign(gtk::Align::End);
    grip.set_valign(gtk::Align::Start);
    grip.set_margin_top(6);
    grip.set_margin_end(6);
    grip.set_visible(false);
    grip.set_cursor(gdk::Cursor::from_name("grab", None).as_ref());
    grip
}

/// Installs `session-grid.css` on the default display once. The provider is
/// display-global, so the [`Once`] keeps a second grid from stacking it — the
/// same shape as the sidebar's own install.
fn install_styles() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let Some(display) = gdk::Display::default() else {
            return;
        };
        let provider = gtk::CssProvider::new();
        provider.load_from_resource("/org/idlemanager/IdleManager/css/session-grid.css");
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });
}

/// An opaque cover carrying the account's name, shown until the page paints.
fn build_cover(display_name: &str) -> (gtk::Box, gtk::Label) {
    let cover = gtk::Box::new(gtk::Orientation::Vertical, 0);
    cover.add_css_class("background");

    let label = gtk::Label::new(None);
    label.set_markup(&cover_markup(display_name));
    label.set_halign(gtk::Align::Center);
    label.set_valign(gtk::Align::Center);
    label.set_hexpand(true);
    label.set_vexpand(true);
    cover.append(&label);

    (cover, label)
}

/// The cover label's markup for `display_name`, shared by [`build_cover`] and
/// [`SessionGrid::sync`] so a rename's refresh renders identically to the
/// name the cover was first built with.
fn cover_markup(display_name: &str) -> String {
    format!(
        "<span size='xx-large'>{}</span>",
        glib::markup_escape_text(display_name)
    )
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
