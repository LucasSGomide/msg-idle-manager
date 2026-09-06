//! Composition root: registers the UI bundle, builds one adapter per port,
//! hands them to the shell, and runs the GTK application.

use std::process::ExitCode;
use std::rc::Rc;

use anyhow::Context;
use gtk::glib;
use gtk::prelude::*;
use gtk4 as gtk;
use idle_manager_core::ProfileLocator;
use idle_manager_shell::Window;
use idle_manager_store::XdgProfileLocator;

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

    let app = gtk::Application::builder().application_id(APP_ID).build();

    app.connect_activate(move |app| {
        tracing::info!("activated; presenting the main window");
        let window = Window::new(app, Rc::clone(&locator));
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
