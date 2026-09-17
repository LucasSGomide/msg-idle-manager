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
- the zip running on a Windows install that has no GTK.

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
