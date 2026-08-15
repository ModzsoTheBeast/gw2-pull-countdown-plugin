use crate::squad::{self, ChatChannel};
use nexus::data_link::get_mumble_link;
use nexus::data_link::mumble::UiState;
use nexus::gamebind::{GameBind, is_gamebind_bound, press_gamebind, release_gamebind};
use nexus::wnd_proc::send_wnd_proc_to_game;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_RETURN;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowThreadProcessId, IsWindowVisible, WM_CHAR, WM_KEYDOWN, WM_KEYUP,
};
use windows::core::BOOL;

/// How long to hold the chat-open gamebind before releasing it.
const CHAT_OPEN_HOLD: Duration = Duration::from_millis(60);
/// How long to wait for the chat textbox to actually gain focus before giving up.
const CHAT_FOCUS_TIMEOUT: Duration = Duration::from_millis(300);
const CHAT_FOCUS_POLL: Duration = Duration::from_millis(5);
/// Fallback wait if MumbleLink is unavailable to poll textbox focus directly.
const CHAT_FOCUS_FALLBACK_SLEEP: Duration = Duration::from_millis(180);
/// Settle time between typing the message and submitting it.
const SUBMIT_SETTLE: Duration = Duration::from_millis(30);

/// Serializes every chat send: cancelling a pull while the chat countdown's tail-tick thread is
/// mid-send used to race with it (both threads opening/typing into the chat box at once), which
/// could garble the message or make `textbox_already_focused()` false-positive and silently drop
/// the cancel line entirely. Every `send_chat_line` call now waits its turn instead.
static CHAT_SEND_LOCK: Mutex<()> = Mutex::new(());

/// `HWND` wraps a raw, non-`Send` pointer, so we cache the discovered handle as a plain `isize`
/// and only reconstruct the `HWND` on the thread that actually uses it.
///
/// We deliberately do NOT use `nexus::wnd_proc::register_wnd_proc` to observe the HWND: Nexus's
/// wnd_proc chain treats a `0` return from any registered callback as "message fully handled,
/// stop the chain" (never forwarding the message to the game's own window procedure at all) -
/// an earlier version of this addon returned `0` unconditionally from its capture callback,
/// which silently ate every mouse/keyboard message for the whole game and froze all input.
/// Finding the window via its owning process instead never touches the message pipeline.
fn game_hwnd_raw() -> Option<isize> {
    if let Some(&cached) = crate::state::GAME_HWND.get() {
        return Some(cached);
    }
    let found = find_game_hwnd()?;
    let _ = crate::state::GAME_HWND.set(found);
    Some(found)
}

struct FindWindowState {
    target_pid: u32,
    found: Option<isize>,
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let state = unsafe { &mut *(lparam.0 as *mut FindWindowState) };

    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };

    if pid == state.target_pid && unsafe { IsWindowVisible(hwnd) }.as_bool() {
        state.found = Some(hwnd.0 as isize);
        return BOOL(0); // stop enumeration
    }
    BOOL(1) // continue
}

/// Finds the game's window by matching MumbleLink's officially-reported process ID against
/// top-level window ownership - no hooking or message interception involved.
fn find_game_hwnd() -> Option<isize> {
    let target_pid = nexus::data_link::read_mumble_link()?.context.process_id;

    let mut state = FindWindowState {
        target_pid,
        found: None,
    };
    let lparam = LPARAM(&mut state as *mut FindWindowState as isize);
    unsafe {
        let _ = EnumWindows(Some(enum_windows_callback), lparam);
    }
    state.found
}

/// Handles an explicit Pull trigger (quick access icon/keybind): toggles between starting a
/// pull and cancelling one already in progress. Shows the "only the commander" alert if
/// rejected - see `on_ready_check_succeeded` for the silent variant.
pub fn on_pull_pressed(total: u32) {
    if !squad::am_i_allowed_to_pull() {
        nexus::alert::send_alert("PullSync: only the squad commander can control the pull.");
        return;
    }
    if crate::countdown::is_active() {
        cancel_pull();
    } else {
        start_pull(total);
    }
}

