# Sidebar tree

## Purpose

The single index of every account, grouped under its workspace, where the user
switches workspace by clicking an account and reaches each heading's and each
account's menu.

## Where it sits

Renders the sidebar `**Entry**` bullets (the two-level list, the heading ⋯ menu,
`Delete account…` in the row ⋯ menu), the switching and expanding `**Flow**`
bullets, the "named workspace with no accounts" and "account rows" `**States**`,
and the Browsing state of the "Sidebar modes" diagram. The "Switching workspace
from the sidebar" diagram starts here.

## The screen

The sidebar, now 200 px wide. A title row reads "Accounts" with a `Select` toggle
on its trailing edge. Below it, one heading per workspace in list order with
`Ungrouped` last. A heading is an expander arrow, the workspace name in plain
weight and a ⋯ menu button; it has no dot, no bold and no mark of which workspace
is shown. `Ungrouped`'s heading has no ⋯ button at all. Expanded headings show
their accounts indented beneath, each the same row as today: name, status dot,
keep-awake diamond when on, ⋯ menu. A named workspace with no accounts shows one
dim "No accounts" line when expanded. The memory footer stays pinned at the foot
and counts every workspace's running accounts.

Here, `Party A` is the shown workspace: its accounts carry green dots and `Main`
is bold. Accounts in `Ungrouped` are running out of sight, so they carry amber
dots with dimmed names.

```
┌────────────────────────────────┐
│ Accounts              [Select] │
│ ▾ Party A                    ⋯ │
│     Main         ●  ◆        ⋯ │   bold, green glow
│     Alt 1        ●           ⋯ │   green
│ ▾ Farm crew                  ⋯ │
│     No accounts                │   dim
│ ▸ Party B                    ⋯ │   collapsed
│ ▾ Ungrouped                    │   no menu
│     Farm         ●           ⋯ │   dimmed name, amber
├────────────────────────────────┤
│ Shell                  142 MiB │
│ 3 running              910 MiB │
│ Total                1 052 MiB │
└────────────────────────────────┘

Heading ⋯            Account ⋯
┌──────────────────┐ ┌──────────────────────────┐
│ Rename…          │ │ Park                     │
│ Remove workspace │ │ Keep running when hidden │
└──────────────────┘ │ Rename…                  │
                     │ Delete account…          │
                     └──────────────────────────┘
```

A heading click, or its arrow, expands or collapses it and never switches. An
account click in a workspace that is not shown switches to it.

## Design rules

- Rule 1 — an account row's state is one coloured dot, keyed first by liveness
  and then by visibility; an account in a workspace that is not shown keys as
  `background`, never `current` or `visible`
- Rule 3 — the name is dimmed when the account is out of sight or parked, and
  bold only on the current row
- Rule 5 — settings and actions sit behind one ⋯ menu on the trailing edge,
  action first; the heading menu uses the same shape
- Rule 6 — the name gives way to the trailing edge; the width moves 150 → 200 so
  indented short names still render in full, settled by eye
- Rule 11 — the footer's figures stay right-aligned and fixed-width, unchanged
  by the tree
