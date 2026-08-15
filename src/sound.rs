use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};
use windows::core::PCWSTR;

const PULL_SOUND: &[u8] = include_bytes!("../assets/pull.wav");

/// Plays the "PULL!" alert sound, if enabled in settings.
///
/// `SND_ASYNC` makes this fire-and-forget - it never blocks the caller (safe to call directly
/// from the render thread), and `PlaySoundW` itself replaces any sound already playing rather
/// than queuing, so repeated triggers can't pile up.
pub fn play_pull_sound_if_enabled() {
    if !crate::state::SETTINGS.lock().unwrap().sound_enabled {
        return;
    }

    // Safety: `SND_MEMORY` tells `PlaySoundW` to treat `pszSound` as a pointer to an in-memory
    // WAV image (not a filename) of the given, `'static` embedded bytes - always valid for the
    // call's duration.
    unsafe {
        let _ = PlaySoundW(
            PCWSTR(PULL_SOUND.as_ptr() as *const u16),
            None,
            SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
        );
    }
}
