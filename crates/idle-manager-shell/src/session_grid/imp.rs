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
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{
    DEFAULT_MOBILE_VIEWPORT, Layout, Liveness, SessionId, SlotId, Viewport, Visibility,
    WorkspaceBook,
};

use crate::slot_placeholder::SlotPlaceholder;
use crate::web_engine::EngineView;

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
    /// The readout's own native surface on Windows, where a `WebView2` child
    /// window always draws above a plain overlay layer (roadmap item 12 task
    /// 05, design rule 14). `label` is its child; showing and hiding the
    /// figure is `popup()`/`popdown()` on this instead of `label`'s own
    /// visibility. Absent on Linux, where the overlay layer still wins.
    #[cfg(windows)]
    popover: gtk::Popover,
    timer: RefCell<Option<glib::SourceId>>,
}

/// One session's view and where it currently sits.
struct SlotEntry {
    id: SessionId,
    overlay: gtk::Overlay,
    /// The widget actually parented to the grid: `overlay` itself on Linux,
    /// or, on Windows, the vertical box wrapping the `.grip-strip` row above
    /// it and `overlay` below (design rule 14, [`mount_grip`]). Every place
    /// that used to parent, unparent or allocate `overlay` directly now goes
    /// through this instead, so a Windows slot's strip moves and sizes with
    /// its place. The two are the same widget on Linux — `mount_grip`
    /// upcasts `overlay` unchanged — so nothing there actually changes.
    mount: gtk::Widget,
    /// The engine view currently in the overlay, or `None` while the account
    /// is parked or restored with no view built yet. Kept here, not read back
    /// off the overlay's child, so [`SessionGrid::reload_focused`] can reload
    /// it without downcasting a raw widget to an engine-specific type — the
    /// grid stays engine-neutral (architecture rule 6).
    view: RefCell<Option<EngineView>>,
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
    /// The `.grip-strip` row above this place on Windows, shown only while
    /// the place actually holds a live game there is somewhere to move
    /// ([`sync_grip_strip`], design rules 4 and 14). Absent on Linux, where
    /// the grip is an overlay layer over the place with no row of its own.
    #[cfg(windows)]
    strip: gtk::Box,
    placement: Visibility,
}

/// The composite-template backing object for [`super::SessionGrid`].
#[derive(Default, CompositeTemplate)]
#[template(resource = "/org/idlemanager/IdleManager/ui/session-grid.ui")]
pub struct SessionGrid {
    slots: RefCell<Vec<SlotEntry>>,
    layout: Cell<Layout>,
    focused: Cell<usize>,
    /// The phone viewport the one `Mobile` slot is shaped as, read off the
    /// book on every [`Self::sync`]; `None` while the mode is off (roadmap
    /// item 13). Falls back to [`DEFAULT_MOBILE_VIEWPORT`] if the layout is
    /// `Mobile` with no viewport, which `WorkspaceBook::enter_mobile_mode`
    /// never produces.
    mobile_viewport: Cell<Option<Viewport>>,
    /// The `Mobile` slot's last logged allocation, so the debug line is
    /// written when it changes and not on every allocation pass.
    mobile_allocation: Cell<Option<SlotRect>>,
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
            #[cfg(windows)]
            entry.readout.popover.unparent();
            entry.mount.unparent();
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
    pub(super) fn add_session(&self, id: &SessionId, display_name: &str, view: &EngineView) {
        self.register_slot(id, display_name, Some(view));
    }

    /// Registers a restored account with no view yet (item 07 task 04). The
    /// placeholder shows immediately — its line and button come from the next
    /// `sync` reading the account's liveness — and the start queue attaches a
    /// view later with [`SessionGrid::attach_view`].
    pub(super) fn add_dormant_session(&self, id: &SessionId, display_name: &str) {
        self.register_slot(id, display_name, None);
    }

