# 08 — Windows verification, the measurements and the docs

**Roadmap:** [13](../../roadmap/13-phone-operation/README.md) · **Scope:** full-stack · **Depends on:** 05, 07

## Context

Everything the phone feature needs is now built. This closing slice proves two
promises the item made and writes down what it found.

The first promise is that the feature works whether the desktop runs Linux or
Windows. The server crate is plain standard-library code and compiles for both
already; the snapshot and script calls have a Windows implementation behind
the engine seam. What has not happened is running the whole path against the
Windows build. The project has no Windows computer, only a Windows 11 virtual
machine inside a container on the Linux development machine, and that machine
has no mesh network client and sits behind Docker's own network. So this slice
teaches the virtual machine's setup to publish the server's port and uses the
listen override in the phone record file to bind to the machine's own address,
then walks the phone through enrolment, watching, tapping and parking against
the Windows build.

The second promise is cost. The project claims that a phone looking at one
game costs only that one game, and that with no phone attached nothing costs
more than before. This slice measures both: how many milliseconds pass
between a tap and its visible result on the home network and on mobile data,
how many frames a second actually arrive, how much processor time the whole
process tree spends while a phone is attached compared with resting, and how
much memory the listening server adds at rest. The figures go into the
roadmap item as a measured section and into the memory budget document,
following what earlier items did.

Finally the docs catch up: the stack document names the new crates and the
mesh network prerequisite, and the Windows virtual machine document explains
the port and the override.

## User experience

- **Flow** — On Windows, the same header toggle, menu and dialog appear, and
  the phone shows the same screens as against Linux.
- **States** — Nothing new is drawn; this slice adds no screen.

## Technical details

- **Architecture** — `scripts/windows-vm/compose.yml` publishes the phone
  port (7466) from the VM to the host; `docs/windows-vm.md` gains a section
  on setting `[listen] address = "<vm address>:7466"` in the VM's
  `%APPDATA%\idle-manager\phone.toml` and reaching it from a phone on the same
  LAN or through the host.
- **Architecture** — the Windows capture and script paths are exercised end
  to end in the VM: enrol, attach, watch, tap, scroll, park, start, leave,
  revoke; any Windows-only defect found is fixed inside
  `web_engine/webview2.rs` or `ffi.rs` (code standards rule 28).
- **Architecture** — `docs/roadmap/13-phone-operation/README.md` gains a
  `## Measured` section (item 04's precedent) with: tap-to-visible latency and
  frames per second on the home network and on mobile data (median of ten
  taps, timed by filming the phone and the desktop side by side or by a
  timestamp overlay the debug switch enables); process-tree processor time
  over five minutes attached versus five minutes resting, sampled with the
  method `make memory-report` uses; memory at rest with the server listening
  versus without.
- **Architecture** — `docs/memory-budget.md` gains one line for the resting
  state with the server listening, which must sit within measurement noise
  of the recorded figures (`FR.4.1`), and one line for the attached state.
- **Architecture** — `docs/stack.md` "Target" names the mesh network
  (Tailscale or any WireGuard mesh) as the phone path's prerequisite and
  "Libraries" carries the remote crate's dependencies with reasons;
  `docs/architecture.md` is checked to already name the sixth crate.
- **Code standards** — no code changes beyond fixes for defects the VM run
  finds; every fix carries the constraint that forced it (rule 18).

## Acceptance criteria

- [ ] `(integration)` `make verify`, `make windows-check` and
      `make windows-package` pass
- [ ] `(manual)` in the Windows VM with the `[listen]` override, the phone
      enrols through the QR code, turns mobile mode on, sees the game move,
      taps a game button with visible effect, scrolls, parks and starts an
      account, leaves, and is cut off by `Un-enrol the phone`
- [ ] `(manual)` on Linux, the median tap-to-visible time over ten taps is
      under one second on the home network and on mobile data, recorded in
      `## Measured`
- [ ] `(manual)` frames arrive at 10 or more per second on the home network,
      recorded in `## Measured`
- [ ] `(manual)` processor time of the process tree while attached is
      recorded beside the resting figure, and the resting figure with the
      server listening is within noise of the one without
- [ ] `(manual)` `docs/memory-budget.md` carries the resting-with-server and
      attached lines, and `docs/stack.md` names the mesh prerequisite and the
      new crates

## References

- [Roadmap item](../../roadmap/13-phone-operation/README.md) — Back-end
  "Measurements"; Technical References: `jpeg-encoder` yields roughly 30 to
  60 KiB per 412 × 915 frame, so twelve frames a second cost under 1 MiB/s on
  the wire — the figure the frame-rate and latency measurements are checked
  against; Blockers 1, 5
- [`openapi.json`](../../roadmap/13-phone-operation/openapi.json) — the
  whole item
- [Sequence diagrams](../../roadmap/13-phone-operation/sequence-diagrams.md)
  — the whole item
- [Wireframes](../../roadmap/13-phone-operation/wireframes/) — the whole item
- [`docs/requirements.md`](../../requirements.md) — Remote Access `FR.1.4`,
  `FR.1.6`, `FR.4.1`, `FR.4.2`
- [`docs/windows-vm.md`](../../windows-vm.md) — the VM loop
- [`docs/memory-budget.md`](../../memory-budget.md) — the figures to sit
  beside
- [`docs/architecture.md`](../../architecture.md) — rule 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 18, 28

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`. The Windows `(manual)` step
runs in the VM ([`docs/windows-vm.md`](../../windows-vm.md)).
