# 07 — The update notice, the menu and the window wiring

**Roadmap:** [16](../../roadmap/16-release-pipeline-and-in-app-update/README.md) · **Scope:** front-end · **Depends on:** 06

## Context

This is the slice the user sees. The previous one decided what an update's
states are and how the app talks to GitHub; this one puts those states on
screen and lets the user act on them.

A new bar appears directly under the header bar, spanning the whole window,
in the theme's warning tint. It shows one line of text, a "What's new" link,
one button and a dismiss cross. Only the text, the link's visibility and the
button's label change between states; nothing moves. When a check finds a
newer version the line names it and the button reads "Update". Pressing it
downloads in the background, with the percentage in the line and every game
still running. When the download has passed its checks the line says the
version is ready and installs when the app quits, and the button becomes
"Restart now". A failed download or a failed check by hand says so in the
same line with a "Try again" button. The bar leaves only when its cross is
pressed, and a ready update still installs after the bar is dismissed.

The app checks once at launch and once a day, and the header bar's main menu
gains two items: the running version, drawn insensitive, and "Check for
updates", which runs the check now and always answers on screen, even when
the answer is "you have the latest version". The app never restarts on its
own. "Restart now" and an ordinary quit both go through the window's normal
close, so the arrangement is saved first; the swap happens after the process
has exited, and only "Restart now" brings the app back. If a swap did not
apply, the next launch says so in the existing message strip.

Every network call and file check runs on a worker thread and comes back to
the GTK thread with a result. The window forwards button presses as events to
the pure policy and redraws the bar from whatever state the policy answers, so
the shell decides nothing about updates. This slice also writes the design rule
the new bar owes, since the existing rule for the message strip forbids an
action button on purpose.

## User experience

- **Entry** — The notice appears on its own under the header bar after a
  launch or daily check finds a newer version. `Check for updates` in the `☰`
  menu runs the check now, with `Idle Manager <version>` shown insensitive
  above it.
- **Flow** — `Update` downloads in the background (`Downloading version
  0.3.0… 42%`, button insensitive, games running), then `Checking the
  download…`, then `Version 0.3.0 is ready. It installs when you quit Idle
  Manager.` with `Restart now`.
- **Flow** — `Restart now` closes the window through the normal close, the
  swap applies, the app relaunches at the new version and the workspace
  restores. Quitting any other way while ready applies without relaunching.
- **Flow** — `Check for updates` shows `Checking for updates…`, then `You have
  the latest version, 0.2.0.` with only `×`, or the available state.
- **States** — Automatic check finds nothing or fails: nothing on screen, one
  warning line in the log for a failure. Manual check fails: `Could not check
  for updates: <reason>.` with `×`. Download or verification fails: `The update
  could not be verified and was discarded.` / `The download failed: <reason>.`
  with `Try again`. Dismissed: hidden for the run, `Ready` still installs.
  Strip and notice both showing: strip first, notice beneath. Swap did not
  apply: the message strip reads `The update to 0.3.0 did not apply; the
  previous version is running.` at the next launch.
- **Pattern** — Rule 9's dismiss-only-by-hand and warning tint; rule 8's
  one-line failure inside the thing the user asked for; rule 23's insensitive
  no-action menu item; rule 12's nothing-moves. **New pattern** — a bar under
  the header bar carrying one action; the design doc gains the rule in this
  slice.

## Technical details

- **Design** — `crates/idle-manager-shell/src/update_notice.rs` +
  `update_notice/imp.rs` + `resources/ui/update-notice.ui`: a `gtk::Box`
  subclass with `message: gtk::Label` (`ellipsize = end`, `hexpand`),
  `whats_new: gtk::LinkButton`, `action: gtk::Button`, `dismiss: gtk::Button`
  (`window-close-symbolic`, `.flat`), styled `.update-notice` in `window.css`
  with `@warning_bg_color`; `pub(crate) fn render(&self, state:
  &UpdateState)` maps every variant to visibility, text, URI, label and
  sensitivity exactly as the wireframe's table; signals `fetch-requested`,
  `restart-requested`, `dismissed` (architecture rules 12, 13; naming rules
  2, 4, 7); registered in `idle-manager.gresource.xml` with a
  "template is readable" test beside the strip's in `lib.rs`.
- **Design** — `window/imp.rs`: `root_box` appends `update_notice` directly
  after `message_strip`; `WindowPorts` gains `update: Arc<dyn UpdateChannel>`;
  fields `update_state: RefCell<UpdateState>`, `update_policy:
  RefCell<UpdatePolicy>`, `last_update_check_millis: Cell<Option<u64>>`,
  `verified_package: RefCell<Option<VerifiedPackage>>`,
  `relaunch_after_update: Cell<bool>`; one method `drive_update(&self, event:
  UpdateEvent)` applying the policy, storing the state, calling `render`, and
  running the effect: `RunCheck` → `gio::spawn_blocking(move || channel.check())`
  then `Found` / `NothingNewer` / `CheckFailed` on the main context;
  `Download` → `spawn_blocking` with a `glib::MainContext::channel` feeding
  `Progress`, ending in `Verified` (storing the package) or `Rejected`
  (architecture rules 8, 10).
