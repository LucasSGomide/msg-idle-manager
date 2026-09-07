//! Composition root: registers the UI bundle, builds one adapter per port,
//! hands them to the shell, and runs the GTK application.

use std::process::ExitCode;
use std::rc::Rc;

use anyhow::Context;
use gtk::glib;
use gtk::prelude::*;
use gtk4 as gtk;
use idle_manager_core::{PresetCatalogue, ProfileLocator, WorkspaceStore};
use idle_manager_shell::Window;
use idle_manager_store::{TomlPresetCatalogue, TomlWorkspaceStore, XdgProfileLocator};

/// The application's D-Bus and settings identifier.
const APP_ID: &str = "org.idlemanager.IdleManager";

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

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
    let store: Rc<dyn WorkspaceStore> = Rc::new(store);

    let app = gtk::Application::builder().application_id(APP_ID).build();

    // A second launch re-activates this window rather than starting a second
    // process (the D-Bus application id), so only one process ever writes the
    // workspace or opens an account's storage.
    app.connect_activate(move |app| {
        tracing::info!("activated; presenting the main window");
        idle_manager_shell::configure_web_engine();

        // Read the workspace before the window is built, so the window can draw
        // the whole restored arrangement at once (architecture rule 3).
        let read_outcome = store.read();
        let window = Window::new(
            app,
            Rc::clone(&locator),
            Rc::clone(&catalogue),
            Rc::clone(&store),
            read_outcome,
        );
        window.present();
    });

    Ok(exit_code(app.run()))
}

fn exit_code(code: glib::ExitCode) -> ExitCode {
    if code == glib::ExitCode::SUCCESS {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
