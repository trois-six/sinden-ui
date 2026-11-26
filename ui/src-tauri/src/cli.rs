//! Command-line arguments and the first-run default.

use std::fs;

use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_autostart::ManagerExt;

/// The arguments the application understands; others are ignored.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Args {
    /// `--tray`: start in the tray, without opening the settings window.
    pub tray: bool,
    /// `--autostart=on|off`: enable or disable the login item, then quit.
    pub autostart: Option<bool>,
}

impl Args {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Self {
        let mut parsed = Self::default();
        for arg in args {
            match arg.as_str() {
                crate::TRAY_ARG => parsed.tray = true,
                "--autostart=on" => parsed.autostart = Some(true),
                "--autostart=off" => parsed.autostart = Some(false),
                _ => {}
            }
        }
        parsed
    }
}

/// The marker of the first run, in the app's configuration directory.
const FIRST_RUN_MARKER: &str = "autostart-initialised";

/// On the first run, enables the login item (the default); later runs keep
/// the user's choice. `--autostart` counts as a first run's choice. Debug
/// builds never register themselves.
pub fn first_run<R: Runtime>(app: &AppHandle<R>) {
    if cfg!(debug_assertions) {
        return;
    }
    let Ok(dir) = app.path().app_config_dir() else {
        return;
    };
    let marker = dir.join(FIRST_RUN_MARKER);
    if marker.exists() {
        return;
    }
    if let Err(err) = app.autolaunch().enable() {
        eprintln!("sinden-driver-ui: cannot enable start at login: {err}");
    }
    mark(&dir, &marker);
}

/// Applies `--autostart=on|off` and records the choice.
pub fn apply_autostart<R: Runtime>(app: &AppHandle<R>, enable: bool) {
    let autolaunch = app.autolaunch();
    let result = if enable {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };
    if let Err(err) = result {
        eprintln!("sinden-driver-ui: cannot change start at login: {err}");
    }
    if let Ok(dir) = app.path().app_config_dir() {
        let marker = dir.join(FIRST_RUN_MARKER);
        mark(&dir, &marker);
    }
}

fn mark(dir: &std::path::Path, marker: &std::path::Path) {
    if let Err(err) = fs::create_dir_all(dir).and_then(|()| fs::write(marker, b"")) {
        eprintln!("sinden-driver-ui: cannot write {}: {err}", marker.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Args {
        Args::parse(args.iter().map(|s| (*s).to_owned()))
    }

    #[test]
    fn arguments() {
        assert_eq!(parse(&[]), Args::default());
        assert_eq!(
            parse(&["--tray"]),
            Args {
                tray: true,
                autostart: None
            }
        );
        assert_eq!(parse(&["--autostart=on"]).autostart, Some(true));
        assert_eq!(
            parse(&["--autostart=off", "--unknown"]).autostart,
            Some(false)
        );
    }
}
