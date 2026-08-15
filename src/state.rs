use crate::countdown::CountdownState;
use crate::settings::{Profile, Settings};
use nexus::rtapi::GroupMemberOwned;
use std::sync::{Mutex, OnceLock};

/// The active profile's settings - what everything else in the addon reads/writes. Kept as its
/// own static (rather than looked up through `PROFILES` each time) since it's read constantly
/// from the render loop; `settings::save()` writes it back into the active profile's slot.
pub static SETTINGS: Mutex<Settings> = Mutex::new(Settings::const_default());

/// All saved profiles, in stored order.
pub static PROFILES: Mutex<Vec<Profile>> = Mutex::new(Vec::new());

/// Name of the profile currently loaded into `SETTINGS`.
pub static ACTIVE_PROFILE: Mutex<String> = Mutex::new(String::new());

pub static SQUAD: Mutex<Vec<GroupMemberOwned>> = Mutex::new(Vec::new());

pub static COUNTDOWN: Mutex<Option<CountdownState>> = Mutex::new(None);

/// Raw HWND (as `isize`) of the game window, found once via `chat_send::find_game_hwnd` and
/// cached here (a raw pointer isn't `Send`/`Sync`, so it can't be stored as `HWND` directly).
pub static GAME_HWND: OnceLock<isize> = OnceLock::new();

pub fn update_squad_member(member: GroupMemberOwned) {
    let mut squad = SQUAD.lock().unwrap();
    if let Some(existing) = squad
        .iter_mut()
        .find(|m| m.account_name == member.account_name)
    {
        *existing = member;
    } else {
        squad.push(member);
    }
}

pub fn remove_squad_member(account_name: &str) {
    SQUAD.lock().unwrap().retain(|m| m.account_name != account_name);
}
