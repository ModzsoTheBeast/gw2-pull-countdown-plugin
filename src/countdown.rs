use crate::state::COUNTDOWN;
use std::time::{Duration, Instant};

/// How long the "PULL!" banner lingers after hitting zero before disappearing.
const LINGER: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy)]
pub struct CountdownState {
    pub start: Instant,
    pub total: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct CountdownSnapshot {
    pub remaining: u32,
    pub is_pull: bool,
}

/// Starts a new countdown from `total` seconds, overwriting any countdown already running.
pub fn start(total: u32) {
    *COUNTDOWN.lock().unwrap() = Some(CountdownState {
        start: Instant::now(),
        total,
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
    Some(CountdownSnapshot {
        remaining,
        is_pull: remaining == 0,
    })
}
