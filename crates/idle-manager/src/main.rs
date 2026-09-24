//! Composition root: registers the UI bundle, builds one adapter per port,
//! hands them to the shell, and runs the GTK application.

// A release build on Windows opens no console window behind the application
// (roadmap item 12 task 08, `FR.3.4`): a program someone unzips and
// double-clicks is a desktop application, and a black console beside its
// window reads as something having gone wrong. Debug builds keep the console,
// because that is where `RUST_LOG` output goes while a change is being tried
// in the VM (`docs/windows-vm.md`). Linux is unaffected either way — the
// attribute exists only for the Windows subsystem field in a PE header.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::cell::RefCell;
#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;
#[cfg(target_os = "linux")]
use std::process::Command;
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Context;
use gtk::glib;
use gtk::prelude::*;
use gtk4 as gtk;
use idle_manager_core::{
    MemoryProbe, PhoneRecordStore, PresetCatalogue, ProfileLocator, ProfileRemoval, RemoteIntent,
    WorkspaceStore, ZoomMemory,
};
#[cfg(target_os = "linux")]
use idle_manager_metrics::ProcPssProbe;
#[cfg(windows)]
use idle_manager_metrics::ProcessTreeProbe;
use idle_manager_remote::{NotListeningLink, RemoteConfig, RemoteServer, StartError, bind_address};
use idle_manager_shell::{PhonePorts, Window, WindowPorts};
use idle_manager_store::{
    TomlPhoneRecord, TomlPresetCatalogue, TomlWorkspaceStore, TomlZoomMemory, XdgProfileLocator,
    XdgProfileRemoval,
};

/// The application's D-Bus and settings identifier.
const APP_ID: &str = "org.idlemanager.IdleManager";

/// The variable GTK reads, once at start-up, to choose its renderer.
#[cfg(target_os = "linux")]
const RENDERER_ENV: &str = "GSK_RENDERER";

/// The renderer the window draws with unless the environment already names
/// one.
///
/// GTK 4.22 defaults to Vulkan, and on the Intel Arc (Lunar Lake) this was
/// measured on it cannot import the buffers `WebKitGTK` renders into (`XR24`
/// with the tiled modifier `0x100000000000010`; `GDK_DEBUG=dmabuf` logs "Vulkan
/// driver does not support format"). GTK then imports each frame through GL,
/// downloads it to memory and uploads it again — for every account, every
/// frame — which with four games in play held seven GTK worker threads and the
/// main thread at about 1.6 cores (2026-09-12). The GL renderer imports the
/// same buffers directly. A user-set `GSK_RENDERER` still wins (code standards
/// rule 18). Not forced on Windows: the development VM has no GPU, so there is
/// nothing to measure one renderer against another there (roadmap item 12).
#[cfg(target_os = "linux")]
const RENDERER: &str = "opengl";

fn main() -> ExitCode {
    // Velopack's helper restarts this same binary with hook arguments during
    // an install, an update or an uninstall and expects it to exit at once,
    // before the window, the tracing subscriber or any store exists (code
    // standards rule 18; roadmap item 16 task 03). This must be the first
    // statement in `main` so an ordinary launch is the only path that reaches
    // anything below it.
    idle_manager_update::run_hooks();

    #[cfg(target_os = "linux")]
    let renderer_error = select_renderer();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    #[cfg(target_os = "linux")]
    if let Some(error) = renderer_error {
        tracing::warn!(%error, renderer = RENDERER, "could not select the renderer; GTK picks its default");
    }

    match run() {
        Ok(code) => code,
        Err(error) => {
            tracing::error!(error = %error, "idle-manager exited with an error");
            ExitCode::FAILURE
        }
    }
}

fn run() -> anyhow::Result<ExitCode> {
    tracing::info!(app_id = APP_ID, "starting idle-manager");

    idle_manager_shell::register_resources().context("register the UI resource bundle")?;

    let locator = XdgProfileLocator::new().context("resolve the XDG data directory")?;
    let locator: Rc<dyn ProfileLocator> = Rc::new(locator);

    let catalogue = TomlPresetCatalogue::new().context("resolve the XDG config directory")?;
    let catalogue: Rc<dyn PresetCatalogue> = Rc::new(catalogue);

    let store = TomlWorkspaceStore::new().context("resolve the XDG config directory")?;
    let store: Arc<dyn WorkspaceStore> = Arc::new(store);

    let zoom_memory = TomlZoomMemory::new().context("resolve the XDG data directory")?;
    let zoom_memory: Rc<dyn ZoomMemory> = Rc::new(zoom_memory);

    let removal = XdgProfileRemoval::new().context("resolve the XDG data directory")?;
    let removal: Arc<dyn ProfileRemoval> = Arc::new(removal);

    #[cfg(target_os = "linux")]
    let probe: Arc<dyn MemoryProbe> = Arc::new(ProcPssProbe::new());
    #[cfg(windows)]
    let probe: Arc<dyn MemoryProbe> = Arc::new(ProcessTreeProbe::new());

    // One link, one intent channel: the first activation takes them, since
    // two windows looping over one channel would each see half the phone's
    // messages.
    let phone = RefCell::new(Some(start_phone_server()?));

    let app = gtk::Application::builder().application_id(APP_ID).build();

    // A second launch re-activates this window rather than starting a second
    // process (the D-Bus application id), so only one process ever writes the
    // workspace or opens an account's storage.
    app.connect_activate(move |app| {
        tracing::info!("activated; presenting the main window");

        // Before any store is opened (`FR.1.10`): a missing engine means
        // there is nothing safe to read or write yet, and the window itself
        // never gets built.
        #[cfg(windows)]
        if let Err(reason) = idle_manager_shell::runtime_version() {
            tracing::error!(reason, "no WebView2 runtime available");
            idle_manager_shell::show_missing_engine(app, &reason);
            return;
        }

        #[cfg(target_os = "linux")]
        idle_manager_shell::configure_web_engine();
        #[cfg(windows)]
        idle_manager_shell::configure_web_engine(idle_manager_store::engine_data_root().ok());

        // Read the workspace before the window is built, so the window can draw
        // the whole restored arrangement at once (architecture rule 3).
        let read_outcome = store.read();
        let window = Window::new(
            app,
            WindowPorts {
                locator: Rc::clone(&locator),
                catalogue: Rc::clone(&catalogue),
                store: Arc::clone(&store),
                zoom_memory: Rc::clone(&zoom_memory),
                probe: Arc::clone(&probe),
                removal: Arc::clone(&removal),
                phone: phone.borrow_mut().take(),
            },
            read_outcome,
        );
        window.present();
    });

    Ok(exit_code(app.run()))
}