/// Handles an automatic Pull trigger (after a successful ready check). Every squad member's
/// client sees the same ready-check event, so rejection here must be silent - showing "only the
/// commander can pull" to everyone whenever a ready check succeeds would be pure noise. Never
/// cancels - `ready_check` only calls this once per successful check.
pub fn on_ready_check_succeeded(total: u32) {
    if !squad::am_i_allowed_to_pull() {
        return;
    }
    start_pull(total);
}

/// Starts the local countdown immediately, then - on a background thread, so the synthetic-
/// input wait/poll steps never hitch the render loop - sends chat message(s) announcing it.
fn start_pull(total: u32) {
    crate::countdown::start(total);

    let Some(target) = resolve_send_target() else {
        return;
    };

    thread::spawn(move || run_chat_sequence(target, total));
}

/// Cancels the local countdown immediately, then - same background-thread reasoning as
/// `start_pull` - sends a chat message announcing the cancellation.
fn cancel_pull() {
    crate::countdown::cancel();
    // Otherwise the 5s countdown sound keeps beeping toward a pull that isn't happening.
    crate::sound::stop();

    let Some((hwnd_raw, channel)) = resolve_send_target() else {
        return;
    };

    thread::spawn(move || {
        let (text, use_broadcast) = {
            let s = crate::state::SETTINGS.lock().unwrap();
            (s.chat_cancel_text.clone(), s.use_squad_broadcast)
        };
        send_chat_line(hwnd_raw, channel, &text, use_broadcast);
    });
}

/// Resolves the game window handle and current chat channel shared by both `start_pull` and
/// `cancel_pull`, alerting once if the window can't be found.
fn resolve_send_target() -> Option<(isize, ChatChannel)> {
    let Some(hwnd_raw) = game_hwnd_raw() else {
        log::warn!("game window handle not yet captured, not sending a chat message");
        nexus::alert::send_alert("PullSync: couldn't find the game window, chat message not sent.");
        return None;
    };
    Some((hwnd_raw, squad::current_channel()))
}

/// Substitutes `{n}` in a template with the given count.
fn render_message(template: &str, n: u32) -> String {
    template.replace("{n}", &n.to_string())
}

fn run_chat_sequence(target: (isize, ChatChannel), total: u32) {
    let (hwnd_raw, channel) = target;
    let (template, pull_text, countdown_enabled, countdown_start, use_broadcast) = {
        let s = crate::state::SETTINGS.lock().unwrap();
        (
            s.chat_message_template.clone(),
            s.chat_pull_text.clone(),
            s.chat_countdown_enabled,
            s.chat_countdown_start,
            s.use_squad_broadcast,
        )
    };

    if !countdown_enabled {
        let message = render_message(&template, total);
        if !send_chat_line(hwnd_raw, channel, &message, use_broadcast) {
            nexus::alert::send_alert("PullSync: couldn't send the chat message (see Nexus's log for why).");
        }
        return;
    }

    let start_from = countdown_start.min(total);

    // Upfront heads-up with the full count, same as the non-countdown mode - only skipped if
    // the chat countdown covers the whole thing anyway (start_from == total), so it isn't sent
    // twice back to back.
    if start_from < total {
        let message = render_message(&template, total);
        if !send_chat_line(hwnd_raw, channel, &message, use_broadcast) {
            nexus::alert::send_alert("PullSync: couldn't send the chat message (see Nexus's log for why).");
        }
    }

    let Some(start_instant) = crate::countdown::start_instant() else {
        return; // countdown already ended somehow before this thread got going
    };

    for n in (0..=start_from).rev() {
        let target = start_instant + Duration::from_secs((total - n) as u64);
        let now = Instant::now();
        if target > now {
            thread::sleep(target - now);
        }

        // Bail out early if the countdown was cancelled mid-sequence - no point still typing
        // tail ticks for a pull that no longer exists.
        if !crate::countdown::is_active() {
            return;
        }

        let message = if n == 0 {
            pull_text.clone()
        } else {
            render_message(&template, n)
        };

        // Squad broadcast stays on screen for several seconds and queues up (falling out of
        // sync with the overlay) if sent faster than that - so only the final "pull now" tick
        // ever broadcasts here. The *first* tick also broadcasts, but only when it's standing in
        // for the upfront message (i.e. that message was skipped above because the chat
        // countdown covers the whole range) - every tick in between always uses normal chat.
        let is_first_tick = start_from >= total && n == start_from;
        let tick_use_broadcast = use_broadcast && (n == 0 || is_first_tick);

        // Best-effort per tick - a single skipped/failed line (e.g. the player's own chat box
        // was open at that instant) just means that one line doesn't show up; not worth an
        // alert for every tick of a several-second countdown.
        send_chat_line(hwnd_raw, channel, &message, tick_use_broadcast);
    }
}