    fn register_slot(&self, id: &SessionId, display_name: &str, view: Option<&EngineView>) {
        let overlay = gtk::Overlay::new();

        let (cover, cover_label) = build_cover(display_name);
        overlay.add_overlay(&cover);

        let placeholder = SlotPlaceholder::new();
        placeholder.set_name(display_name);
        placeholder.set_button_label("Start");
        overlay.add_overlay(&placeholder);

        let readout = build_readout(&overlay);
        #[cfg(not(windows))]
        overlay.add_overlay(&readout.label);

        let grip_handle = build_grip();
        self.wire_grip(&overlay, &grip_handle, id, display_name);
        let mount = mount_grip(&overlay, &grip_handle);

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
            // Zoom is locked in `Mobile` — the slot's pixel size is the
            // phone's viewport — so the gesture changes nothing and flashes
            // no readout there (Remote Access `FR.3.2`, design rule 10). It
            // is still swallowed, so the page underneath never sees a
            // `Ctrl`+wheel it could act on itself.
            if grid.imp().layout.get() == Layout::Mobile {
                return glib::Propagation::Stop;
            }
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
                overlay.set_child(Some(&view.widget()));
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

        mount.widget.set_parent(&*self.obj());

        self.slots.borrow_mut().push(SlotEntry {
            id: id.clone(),
            overlay,
            mount: mount.widget,
            #[cfg(windows)]
            strip: mount.strip,
            view: RefCell::new(view.cloned()),
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
        // a one-slot layout never shows a grip at all, even for the one place
        // that is on screen, so there is nowhere to move an account (design's
        // grip rule this slice owes; wireframe "one-place arrangement").
        let hover = gtk::EventControllerMotion::new();
        let enter_owner = self.obj().downgrade();
        let enter_handle = handle.clone();
        hover.connect_enter(move |_, _, _| {
            if let Some(owner) = enter_owner.upgrade()
                && has_somewhere_to_drop(owner.imp().layout.get())
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
        #[cfg(windows)]
        let begin_owner = self.obj().downgrade();
        source.connect_drag_begin(move |_source, drag| {
            let chip = gtk::Label::new(Some(&chip_text));
            chip.add_css_class("drag-chip");
            gtk::DragIcon::for_drag(drag).set_child(Some(&chip));
            begin_handle.set_visible(false);

            // Every live game, not just this one, shrinks to nothing for the
            // drag's duration — the drop highlight and slot lines are GTK
            // overlay drawing, and a `WebView2` child window would draw over
            // them the same way it would over a grip or a cover (design rule
            // 14, roadmap item 12 task 05). `connect_drag_end` below is what
            // restores them, once the swap or fill this drag might end in has
            // already re-seated the slots.
            #[cfg(windows)]
            if let Some(owner) = begin_owner.upgrade() {
                owner.imp().collapse_live_views();
            }
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
            if end_hover.contains_pointer() && has_somewhere_to_drop(owner.imp().layout.get()) {
                end_handle.set_visible(true);
            }

            // Reads each host's allocation fresh, so a game whose slot moved
            // in a swap or fill restores into its new place, not its old one
            // (`EngineHost::restore`'s own doc comment).
            #[cfg(windows)]
            owner.imp().restore_live_views();
        });

        claim.group_with(&source);
        handle.add_controller(claim);
        handle.add_controller(source);
    }

    /// Shrinks every live account's hosted view to nothing (design rule 14,
    /// roadmap item 12 task 05). Windows only: on Linux the grid's own
    /// overlay already draws above the page, so a drag's drop highlight and
    /// slot lines need no help winning the airspace.
    #[cfg(windows)]
    fn collapse_live_views(&self) {
        use crate::web_engine::EngineHost;
        for entry in self.slots.borrow().iter() {
            if let Some(view) = entry.view.borrow().as_ref()
                && let Ok(host) = view.widget().downcast::<EngineHost>()
            {
                host.collapse();
            }
        }
    }

    /// Reverses [`SessionGrid::collapse_live_views`] once a drag ends,
    /// dropped or cancelled.
    #[cfg(windows)]
    fn restore_live_views(&self) {
        use crate::web_engine::EngineHost;
        for entry in self.slots.borrow().iter() {
            if let Some(view) = entry.view.borrow().as_ref()
                && let Ok(host) = view.widget().downcast::<EngineHost>()
            {
                host.restore();
            }
        }
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
        if let Some(view) = entry.view.borrow().as_ref() {
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
        entry.view.borrow_mut().take();
        entry.overlay.set_child(None::<&gtk::Widget>);
        entry.cover.set_visible(false);
        entry.placeholder.set_state_text("Parked");
        entry.placeholder.set_button_label("Start");
        entry.placeholder.set_button_sensitive(true);
        entry.placeholder.set_spinner_visible(false);
        let is_focused = matches!(entry.placement, Visibility::InSlot(slot) if slot.index() == self.focused.get());
        entry
            .placeholder
            .set_button_tooltip(if is_focused { "Start (Ctrl+S)" } else { "" });
        entry.placeholder.set_visible(true);
        sync_grip_strip(entry, self.layout.get());
        tracing::debug!(session = %id, "parked: showing the slot placeholder");
    }

    /// Puts a freshly started account's view back into the slot it still holds.
    /// The placeholder stays up, now reading `Starting` with its button
    /// disabled, and covers the blank loading view until the page paints. The
    /// `SlotEntry` and its placement are unchanged. A no-op for an account the
    /// grid has no entry for.
    pub(super) fn attach_view(&self, id: &SessionId, view: &EngineView) {
        let slots = self.slots.borrow();
        let Some(entry) = slots.iter().find(|entry| &entry.id == id) else {
            return;
        };
        entry.overlay.set_child(Some(&view.widget()));
        *entry.view.borrow_mut() = Some(view.clone());
        entry.cover.set_visible(false);
        entry.placeholder.set_state_text("Starting…");
        entry.placeholder.set_button_sensitive(false);
        entry.placeholder.set_spinner_visible(true);
        collapse_view_until_painted(view);
        sync_grip_strip(entry, self.layout.get());

        let placeholder = entry.placeholder.clone();
        view.connect_painted(move || {
            placeholder.set_visible(false);
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
        #[cfg(windows)]
        entry.readout.popover.unparent();
        entry.mount.unparent();
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
        show_readout(&readout, &entry.overlay);

        let armed = Rc::downgrade(&readout);
        let timer = glib::timeout_add_local_once(
            Duration::from_millis(ZOOM_READOUT_FADE_MILLIS),
            move || {
                if let Some(readout) = armed.upgrade() {
                    readout.timer.borrow_mut().take();
                    hide_readout(&readout);
                }
            },
        );
        *readout.timer.borrow_mut() = Some(timer);
    }

    pub(super) fn sync(&self, book: &WorkspaceBook) {
        let layout_changed = self.layout.replace(book.active().layout()) != book.active().layout();
        self.focused.set(book.active().focused_slot().index());
        let viewport_changed =
            self.mobile_viewport.replace(book.mobile_viewport()) != book.mobile_viewport();

        let nowhere_to_drop = !has_somewhere_to_drop(self.layout.get());
        let focused_index = self.focused.get();
        for entry in self.slots.borrow_mut().iter_mut() {
            // An account not in the shown workspace is off-grid explicitly —
            // today an entry the book cannot place at all would otherwise keep
            // its stale placement, which with several workspaces would leave a
            // hidden one's view drawn in a place (`FR.18.1`). Computed before
            // the placeholder below, which needs to know whether this place
            // is the focused one (design rule 26).
            entry.placement = book.placement(&entry.id).unwrap_or(Visibility::OffGrid);
            let is_focused = matches!(entry.placement, Visibility::InSlot(slot) if slot.index() == focused_index);

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
                apply_placeholder(
                    &entry.placeholder,
                    placeholder_panel(session.liveness(), is_focused),
                );
            }
            // There is nowhere to drop an account off-grid or in a one-slot
            // layout, so the grip never shows there — a hidden widget is never
            // picked, so a hidden grip cannot take a press either.
            if nowhere_to_drop || entry.placement == Visibility::OffGrid {
                entry.grip.set_visible(false);
            }
            // The row the grip lives in on Windows follows the same rule one
            // step further: no live game in this place, no strip above it.
            sync_grip_strip(entry, self.layout.get());
        }

        let obj = self.obj();
        obj.queue_allocate();
        obj.queue_draw();
        if layout_changed || viewport_changed {
            self.allocate_slots_now();
        }
    }

    /// Allocates every child right away rather than on the frame clock's next
    /// layout phase. A Wayland compositor stops ticking the frame clock of a
    /// window it is not showing — occluded, or minimised — and GTK lays out
    /// only on a tick, so an arrangement a phone asked for would otherwise
    /// wait until the desktop is next looked at (measured 2026-09-20: two
    /// seconds to well over twenty-five). The engine views take their new
    /// size from this allocation alone, and the phone's pictures follow that
    /// size, which is what makes the wait matter (roadmap item 13 task 06,
    /// `FR.3.2`; code standards rule 18). Nothing to do before the grid has a
    /// size of its own; the first layout pass allocates then.
    fn allocate_slots_now(&self) {
        let obj = self.obj();
        let (width, height) = (obj.width(), obj.height());
        if width <= 0 || height <= 0 {
            return;
        }
        self.allocate_slots(width, height);
    }

    /// Allocates every child. Called by [`super::SlotLayout`] on the frame
    /// clock's layout phase, and by [`Self::allocate_slots_now`] when the
    /// arrangement changed.
    pub(super) fn allocate_slots(&self, width: i32, height: i32) {
        let layout = self.layout.get();
        let viewport = self.viewport();
        let cell = slot_rect(layout, viewport, 0, width, height);

        for entry in self.slots.borrow().iter() {
            let rect = match entry.placement {
                Visibility::InSlot(slot) => {
                    slot_rect(layout, viewport, slot.index(), width, height)
                }
                // Outside our own bounds and clipped by
                // `set_overflow(Hidden)`: still realised, mapped and
                // allocated, so WebKit keeps it running. A `GtkStack` would
                // unrealise it, and WebKit throttles a view it believes
                // hidden — for an idle game that is lost progress
                // (code-standards rule 18).
                Visibility::OffGrid => SlotRect {
                    x: width + cell.width.max(1),
                    y: height + cell.height.max(1),
                    width: cell.width.max(1),
                    height: cell.height.max(1),
                },
            };
            entry.mount.size_allocate(
                &gdk::Rectangle::new(rect.x, rect.y, rect.width, rect.height),
                -1,
            );
        }

        self.log_mobile_allocation(layout, cell, width, height);
    }

    /// The viewport the `Mobile` slot is shaped as right now.
    fn viewport(&self) -> Viewport {
        self.mobile_viewport
            .get()
            .unwrap_or(DEFAULT_MOBILE_VIEWPORT)
    }

    /// Writes the `Mobile` slot's rectangle at debug level whenever it
    /// changes — the one line a headless run has to prove the slot took the
    /// phone's shape (roadmap item 13 task 03) — and nothing at all in the
    /// other layouts.
    fn log_mobile_allocation(&self, layout: Layout, cell: SlotRect, width: i32, height: i32) {
        let allocation = (layout == Layout::Mobile).then_some(cell);
        if self.mobile_allocation.replace(allocation) == allocation {
            return;
        }
        if let Some(rect) = allocation {
            tracing::debug!(
                size = %format_args!("{}x{}", rect.width, rect.height),
                x = rect.x,
                y = rect.y,
                grid_width = width,
                grid_height = height,
                "mobile slot allocated"
            );
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

        // The grid reports the intent and lets the window decide: it does
        // not set its own `focused` here, so an empty trailing slot never
        // shows an outline the book itself would refuse (architecture rule
        // 8). `sync`, called by the window only when the book's focus
        // actually moved, is what updates `self.focused` and redraws.
        if let Some(handler) = self.on_slot_focused.borrow().as_ref() {
            handler(slot);
        }
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
        if layout == Layout::Mobile {
            let rect = slot_rect(layout, self.viewport(), 0, width, height);
            return rect.contains(x, y).then_some(SlotId::FIRST);
        }
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
        let layout = self.layout.get();
        if index >= layout.slot_count() {
            return;
        }
        let rect = slot_rect(layout, self.viewport(), index, obj.width(), obj.height());

        let base = obj.color();
        let tint = gdk::RGBA::new(base.red(), base.green(), base.blue(), DROP_HIGHLIGHT_ALPHA);
        snapshot.append_color(&tint, &rect.to_graphene());
    }

    fn draw_slot_lines(&self, snapshot: &gtk::Snapshot) {
        if self.slots.borrow().is_empty() {
            return;
        }

        let obj = self.obj();
        let (width, height) = (obj.width(), obj.height());
        let layout = self.layout.get();
        let viewport = self.viewport();
        let (columns, rows) = grid_dimensions(layout);

        let base = obj.color();
        let hairline = gdk::RGBA::new(base.red(), base.green(), base.blue(), 0.15);
        // The focused slot's outline (design rule 26): the theme's own
        // selection colour, not a tint of the plain foreground — a heavier,
        // more legible claim on the one place the window is currently about.
        let marker = selection_color(&*obj)
            .unwrap_or_else(|| gdk::RGBA::new(base.red(), base.green(), base.blue(), 0.55));

        for column in 1..columns {
            let x = width as f32 * column as f32 / columns as f32;
            snapshot.append_color(
                &hairline,
                &graphene::Rect::new(x - 0.5, 0.0, 1.0, height as f32),
            );
        }
        for row in 1..rows {
            let y = height as f32 * row as f32 / rows as f32;
            snapshot.append_color(
                &hairline,
                &graphene::Rect::new(0.0, y - 0.5, width as f32, 1.0),
            );
        }

        // The `Mobile` slot does not reach the grid's edges, so the hairlines
        // above have nothing to divide; its own outline is what shows the
        // phone shape against the window background (roadmap item 13's new
        // pattern).
        if layout == Layout::Mobile {
            outline(
                snapshot,
                &hairline,
                slot_rect(layout, viewport, 0, width, height),
                1.0,
            );
        }

        let focused = self.focused.get();
        if focused < layout.slot_count() {
            outline(
                snapshot,
                &marker,
                slot_rect(layout, viewport, focused, width, height),
                3.0,
            );
        }
    }
}

/// The theme's own selection colour, `@theme_selected_bg_color` — `None` if
/// the running theme never named it, which [`SessionGrid::draw_slot_lines`]
/// falls back from (constraint 6: unverified on Windows, whether the shipped
/// theme names it at all).
///
/// `StyleContext::lookup_color` is the only avenue gtk4-rs 4.10 offers for a
/// *named* theme colour — the non-deprecated `Widget::color()` reads the
/// plain foreground, not a specific palette entry — so this is the one place
/// the deprecated call is still reached for, kept to this single line.
#[allow(deprecated)]
fn selection_color(widget: &impl IsA<gtk::Widget>) -> Option<gdk::RGBA> {
    widget
        .style_context()
        .lookup_color("theme_selected_bg_color")
}

/// Draws `rect`'s four edges `thickness` pixels wide, inside the rectangle.
fn outline(snapshot: &gtk::Snapshot, color: &gdk::RGBA, rect: SlotRect, thickness: f32) {
    let (x, y) = (rect.x as f32, rect.y as f32);
    let (width, height) = (rect.width as f32, rect.height as f32);
    snapshot.append_color(color, &graphene::Rect::new(x, y, width, thickness));
    snapshot.append_color(
        color,
        &graphene::Rect::new(x, y + height - thickness, width, thickness),
    );
    snapshot.append_color(color, &graphene::Rect::new(x, y, thickness, height));
    snapshot.append_color(
        color,
        &graphene::Rect::new(x + width - thickness, y, thickness, height),
    );
}

/// Columns and rows for each layout: one cell, two side by side, two by two,
/// or the one phone-shaped cell.
fn grid_dimensions(layout: Layout) -> (usize, usize) {
    match layout {
        Layout::Single | Layout::Mobile => (1, 1),
        Layout::SideBySide => (2, 1),
        Layout::Grid => (2, 2),
    }
}

/// Whether `layout` has a second slot an account could be dragged to. The
/// grip, its strip and the drop tint all follow this one answer: a one-slot
/// layout — `Single` or `Mobile` — offers nowhere to drop.
fn has_somewhere_to_drop(layout: Layout) -> bool {
    layout.slot_count() > 1
}

/// One slot's rectangle in the grid's own logical pixels — plain integers
/// rather than a `gdk::Rectangle` so the geometry is unit-tested without a
/// display (code standards rule 25).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SlotRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl SlotRect {
    /// Whether the point `(x, y)` falls inside this rectangle.
    fn contains(self, x: f64, y: f64) -> bool {
        x >= f64::from(self.x)
            && y >= f64::from(self.y)
            && x < f64::from(self.x + self.width)
            && y < f64::from(self.y + self.height)
    }

    fn to_graphene(self) -> graphene::Rect {
        graphene::Rect::new(
            self.x as f32,
            self.y as f32,
            self.width as f32,
            self.height as f32,
        )
    }
}

/// The rectangle slot `index` occupies in a grid `width` by `height` under
/// `layout`. The ordinary layouts divide the grid evenly; `Mobile` gives its
/// one slot exactly `viewport`'s size, centred horizontally and top-aligned,
/// never scaled — a grid shorter than the slot clips its bottom through the
/// grid's `set_overflow(Hidden)`, because the page's size is the whole point
/// (Remote Access `FR.3.2`). `index` is not checked against the layout; a
/// caller passes a slot the layout has.
fn slot_rect(
    layout: Layout,
    viewport: Viewport,
    index: usize,
    width: i32,
    height: i32,
) -> SlotRect {
    if layout == Layout::Mobile {
        let slot_width = i32::try_from(viewport.width).unwrap_or(i32::MAX);
        let slot_height = i32::try_from(viewport.height).unwrap_or(i32::MAX);
        return SlotRect {
            x: width.saturating_sub(slot_width) / 2,
            y: 0,
            width: slot_width,
            height: slot_height,
        };
    }

    let (columns, rows) = grid_dimensions(layout);
    let slot_width = width / i32::try_from(columns).unwrap_or(1);
    let slot_height = height / i32::try_from(rows).unwrap_or(1);
    SlotRect {
        x: i32::try_from(index % columns).unwrap_or(0) * slot_width,
        y: i32::try_from(index / columns).unwrap_or(0) * slot_height,
        width: slot_width,
        height: slot_height,
    }
}

/// What a slot's placeholder panel reads for an account in `liveness` seated
/// at `is_focused`, or `None` when the account is live and its view fills the
/// slot.
///
/// Derived during `sync` — not only from the imperative `release_view` /
/// `attach_view` calls — so a slot drawn straight from a restored book reads
/// correctly before any view exists (design rule 4, architecture rule 8). A
/// queued account gets the line and no button at all: the start queue owns the
/// order and there is nothing useful to press while it drains. `is_focused`
/// only ever changes a parked slot's button tooltip, naming the chord that
/// would press it (design rule 26) — the window's own keyboard shortcut acts
/// on the focused slot alone, so a tooltip on any other slot would name a key
/// that would do nothing there.
fn placeholder_panel(liveness: Liveness, is_focused: bool) -> Option<PlaceholderPanel> {
    match liveness {
        Liveness::Parked => Some(PlaceholderPanel {
            state_text: "Parked",
            button_visible: true,
            button_sensitive: true,
            button_tooltip: if is_focused { "Start (Ctrl+S)" } else { "" },
            spinner_visible: false,
        }),
        Liveness::Queued => Some(PlaceholderPanel {
            state_text: "Queued",
            button_visible: false,
            button_sensitive: false,
            button_tooltip: "",
            spinner_visible: false,
        }),
        Liveness::Starting => Some(PlaceholderPanel {
            state_text: "Starting…",
            button_visible: true,
            button_sensitive: false,
            button_tooltip: "",
            spinner_visible: true,
        }),
        Liveness::Live => None,
    }
}

/// The placeholder facts `placeholder_panel` derives from a liveness and a
/// focus state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlaceholderPanel {
    state_text: &'static str,
    button_visible: bool,
    button_sensitive: bool,
    button_tooltip: &'static str,
    spinner_visible: bool,
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
    placeholder.set_button_tooltip(panel.button_tooltip);
    placeholder.set_spinner_visible(panel.spinner_visible);
    placeholder.set_visible(true);
}

/// Hides `cover` the first time `view` paints — the name shows on the
/// window's own background until then, and the live page covers it after.
/// Also collapses `view` until then on Windows ([`collapse_view_until_painted`]).
fn hide_cover_once_painted(view: &EngineView, cover: &gtk::Box) {
    collapse_view_until_painted(view);

    let cover = cover.clone();
    view.connect_painted(move || {
        cover.set_visible(false);
    });
}

/// Shrinks `view`'s hosted native window to nothing until its page first
/// paints, so whatever GTK draws over that place in the meantime — the name
/// cover ([`hide_cover_once_painted`]), the "Starting" placeholder
/// ([`SessionGrid::attach_view`]) — is not hidden underneath a `WebView2`
/// child window that would otherwise always win the airspace (design rule
/// 14, roadmap item 12 task 05). A no-op on Linux, where the grid's own
/// overlay already draws above the page, and where `EngineView::widget()`
/// is not an [`crate::web_engine::EngineHost`] to downcast to at all.
#[cfg_attr(not(windows), allow(unused_variables))]
fn collapse_view_until_painted(view: &EngineView) {
    #[cfg(windows)]
    {
        use crate::web_engine::EngineHost;
        let Ok(host) = view.widget().downcast::<EngineHost>() else {
            return;
        };
        host.collapse();
        let restore_host = host.clone();
        view.connect_painted(move || {
            restore_host.restore();
        });
    }
}

/// The transient zoom figure for one place: a short label on its own opaque
/// ground, centred horizontally and low in the place so it never covers what
/// the reader is adjusting (item 09 wireframe). Hidden until a gesture.
///
/// On Windows the label is the child of its own [`gtk::Popover`], parented to
/// `overlay`, instead of another overlay layer — a native `WebView2` child
/// window would otherwise always draw over it (design rule 14). The popover
/// takes no focus and ignores clicks (`can_focus(false)`, `can_target(false)`)
/// to stay true to design rule 10's "acknowledgement only, never an action,"
/// and `has_arrow(false)`/`autohide(false)` because it is a transient figure,
/// not a menu. [`show_readout`] and [`hide_readout`] are what actually show
/// and hide it; the label's own visibility only matters on Linux.
#[cfg_attr(not(windows), allow(unused_variables))]
fn build_readout(overlay: &gtk::Overlay) -> Rc<Readout> {
    let label = gtk::Label::new(None);
    label.add_css_class("zoom-readout");
    label.set_halign(gtk::Align::Center);
    label.set_valign(gtk::Align::End);

    #[cfg(windows)]
    let popover = {
        let popover = gtk::Popover::new();
        popover.set_autohide(false);
        popover.set_has_arrow(false);
        popover.set_can_focus(false);
        popover.set_can_target(false);
        popover.set_child(Some(&label));
        popover.set_parent(overlay);
        popover
    };
    #[cfg(not(windows))]
    {
        label.set_margin_bottom(24);
        label.set_visible(false);
    }

    Rc::new(Readout {
        label,
        #[cfg(windows)]
        popover,
        timer: RefCell::new(None),
    })
}

/// Shows `readout`'s current figure over `overlay`'s place: on Windows, a
/// fresh `pointing_to` at the overlay's current bottom centre (`24px` up,
/// matching the Linux label's own `margin_bottom`) and `popup()`, so it
/// tracks a resized place the way the Linux label already does through plain
/// layout; a plain overlay-layer show on Linux, unchanged.
#[cfg_attr(not(windows), allow(unused_variables))]
fn show_readout(readout: &Readout, overlay: &gtk::Overlay) {
    #[cfg(windows)]
    {
        let point = gdk::Rectangle::new(overlay.width() / 2, (overlay.height() - 24).max(0), 1, 1);
        readout.popover.set_pointing_to(Some(&point));
        readout.popover.popup();
    }
    readout.label.set_visible(true);
}

/// Reverses [`show_readout`]: `popdown()` on Windows, hiding the label either
/// way.
fn hide_readout(readout: &Readout) {
    #[cfg(windows)]
    readout.popover.popdown();
    readout.label.set_visible(false);
}

/// The drag grip for a place (item 10 task 04's new pattern, moved to its own
/// strip above the place on Windows by [`mount_grip`] — design rule 14).
/// Hidden until the pointer hovers the place.
fn build_grip() -> gtk::Image {
    let grip = gtk::Image::from_icon_name("list-drag-handle-symbolic");
    grip.add_css_class("slot-grip");
    grip.set_halign(gtk::Align::End);
    grip.set_valign(gtk::Align::Start);
    grip.set_visible(false);
    grip.set_cursor(gdk::Cursor::from_name("grab", None).as_ref());
    #[cfg(windows)]
    grip.set_hexpand(true);
    #[cfg(not(windows))]
    {
        grip.set_margin_top(6);
        grip.set_margin_end(6);
    }
    grip
}

/// Mounts `grip` where it belongs and returns the widget `register_slot`
/// parents to the grid (design rule 14, roadmap item 12 task 05, code
/// standards rule 6): on Linux, unchanged — `grip` becomes another overlay
/// layer over `overlay`, and `overlay` itself is what gets parented, exactly
/// as before this task. On Windows — where a `WebView2` child window would
/// always draw over an overlaid grip — `grip` instead goes at the trailing
/// end of a `.grip-strip` row (its old per-widget margins now the strip's own
/// padding, in CSS), stacked above `overlay` in a new vertical box, and that
/// box is what gets parented instead.
fn mount_grip(overlay: &gtk::Overlay, grip: &gtk::Image) -> SlotMount {
    #[cfg(windows)]
    {
        let strip = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        strip.add_css_class("grip-strip");
        strip.append(grip);
        // Built hidden: a place is off-grid until the first `sync`, and an
        // empty row above a place nothing can be dragged out of is exactly
        // what `sync_grip_strip` exists to keep off the screen.
        strip.set_visible(false);

        let column = gtk::Box::new(gtk::Orientation::Vertical, 0);
        overlay.set_vexpand(true);
        column.append(&strip);
        column.append(overlay);
        SlotMount {
            widget: column.upcast(),
            strip,
        }
    }
    #[cfg(not(windows))]
    {
        overlay.add_overlay(grip);
        SlotMount {
            widget: overlay.clone().upcast(),
        }
    }
}

/// What [`mount_grip`] built for one place: `widget` is what `register_slot`
/// parents to the grid, and `strip` — Windows only — is the `.grip-strip` row
/// whose visibility follows the place's own state ([`sync_grip_strip`]).
struct SlotMount {
    widget: gtk::Widget,
    #[cfg(windows)]
    strip: gtk::Box,
}

/// Shows `entry`'s grip strip only where its grip could ever appear: the
/// place holds a live game, sits on the grid, and `layout` has somewhere else
/// to drop it. A parked, queued or off-grid place shows the plain panel with
/// no empty row above it, exactly as on Linux (design rules 4 and 14) — the
/// grip's own hover rule still decides whether the row has anything in it.
/// A no-op on Linux, where the grip is an overlay layer and there is no row
/// to show or hide ([`mount_grip`]).
#[cfg_attr(not(windows), allow(unused_variables))]
fn sync_grip_strip(entry: &SlotEntry, layout: Layout) {
    #[cfg(windows)]
    entry.strip.set_visible(
        has_somewhere_to_drop(layout)
            && entry.placement != Visibility::OffGrid
            && entry.view.borrow().is_some(),
    );
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
        let panel =
            placeholder_panel(Liveness::Queued, false).expect("a queued account shows a panel");

        assert_eq!((panel.state_text, panel.button_visible), ("Queued", false));
    }

    #[test]
    fn a_parked_accounts_panel_is_unchanged_by_the_queued_state() {
        let panel =
            placeholder_panel(Liveness::Parked, false).expect("a parked account shows a panel");

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
        assert_eq!(placeholder_panel(Liveness::Live, false), None);
        assert_eq!(placeholder_panel(Liveness::Live, true), None);
    }

    #[test]
    fn a_starting_accounts_panel_names_the_ellipsis_and_spins() {
        let panel =
            placeholder_panel(Liveness::Starting, false).expect("a starting account shows a panel");

        assert_eq!(
            (panel.state_text, panel.spinner_visible),
            ("Starting…", true)
        );
    }

    #[test]
    fn only_a_focused_parked_slot_names_its_own_chord_in_the_button_tooltip() {
        let focused =
            placeholder_panel(Liveness::Parked, true).expect("a parked account shows a panel");
        let unfocused =
            placeholder_panel(Liveness::Parked, false).expect("a parked account shows a panel");

        assert_eq!(
            (focused.button_tooltip, unfocused.button_tooltip),
            ("Start (Ctrl+S)", "")
        );
    }

    #[test]
    fn a_queued_or_starting_slots_button_tooltip_is_never_set_even_when_focused() {
        let queued =
            placeholder_panel(Liveness::Queued, true).expect("a queued account shows a panel");
        let starting =
            placeholder_panel(Liveness::Starting, true).expect("a starting account shows a panel");

        assert_eq!((queued.button_tooltip, starting.button_tooltip), ("", ""));
    }

    #[test]
    fn the_mobile_slot_is_the_viewport_centred_horizontally_and_top_aligned() {
        let rect = slot_rect(Layout::Mobile, DEFAULT_MOBILE_VIEWPORT, 0, 1000, 800);

        assert_eq!(
            rect,
            SlotRect {
                x: 294,
                y: 0,
                width: 412,
                height: 915,
            }
        );
    }

    #[test]
    fn the_mobile_slot_keeps_the_viewports_size_in_a_grid_narrower_than_it() {
        let rect = slot_rect(Layout::Mobile, DEFAULT_MOBILE_VIEWPORT, 0, 300, 800);

        assert_eq!((rect.x, rect.width), (-56, 412));
    }

    #[test]
    fn a_grid_slot_is_its_share_of_the_grid() {
        let rect = slot_rect(Layout::Grid, DEFAULT_MOBILE_VIEWPORT, 3, 1000, 800);

        assert_eq!(
            rect,
            SlotRect {
                x: 500,
                y: 400,
                width: 500,
                height: 400,
            }
        );
    }

    #[test]
    fn only_the_one_slot_layouts_offer_nowhere_to_drop() {
        let answers: Vec<bool> = [
            Layout::Single,
            Layout::SideBySide,
            Layout::Grid,
            Layout::Mobile,
        ]
        .into_iter()
        .map(has_somewhere_to_drop)
        .collect();

        assert_eq!(answers, vec![false, true, true, false]);
    }

    #[test]
    fn a_point_inside_the_mobile_slot_is_in_it_and_one_beside_it_is_not() {
        let rect = slot_rect(Layout::Mobile, DEFAULT_MOBILE_VIEWPORT, 0, 1000, 800);

        assert_eq!(
            (rect.contains(500.0, 10.0), rect.contains(100.0, 10.0)),
            (true, false)
        );
    }
}
