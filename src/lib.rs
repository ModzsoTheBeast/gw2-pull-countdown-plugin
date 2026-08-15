mod chat_listen;
mod chat_send;
mod countdown;
mod extras_squad;
mod overlay_font;
mod quick_access;
mod ready_check;
mod settings;
mod sound;
mod squad;
mod state;
mod ui;

use nexus::gui::{RenderType, register_render, render};

nexus::export! {
    name: "PullSync",
    signature: -0x50554C43,
    load: load,
    unload: unload,
    flags: nexus::AddonFlags::None,
    // The update link is derived from Cargo.toml's `repository` field for the GitHub provider,
    // so it lives in exactly one place rather than being duplicated here.
    provider: nexus::UpdateProvider::GitHub,
    log_filter: "warn,gw2_pull_countdown_plugin=info",
}

fn load() {
    settings::load();
    overlay_font::init(state::SETTINGS.lock().unwrap().overlay_font_size);
    quick_access::init();
    squad::subscribe_all();
    extras_squad::subscribe();
    chat_listen::subscribe();
    ready_check::subscribe();
    register_render(RenderType::Render, render!(ui::render_frame)).revert_on_unload();
    register_render(RenderType::OptionsRender, render!(ui::render_options)).revert_on_unload();
}

fn unload() {
    settings::save();
}