/// Types one line into chat (or squad broadcast) and submits it. Returns whether it was
/// actually sent. `message` is the raw content, without any channel-command prefix - that's
/// added here (skipped entirely for broadcast, which isn't a `/`-routed channel).
fn send_chat_line(hwnd_raw: isize, channel: ChatChannel, message: &str, use_broadcast: bool) -> bool {
    let _guard = CHAT_SEND_LOCK.lock().unwrap();

    if textbox_already_focused() {
        log::info!("a textbox was already focused, not auto-typing: {message}");
        return false;
    }

    // Squad broadcast only exists for squads, not parties - fall back to normal chat.
    let broadcast = use_broadcast && matches!(channel, ChatChannel::Squad);
    let (gamebind, text) = if broadcast {
        (GameBind::UiSquadBroadcastChatFocus, message.to_string())
    } else {
        (GameBind::UiChatCommand, format!("{} {message}", channel.command_word()))
    };

    if !is_gamebind_bound(gamebind) {
        log::warn!("the {gamebind:?} keybind is unbound, cannot auto-open chat");
        return false;
    }

    let hwnd = HWND(hwnd_raw as *mut _);

    press_gamebind(gamebind);
    thread::sleep(CHAT_OPEN_HOLD);
    release_gamebind(gamebind);

    if !wait_for_textbox_focus() {
        log::warn!("chat box did not focus in time, aborting message send");
        return false;
    }

    // UiChatCommand opens the box with "/" already typed (so `text` skips it); broadcast has
    // no command prefix to begin with.
    for ch in text.chars() {
        let _ = send_wnd_proc_to_game(hwnd, WM_CHAR, WPARAM(ch as usize), LPARAM(0));
    }

    thread::sleep(SUBMIT_SETTLE);
    submit_message(hwnd);
    log::info!("sent {}: \"{text}\"", if broadcast { "broadcast" } else { "chat line" });
    true
}

fn textbox_already_focused() -> bool {
    get_mumble_link()
        .map(|link| link.read_ui_state().contains(UiState::TEXTBOX_HAS_FOCUS))
        .unwrap_or(false)
}

fn wait_for_textbox_focus() -> bool {
    let Some(link) = get_mumble_link() else {
        thread::sleep(CHAT_FOCUS_FALLBACK_SLEEP);
        return true;
    };

    let deadline = Instant::now() + CHAT_FOCUS_TIMEOUT;
    while Instant::now() < deadline {
        if link.read_ui_state().contains(UiState::TEXTBOX_HAS_FOCUS) {
            return true;
        }
        thread::sleep(CHAT_FOCUS_POLL);
    }
    false
}

fn submit_message(hwnd: HWND) {
    const SCANCODE: isize = 0x1C;
    let down_lparam = LPARAM(1 | (SCANCODE << 16));
    let up_lparam = LPARAM(down_lparam.0 | (1 << 30) | (1 << 31));
    let _ = send_wnd_proc_to_game(hwnd, WM_KEYDOWN, WPARAM(VK_RETURN.0 as usize), down_lparam);
    let _ = send_wnd_proc_to_game(hwnd, WM_KEYUP, WPARAM(VK_RETURN.0 as usize), up_lparam);
}
