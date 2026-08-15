use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const ADDON_DIR_NAME: &str = "pull_countdown";
const SETTINGS_FILE_NAME: &str = "settings.json";
const DEFAULT_PROFILE_NAME: &str = "Default";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub start_count: u32,

    /// Center point of the overlay window, as a fraction of screen size (0.0-1.0 each axis).
    /// Anchored on the center (not a corner) so differently-sized text ("10" vs "PULL!") stays
    /// centered on the same spot instead of one edge drifting as the content width changes.
    pub overlay_pos_frac: [f32; 2],
    /// Point size of the dedicated overlay font (not a scale multiplier - the font itself is
    /// rebaked at this size, so it stays crisp instead of getting blurry when large).
    pub overlay_font_size: f32,
    /// RGB color of the overlay countdown numbers (not the "PULL!" flash, which is always red).
    pub overlay_color: [f32; 3],
    /// Whether the overlay is pinned in place. While unlocked, it can be dragged and shows a
    /// preview even with no countdown running, so it can be positioned without spamming Pull.
    pub overlay_locked: bool,
    /// On by default: plays a short alert sound the moment the count reaches "PULL!" (local
    /// only - it's not sent to anyone else).
    pub sound_enabled: bool,

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
    /// chat automatically when not in a squad (e.g. just a party). Only ever applies to the
    /// upfront heads-up message and the final "pull now" line - broadcasts stay on screen for
    /// several seconds and queue up if sent faster than that, so a per-second chat countdown's
    /// tail ticks always use normal chat regardless of this setting (see `chat_send`).
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
            sound_enabled: true,
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

/// One named, independently switchable configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub settings: Settings,
}

/// On-disk shape: every saved profile, plus which one is active.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProfileStore {
    active: String,
    profiles: Vec<Profile>,
}

impl ProfileStore {
    fn fallback() -> Self {
        Self {
            active: DEFAULT_PROFILE_NAME.to_string(),
            profiles: vec![Profile {
                name: DEFAULT_PROFILE_NAME.to_string(),
                settings: Settings::default(),
            }],
        }
    }
}

fn settings_path() -> Option<PathBuf> {
    nexus::paths::get_addon_dir(ADDON_DIR_NAME)
        .ok()
        .map(|dir| dir.join(SETTINGS_FILE_NAME))
}

/// Loads all profiles from disk and activates whichever one was last active, falling back to a
/// single "Default" profile if the file is missing/corrupt - or if it's still in the old
/// pre-profile format (a bare `Settings` object), which gets migrated into a "Default" profile.
pub fn load() {
    let contents = settings_path().and_then(|path| fs::read_to_string(path).ok());

    let store = match contents {
        Some(contents) => serde_json::from_str::<ProfileStore>(&contents)
            .or_else(|_| {
                serde_json::from_str::<Settings>(&contents).map(|settings| ProfileStore {
                    active: DEFAULT_PROFILE_NAME.to_string(),
                    profiles: vec![Profile {
                        name: DEFAULT_PROFILE_NAME.to_string(),
                        settings,
                    }],
                })
            })
            .unwrap_or_else(|_| ProfileStore::fallback()),
        None => ProfileStore::fallback(),
    };

    let index = store
        .profiles
        .iter()
        .position(|p| p.name == store.active)
        .unwrap_or(0);
    let active = store.profiles.get(index).cloned().unwrap_or(Profile {
        name: DEFAULT_PROFILE_NAME.to_string(),
        settings: Settings::default(),
    });

    *crate::state::SETTINGS.lock().unwrap() = active.settings;
    *crate::state::ACTIVE_PROFILE.lock().unwrap() = active.name;
    *crate::state::PROFILES.lock().unwrap() = store.profiles;
}

/// Persists the current in-memory settings into the active profile's slot, then writes every
/// profile to disk.
pub fn save() {
    let active_name = crate::state::ACTIVE_PROFILE.lock().unwrap().clone();
    let current_settings = crate::state::SETTINGS.lock().unwrap().clone();

    let store = {
        let mut profiles = crate::state::PROFILES.lock().unwrap();
        if let Some(p) = profiles.iter_mut().find(|p| p.name == active_name) {
            p.settings = current_settings;
        }
        ProfileStore {
            active: active_name,
            profiles: profiles.clone(),
        }
    };

    write_store(&store);
}

