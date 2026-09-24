# 16 — Releasing from GitHub, and updating from inside the app

Sliced along the pipeline first and the app second. The verify workflow and
the version-and-notes tooling come first because every later slice is checked
by them. Packaging follows, because it creates the seventh crate and the
Velopack dependency everything on the app side builds on; signing sits on top
of packaging. The publish workflow and the core's update port can then be
written apart; the notice widget wires the port into the window; one closing
slice runs the whole round trip on both systems and writes the user-facing docs.

Note that the publish workflow (05) starts releasing for real the moment it
lands on `main`: the first release ships without the in-app updater, and the
release that carries 07 is the first one a running copy can install itself.

**Waves.** 01 and 02 run in parallel: 01 touches only `.github/workflows/ci.yml`,
02 the Makefile, `cliff.toml`, the release script and the code-standards doc.
03 runs alone after 02, because both edit the Makefile. 04 runs alone after 03,
because both edit the Makefile and the new crate. 05 and 06 run in parallel
after 04: 05 edits `.github/` only, 06 edits the core and the update crate. 07
runs alone after 06, since it edits the shell and the binary. 08 runs alone
last.

**09** was added on 2026-09-24, once 01–07 were built: it holds, unchanged,
the criteria of 01, 03, 05 and 07 that only a real GitHub run or a release
installed copy can prove, so those slices are accepted on what they built.
09 runs right after the branch lands; 08's round trip follows it, since it
needs two real releases.

| # | Task | Scope | Depends on | Criteria | Status |
|---|---|---|---|---|---|
| [01](01-verifying-every-push-on-github.md) | Verifying every push on GitHub | back-end | — | 1/1 | done |
| [02](02-the-version-and-the-notes-from-the-commits.md) | The version and the notes from the commits | back-end | — | 7/7 | done |
| [03](03-packaging-both-systems-with-velopack.md) | Packaging both systems with Velopack | back-end | 02 | 6/6 | done |
| [04](04-signing-checksums-and-the-verifier.md) | Signing, checksums and the verifier | back-end | 03 | 7/7 | done |
| [05](05-the-release-workflow.md) | The release workflow | back-end | 02, 03, 04 | 1/1 | done |
| [06](06-the-update-port-policy-and-channel.md) | The update port, the policy and the Velopack channel | back-end | 03, 04 | 9/9 | done |
| [07](07-the-update-notice-the-menu-and-the-window.md) | The update notice, the menu and the window wiring | front-end | 06 | 3/3 | done |
| [08](08-the-round-trip-on-both-systems-and-the-docs.md) | The round trip on both systems, and the docs | full-stack | 05, 07, 09 | 0/6 | not-started |
| [09](09-proving-the-pipeline-on-github.md) | Proving the pipeline on GitHub | back-end | 01, 03, 05, 07 | 0/15 | not-started |
