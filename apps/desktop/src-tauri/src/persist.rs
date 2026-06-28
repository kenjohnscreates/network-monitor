//! Persistence for [`Settings`] in the OS app config directory.
//!
//! File layout: `<app_config_dir>/settings.json`. Failures are non-fatal:
//! we always fall back to defaults so the app still launches.

use std::fs;
use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use crate::state::Settings;

const FILE_NAME: &str = "settings.json";

pub fn settings_path(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_config_dir().ok()?;
    Some(dir.join(FILE_NAME))
}

pub fn load(app: &AppHandle) -> Option<Settings> {
    let path = settings_path(app)?;
    let body = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&body).ok()
}

pub fn save(app: &AppHandle, settings: &Settings) -> std::io::Result<()> {
    let path = settings_path(app).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "no app config dir")
    })?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(settings).map_err(std::io::Error::other)?;
    fs::write(path, body)
}
