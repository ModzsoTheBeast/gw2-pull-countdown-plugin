use std::sync::Mutex;
use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT, SND_PURGE};
use windows::core::PCWSTR;

/// Five one-second beats, so it only fits countdowns of at least this long and has to start
/// exactly when this many seconds remain - see `countdown::COUNTDOWN_SOUND_SECS`.
const COUNTDOWN_SOUND: &[u8] = include_bytes!("../assets/countdown.wav");
const PULL_SOUND: &[u8] = include_bytes!("../assets/pull.wav");

/// Holds a volume-scaled copy of whatever's currently playing, keeping it alive for as long as
/// `PlaySoundW` might still be reading from it. Only used away from unity gain - at exactly 1.0
/// the static embedded bytes are played directly and this stays `None`.
static CURRENT_BUFFER: Mutex<Option<Vec<u8>>> = Mutex::new(None);

/// Upper bound for the volume sliders. The bundled recordings peak quiet - `countdown.wav` only
/// reaches ~11% of full scale, `pull.wav` ~7.5% - so 1.0 (unity gain, i.e. play the file exactly
/// as recorded) wasn't loud enough on its own for every system/headset. 8x amplification is the
/// most either file can take before its own loudest peak starts clipping (`countdown.wav` is the
/// tighter of the two, clipping above ~8.86x).
pub const MAX_VOLUME: f32 = 8.0;

/// Plays the last-five-seconds countdown sound, if enabled in settings.
pub fn play_countdown_if_enabled() {
    let (enabled, volume) = {
        let s = crate::state::SETTINGS.lock().unwrap();
        (s.sound_countdown_enabled, s.sound_countdown_volume)
    };
    if enabled {
        play(COUNTDOWN_SOUND, volume);
    }
}

/// Plays the "PULL!" alert sound, if enabled in settings.
pub fn play_pull_if_enabled() {
    let (enabled, volume) = {
        let s = crate::state::SETTINGS.lock().unwrap();
        (s.sound_pull_enabled, s.sound_pull_volume)
    };
    if enabled {
        play(PULL_SOUND, volume);
    }
}

/// Stops whatever is currently playing - used when a pull is cancelled, so the countdown sound
/// doesn't keep beeping down to a pull that is no longer happening.
pub fn stop() {
    unsafe {
        let _ = PlaySoundW(PCWSTR::null(), None, SND_PURGE);
    }
    *CURRENT_BUFFER.lock().unwrap() = None;
}

/// `SND_ASYNC` makes this fire-and-forget, so it never blocks the render thread it's called
/// from. Note `PlaySoundW` only keeps one sound going at a time: a later call replaces an
/// earlier one rather than mixing. That's exactly what's wanted here - the countdown sound runs
/// out precisely as the pull sound starts, and a cancel should cut the countdown short.
fn play(wav: &'static [u8], volume: f32) {
    let volume = volume.clamp(0.0, MAX_VOLUME);
    let mut guard = CURRENT_BUFFER.lock().unwrap();

    if (volume - 1.0).abs() < 0.001 {
        // Safety: `SND_MEMORY` means `pszSound` is a pointer to an in-memory WAV image rather
        // than a filename. The data is `'static` (embedded in the binary), so it outlives the
        // call, and `SND_ASYNC` playback reads from it afterwards.
        *guard = None;
        unsafe {
            let _ = PlaySoundW(
                PCWSTR(wav.as_ptr() as *const u16),
                None,
                SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
            );
        }
        return;
    }

    // Below full volume: bake the scaling into a heap copy of the WAV up front, since
    // `PlaySoundW` has no per-call volume knob (only a device-wide one via `waveOutSetVolume`,
    // which would affect the player's whole system rather than just this sound). Stashing the
    // buffer in `CURRENT_BUFFER` keeps it alive for the async playback; the *previous* buffer is
    // safe to drop here since `PlaySoundW` (called without `SND_NOSTOP`) always stops whatever
    // was playing before starting this one.
    let scaled = scale_volume(wav, volume);
    unsafe {
        let _ = PlaySoundW(
            PCWSTR(scaled.as_ptr() as *const u16),
            None,
            SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
        );
    }
    *guard = Some(scaled);
}

/// Returns a copy of `wav` with every sample in its `data` chunk scaled by `volume`. Only 16-bit
/// PCM (what the bundled assets were converted to) is actually scaled; anything else - or a
/// malformed header - is returned unscaled rather than risking corrupt audio.
fn scale_volume(wav: &[u8], volume: f32) -> Vec<u8> {
    let mut out = wav.to_vec();
    if let Some((data_start, data_len, bits_per_sample)) = find_data_chunk(wav) {
        if bits_per_sample == 16 {
            let data_end = (data_start + data_len).min(out.len());
            let mut i = data_start;
            while i + 1 < data_end {
                let sample = i16::from_le_bytes([out[i], out[i + 1]]);
                let scaled = (sample as f32 * volume).round().clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                let bytes = scaled.to_le_bytes();
                out[i] = bytes[0];
                out[i + 1] = bytes[1];
                i += 2;
            }
        }
    }
    out
}

/// Walks the WAV's RIFF chunks to find the `data` chunk's byte range and the `fmt ` chunk's
/// bits-per-sample. Returns `None` if the file isn't well-formed enough to parse.
fn find_data_chunk(wav: &[u8]) -> Option<(usize, usize, u16)> {
    if wav.len() < 12 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" {
        return None;
    }

    let mut pos = 12usize;
    let mut bits_per_sample = 0u16;
    let mut data_range = None;

    while pos + 8 <= wav.len() {
        let chunk_id = &wav[pos..pos + 4];
        let chunk_size = u32::from_le_bytes(wav[pos + 4..pos + 8].try_into().ok()?) as usize;
        let body_start = pos + 8;

        if chunk_id == b"fmt " && body_start + 16 <= wav.len() {
            bits_per_sample = u16::from_le_bytes(wav[body_start + 14..body_start + 16].try_into().ok()?);
        } else if chunk_id == b"data" {
            let len = chunk_size.min(wav.len().saturating_sub(body_start));
            data_range = Some((body_start, len));
        }

        // RIFF chunks are word-aligned: a chunk with an odd size has one byte of padding after it.
        pos = body_start.saturating_add(chunk_size).saturating_add(chunk_size % 2);
    }

    data_range.map(|(start, len)| (start, len, bits_per_sample))
}
