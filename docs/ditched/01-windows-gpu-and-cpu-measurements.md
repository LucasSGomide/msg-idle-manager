# 01 — Measuring Windows GPU use, idle CPU and renderer choice on real hardware

**Ditched:** 2026-09-17 · **Estimate:** 3

## Idea

Roadmap item 12 would have gated shipping on a real Windows PC. It would have
checked three things:
- that game pages draw with hardware acceleration;
- that four idle accounts use no more processor time than the same four tabs in
  Edge;
- whether to force a GSK renderer on Windows.

## Why not

- Nobody on the project has a Windows machine. Windows work is verified in the
  `dockur/windows` VM (`scripts/windows-vm/compose.yml`, `docs/windows-vm.md`),
  and the owner chose to gate acceptance only on what that VM can show
  (requirements `FR.3.5`).
- The VM has no GPU. The host's Intel Arc 130V/140V (Lunar Lake) is not passed
  through, so Windows draws in software. In the VM, hardware acceleration always
  fails and idle processor figures overstate the cost. Neither says anything
  about real hardware.
- The memory comparison against Edge still runs in the VM, where both draw the
  same way (`FR.2.5`, superseding `FR.2.3`;
  `docs/tasks/12-windows-support/08-release-zip-and-measurements.md`).
- GTK's default renderer is kept on Windows, and
  `crates/idle-manager/src/main.rs`'s `GSK_RENDERER` re-exec stays Linux-only.
  A developer with a real Windows machine may check these figures informally;
  a finding there would come back as a new item.
