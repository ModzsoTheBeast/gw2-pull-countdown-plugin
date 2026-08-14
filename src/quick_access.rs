use nexus::keybind::{keybind_handler, register_keybind_with_string};
use nexus::quick_access::add_quick_access;
use nexus::texture::load_texture_from_memory;

const TEXTURE_ID: &str = "PULLSYNC_ICON";
const TEXTURE_HOVER_ID: &str = "PULLSYNC_ICON_HOVER";
const KEYBIND_ID: &str = "PULLSYNC_PULL";
const QUICK_ACCESS_ID: &str = "PULLSYNC_QUICK_ACCESS";

const ICON: &[u8] = include_bytes!("../assets/quick_access_icon.png");
const ICON_HOVER: &[u8] = include_bytes!("../assets/quick_access_icon_hover.png");

/// Registers the toolbar icon (and the keybind it's tied to) that starts or cancels a pull.
/// This is the addon's only trigger now - there's no always-open panel anymore.
pub fn init() {
    load_texture_from_memory(TEXTURE_ID, ICON, None);
    load_texture_from_memory(TEXTURE_HOVER_ID, ICON_HOVER, None);

    register_keybind_with_string(
        KEYBIND_ID,
        keybind_handler!(|_id, is_release| {
            if !is_release {
                let total = crate::state::SETTINGS.lock().unwrap().start_count;
                crate::chat_send::on_pull_pressed(total);
            }
        }),
        "ALT+SHIFT+P",
    )
    .revert_on_unload();

    add_quick_access(
        QUICK_ACCESS_ID,
        TEXTURE_ID,
        TEXTURE_HOVER_ID,
        KEYBIND_ID,
        "PullSync: start a pull countdown (click again to cancel)",
    )
    .revert_on_unload();
}
