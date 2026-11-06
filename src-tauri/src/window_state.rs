//! Remembers the main window's size and position between sessions (from Masters Companion, plus the position).
//! The window starts hidden (tauri.conf.json), gets its saved size and place, and is shown once the page has loaded,
//! so the move is never visible. It's saved when the window closes.

use crate::storage::{to_json, write_atomic};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, Window};

const FILE_NAME: &str = "window-state.json";
const MIN: (f64, f64) = (360.0, 600.0); // same as minWidth/minHeight in tauri.conf.json
/// How much of the title bar must be on a screen for the saved place to be used (physical pixels).
const GRIP: (i32, i32) = (100, 20);

/// The window's last size (logical) and position (physical) while neither maximized nor minimized: what a maximized
/// window goes back to, and what's saved when it closes maximized or minimized.
static NORMAL: Mutex<Option<Normal>> = Mutex::new(None);

#[derive(Debug, Clone, Copy, Default)]
struct Normal {
    size: Option<(f64, f64)>,
    position: Option<(i32, i32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
    /// Size in logical pixels, so it survives a change of display scaling.
    pub width: f64,
    pub height: f64,
    /// The top-left corner in physical pixels, as the screens are laid out; missing in files from before it was saved.
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,
    pub maximized: bool,
}

/// A screen's top-left corner and size, in physical pixels.
pub type Screen = (i32, i32, u32, u32);

fn path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|dir| dir.join(FILE_NAME))
}

fn load(file: &Path) -> Option<WindowState> {
    serde_json::from_str(&fs::read_to_string(file).ok()?).ok()
}

/// Fits a saved size within the minimum size and the screen it opens on.
pub fn fit(saved: WindowState, screen: Option<(f64, f64)>) -> (f64, f64) {
    let (mut width, mut height) = (saved.width.max(MIN.0), saved.height.max(MIN.1));
    if let Some((screen_w, screen_h)) = screen {
        width = width.min(screen_w.max(MIN.0));
        height = height.min(screen_h.max(MIN.1));
    }
    (width, height)
}

/// Whether a window at this top-left corner has the start of its title bar on one of the screens, so it can be
/// grabbed. A screen that's been unplugged since leaves the window where the OS puts it.
pub fn on_screen((x, y): (i32, i32), screens: &[Screen]) -> bool {
    let (gx, gy) = (x.saturating_add(GRIP.0), y.saturating_add(GRIP.1));
    screens.iter().any(|&(sx, sy, w, h)| gx >= sx && gy >= sy && i64::from(gx) < i64::from(sx) + i64::from(w) && i64::from(gy) < i64::from(sy) + i64::from(h))
}

/// Applies the saved size and place before the window is first shown. Without them, the config's size stays.
pub fn restore(window: &Window) {
    let Some(saved) = path(window.app_handle()).and_then(|f| load(&f)) else { return };
    // Failures here just mean the default size or place; not worth stopping the app for.
    if let (Some(x), Some(y)) = (saved.x, saved.y) {
        let screens: Vec<Screen> = window
            .available_monitors()
            .unwrap_or_default()
            .iter()
            .map(|m| (m.position().x, m.position().y, m.size().width, m.size().height))
            .collect();
        if on_screen((x, y), &screens) {
            let _ = window.set_position(PhysicalPosition::new(x, y));
        }
    }
    let screen = window.current_monitor().ok().flatten().map(|m| {
        let size = m.size().to_logical::<f64>(m.scale_factor());
        (size.width, size.height)
    });
    let (width, height) = fit(saved, screen);
    let _ = window.set_size(LogicalSize::new(width, height));
    if saved.maximized {
        let _ = window.maximize();
    }
}

fn logical_size(window: &Window) -> Option<(f64, f64)> {
    let size = window.inner_size().ok()?.to_logical::<f64>(window.scale_factor().unwrap_or(1.0));
    Some((size.width, size.height))
}

