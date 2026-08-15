use nexus::event::event_consume;
use nexus::event::extras::{EXTRAS_SQUAD_UPDATE, SquadUpdate, UserRole};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// Account name (without the leading ':') -> role in the current squad.
///
/// Sourced from Unofficial Extras rather than RTAPI deliberately: Extras is already required
/// for the chat listener to work at all, and its account names come from the same helper as
/// `SquadMessage::account_name`, so the two match without any normalisation guesswork.
static ROSTER: LazyLock<Mutex<HashMap<String, UserRole>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn subscribe() {
    EXTRAS_SQUAD_UPDATE
        .subscribe(event_consume!(<SquadUpdate> |update| {
            if let Some(update) = update {
                on_squad_update(update);
            }
        }))
        .revert_on_unload();
}

fn on_squad_update(update: &SquadUpdate) {
    let mut roster = ROSTER.lock().unwrap();
    for user in update.iter() {
        let Some(name) = user.account_name() else {
            continue;
        };
        match user.role {
            // Extras reports `None` for someone who left the squad.
            UserRole::None | UserRole::Invalid => {
                roster.remove(&normalize(name));
            }
            role => {
                roster.insert(normalize(name), role);
            }
        }
    }
}

/// Whether the given account is allowed to start or cancel a pull for everyone else.
///
/// Only squad leaders and lieutenants qualify. Someone we have no role for is rejected: in a
/// squad Extras always pushes the roster, so an unknown sender means either a tracking gap or
/// someone who isn't really in the squad, and neither should be able to drive the countdown.
pub fn may_control_pull(account_name: &str) -> bool {
    let roster = ROSTER.lock().unwrap();
    match roster.get(&normalize(account_name)) {
        Some(UserRole::SquadLeader | UserRole::Lieutenant) => true,
        Some(_) => false,
        None => {
            log::info!("ignoring pull message from unknown-role account \"{account_name}\"");
            false
        }
    }
}

/// Account names come from the same Extras helper on both sides, so this is belt-and-braces:
/// it just guards against stray whitespace or case differences silently breaking the match.
fn normalize(account_name: &str) -> String {
    account_name.trim().trim_start_matches(':').to_ascii_lowercase()
}

/// Number of squad members currently tracked.
///
/// Extras only ever populates this roster for real squads - the in-game squad panel it reads
/// doesn't exist for a plain party - so a non-zero count is also usable as a "we're in a squad,
/// not just a party" signal when RTAPI can't confirm that directly (see `squad::current_channel`
/// and `ready_check`, both of which need a squad-vs-party or member-count answer without
/// requiring the separate RTAPI addon).
pub fn roster_len() -> usize {
    ROSTER.lock().unwrap().len()
}
