use nexus::font::{add_font_from_memory, font_receive, resize_font};
use nexus::imgui::sys::ImFont;
use std::sync::atomic::{AtomicUsize, Ordering};

const FONT_IDENTIFIER: &str = "PULLSYNC_COUNTDOWN_FONT";
const FONT_BYTES: &[u8] = include_bytes!("../assets/Roboto.ttf");

/// Raw `*mut ImFont` isn't `Send`/`Sync`, so the pointer is stashed as its address instead.
/// 0 means "not loaded yet" - font loading happens asynchronously in Nexus.
static FONT_PTR: AtomicUsize = AtomicUsize::new(0);

/// Loads the dedicated overlay font at the given point size.
pub fn init(initial_size: f32) {
    add_font_from_memory(
        FONT_IDENTIFIER,
        FONT_BYTES,
        initial_size,
        None,
        font_receive!(|_id, font| {
            let addr = font.map(|f| f as *mut ImFont as usize).unwrap_or(0);
            FONT_PTR.store(addr, Ordering::Relaxed);
        }),
    )
    .revert_on_unload();
}

/// Rebakes the overlay font at a new point size, so it stays crisp instead of being stretched.
pub fn resize(new_size: f32) {
    resize_font(FONT_IDENTIFIER, new_size);
}

/// Returns the current font pointer, if it has finished loading.
pub fn current() -> Option<*mut ImFont> {
    let addr = FONT_PTR.load(Ordering::Relaxed);
    (addr != 0).then_some(addr as *mut ImFont)
}
