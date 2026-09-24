# Idle Manager

Idle Manager keeps several browser idle games running at once in one window,
on Linux and on Windows: isolated accounts, a session sidebar, keep-awake,
workspaces and interactive zoom, with no browser tabs to lose track of.

## Download

Every release publishes one file per system, both built from the same source
and the same version number:

| System | File | Needs |
| --- | --- | --- |
| Windows 10 22H2 / 11, 64-bit | `IdleManager-win-Portable.zip` | nothing extra — the WebView2 runtime Windows already ships |
| Linux, Ubuntu 24.04+ or Debian 13+, 64-bit | `IdleManager.AppImage` | the system's own GTK 4 (4.10+) and WebKitGTK 6.0 (2.42+), already met by those distributions |

Get the latest release from the
[Releases page](https://github.com/LucasSGomide/msg-idle-manager/releases/latest).
`SHA256SUMS` and a `.minisig` beside every file let you check by hand what you
downloaded; `release/minisign.pub` is the public key ([`release/README.md`](release/README.md)
has the detail on how that pair works).

**Windows.** Unzip `IdleManager-win-Portable.zip` anywhere — no installer, no
administrator rights — and run `IdleManager.exe` inside the unzipped
`current\` folder. Because the program is not signed with a code-signing
certificate, the first run shows Windows' own **"Windows protected your PC"**
warning (SmartScreen). This is expected: click **More info**, then
**Run anyway**. A Windows 11 machine with Smart App Control turned on may
refuse the program outright instead of showing that prompt; turning Smart App
Control off is the only way past it today.

**Linux.** Download `IdleManager.AppImage`, mark it executable
(`chmod +x IdleManager.AppImage`), and run it — no installation, no root, and
nothing but GTK 4 and WebKitGTK need to already be on the system.

**Updating.** The app checks GitHub for a newer release once at launch and
once every 24 hours, and `Check for updates` in the header bar's `☰` menu
runs the same check by hand at any time. When a newer version exists, a
notice offers **Update**: it downloads in the background while every game
keeps running, verifies the download's signature, and then offers
**Restart now**. The update installs the moment you quit either way — by
`Restart now` or by closing the app any other way — and every account, login,
workspace, preset and zoom level is exactly where you left it afterwards.
