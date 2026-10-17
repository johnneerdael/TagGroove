//! Options → General → Notification area: an icon in the Windows notification
//! area or the Linux status area, and minimizing the main window into it so the
//! task bar shows no entry. Both are off by default. macOS has no notification
//! area for this (the Dock keeps minimized windows), so it reports no support.
#![cfg_attr(
    not(any(windows, target_os = "linux")),
    allow(dead_code, unused_imports)
)]
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrayOptions {
    /// Always show the icon.
    pub show_icon: bool,
    /// Hide the window into the icon when it is minimized.
    pub minimize_to_tray: bool,
}

#[derive(Debug, Serialize)]
pub struct TrayState {
    pub supported: bool,
    /// Whether the icon is shown now.
    pub active: bool,
}

/// The icon exists while it is always shown or while the window is hidden in it.
pub fn icon_wanted(options: TrayOptions, hidden: bool) -> bool {
    options.show_icon || hidden
}
/// A minimized window goes to the notification area only with that option on.
pub fn hide_on_minimize(options: TrayOptions, minimized: bool, hidden: bool) -> bool {
    options.minimize_to_tray && minimized && !hidden
}

static OPTIONS: Mutex<TrayOptions> = Mutex::new(TrayOptions {
    show_icon: false,
    minimize_to_tray: false,
});
static HIDDEN: AtomicBool = AtomicBool::new(false);

/// Menu labels of the icon in the UI language (Options → Language).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct TrayLabels {
    pub show: String,
    pub quit: String,
}
static LABELS: Mutex<Option<TrayLabels>> = Mutex::new(None);

fn labels() -> TrayLabels {
    LABELS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .unwrap_or_else(|| TrayLabels {
            show: "Show TagGroove".into(),
            quit: "Quit TagGroove".into(),
        })
}
/// Store new labels; returns whether they changed.
fn relabel(next: Option<TrayLabels>) -> bool {
    let Some(next) = next.filter(|l| !l.show.trim().is_empty() && !l.quit.trim().is_empty()) else {
        return false;
    };
    let mut current = LABELS.lock().unwrap_or_else(|e| e.into_inner());
    let changed = current.as_ref() != Some(&next);
    *current = Some(next);
    changed
}

fn options() -> TrayOptions {
    *OPTIONS.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(any(windows, target_os = "linux"))]
mod native {
    use super::{icon_wanted, labels, options, HIDDEN};
    use std::sync::atomic::Ordering;
    use tauri::{
        menu::{MenuBuilder, MenuItemBuilder},
        tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
        AppHandle, Manager,
    };
    const TRAY_ID: &str = "opentag-tray";

    /// The status area on Linux needs libayatana-appindicator (or libappindicator)
    /// at run time; without it, report the problem instead of creating the icon.
    #[cfg(target_os = "linux")]
    fn indicator_available() -> Result<(), String> {
        for name in ["libayatana-appindicator3.so.1", "libappindicator3.so.1"] {
            // SAFETY: only probes that the library loads; no symbol is called.
            if unsafe { libloading::Library::new(name) }.is_ok() {
                return Ok(());
            }
        }
        Err("The notification area icon needs libayatana-appindicator3 (or libappindicator3). Install it to use this option.".into())
    }

    pub fn restore(app: &AppHandle) {
        HIDDEN.store(false, Ordering::SeqCst);
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
        let _ = sync(app);
    }

