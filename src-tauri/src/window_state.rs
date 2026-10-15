//! The main window's size, position and maximized state across restarts, as
//! Mp3tag keeps them. Stored in `window-state.json` next to the settings, not
//! in the options, so configuration backups stay portable between screens.
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Mutex};
use tauri::{Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowState {
    /// Outer position and inner size in physical pixels.
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

/// A monitor's area in physical pixels.
#[derive(Debug, Clone, Copy)]
pub struct Area {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Part of the title bar that must stay on some screen for a restored position.
const VISIBLE_WIDTH: i64 = 120;
const VISIBLE_HEIGHT: i64 = 40;

impl WindowState {
    /// The state to apply on these monitors: the size is capped to the largest
    /// monitor, and a position that would leave the window unreachable (a
    /// disconnected display) is dropped.
    pub fn fitted(self, monitors: &[Area]) -> (Option<(i32, i32)>, (u32, u32)) {
        let width = monitors.iter().map(|m| m.width).max().unwrap_or(self.width);
        let height = monitors
            .iter()
            .map(|m| m.height)
            .max()
            .unwrap_or(self.height);
        let size = (self.width.min(width), self.height.min(height));
        let reachable = monitors.iter().any(|m| {
            let left = i64::from(self.x).max(i64::from(m.x));
            let right =
                (i64::from(self.x) + i64::from(size.0)).min(i64::from(m.x) + i64::from(m.width));
            let top = i64::from(self.y);
            right - left >= VISIBLE_WIDTH
                && top >= i64::from(m.y)
                && top + VISIBLE_HEIGHT <= i64::from(m.y) + i64::from(m.height)
        });
        (reachable.then_some((self.x, self.y)), size)
    }
}

static LAST: Mutex<Option<WindowState>> = Mutex::new(None);

fn path(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join("window-state.json"))
}

/// Apply the saved state to the freshly created main window.
pub fn restore(window: &WebviewWindow) {
    let Some(file) = path(window.app_handle()) else {
        return;
    };
    let Some(state) = std::fs::read(&file)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<WindowState>(&bytes).ok())
        .filter(|s| s.width > 0 && s.height > 0)
    else {
        return;
    };
    let monitors: Vec<Area> = window
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| Area {
            x: m.position().x,
            y: m.position().y,
            width: m.size().width,
            height: m.size().height,
        })
        .collect();
    let (position, (width, height)) = state.fitted(&monitors);
    let _ = window.set_size(PhysicalSize::new(width, height));
    if let Some((x, y)) = position {
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }
    if state.maximized {
        let _ = window.maximize();
    }
    if let Ok(mut last) = LAST.lock() {
        *last = Some(state);
    }
}

/// Remember the normal (not maximized or full-screen) geometry as it changes.
pub fn track(window: &tauri::Window, event: &tauri::WindowEvent) {
    if window.label() != "main" {
        return;
    }
    match event {
        tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) => {
            let maximized = window.is_maximized().unwrap_or(false);
            let hidden =
                window.is_minimized().unwrap_or(false) || window.is_fullscreen().unwrap_or(false);
            let Ok(mut last) = LAST.lock() else {
                return;
            };
            if hidden {
                return;
            }
            if maximized {
                if let Some(state) = last.as_mut() {
                    state.maximized = true;
                }
                return;
            }
            if let (Ok(position), Ok(size)) = (window.outer_position(), window.inner_size()) {
                *last = Some(WindowState {
                    x: position.x,
                    y: position.y,
                    width: size.width,
                    height: size.height,
                    maximized: false,
                });
            }
        }
        tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed => {
            save(window.app_handle());
        }
        _ => {}
    }
}

/// Write the last known state; called when the window closes and on exit.
pub fn save(app: &tauri::AppHandle) {
    let Some(state) = LAST.lock().ok().and_then(|last| *last) else {
        return;
    };
    let Some(file) = path(app) else {
        return;
    };
    if let Ok(bytes) = serde_json::to_vec_pretty(&state) {
        let _ = crate::action_host::atomic_output(&file, &bytes, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAPTOP: Area = Area {
        x: 0,
        y: 0,
        width: 2880,
        height: 1800,
    };
    const EXTERNAL: Area = Area {
        x: 2880,
        y: -200,
        width: 3840,
        height: 2160,
    };

    fn state(x: i32, y: i32, width: u32, height: u32) -> WindowState {
        WindowState {
            x,
            y,
            width,
            height,
            maximized: false,
        }
    }

    #[test]
    fn positions_on_a_connected_screen_are_kept() {
        assert_eq!(
            state(100, 80, 1600, 1000).fitted(&[LAPTOP]),
            (Some((100, 80)), (1600, 1000))
        );
        assert_eq!(
            state(3000, 0, 2000, 1200).fitted(&[LAPTOP, EXTERNAL]),
            (Some((3000, 0)), (2000, 1200))
        );
    }

    #[test]
    fn unreachable_positions_are_dropped_and_sizes_capped() {
        // The external display was disconnected.
        assert_eq!(
            state(3000, 0, 2000, 1200).fitted(&[LAPTOP]),
            (None, (2000, 1200))
        );
        // The title bar would sit above the top of the screen.
        assert_eq!(state(100, -500, 800, 600).fitted(&[LAPTOP]).0, None);
        // Only a sliver would be visible at the right edge.
        assert_eq!(state(2800, 100, 800, 600).fitted(&[LAPTOP]).0, None);
        assert_eq!(state(0, 0, 9000, 9000).fitted(&[LAPTOP]).1, (2880, 1800));
        // Without monitor information only the size is restored.
        assert_eq!(state(5, 5, 900, 700).fitted(&[]), (None, (900, 700)));
    }

    #[test]
    fn state_round_trips_as_json() {
        let saved = WindowState {
            maximized: true,
            ..state(-10, 20, 1300, 900)
        };
        let json = serde_json::to_string(&saved).unwrap();
        assert_eq!(serde_json::from_str::<WindowState>(&json).unwrap(), saved);
    }
}
