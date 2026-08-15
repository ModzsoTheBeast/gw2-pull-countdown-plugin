use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT, SND_PURGE};
use windows::core::PCWSTR;

/// Five one-second beats, so it only fits countdowns of at least this long and has to start
/// exactly when this many seconds remain - see `countdown::COUNTDOWN_SOUND_SECS`.
const COUNTDOWN_SOUND: &[u8] = include_bytes!("../assets/countdown.wav");
const PULL_SOUND: &[u8] = include_bytes!("../assets/pull.wav");

/// Plays the last-five-seconds countdown sound, if enabled in settings.
pub fn play_countdown_if_enabled() {
    if crate::state::SETTINGS.lock().unwrap().sound_countdown_enabled {
        play(COUNTDOWN_SOUND);
    }
}

/// Plays the "PULL!" alert sound, if enabled in settings.
pub fn play_pull_if_enabled() {
    if crate::state::SETTINGS.lock().unwrap().sound_pull_enabled {
        play(PULL_SOUND);
    }
}

/// Stops whatever is currently playing - used when a pull is cancelled, so the countdown sound
/// doesn't keep beeping down to a pull that is no longer happening.
pub fn stop() {
    unsafe {
        let _ = PlaySoundW(PCWSTR::null(), None, SND_PURGE);
    }
}

/// `SND_ASYNC` makes this fire-and-forget, so it never blocks the render thread it's called
/// from. Note `PlaySoundW` only keeps one sound going at a time: a later call replaces an
/// earlier one rather than mixing. That's exactly what's wanted here - the countdown sound runs
/// out precisely as the pull sound starts, and a cancel should cut the countdown short.
fn play(wav: &'static [u8]) {
    // Safety: `SND_MEMORY` means `pszSound` is a pointer to an in-memory WAV image rather than
    // a filename. The data is `'static` (embedded in the binary), so it outlives the call, and
    // `SND_ASYNC` playback reads from it afterwards.
    unsafe {
        let _ = PlaySoundW(
            PCWSTR(wav.as_ptr() as *const u16),
            None,
            SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
        );
    }
}
