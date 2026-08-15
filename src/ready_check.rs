use nexus::event::event_consume;
use nexus::event::extras::{EXTRAS_SQUAD_UPDATE, SquadUpdate, UserRole};
use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};

struct ReadyCheckState {
    ready_accounts: HashSet<String>,
    triggered: bool,
}

static STATE: LazyLock<Mutex<ReadyCheckState>> = LazyLock::new(|| {
    Mutex::new(ReadyCheckState {
        ready_accounts: HashSet::new(),
        triggered: false,
    })
});

/// Subscribes to the Unofficial Extras squad update bridge to detect ready checks succeeding
/// (same requirement as chat mirroring: only fires if the local client also has arcdps +
/// Unofficial Extras installed). Gated on `Settings::auto_pull_after_ready_check` at the top of
/// the handler rather than at subscribe time, so toggling the setting takes effect immediately.
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
    if !crate::state::SETTINGS.lock().unwrap().auto_pull_after_ready_check {
        return;
    }

    let mut state = STATE.lock().unwrap();

    for user in update.iter() {
        if user.role == UserRole::SquadLeader && user.ready_status {
            // Per Unofficial Extras' docs, the leader's own ready_status flipping to true
            // signals that a ready check was just started - reset tracking for the new round.
            // This does NOT skip counting the leader as ready (a bug in an earlier version):
            // the leader is a squad member too and counts toward "everyone ready", so without
            // also falling through to the insert below, the tally could never reach the full
            // member count and a pull would never auto-trigger.
            state.ready_accounts.clear();
            state.triggered = false;
        }
        if user.ready_status {
            if let Some(name) = user.account_name() {
                state.ready_accounts.insert(name.to_string());
            }
        }
    }

    if state.triggered {
        return;
    }

    // Can't confirm "everyone" without a member count - don't guess. Prefer RTAPI's count, but
    // this whole feature already requires Unofficial Extras to detect the ready check at all
    // (this handler only runs because of its EXTRAS_SQUAD_UPDATE event), so fall back to the
    // size of the same roster `extras_squad` builds from that same event stream rather than
    // additionally requiring the separate RTAPI addon just for a headcount.
    let Some(total_members) = crate::squad::group_member_count().or_else(|| {
        let roster_len = crate::extras_squad::roster_len();
        (roster_len > 0).then_some(roster_len as u32)
    }) else {
        return;
    };

    if total_members > 0 && state.ready_accounts.len() as u32 >= total_members {
        state.triggered = true;
        drop(state);
        let start_count = crate::state::SETTINGS.lock().unwrap().start_count;
        crate::chat_send::on_ready_check_succeeded(start_count);
    }
}
