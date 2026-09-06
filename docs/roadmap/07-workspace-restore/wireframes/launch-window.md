# The window at launch

## Purpose

What the user sees in the seconds after opening the application, and what they
see when the saved workspace could not be read.

## Where it sits

Renders the `**Flow**` and `**States**` bullets in `## User Experience` and the
"Restoring on launch" interaction diagram. The "Saving after a change" diagram
has no screen at all — that is the point of it — and is not drawn here.

## The screen

The main window from item 01 with its arrangement already correct: the right
layout selected, the right accounts in the right slots, the sidebar listing all
of them. Nothing about the shape of the window arrives gradually; only the
contents of the slots do.

Rows waiting their turn read as queued on the trailing edge. The one being
started reads as starting, reusing item 03's wording exactly rather than
inventing a second word for the same thing. Parked accounts read as parked
immediately and never pass through either state. A slot whose account is queued
or starting shows item 03's placeholder panel with the matching line of text and
no button — there is nothing useful for the user to press while a queue is
draining.

On a first run there is no restoration at all: the window is item 01's empty
state, unchanged.

When the saved workspace could not be read, a strip spans the full width of the
window directly under the header bar, above both the sidebar and the grid. It
holds one line — that the workspace could not be read and where the old file was
kept — and a close button on its trailing edge. Below it the window is a first
run. The strip is also where a failed save is reported later in the session.

```
+--------------------------------------------------------+
| [ 1 | 2 | 4 ]                             [ + Add game ]|
+--------------------------------------------------------+
| ! Workspace could not be read. Kept as sessions.bad  [x]|
+------------------+-------------------------------------+
| Main account   1 |                                     |
| Alt      2·start |            (starting…)              |
| Farm 1    queued |                                     |
| Farm 2    parked |                                     |
+------------------+-------------------------------------+
```

## Design rules

- `docs/design.md` has no numbered rules yet. Two debts: the row's state
  vocabulary, which now has five values and needs an ordering rather than an
  accumulation, and where a window-level message goes, how it is dismissed and
  whether it ever goes away on its own.
- `docs/architecture.md` rule 10 — the workspace is written off the main context
  and only its result touches this window.
- `docs/architecture.md` rules 12 and 13, `docs/naming.md` rules 1 and 4 —
  `message-strip.ui` beside `message_strip.rs` and its `imp` module.
- `docs/code-standards.md` rule 14 — a failed save reaches this strip; it is
  never dropped silently, which is what makes the strip part of the design
  rather than an extra.
