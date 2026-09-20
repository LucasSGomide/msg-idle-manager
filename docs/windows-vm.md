# Windows VM

Roadmap item 12 is built on Linux and run in a Windows 11 virtual machine on the
same computer. The VM is [`dockur/windows`](https://github.com/dockur/windows):
a full Windows install under QEMU/KVM, packaged as a Docker container. The
compose file is [`scripts/windows-vm/compose.yml`](../scripts/windows-vm/compose.yml).

## One-time setup

1. **Install Docker Engine.** The image cannot run under Docker Desktop, because
   Docker Desktop on Linux gives containers no `/dev/kvm`. The two coexist, and
   the Engine answers on the `default` context:

   ```
   sudo apt install docker.io                # the compose plugin Docker Desktop installed is reused
   sudo usermod -aG docker "$USER"          # then log out and back in
   docker context use default
   docker run --rm hello-world               # proves the Engine answers
   ```

   `/dev/kvm` must be readable and writable by you. Check with
   `getfacl /dev/kvm`.

2. **Start the VM.** The first start downloads about 6.6 GiB from Microsoft and
   installs Windows unattended, which takes a while.

   ```
   mkdir -p ~/.local/share/idle-manager-windows-vm/storage dist
   docker compose -f scripts/windows-vm/compose.yml up -d
   ```

   Watch the install at <http://127.0.0.1:8006>. The login is `Docker` /
   `admin`.

3. **Connect over RDP** for daily use. It is smoother than the browser view and
   supports display scaling:

   ```
   sudo apt install freerdp3-x11
   xfreerdp3 /v:127.0.0.1 /u:Docker /p:admin /dynamic-resolution /scale:100
   ```

   Use `/scale:140` or `/scale:180`, or Windows' own *Settings → Display →
   Scale*, for the scaling checks.

## Daily loop

1. On Linux, `make windows-package` writes
   `dist/idle-manager-<version>-windows-x64.zip`. Task 02 of item 12 creates
   the build target and task 08 the package target.
2. In the VM, the zip is on drive `Z:`. Unzip it to a local folder such as
   `C:\idle-manager`; don't run it from `Z:`, because a network share is slow
   and locks files. Run `idle-manager.exe`.
3. Logs: run from `cmd` with `set RUST_LOG=idle_manager=debug` for a debug
   build, whose console window stays open.

Stop the VM with `docker compose -f scripts/windows-vm/compose.yml stop`. Its
disk survives. `down` also keeps the disk, because it lives in the bind-mounted
storage folder.

## The phone path (item 13)

The phone reaches a real desktop over the mesh network, and the server binds
the desktop's `100.x` address on its own. The VM has no mesh client and its
network is Docker's, so against the Windows build two things stand in:

1. **The listen override.** In the VM, create
   `%APPDATA%\idle-manager\phone.toml` (the folder is beside `sessions.toml`,
   `FR.1.4`) holding

   ```toml
   [listen]
   address = "0.0.0.0:7466"
   ```

   and start `idle-manager.exe`. With no override the log reads
   `the phone server is not listening kind=NoMeshAddress` and the phone
   dialog's status line says so; with it, `the phone server is listening
   address=0.0.0.0:7466`. `0.0.0.0` rather than the VM's own address because
   Docker's port publishing reaches the VM through its NAT, and the address
   the QR code carries is rewritten by hand in the next step anyway.

2. **The published port.** `compose.yml` publishes `7466/tcp` from the VM on
   every host interface, so a phone on the same Wi-Fi as the host reaches the
   Windows build at `http://<host LAN address>:7466/` — `ip -4 addr show`
   on the host names the address. The QR code the VM's dialog shows carries
   `http://0.0.0.0:7466/enrol/<code>`, which no phone can open: type the
   address by hand with the host's LAN address in place of `0.0.0.0`, or
   select the dialog's text line and edit it in the phone's browser. The
   code itself is what enrols; the host part only has to reach the server.

The override is the only reason a plain machine would set one too: a desktop
with no mesh network but a trusted LAN can listen on its own LAN address and
enrol a phone on the same Wi-Fi. `FR.5.1`'s reach from anywhere still needs
the mesh.

After enrolment the walk is the same as on Linux: turn mobile mode on, watch
the game move, tap, scroll, park and start an account, leave, and
`Un-enrol the phone` from the header menu. The Windows-only code on that path
is `web_engine/webview2.rs`'s `capture_frame`, `run_script` and `set_watched`
(item 13 task 02); a defect found here is fixed there or in `ffi.rs`.

## What the VM can and cannot prove

It proves everything functional:
- pages showing, and accounts staying isolated;
- window resizing and scaling;
- page scripts, popups, zoom, and killing a process;
- the grip strip and zoom popup;
- keep-awake tick rates while minimised;
- deleting an account;
- the memory footer against Task Manager;
- the missing-runtime dialog;
- the zip running on a Windows install that has no GTK;
- the phone path end to end, through the published port above.
- `Shift`+`Tab` and `Ctrl`+`Tab` reaching the shortcut table through
  `WebView2`'s accelerator-key callback while a game page holds the keyboard
  (roadmap item 14 task 05), and the header-bar pager turning and wrapping
  pages, including whether its readout's tabular figures hold their width
  under the VM's default font (item 14 task 08).

It has no GPU, so Windows draws in software. Hardware acceleration and idle
processor figures are therefore not measured here. Memory is compared against
Edge in the same VM, where both draw the same way, and that comparison is
indicative only.

A developer on a real Windows machine may run the same steps, but those runs do
not gate acceptance.

Two cautions:
- **Minimise inside Windows, never the RDP client.** Minimising the client stops
  the whole session from drawing, which skews every keep-awake check.
- **Apps and games need the internet.** The VM reaches it through the host;
  sign-in popups need real game accounts.
