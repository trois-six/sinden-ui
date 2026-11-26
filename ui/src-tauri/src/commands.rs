//! The commands the settings window calls.

// Tauri hands commands their arguments by value.
#![allow(clippy::needless_pass_by_value)]

use std::collections::BTreeMap;

use sinden_control::{Client, ConfigSnapshot};
use tauri::{AppHandle, Runtime, State};
use tauri_plugin_autostart::ManagerExt;

use crate::AppState;
use crate::i18n::Lang;
use crate::monitor::ServiceStatus;

/// Errors cross to the window as their message.
type CommandResult<T> = Result<T, String>;

#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> ServiceStatus {
    state.monitor.status()
}

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> CommandResult<ConfigSnapshot> {
    let endpoint = state.monitor.endpoint().clone();
    blocking(move || Client::connect(&endpoint)?.get_config()).await
}

#[tauri::command]
pub async fn set_config(
    state: State<'_, AppState>,
    values: BTreeMap<String, String>,
) -> CommandResult<ConfigSnapshot> {
    let endpoint = state.monitor.endpoint().clone();
    blocking(move || Client::connect(&endpoint)?.set_config(&values)).await
}

#[tauri::command]
pub fn get_autostart<R: Runtime>(app: AppHandle<R>) -> CommandResult<bool> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_autostart<R: Runtime>(app: AppHandle<R>, enabled: bool) -> CommandResult<()> {
    let autolaunch = app.autolaunch();
    if enabled {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    }
    .map_err(|e| e.to_string())
}

/// The window's language choice; the tray follows it.
#[tauri::command]
pub fn set_language<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>, lang: &str) {
    let lang = Lang::from_tag(lang);
    *state
        .lang
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = lang;
    crate::tray::update(&app, &state.monitor.status());
}

/// Runs a blocking control API call off the async runtime.
async fn blocking<T: Send + 'static>(
    call: impl FnOnce() -> sinden_control::Result<T> + Send + 'static,
) -> CommandResult<T> {
    tauri::async_runtime::spawn_blocking(call)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}