/// Makes GTK start with [`RENDERER`] unless the environment already names one.
///
/// GTK reads the variable only from the environment it starts in, and changing
/// this process's own environment is `unsafe` on edition 2024 (code standards
/// rule 28). So the process replaces itself with a copy of itself that has the
/// variable set — same binary, same arguments, same process id — before GTK or
/// any thread exists. The copy finds the variable set and carries on. Returns
/// the error only when that replacement failed, in which case start-up
/// continues on GTK's default renderer. `cfg(target_os = "linux")`: it needs
/// `CommandExt::exec`, Unix-only, and Windows never forces a renderer at all
/// (see [`RENDERER`]'s doc comment).
#[cfg(target_os = "linux")]
fn select_renderer() -> Option<std::io::Error> {
    if std::env::var_os(RENDERER_ENV).is_some() {
        return None;
    }
    let executable = match std::env::current_exe() {
        Ok(executable) => executable,
        Err(error) => return Some(error),
    };

    let mut arguments = std::env::args_os();
    let mut command = Command::new(executable);
    if let Some(program_name) = arguments.next() {
        command.arg0(program_name);
    }
    Some(command.args(arguments).env(RENDERER_ENV, RENDERER).exec())
}

/// Builds the phone's way in (roadmap item 13): the record on disk, the
/// address it names or the mesh address discovered, and the server on it. A
/// server that cannot start — no mesh address, a taken port — is a status the
/// phone dialog shows, never a failed launch (`FR.5.1`); a record that will
/// not parse is logged and the server starts with no phone enrolled, the
/// store having already moved the file aside.
///
/// # Errors
///
/// Only when the XDG config directory itself cannot be resolved, the same
/// failure every other store reports.
fn start_phone_server() -> anyhow::Result<PhonePorts> {
    let record = TomlPhoneRecord::new().context("resolve the XDG config directory")?;
    if let Err(error) = record.read() {
        tracing::warn!(%error, "the phone record could not be read; starting with no phone enrolled");
    }
    let listen_override = record.listen_override();
    let record: Arc<dyn PhoneRecordStore> = Arc::new(record);

    let started = bind_address(listen_override.as_deref())
        .and_then(|bind| RemoteServer::start(RemoteConfig::new(bind), record));
    Ok(match started {
        Ok((handle, intents)) => {
            tracing::info!(address = %handle.local_addr(), "the phone link is ready");
            PhonePorts {
                link: Arc::new(handle),
                intents,
            }
        }
        Err(error) => phone_ports_not_listening(&error),
    })
}

/// The ports for a server that could not start: a link whose every status is
/// `NotListening` with `error`'s text, and a channel whose sender is dropped
/// at once, so the window's intent loop ends the moment it starts.
fn phone_ports_not_listening(error: &StartError) -> PhonePorts {
    tracing::warn!(kind = ?error, %error, "the phone server is not listening");
    let (sender, intents) = async_channel::unbounded::<RemoteIntent>();
    drop(sender);
    PhonePorts {
        link: Arc::new(NotListeningLink::new(error.to_string())),
        intents,
    }
}

fn exit_code(code: glib::ExitCode) -> ExitCode {
    if code == glib::ExitCode::SUCCESS {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use idle_manager_core::{DEFAULT_MOBILE_VIEWPORT, Frame, PhoneStatus, RemoteState};

    use super::*;

    #[test]
    fn a_server_that_could_not_start_becomes_a_not_listening_link_over_a_closed_channel() {
        let ports = phone_ports_not_listening(&StartError::NoMeshAddress);

        ports.link.publish_state(&RemoteState {
            mobile_mode: false,
            viewport: DEFAULT_MOBILE_VIEWPORT,
            current: None,
            workspaces: Vec::new(),
        });
        ports.link.publish_frame(Frame::Jpeg(Vec::new()));
        ports.link.revoke_phone();

        assert_eq!(
            (ports.link.phone_status(), ports.intents.is_closed()),
            (
                PhoneStatus::NotListening {
                    reason: StartError::NoMeshAddress.to_string()
                },
                true
            )
        );
    }
}