/// Neither maximized nor minimized (a minimized window on Windows sits far off screen with no size).
fn is_normal(window: &Window) -> bool {
    !window.is_maximized().unwrap_or(false) && !window.is_minimized().unwrap_or(false)
}

/// Remembers the size and place on every resize or move of a normal window.
pub fn track(window: &Window) {
    if !is_normal(window) {
        return;
    }
    let mut normal = NORMAL.lock().unwrap_or_else(|p| p.into_inner());
    let tracked = normal.get_or_insert_with(Normal::default);
    tracked.size = logical_size(window).or(tracked.size);
    tracked.position = window.outer_position().ok().map(|p| (p.x, p.y)).or(tracked.position);
}

/// Stores the window's size and place when it closes. Closed maximized or minimized, it keeps the size and place it
/// had before, so un-maximizing next session returns to the user's own.
pub fn save(window: &Window) {
    let Some(file) = path(window.app_handle()) else { return };
    let maximized = window.is_maximized().unwrap_or(false);
    let saved = load(&file);
    let (size, position) = if is_normal(window) {
        (logical_size(window), window.outer_position().ok().map(|p| (p.x, p.y)))
    } else {
        let tracked = NORMAL.lock().unwrap_or_else(|p| p.into_inner()).unwrap_or_default();
        let config = window.app_handle().config().app.windows.first().map(|w| (w.width, w.height));
        (
            tracked.size.or(saved.map(|s| (s.width, s.height))).or(config),
            tracked.position.or(saved.and_then(|s| s.x.zip(s.y))),
        )
    };
    let Some((width, height)) = size else { return };
    let state = WindowState { width, height, x: position.map(|p| p.0), y: position.map(|p| p.1), maximized };
    // The app is closing, so there's no screen left to show an error on; log it instead.
    if let Err(e) = to_json(&state).and_then(|json| write_atomic(&file, &json)) {
        eprintln!("Couldn't save the window size and position: {e:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved(width: f64, height: f64) -> WindowState {
        WindowState { width, height, x: None, y: None, maximized: false }
    }

    #[test]
    fn fit_keeps_the_size_within_minimum_and_screen() {
        assert_eq!(fit(saved(1400.0, 900.0), Some((1920.0, 1080.0))), (1400.0, 900.0));
        assert_eq!(fit(saved(2560.0, 1440.0), Some((1920.0, 1080.0))), (1920.0, 1080.0));
        assert_eq!(fit(saved(100.0, 100.0), Some((1920.0, 1080.0))), (360.0, 600.0));
        assert_eq!(fit(saved(1400.0, 900.0), None), (1400.0, 900.0));
    }

    #[test]
    fn a_saved_place_is_used_only_when_its_title_bar_is_on_a_screen() {
        // A main screen and a second one to its left.
        let screens = [(0, 0, 1920, 1080), (-1920, 0, 1920, 1080)];
        assert!(on_screen((200, 100), &screens));
        assert!(on_screen((-1500, 50), &screens), "on the second screen");
        assert!(on_screen((-50, 0), &screens), "across both");
        assert!(!on_screen((1900, 100), &screens), "only a sliver showing on the right");
        assert!(!on_screen((-4000, 100), &screens), "a screen that's been unplugged");
        assert!(!on_screen((-32000, -32000), &screens), "where Windows parks a minimized window");
        assert!(!on_screen((200, 100), &[]));
    }

    #[test]
    fn state_round_trips_and_older_files_without_a_place_still_load() {
        let state = WindowState { width: 1280.5, height: 800.0, x: Some(-1500), y: Some(40), maximized: true };
        let back: WindowState = serde_json::from_str(&to_json(&state).unwrap()).unwrap();
        assert_eq!(back, state);
        let old: WindowState = serde_json::from_str(r#"{ "width": 1280, "height": 800, "maximized": false }"#).unwrap();
        assert_eq!((old.x, old.y), (None, None));
    }
}
