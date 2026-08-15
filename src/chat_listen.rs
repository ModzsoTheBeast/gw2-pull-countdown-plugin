use crate::chat_send::CHAT_MARKER;
use nexus::event::event_consume;
use nexus::event::extras::{CHAT_MESSAGE, ChannelType, SquadMessage};

/// Subscribes to the Unofficial Extras chat bridge, if that addon is installed alongside
/// arcdps. Only fires on clients that also have arcdps + Unofficial Extras loaded - clients
/// without it simply never get this event, and just see the plain chat text instead.
pub fn subscribe() {
    CHAT_MESSAGE
        .subscribe(event_consume!(<SquadMessage> |msg| {
            if let Some(msg) = msg {
                on_squad_message(msg);
            }
        }))
        .revert_on_unload();
}

fn on_squad_message(msg: &SquadMessage) {
    if !matches!(msg.channel_type, ChannelType::Party | ChannelType::Squad) {
        return;
    }

    // Only act on messages this addon generated. Anything a human typed by hand is ignored,
    // however much it looks like a countdown.
    let Some((_, body)) = msg.text().split_once(CHAT_MARKER) else {
        return;
    };

    if !sender_may_control(msg) {
        return;
    }

    if crate::countdown::is_active() {
        // A countdown is already running, so a repeated start is just the sender's own message
        // echoing back (or a duplicate) - ignore it rather than restarting everyone. A cancel
        // still applies.
        if is_cancel(body) {
            crate::countdown::cancel();
            crate::sound::stop();
        }
        return;
    }

    if let Some(total) = parse_count(body) {
        crate::countdown::start(total);
    }
}

/// Whether the sender is allowed to drive everyone else's countdown.
///
/// In a squad that means a commander or lieutenant - otherwise any member could start or cancel
/// pulls for the whole squad. In a plain party there is no leader to check against (and Extras
/// reports no squad roster), so anyone in the party may.
///
/// Extras reports messages sent to party chat *while in a squad* as [`ChannelType::Squad`], so
/// this genuinely distinguishes "in a party" from "in a squad".
fn sender_may_control(msg: &SquadMessage) -> bool {
    match msg.channel_type {
        ChannelType::Party => true,
        _ => crate::extras_squad::may_control_pull(msg.account_name()),
    }
}

/// Whether an addon-generated message is a cancellation. Only reached for messages already
/// carrying the marker and from an authorised sender, so matching on the word is safe here -
/// see `settings::Settings::chat_cancel_text`, which is expected to contain "cancel".
fn is_cancel(body: &str) -> bool {
    body.to_ascii_lowercase().contains("cancel")
}

/// Pulls the countdown's starting number out of an addon-generated message.
///
/// The wording around it is user-configurable, so this only looks for the first run of digits -
/// which is what `{n}` expands to. Messages with no number (the final "Pull!" line, or a
/// cancellation) yield `None`.
fn parse_count(body: &str) -> Option<u32> {
    let mut digits = String::new();
    for ch in body.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else if !digits.is_empty() {
            break;
        }
    }
    digits.parse().ok()
}
