use crate::state::{self, SQUAD};
use nexus::event::event_consume;
use nexus::event::rtapi::{RTAPI_GROUP_MEMBER_JOINED, RTAPI_GROUP_MEMBER_LEFT, RTAPI_GROUP_MEMBER_UPDATE};
use nexus::rtapi::{GroupMember, GroupType, RealTimeApi};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatChannel {
    Party,
    Squad,
}

impl ChatChannel {
    pub fn command_word(self) -> &'static str {
        match self {
            ChatChannel::Party => "party",
            ChatChannel::Squad => "squad",
        }
    }
}

/// Subscribes to RealTime API group membership events to keep `state::SQUAD` up to date.
///
/// There is no synchronous "give me all current members" call in RTAPI - membership is only
/// known via these push events, so we accumulate our own roster from them.
pub fn subscribe_all() {
    RTAPI_GROUP_MEMBER_JOINED
        .subscribe(event_consume!(<GroupMember> |member| {
            if let Some(member) = member {
                state::update_squad_member(member.to_owned());
            }
        }))
        .revert_on_unload();

    RTAPI_GROUP_MEMBER_UPDATE
        .subscribe(event_consume!(<GroupMember> |member| {
            if let Some(member) = member {
                state::update_squad_member(member.to_owned());
            }
        }))
        .revert_on_unload();

    RTAPI_GROUP_MEMBER_LEFT
        .subscribe(event_consume!(<GroupMember> |member| {
            if let Some(member) = member {
                state::remove_squad_member(&member.account_name());
            }
        }))
        .revert_on_unload();
}

/// Whether the RTAPI link is active - i.e. whether the player has RTAPI installed as its own
/// separate Nexus addon (https://github.com/RaidcoreGG/GW2-RealTime-API-Releases). It's not a
/// GW2 setting, despite what an earlier version of this addon's UI incorrectly claimed.
pub fn rtapi_available() -> bool {
    RealTimeApi::get().is_some_and(|api| api.is_active())
}

/// Whether the local player is the commander of the current squad, based on the roster
/// accumulated from RTAPI group events.
pub fn am_i_commander() -> bool {
    SQUAD
        .lock()
        .unwrap()
        .iter()
        .any(|m| m.is_self && m.is_commander)
}

/// Whether the local player is a squad lieutenant, based on the same RTAPI-accumulated roster.
pub fn am_i_lieutenant() -> bool {
    SQUAD
        .lock()
        .unwrap()
        .iter()
        .any(|m| m.is_self && m.is_lieutenant)
}

/// Whether the local player is allowed to trigger a pull: fails open if RTAPI can't confirm
/// group state at all. Otherwise mirrors the same rule `chat_listen::sender_may_control` applies
/// to received messages - a party has no leader to check against, so anyone in it may; a squad
/// requires being the commander or a lieutenant.
pub fn am_i_allowed_to_pull() -> bool {
    if !rtapi_available() {
        return true;
    }
    match current_channel() {
        ChatChannel::Party => true,
        ChatChannel::Squad => am_i_commander() || am_i_lieutenant(),
    }
}

/// Number of members in the current group, if RTAPI can report it.
pub fn group_member_count() -> Option<u32> {
    RealTimeApi::get()
        .and_then(|rtapi| rtapi.read_group())
        .map(|group| group.group_member_count)
}

/// The chat channel to use for the pull message.
///
/// RTAPI can tell us for certain whether it's a party or a squad, but plenty of players won't
/// have the separate RTAPI addon installed - being in a group at all is independent of that, so
/// this must not require it. Without RTAPI, guessing "squad" unconditionally is actively wrong
/// for a plain party: GW2 silently drops a `/squad` command typed while not in a squad (a local
/// "you are not in a squad" message only the sender sees), so the pull would start locally but
/// never reach anyone else. Instead fall back to Unofficial Extras' squad roster
/// (`extras_squad`) - it's only ever populated for real squads (its squad panel doesn't exist
/// for parties), so a non-empty roster means squad, empty means party. If truly ungrouped, the
/// guess doesn't matter either way - GW2 rejects both commands locally with nothing sent.
pub fn current_channel() -> ChatChannel {
    RealTimeApi::get()
        .and_then(|rtapi| rtapi.read_group())
        .and_then(|group| match group.group_type {
            Ok(GroupType::Party) => Some(ChatChannel::Party),
            Ok(GroupType::Squad | GroupType::RaidSquad) => Some(ChatChannel::Squad),
            _ => None,
        })
        .unwrap_or_else(|| {
            if crate::extras_squad::roster_len() > 0 {
                ChatChannel::Squad
            } else {
                ChatChannel::Party
            }
        })
}
