# GTK 4 research — moving accounts, and the accordion sidebar

Findings from the research pass run during `/msg-pre-roadmap` on 2026-09-08, for
the requirements recorded under **Rearranging Accounts** (`UN.13`, `UN.14`) and
**Account Workspaces** (`UN.15`–`UN.18`).

Kept for `msg-roadmap-plan-item`, which turns these into an item's
`### Technical References` section. Each section below belongs to one planned
roadmap item; nothing here applies to the whole feature set at once.

Nothing here applies to **renaming an account** (`UN.13`). That item is a menu
entry, a dialog and one string field — no GTK behaviour was in doubt.

## For the item that adds the drag grip (`UN.14`)

**A drag source on a widget overlaid above a `WebKitWebView` gets the press.**
`WebKitWebViewBase` registers its scroll, motion, focus, key, click, zoom,
long-press, drag and swipe controllers in the bubble phase, and sets only its
legacy controller to target phase. Nothing on the view runs before the picked
widget, so the grip is the pick target. No workaround is needed.

**The hazard is the reverse: the grip's container blocking the page.**
GTK 3's overlay `pass-through` child property is gone. Its GTK 4 replacement is
`gtk_widget_set_can_target(child, false)`. Any container used to position the
grip in a slot's corner must carry it, or it swallows every click meant for the
game. Only the grip itself keeps `can-target = true`.

**`GtkDragSource` never claims the event sequence, so it cannot suppress the
grid's click-to-focus by itself.** `gtk_drag_source_begin`, `_update` and
`_drag_begin` make no `gtk_gesture_set_state` call; `drag_begin` only calls
`gtk_widget_reset_controllers` on the grip. A drag also only starts once the
pointer crosses `gtk_settings_get_dnd_drag_threshold`, so a press-and-release on
the grip is never a drag at all.

**The recipe that makes the grip win:** a `GtkGestureClick` on the grip calling
`set_state(EventSequenceState::Claimed)` in `pressed`, `gtk_gesture_group`'d
with the grip's `GtkDragSource`. Claiming denies the sequence to every gesture
on ancestors in the propagation chain, which is what stops the grid focusing the
slot. The grouping is required: without it the click's claim would deny the
sequence to the drag source on the same widget and kill the drag. The grid's own
`GestureClick` must stay in the bubble phase — moved to capture, it runs first
and the grip can never win.

**The custom `LayoutManager` costs nothing in hit-testing.** Picking runs over
the allocations the layout manager sets through `gtk_widget_allocate`. Two
things do matter: `can-target`, above, and `overflow` — a slot allocated partly
outside the grid's bounds is clipped out of picking when overflow is `HIDDEN`.

**Slot drop targets belong in the capture phase, and the payload must be a
private type.** `WebKitWebViewBase` constructs a `DropTarget` of its own
(`priv->dropTarget = makeUnique<DropTarget>(viewWidget)`). Capture phase
guarantees an internal drag never reaches the page. Independently, the payload
must be a registered application type via `gdk_content_provider_new_typed`,
never `G_TYPE_STRING`: a text-typed drag is exactly what a web page accepts and
pastes into itself. Which mime types WebKit's drop target accepts could not be
confirmed — its source 404s — so treat the capture phase as a precaution rather
than a measured fact.

**Highlight with `GtkDropControllerMotion`, not `DropTarget`'s own signals.**
Its `contains-pointer` is true while the pointer is in the widget *or a
descendant*, which is the case throughout a drag, since the view fills the slot.
Toggle a CSS class from `notify::contains-pointer`. It accepts no drops, so it
composes with the drop target rather than competing with it.

**Drag icon:** `gtk_drag_source_set_icon` from `::drag-begin`, with
`gtk_widget_paintable_new(widget)` as the paintable — a live snapshot of an
existing widget, and the cheapest way to show a thumbnail of what is being
dragged. Drag icons take no input.

**Measured 2026-09-13, while planning roadmap item 10.** A throwaway GTK 4.22 /
WebKitGTK program ran twice, with identical results both times. The first run was
under Xvfb (X11, no window manager) with XTest input. The second was fullscreen
on Wayland, under a headless GNOME Shell 50.1 started with
`gnome-shell --headless --no-x11 --virtual-monitor 1000x500 --wayland-display <name>`
inside a private `dbus-run-session`, and driven through
`org.gnome.Mutter.RemoteDesktop`. It settled two points above that were left
open:

- *The drop target's phase is required, not a precaution.* A private-type drop
  target on the grid in the **capture** phase received the drop over a page, and
  the page saw no drag events at all. In the **bubble** phase the grid never
  received it: WebKit's drop target consumed it and the drag ended as accepted.
  A **string** payload with no grid target was pasted into the page's text box.
- *The grid's click gesture can stay in the capture phase.* Its handler
  `pick`s under the press and skips focusing when the picked widget is the grip.
  The grip's claiming click, grouped with its `DragSource`, still starts the
  drag. The bubble-phase requirement above only matters if the grip has to beat
  the ancestor's gesture outright.

## For the item that adds workspaces (`UN.15`–`UN.18`)

**The accordion's model:** a root `gio::ListStore` of workspaces wrapped in
`TreeListModel::new(root, passthrough = false, autoexpand = false, create_model_func)`,
where the function returns a workspace's account store and `None` for an
account. `passthrough = false` is mandatory — it is what makes the model hand
out the `GtkTreeListRow` that `gtk_tree_expander_set_list_row` requires.

**One factory, building a superset — it cannot branch by row type.** `::setup`
runs before the item is bound, so the row type is unknown when widgets are
created. Build the union once — a `GtkTreeExpander` whose child holds both the
heading layout and the account layout — and switch visibility in `::bind` off
the row item's type, undoing it in `::unbind`. Two factories for one
`GtkListView` is not an option.

**Set `GtkListItem:focusable = false` on every row**, so keyboard focus lands in
the expander rather than the list item. GTK 4.12+; on older GTK the double focus
stop stands. `indent-for-depth` and `hide-expander` — the latter useful to keep
account rows aligned without a stray expander — are GTK 4.10+.

**Expansion state is destroyed on collapse and is not persisted for you.**
`gtk_tree_list_row_get_children` returns `NULL` unless the row is expanded, and
collapsing drops the child model the create-model-func produced. So hold
expanded-ness keyed by workspace and re-apply it after a rebuild by walking the
flat model calling `set_expanded(true)` — which is what `FR.16.2` requires.

**Selection over a tree model is flat, with no parent-to-child propagation.**
`GtkMultiSelection` selects `GtkTreeListRow`s independently; ticking a workspace
heading selects none of its accounts. Selecting a whole workspace means walking
`row.children()` by hand.

**Collapsed rows leave the model, taking their selection with them.** This is
the argument behind `FR.17.4`: a `Move to workspace…` driven by the view's
selection would silently ignore every collapsed workspace. A `HashSet` of
account identifiers held by the sidebar survives collapse, reordering, and the
rebuild-on-sync the sidebar already does; positional selection survives none of
them.

**A `GtkCheckButton` in a row does not interfere with row activation — it
pre-empts it.** The check button is the pick target and consumes the click, so
the row is neither selected nor activated, and the row highlight and the tick
disagree unless both are driven from one source. `GtkListItem:selectable` and
`activatable` are the switches for turning the view's own click-to-select and
double-click-to-activate off on heading rows, so a heading click only expands.
