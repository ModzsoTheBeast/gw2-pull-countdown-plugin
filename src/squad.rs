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

/// Whether the local player is allowed to trigger a pull: fails open if RTAPI can't confirm
/// who the commander is, otherwise requires actually being the commander.
pub fn am_i_allowed_to_pull() -> bool {
    !rtapi_available() || am_i_commander()
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
/// this must not require it. Without RTAPI, default to "squad" (the primary use case: a
/// commander leading a squad) rather than silently sending nothing; if the player isn't
/// actually grouped, GW2 itself just shows a local "not in a squad" message and nothing is sent.
pub fn current_channel() -> ChatChannel {
    RealTimeApi::get()
        .and_then(|rtapi| rtapi.read_group())
        .and_then(|group| match group.group_type {
            Ok(GroupType::Party) => Some(ChatChannel::Party),
            Ok(GroupType::Squad | GroupType::RaidSquad) => Some(ChatChannel::Squad),
            _ => None,
        })
        .unwrap_or(ChatChannel::Squad)
}
