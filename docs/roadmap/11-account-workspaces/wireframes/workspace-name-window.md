# Workspace name window

## Purpose

The small modal where a workspace gets its name, either when it is created from
ticked accounts or when an existing one is renamed.

## Where it sits

Renders the `New workspace…` and "renaming a workspace" `**Flow**` bullets, the
"name taken or empty" `**States**` bullet, and the Naming state of the "Sidebar
modes" diagram. The `**Pattern**` bullet names it as `RenameDialog` with a title
and a name check passed in.

## The screen

A modal, non-resizable window over the main window. It is titled "New workspace"
with a `Create` button, or "Rename workspace" with a `Rename` button. It holds
one text field: empty for a new workspace, or the current name fully selected
for a rename. Beneath the field, a dim line reading "Another workspace has this
name." shows only while the trimmed name matches another workspace's, ignoring
case, `Ungrouped` included. The confirm button is the default and is insensitive
while the trimmed name is empty or taken. An empty field shows no line, because
the empty field says it. Enter confirms and Escape cancels.

```
┌ New workspace ───────────────────────┐
│                                      │
│  [ party a                        ]  │
│  Another workspace has this name.    │   dim, only while taken
│                                      │
│                  [Cancel] [ Create ] │   Create insensitive
└──────────────────────────────────────┘
```

## Design rules

- Rule 8 — a problem inside a form is one dim line beneath the field it belongs
  to, and the form keeps working; never an error dialog of its own
- Rule 5 — reached from a heading's ⋯ menu (rename) or the selection bar's menu
  (create), never from a control standing on the row
