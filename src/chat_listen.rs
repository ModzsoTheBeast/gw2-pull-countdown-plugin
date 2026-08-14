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
    let text = msg.text();

    if crate::countdown::is_active() {
        // A countdown is already running - the only thing worth reacting to is a cancel (so
        // the commander's own message echoing back doesn't restart the countdown they already
        // started locally, but a cancel from any client with the addon still takes effect).
        if is_cancel(text) {
            crate::countdown::cancel();
        }
        return;
    }

    if let Some(total) = parse_pull(text) {
        crate::countdown::start(total);
    }
}

/// Whether a chat message looks like a pull cancellation (see
/// `settings::Settings::chat_cancel_text`, customizable but expected to contain "cancel").
fn is_cancel(text: &str) -> bool {
    text.to_ascii_lowercase().contains("cancel")
}

/// Parses a chat message as a pull countdown trigger.
///
/// The chat text is user-customizable (see `settings::Settings::chat_message_template`), so this
/// can't require an exact form. Instead it just requires the word "pull" to appear somewhere
/// (case-insensitive, so "Pulling in..." matches too) alongside a number - good enough to trigger
/// the receiving client's own countdown, which then runs independently to zero regardless of how
/// any later ticks are worded, since `on_squad_message` ignores further messages once a countdown
/// is already running.
fn parse_pull(text: &str) -> Option<u32> {
    if !text.to_ascii_lowercase().contains("pull") {
        return None;
    }

    let mut digits = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else if !digits.is_empty() {
            break;
        }
    }
    digits.parse().ok()
}
