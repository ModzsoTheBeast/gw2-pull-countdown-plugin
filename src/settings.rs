use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const ADDON_DIR_NAME: &str = "pull_countdown";
const SETTINGS_FILE_NAME: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub start_count: u32,

    /// Top-left corner of the overlay window, as a fraction of screen size (0.0-1.0 each axis).
    pub overlay_pos_frac: [f32; 2],
    /// Point size of the dedicated overlay font (not a scale multiplier - the font itself is
    /// rebaked at this size, so it stays crisp instead of getting blurry when large).
    pub overlay_font_size: f32,
    /// RGB color of the overlay countdown numbers (not the "PULL!" flash, which is always red).
    pub overlay_color: [f32; 3],
    /// Whether the overlay is pinned in place. While unlocked, it can be dragged and shows a
    /// preview even with no countdown running, so it can be positioned without spamming Pull.
    pub overlay_locked: bool,

    /// Template for each chat line, with `{n}` replaced by the count. Used as-is for the single
    /// heads-up message when `chat_countdown_enabled` is off, and once per second (for `{n}` from
    /// `chat_countdown_start` down to 1) when it's on.
    pub chat_message_template: String,
    /// Sent instead of the template once the count reaches 0.
    pub chat_pull_text: String,
    /// Sent when an in-progress pull is cancelled (triggering the icon/keybind again while a
    /// countdown is running cancels it instead of starting a new one). Keep the word "cancel" in
    /// there somewhere, same reasoning as "pull" for the countdown messages.
    pub chat_cancel_text: String,
    /// Off by default: post one heads-up chat message when Pull is triggered, instead of a
    /// per-second countdown in chat.
    pub chat_countdown_enabled: bool,
    /// Only used when `chat_countdown_enabled` is on: chat starts counting down from this number
    /// (clamped to the overlay's starting number), landing on 0/`chat_pull_text` at the same
    /// moment the overlay does.
    pub chat_countdown_start: u32,
    /// Automatically trigger a pull the moment a squad ready check finishes with everyone ready.
    /// Requires arcdps + Unofficial Extras to detect the ready check at all.
    pub auto_pull_after_ready_check: bool,
    /// Off by default: send through GW2's squad broadcast (Shift+Enter) instead of a normal
    /// squad/party chat line. More attention-grabbing, but squad-only - falls back to normal
    /// chat automatically when not in a squad (e.g. just a party).
    pub use_squad_broadcast: bool,
}

impl Settings {
    /// Const-evaluable defaults for the static initializer. String fields are left empty here
    /// (`String::from` isn't `const`) - the real text defaults live in `Default::default()`
    /// below, which `load()` falls back to well before anything is ever rendered.
    pub const fn const_default() -> Self {
        Self {
            start_count: 10,
            overlay_pos_frac: [0.42, 0.12],
            overlay_font_size: 72.0,
            overlay_color: [1.0, 1.0, 1.0],
            overlay_locked: true,
            chat_message_template: String::new(),
            chat_pull_text: String::new(),
            chat_cancel_text: String::new(),
            chat_countdown_enabled: false,
            chat_countdown_start: 5,
            auto_pull_after_ready_check: false,
            use_squad_broadcast: false,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            chat_message_template: "Pulling in {n}...".to_string(),
            chat_pull_text: "Pull!".to_string(),
            chat_cancel_text: "Pull cancelled.".to_string(),
            ..Self::const_default()
        }
    }
}

fn settings_path() -> Option<PathBuf> {
    nexus::paths::get_addon_dir(ADDON_DIR_NAME)
        .ok()
        .map(|dir| dir.join(SETTINGS_FILE_NAME))
}

/// Loads settings from disk into `state::SETTINGS`, falling back to defaults
/// if the file is missing or corrupt.
pub fn load() {
    let loaded = settings_path()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default();
    *crate::state::SETTINGS.lock().unwrap() = loaded;
}

/// Persists the current in-memory settings to disk.
pub fn save() {
    let settings = crate::state::SETTINGS.lock().unwrap().clone();
    let Some(path) = settings_path() else {
        log::warn!("could not resolve addon settings directory, not saving settings");
        return;
    };
    if let Some(parent) = path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            log::warn!("failed to create addon settings directory: {err}");
            return;
        }
    }
    match serde_json::to_string_pretty(&settings) {
        Ok(json) => {
            if let Err(err) = fs::write(&path, json) {
                log::warn!("failed to write settings file: {err}");
            }
        }
        Err(err) => log::warn!("failed to serialize settings: {err}"),
    }
}