- **Design** — schedule: `constructed` calls `drive_update(CheckRequested {
  manual: false })` once after `redraw`; `glib::timeout_add_local(60 s)` calls
  it again whenever `UpdateSchedule::next_check_due(last, now)` is true, with
  `now` from `glib::real_time()`; `last_update_check_millis` is set when a
  check completes.
- **Design** — `restart-requested` sets `relaunch_after_update` and calls
  `self.obj().close()`; the existing `connect_close_request` handler, after
  `saver.flush()`, calls `channel.apply_on_exit(&package,
  relaunch_after_update)` when `verified_package` is `Some`, logging an error
  with `tracing::error!` and proceeding (code standards rule 14); at launch,
  `channel.last_apply_failure()` being `Some(reason)` shows `The update to
  <version> did not apply; the previous version is running.` on
  `message_strip` (design rule 9).
- **Design** — `build_main_menu` gains a section before `Keyboard Shortcuts`
  with `gio::MenuItem::new(Some(&format!("Idle Manager {version}")), None)`
  and `Check for updates` bound to `win.check-for-updates`, a new
  `gio::SimpleAction` calling `drive_update(CheckRequested { manual: true })`;
  no accelerator, not in `help-overlay.ui` (design rules 19 by contrast, 23).
- **Design** — `docs/design.md` gains rule 29: a bar under the header bar may
  carry one action only when the action is the user's decision about their
  own program rather than a repair of the machine's problem, keeps rule 9's
  dismiss-only-by-hand and warning tint, and changes only text and sensitivity
  between states; rule 9 gains one sentence pointing at it.
- **Architecture** — `crates/idle-manager/src/main.rs` builds
  `VelopackChannel::new(RELEASE_REPOSITORY)` with `const RELEASE_REPOSITORY:
  &str = "https://github.com/LucasSGomide/msg-idle-manager"` and passes it in
  `WindowPorts`; a setup failure logs a warning and passes a
  `NoUpdateChannel` stub (in the update crate) whose `check` returns
  `UpdateError::Offline`, so a broken channel never stops the app (rule 3).
- **Code standards** — every string the notice shows is a `const` beside
  `render` (rule 5); `tracing::warn!` in fields for the silent automatic
  failure (rule 15); no `unwrap` (rule 13).

## Acceptance criteria

- [x] `(unit)` `update-notice.ui` is readable from the registered resource
      bundle, like every other template
- [x] `(unit)` the notice's text for each `UpdateState` variant is produced by
      a pure `notice_text(&UpdateState) -> NoticeText { line, link_shown,
      button: Option<(&str, bool)> }` helper whose eight arms match the
      wireframe table, one test per arm
- [ ] `(manual)` with a test release newer than the running version, the notice
      appears under the header bar after launch, `Update` downloads with the
      percentage rising, and the line ends at `Version X is ready. It installs
      when you quit Idle Manager.` with `Restart now`, while a game keeps
      ticking in its slot
- [ ] `(manual)` `Restart now` closes the window, `sessions.toml`'s mtime
      updates before the process exits, the app relaunches at the new version,
      and every running account comes back through the start queue
- [ ] `(manual)` `☰` shows `Idle Manager <version>` insensitive and `Check for
      updates`; on the latest version the notice reads `You have the latest
      version, <version>.` with only `×`; offline it reads `Could not check for
      updates: …` with only `×`
- [ ] `(manual)` a test release whose `.minisig` was made with another key
      ends in `The update could not be verified and was discarded.` with
      `Try again`, and Velopack's packages folder holds no `.nupkg`
- [ ] `(manual)` with both a failed save and an available update, the strip
      sits first and the notice directly beneath it; dismissing the notice while
      `Ready` hides it and quitting still applies the update
- [x] `(integration)` `make verify` passes, including `windows-check`

## References

- [Roadmap item](../../roadmap/16-release-pipeline-and-in-app-update/README.md)
  — User Experience, all three interaction diagrams; Front-end, whole section
- [Wireframes](../../roadmap/16-release-pipeline-and-in-app-update/wireframes/)
  — `update-notice.md`, `main-menu-updates.md`
- [`docs/requirements.md`](../../requirements.md) — Distribution `FR.4.1`,
  `FR.4.2`, `FR.4.3`, `FR.4.4`, `FR.4.5`, `FR.4.6`, `FR.4.7`
- [`docs/design.md`](../../design.md) — rules 8, 9, 12, 19, 23, 28 and the new
  rule 29
- [`docs/architecture.md`](../../architecture.md) — rules 3, 8, 10, 12, 13, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 5, 13, 14, 15
- [`docs/naming.md`](../../naming.md) — rules 2, 4, 7

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
