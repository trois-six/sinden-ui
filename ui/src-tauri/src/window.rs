//! The settings window, created on demand and destroyed when closed.

use tauri::{AppHandle, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

pub const SETTINGS: &str = "settings";

/// Shows the settings window, creating it if needed.
pub fn open_settings<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(SETTINGS) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let title = {
        let state = app.state::<crate::AppState>();
        let lang = *state
            .lang
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        crate::i18n::text(lang, crate::i18n::Text::WindowTitle).to_owned()
    };
    let built = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("index.html".into()))
        .title(title)
        .inner_size(900.0, 760.0)
        .min_inner_size(560.0, 420.0)
        .center()
        .build();
    if let Err(err) = built {
        eprintln!("sinden-driver-ui: cannot open the settings window: {err}");
    }
}
