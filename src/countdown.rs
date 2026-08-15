use crate::state::COUNTDOWN;
use std::time::{Duration, Instant};

/// How long the "PULL!" banner lingers after hitting zero before disappearing.
const LINGER: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy)]
pub struct CountdownState {
    pub start: Instant,
    pub total: u32,
    /// Whether `snapshot()` has already reported hitting zero once for this countdown - lets it
    /// tell the caller the *moment* it first reaches "PULL!", instead of every frame during the
    /// linger period (used to fire the alert sound exactly once).
    pull_reported: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct CountdownSnapshot {
    pub remaining: u32,
    pub is_pull: bool,
    /// True only on the single `snapshot()` call where `is_pull` first becomes true.
    pub just_reached_pull: bool,
}

/// Starts a new countdown from `total` seconds, overwriting any countdown already running.
pub fn start(total: u32) {
    *COUNTDOWN.lock().unwrap() = Some(CountdownState {
        start: Instant::now(),
        total,
        pull_reported: false,
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

    Some(CountdownSnapshot {
        remaining,
        is_pull,
        just_reached_pull,
    })
}
