# Missing WebView2 dialog

## Purpose

What a Windows user sees when the web engine the application needs is not
installed or will not start.

## Where it sits

Renders the "Engine missing" `**States**` bullet, the fatal start-up dialog
`**New pattern**`, and the "Starting without WebView2" interaction diagram in
`## User Experience`.

## The screen

A lone system-styled alert dialog with no main window behind it. The heading
names the missing component. One detail line gives the reason the engine
reported and the download address as plain, selectable text. There is a single
`Quit` button. Nothing about accounts or workspaces is shown, because nothing
has been read.

```
┌─────────────────────────────────────────────┐
│ Microsoft Edge WebView2 Runtime is required │
│                                             │
│ The runtime is not installed.               │
│ Get it from                                 │
│ https://developer.microsoft.com/            │
│   microsoft-edge/webview2/                  │
│                                   [ Quit ]  │
└─────────────────────────────────────────────┘
```

## Design rules

- Rule 9 — a failure found before the user acted is named plainly with what
  went wrong. Its strip form needs a window, and this failure comes before one
  exists, so the same wording goes into a dialog and the design doc owes a rule
  for it
- Rule 8 (by contrast) — a failure the user can work around never takes a
  modal. This one leaves no working path, which is why it is the only modal
  failure in the application
