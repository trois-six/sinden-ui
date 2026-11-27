//! The tray icon and its menu: open the settings, one line per gun, quit.

use std::sync::PoisonError;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

use crate::i18n::{self, Lang, Text};
use crate::monitor::ServiceStatus;

const TRAY_ID: &str = "main";
const OPEN: &str = "open";
const QUIT: &str = "quit";

fn lang<R: Runtime>(app: &AppHandle<R>) -> Lang {
    *app.state::<crate::AppState>()
        .lang
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

fn menu<R: Runtime>(
    app: &AppHandle<R>,
    lang: Lang,
    status: &ServiceStatus,
) -> tauri::Result<Menu<R>> {
    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(
        app,
        OPEN,
        i18n::text(lang, Text::OpenSettings),
        true,
        None::<&str>,
    )?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    let update = firmware_line(app, lang);
    let mut lines: Vec<String> = if !status.connected {
        vec![i18n::text(lang, Text::Unreachable).to_owned()]
    } else if status.guns.is_empty() && update.is_none() {
        vec![i18n::text(lang, Text::NoGun).to_owned()]
    } else {
        status
            .guns
            .iter()
            .map(|gun| i18n::gun_line(lang, gun))
            .collect()
    };
    lines.extend(update);
    for (index, line) in lines.iter().enumerate() {
        menu.append(&MenuItem::with_id(
            app,
            format!("gun-{index}"),
            line,
            false,
            None::<&str>,
        )?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        QUIT,
        i18n::text(lang, Text::Quit),
        true,
        None::<&str>,
    )?)?;
    Ok(menu)
}

/// The line of a firmware update running or failed, if any.
fn firmware_line<R: Runtime>(app: &AppHandle<R>, lang: Lang) -> Option<String> {
    let job = app.state::<crate::AppState>().firmware.job()?;
    i18n::firmware_line(lang, &job)
}

fn tooltip(lang: Lang, status: &ServiceStatus, update: Option<String>) -> String {
    let mut text = i18n::text(lang, Text::WindowTitle).to_owned();
    if !status.connected {
        text.push_str(" — ");
        text.push_str(i18n::text(lang, Text::Unreachable));
    }
    for line in status
        .guns
        .iter()
        .map(|gun| i18n::gun_line(lang, gun))
        .chain(update)
    {
        text.push('\n');
        text.push_str(&line);
    }
    text
}

/// Creates the tray icon.
pub fn create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let lang = lang(app);
    let status = app.state::<crate::AppState>().monitor.status();
    let builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu(app, lang, &status)?)
        .tooltip(tooltip(lang, &status, firmware_line(app, lang)))
        // macOS shows the menu on a click; elsewhere a left click opens the
        // settings and a right click shows the menu.
        .show_menu_on_left_click(cfg!(target_os = "macos"))
        .on_menu_event(|app, event| match event.id().as_ref() {
            OPEN => crate::window::open_settings(app),
            QUIT => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                crate::window::open_settings(tray.app_handle());
            }
        });

    // A monochrome template in the macOS menu bar, a white sight with a dark
    // outline on Linux panels, the application icon on Windows.
    #[cfg(target_os = "macos")]
    let builder = builder
        .icon(tauri::include_image!("icons/tray-template.png"))
        .icon_as_template(true);
    #[cfg(all(unix, not(target_os = "macos")))]
    let builder = builder.icon(tauri::include_image!("icons/tray.png"));
    #[cfg(windows)]
    let builder = match app.default_window_icon() {
        Some(icon) => builder.icon(icon.clone()),
        None => builder,
    };

    builder.build(app)?;
    Ok(())
}

/// Refreshes the menu and tooltip after a status or language change.
pub fn update<R: Runtime>(app: &AppHandle<R>, status: &ServiceStatus) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let lang = lang(app);
    match menu(app, lang, status) {
        Ok(menu) => {
            let _ = tray.set_menu(Some(menu));
        }
        Err(err) => eprintln!("sinden-driver-ui: cannot update the tray menu: {err}"),
    }
    let _ = tray.set_tooltip(Some(tooltip(lang, status, firmware_line(app, lang))));
}