fn write_store(store: &ProfileStore) {
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
    match serde_json::to_string_pretty(store) {
        Ok(json) => {
            if let Err(err) = fs::write(&path, json) {
                log::warn!("failed to write settings file: {err}");
            }
        }
        Err(err) => log::warn!("failed to serialize settings: {err}"),
    }
}

/// Names of all saved profiles, in stored order.
pub fn profile_names() -> Vec<String> {
    crate::state::PROFILES
        .lock()
        .unwrap()
        .iter()
        .map(|p| p.name.clone())
        .collect()
}

pub fn active_profile_name() -> String {
    crate::state::ACTIVE_PROFILE.lock().unwrap().clone()
}

/// Switches to a different existing profile, applying its settings - including reapplying
/// anything with a side effect beyond just storage, like the overlay font size. No-op if the
/// name doesn't match a saved profile.
pub fn switch_profile(name: &str) {
    save(); // persist any pending edits into the profile being left

    let new_settings = {
        let profiles = crate::state::PROFILES.lock().unwrap();
        let Some(profile) = profiles.iter().find(|p| p.name == name) else {
            return;
        };
        profile.settings.clone()
    };

    crate::overlay_font::resize(new_settings.overlay_font_size);
    *crate::state::SETTINGS.lock().unwrap() = new_settings;
    *crate::state::ACTIVE_PROFILE.lock().unwrap() = name.to_string();
    save();
}

/// Creates a new profile as a copy of the currently active one, and switches to it. No-op (with
/// an alert) if the name is blank or already taken.
pub fn create_profile(name: &str) {
    let name = name.trim();
    if name.is_empty() {
        return;
    }
    if profile_names().iter().any(|existing| existing == name) {
        nexus::alert::send_alert("PullSync: a profile with that name already exists.");
        return;
    }

    save(); // persist current edits into the profile being duplicated from
    let current_settings = crate::state::SETTINGS.lock().unwrap().clone();
    crate::state::PROFILES.lock().unwrap().push(Profile {
        name: name.to_string(),
        settings: current_settings,
    });
    *crate::state::ACTIVE_PROFILE.lock().unwrap() = name.to_string();
    save();
}

/// Renames the currently active profile. No-op (with an alert) if the new name is blank or
/// already taken by another profile.
pub fn rename_active_profile(new_name: &str) {
    let new_name = new_name.trim();
    let old_name = active_profile_name();
    if new_name.is_empty() || new_name == old_name {
        return;
    }
    if profile_names().iter().any(|existing| existing == new_name) {
        nexus::alert::send_alert("PullSync: a profile with that name already exists.");
        return;
    }

    if let Some(p) = crate::state::PROFILES
        .lock()
        .unwrap()
        .iter_mut()
        .find(|p| p.name == old_name)
    {
        p.name = new_name.to_string();
    }
    *crate::state::ACTIVE_PROFILE.lock().unwrap() = new_name.to_string();
    save();
}

/// Deletes the currently active profile and switches to whichever remains first. Refuses (with
/// an alert) to delete the last remaining profile - there must always be at least one.
pub fn delete_active_profile() {
    let active_name = active_profile_name();
    let fallback = {
        let mut profiles = crate::state::PROFILES.lock().unwrap();
        if profiles.len() <= 1 {
            None
        } else {
            profiles.retain(|p| p.name != active_name);
            profiles.first().cloned()
        }
    };

    let Some(fallback) = fallback else {
        nexus::alert::send_alert("PullSync: can't delete the only profile.");
        return;
    };

    crate::overlay_font::resize(fallback.settings.overlay_font_size);
    *crate::state::SETTINGS.lock().unwrap() = fallback.settings;
    *crate::state::ACTIVE_PROFILE.lock().unwrap() = fallback.name;
    save();
}