    fn create(app: &AppHandle) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        indicator_available()?;
        let error = |e: tauri::Error| e.to_string();
        let labels = labels();
        let show = MenuItemBuilder::with_id("tray-show", labels.show)
            .build(app)
            .map_err(error)?;
        let quit = MenuItemBuilder::with_id("tray-quit", labels.quit)
            .build(app)
            .map_err(error)?;
        let menu = MenuBuilder::new(app)
            .items(&[&show, &quit])
            .build()
            .map_err(error)?;
        let mut builder = TrayIconBuilder::with_id(TRAY_ID)
            .tooltip("TagGroove")
            .menu(&menu)
            .show_menu_on_left_click(false)
            .on_menu_event(|app, event| match event.id().as_ref() {
                "tray-show" => restore(app),
                // Closing asks about unsaved changes like the window's close button.
                "tray-quit" => {
                    restore(app);
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.close();
                    }
                }
                _ => {}
            })
            .on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    restore(tray.app_handle());
                }
            });
        if let Some(icon) = app.default_window_icon() {
            builder = builder.icon(icon.clone());
        }
        builder.build(app).map_err(error)?;
        Ok(())
    }

    /// Rebuild the icon's menu with new labels.
    pub fn rebuild(app: &AppHandle) {
        if app.tray_by_id(TRAY_ID).is_some() {
            let _ = app.remove_tray_by_id(TRAY_ID);
        }
    }

    /// Create or remove the icon to match the options; returns whether it is shown.
    pub fn sync(app: &AppHandle) -> Result<bool, String> {
        let wanted = icon_wanted(options(), HIDDEN.load(Ordering::SeqCst));
        let exists = app.tray_by_id(TRAY_ID).is_some();
        if wanted && !exists {
            create(app)?;
        } else if !wanted && exists {
            let _ = app.remove_tray_by_id(TRAY_ID);
        }
        Ok(wanted)
    }
}

/// Main window events: a minimize hides the window into the notification area.
pub fn track(window: &tauri::Window, event: &tauri::WindowEvent) {
    let minimize_event = matches!(
        event,
        tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Focused(false)
    ) && window.label() == "main";
    #[cfg(any(windows, target_os = "linux"))]
    if minimize_event
        && hide_on_minimize(
            options(),
            window.is_minimized().unwrap_or(false),
            HIDDEN.load(Ordering::SeqCst),
        )
    {
        use tauri::Manager as _;
        HIDDEN.store(true, Ordering::SeqCst);
        // Hide only once the icon exists, so the window can always be restored.
        if native::sync(window.app_handle()).is_ok() {
            let _ = window.hide();
        } else {
            HIDDEN.store(false, Ordering::SeqCst);
        }
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    let _ = minimize_event;
}

#[tauri::command]
pub fn set_tray_options(
    _app: tauri::AppHandle,
    show_icon: bool,
    minimize_to_tray: bool,
    labels: Option<TrayLabels>,
) -> Result<TrayState, String> {
    *OPTIONS.lock().unwrap_or_else(|e| e.into_inner()) = TrayOptions {
        show_icon,
        minimize_to_tray,
    };
    let relabelled = relabel(labels);
    #[cfg(any(windows, target_os = "linux"))]
    {
        if relabelled {
            native::rebuild(&_app);
        }
        let active = native::sync(&_app)?;
        Ok(TrayState {
            supported: true,
            active,
        })
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = relabelled;
        Ok(TrayState {
            supported: false,
            active: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icon_follows_the_options_and_the_hidden_window() {
        let off = TrayOptions::default();
        let always = TrayOptions {
            show_icon: true,
            minimize_to_tray: false,
        };
        let minimize = TrayOptions {
            show_icon: false,
            minimize_to_tray: true,
        };
        assert!(!icon_wanted(off, false));
        assert!(icon_wanted(always, false));
        assert!(!icon_wanted(minimize, false));
        // A window hidden in the notification area keeps its icon until it is restored.
        assert!(icon_wanted(minimize, true));
        assert!(hide_on_minimize(minimize, true, false));
        assert!(!hide_on_minimize(minimize, false, false));
        assert!(!hide_on_minimize(minimize, true, true));
        assert!(!hide_on_minimize(always, true, false));
        assert!(!hide_on_minimize(off, true, false));
    }

    #[test]
    fn menu_labels_follow_the_ui_language_and_ignore_blank_ones() {
        assert_eq!(labels().show, "Show TagGroove");
        let dutch = TrayLabels {
            show: "TagGroove tonen".into(),
            quit: "TagGroove afsluiten".into(),
        };
        assert!(relabel(Some(dutch.clone())));
        assert!(!relabel(Some(dutch.clone())));
        assert!(!relabel(None));
        assert!(!relabel(Some(TrayLabels {
            show: " ".into(),
            quit: "x".into()
        })));
        assert_eq!(labels(), dutch);
    }
}
