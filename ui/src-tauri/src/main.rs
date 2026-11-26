//! sinden-driver-ui: a tray application for the sinden-driver service.
//!
//! It starts in the tray (no window when started with `--tray`, as at
//! login), shows the guns' state from the driver's control API and opens a
//! settings window that edits the driver's configuration.

// No console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cli;
mod commands;
mod i18n;
mod monitor;
mod tray;
mod window;

use std::sync::Mutex;

use tauri::{Manager, RunEvent};
use tauri_plugin_autostart::MacosLauncher;

use crate::cli::Args;
use crate::monitor::Monitor;

/// The argument the login item passes, so that the app starts in the tray.
pub const TRAY_ARG: &str = "--tray";

/// What the app shares between the tray, the monitor and the commands.
#[derive(Debug)]
pub struct AppState {
    pub monitor: Monitor,
    pub lang: Mutex<i18n::Lang>,
}

fn main() {
    let args = Args::parse(std::env::args().skip(1));

    tauri::Builder::default()
        // A second launch hands its arguments to the running instance.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            let args = Args::parse(argv.into_iter().skip(1));
            if let Some(enable) = args.autostart {
                cli::apply_autostart(app, enable);
            } else {
                window::open_settings(app);
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![TRAY_ARG]),
        ))
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::get_config,
            commands::set_config,
            commands::get_autostart,
            commands::set_autostart,
            commands::set_language,
        ])
        .setup(move |app| {
            // `--autostart=on|off` (used by the installers): set it and quit.
            if let Some(enable) = args.autostart {
                cli::apply_autostart(app.handle(), enable);
                app.handle().exit(0);
                return Ok(());
            }
            cli::first_run(app.handle());

            // A tray application: no Dock icon on macOS.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let lang = i18n::Lang::from_system();
            app.manage(AppState {
                monitor: Monitor::default(),
                lang: Mutex::new(lang),
            });
            tray::create(app.handle())?;
            monitor::spawn(app.handle().clone());

            if !args.tray {
                window::open_settings(app.handle());
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("cannot start the application")
        .run(|_app, event| {
            // Closing the settings window keeps the tray running; only
            // "Quit" (an explicit exit code) ends the application.
            if let RunEvent::ExitRequested {
                api, code: None, ..
            } = event
            {
                api.prevent_exit();
            }
        });
}
