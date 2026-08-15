use crate::state::COUNTDOWN;
use std::time::{Duration, Instant};

/// How long the "PULL!" banner lingers after hitting zero before disappearing.
const LINGER: Duration = Duration::from_secs(2);

/// Length of the bundled countdown sound, in seconds. It's a fixed five one-second beats, so it
/// has to start exactly this far from zero to line up with the overlay's digits, and countdowns
/// shorter than this can't use it at all.
pub const COUNTDOWN_SOUND_SECS: u32 = 5;

#[derive(Debug, Clone, Copy)]
pub struct CountdownState {
    pub start: Instant,
    pub total: u32,
    /// Whether `snapshot()` has already reported hitting zero once for this countdown - lets it
    /// tell the caller the *moment* it first reaches "PULL!", instead of every frame during the
    /// linger period (used to fire the alert sound exactly once).
    pull_reported: bool,
    /// Same idea for the countdown sound's one-shot start trigger.
    countdown_sound_started: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct CountdownSnapshot {
    pub remaining: u32,
    pub is_pull: bool,
    /// True only on the single `snapshot()` call where `is_pull` first becomes true.
    pub just_reached_pull: bool,
    /// True only on the single `snapshot()` call where the countdown first enters its last
    /// `COUNTDOWN_SOUND_SECS` seconds - and only for countdowns long enough to fit the sound.
    pub just_entered_countdown_sound_window: bool,
}

/// Starts a new countdown from `total` seconds, overwriting any countdown already running.
pub fn start(total: u32) {
    *COUNTDOWN.lock().unwrap() = Some(CountdownState {
        start: Instant::now(),
        total,
        pull_reported: false,
        countdown_sound_started: false,
    });
}

pub fn is_active() -> bool {
    COUNTDOWN.lock().unwrap().is_some()
}

/// Stops the current countdown immediately (no linger - it just disappears).
pub fn cancel() {
    *COUNTDOWN.lock().unwrap() = None;
}

/// The `Instant` the currently running countdown started, if any - used to schedule the
/// per-second chat countdown in sync with the overlay.
pub fn start_instant() -> Option<Instant> {
    COUNTDOWN.lock().unwrap().map(|s| s.start)
}

/// Computes the current countdown state, clearing it once the linger period has elapsed.
pub fn snapshot() -> Option<CountdownSnapshot> {
    let mut guard = COUNTDOWN.lock().unwrap();
    let state = (*guard)?;
    let elapsed = state.start.elapsed();
    let total_duration = Duration::from_secs(state.total as u64);

    if elapsed >= total_duration + LINGER {
        *guard = None;
        return None;
    }

    let remaining = if elapsed < total_duration {
        (total_duration - elapsed).as_secs_f32().ceil() as u32
    } else {
        0
    };
    let is_pull = remaining == 0;

    let just_reached_pull = is_pull && !state.pull_reported;
    if just_reached_pull {
        if let Some(s) = guard.as_mut() {
            s.pull_reported = true;
        }
    }

    // Fire once, when there's exactly COUNTDOWN_SOUND_SECS left, so the sound's beats land on
    // the overlay's digits. Requires a countdown at least that long, and deliberately doesn't
    // fire if we've somehow already passed zero (e.g. the game was frozen through the window) -
    // starting a 5s countdown sound at "PULL!" would just talk over the pull.
    let secs_left = total_duration.saturating_sub(elapsed).as_secs_f32();
    let just_entered_countdown_sound_window = !state.countdown_sound_started
        && state.total >= COUNTDOWN_SOUND_SECS
        && secs_left > 0.0
        && secs_left <= COUNTDOWN_SOUND_SECS as f32;
    if just_entered_countdown_sound_window {
        if let Some(s) = guard.as_mut() {
            s.countdown_sound_started = true;
        }
    }

    Some(CountdownSnapshot {
        remaining,
        is_pull,
        just_reached_pull,
        just_entered_countdown_sound_window,
    })
}
