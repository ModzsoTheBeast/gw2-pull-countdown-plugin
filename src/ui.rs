use crate::{overlay_font, settings, state};
use nexus::imgui::{ColorEdit, Condition, InputInt, InputText, Slider, StyleVar, Ui, Window};
use std::sync::Mutex;

const PULL_FLASH_COLOR: [f32; 4] = [1.0, 0.2, 0.2, 1.0];

/// Scratch text buffers for the profile name fields - transient UI state, not persisted.
static NEW_PROFILE_NAME: Mutex<String> = Mutex::new(String::new());
static RENAME_PROFILE_NAME: Mutex<String> = Mutex::new(String::new());

/// Per-frame render callback: just the countdown overlay. There's no always-open panel - the
/// pull is triggered from a quick access icon, and settings live in Nexus's own addon Options.
pub fn render_frame(ui: &Ui) {
    draw_overlay(ui);
}

fn draw_overlay(ui: &Ui) {
    let snapshot = crate::countdown::snapshot();
    let (pos_frac, color, locked) = {
        let s = state::SETTINGS.lock().unwrap();
        (s.overlay_pos_frac, s.overlay_color, s.overlay_locked)
    };

    // Nothing to show and not in the middle of positioning it - stay hidden.
    if snapshot.is_none() && locked {
        return;
    }

    if let Some(s) = snapshot {
        if s.just_entered_countdown_sound_window {
            crate::sound::play_countdown_if_enabled();
        }
        if s.just_reached_pull {
            crate::sound::play_pull_if_enabled();
        }
    }

    let (text, text_color) = match snapshot {
        Some(s) if s.is_pull => ("PULL!".to_string(), PULL_FLASH_COLOR),
        Some(s) => (s.remaining.to_string(), [color[0], color[1], color[2], 1.0]),
        None => ("10".to_string(), [color[0], color[1], color[2], 1.0]), // preview while editing
    };

    let display_size = ui.io().display_size;
    // `pos_frac` is the CENTER of the overlay, not its corner - "10" and "PULL!" are different
    // widths, and anchoring by a corner would leave one edge fixed while the other drifts,
    // making wider text look off-center relative to narrower text at the same spot.
    let center = [pos_frac[0] * display_size[0], pos_frac[1] * display_size[1]];

    let mut window = Window::new("##pull_countdown_overlay")
        .title_bar(false)
        .resizable(false)
        .scroll_bar(false)
        .collapsible(false)
        .always_auto_resize(true);

    window = if locked {
        window
            .position(center, Condition::Always)
            .position_pivot([0.5, 0.5])
            .movable(false)
            .bg_alpha(0.0)
    } else {
        window.movable(true).bg_alpha(0.35)
    };

    // Nexus's theme draws a 1px window border, which reads as a stray outline around the bare
    // countdown text. Suppressed while locked; kept while unlocked, where it usefully shows the
    // bounds of the thing being dragged. Must be pushed before build(), since the border is
    // resolved when the window begins, and it pops on drop at the end of this function.
    let _no_border = locked.then(|| ui.push_style_var(StyleVar::WindowBorderSize(0.0)));

    let font = overlay_font::current();
    let final_center = window.build(ui, || {
        if let Some(font) = font {
            unsafe { nexus::imgui::sys::igPushFont(font) };
        }
        ui.text_colored(text_color, &text);
        if font.is_some() {
            unsafe { nexus::imgui::sys::igPopFont() };
        }
        if !locked {
            ui.text_disabled("drag to reposition, then lock it in PullSync's Nexus options");
        }
        // `window_pos` is the top-left corner regardless of pivot - convert to center here so
        // the stored value always means the same thing as `center` above.
        let top_left = ui.window_pos();
        let size = ui.window_size();
        [top_left[0] + size[0] / 2.0, top_left[1] + size[1] / 2.0]
    });

    if !locked {
        if let Some(new_center) = final_center {
            let mut s = state::SETTINGS.lock().unwrap();
            s.overlay_pos_frac = [
                (new_center[0] / display_size[0]).clamp(0.0, 1.0),
                (new_center[1] / display_size[1]).clamp(0.0, 1.0),
            ];
        }
    }
}

/// Shows a tooltip on the previously drawn widget, if it's currently hovered.
fn tooltip(ui: &Ui, text: &str) {
    if ui.is_item_hovered() {
        ui.tooltip_text(text);
    }
}

/// Always-visible reference block explaining what this addon needs (and what it needs *other*
/// people to have) - shown every time Configure is opened rather than a one-time popup, so it's
/// there whenever someone forgets or a new squad member asks.
fn draw_info_block(ui: &Ui) {
    ui.text_colored([0.5, 0.8, 1.0, 1.0], "How PullSync works");
    ui.text_wrapped(
        "- Click the toolbar icon (or its keybind, see Keybinds above) to start a pull; click it again while one is running to cancel it.",
    );
    ui.text_wrapped(
        "- Everyone in your squad/party sees the chat message this posts, even with no addons at all.",
    );
    ui.text_wrapped(
        "- For the big on-screen countdown to also appear on someone else's screen automatically, THEY need Nexus + PullSync + arcdps + Unofficial Extras installed too - otherwise they just see the chat text.",
    );
    ui.text_wrapped(
        "- In a squad only the commander and lieutenants can start or cancel a pull for everyone; in a party anyone can. Messages are tagged [PullSync], so ordinary chat mentioning \"pull\" or \"cancel\" never triggers anything.",
    );
    ui.text_wrapped(
        "- Install RTAPI (a separate Nexus addon by the Raidcore team - search \"RTAPI\" in Nexus's Library tab) so only the actual commander can trigger this; it's not a GW2 setting. Without it, anyone can trigger a pull.",
    );
}

/// Profile switcher: a dropdown of saved profiles, plus create/rename/delete for the active
/// one. Every other setting below belongs to whichever profile is selected here.
fn draw_profile_section(ui: &Ui) {
    ui.text("Profile");

    let names = settings::profile_names();
    let active_name = settings::active_profile_name();
    let mut current_index = names.iter().position(|n| *n == active_name).unwrap_or(0);
    if ui.combo_simple_string("Active profile", &mut current_index, &names) {
        if let Some(name) = names.get(current_index) {
            settings::switch_profile(name);
        }
    }
    tooltip(
        ui,
        "Switch between saved configurations - each profile has its own starting number, chat\nwording, overlay appearance, and automation settings.",
    );

    let mut new_name = NEW_PROFILE_NAME.lock().unwrap().clone();
    InputText::new(ui, "New profile name", &mut new_name).build();
    *NEW_PROFILE_NAME.lock().unwrap() = new_name.clone();
    ui.same_line();
    if ui.button("Create") {
        settings::create_profile(&new_name);
        *NEW_PROFILE_NAME.lock().unwrap() = String::new();
    }
    tooltip(ui, "Creates a new profile as a copy of the current one, and switches to it.");

    let mut rename_to = RENAME_PROFILE_NAME.lock().unwrap().clone();
    InputText::new(ui, "Rename active profile to", &mut rename_to).build();
    *RENAME_PROFILE_NAME.lock().unwrap() = rename_to.clone();
    ui.same_line();
    if ui.button("Rename") {
        settings::rename_active_profile(&rename_to);
        *RENAME_PROFILE_NAME.lock().unwrap() = String::new();
    }
    tooltip(ui, "Renames the currently active profile (the one selected above).");

    if ui.button("Delete active profile") {
        settings::delete_active_profile();
    }
    tooltip(
        ui,
        "Deletes the currently active profile and switches to another one.\nRefuses if it's the only profile left - there's always at least one.",
    );
}

/// Rendered by Nexus inside its own "Configuring: PullSync" window (opened via the addon's
/// Configure button) - no `Window::new` wrapper needed here, Nexus already provides one.
pub fn render_options(ui: &Ui) {
    draw_info_block(ui);
    ui.separator();

    draw_profile_section(ui);
    ui.separator();

    let mut start_count = state::SETTINGS.lock().unwrap().start_count as i32;
    if InputInt::new(ui, "Starting number", &mut start_count).build() {
        state::SETTINGS.lock().unwrap().start_count = start_count.clamp(1, 999) as u32;
        settings::save();
    }
    tooltip(ui, "The overlay counts down from this number when Pull is triggered.");

    ui.separator();
    ui.text("Chat message");

    let mut chat_countdown_enabled = state::SETTINGS.lock().unwrap().chat_countdown_enabled;
    if ui.checkbox("Countdown in chat", &mut chat_countdown_enabled) {
        state::SETTINGS.lock().unwrap().chat_countdown_enabled = chat_countdown_enabled;
        settings::save();
    }
    tooltip(
        ui,
        "Off (default): one heads-up chat message when Pull is triggered.\nOn: chat also counts down each second for the last few seconds,\ntimed with the on-screen overlay.",
    );

    if chat_countdown_enabled {
        let mut chat_countdown_start = state::SETTINGS.lock().unwrap().chat_countdown_start as i32;
        if InputInt::new(ui, "Chat countdown start", &mut chat_countdown_start).build() {
            state::SETTINGS.lock().unwrap().chat_countdown_start =
                chat_countdown_start.clamp(1, 999) as u32;
            settings::save();
        }
        tooltip(
            ui,
            "Chat starts counting down from this number (capped to the starting number above),\nlanding on 0 / the pull text at the same moment the overlay does.",
        );
    }

    let mut template = state::SETTINGS.lock().unwrap().chat_message_template.clone();
    if InputText::new(ui, "Chat message", &mut template).build() {
        state::SETTINGS.lock().unwrap().chat_message_template = template;
        settings::save();
    }
    tooltip(
        ui,
        "Text posted to chat, with {n} replaced by the count.\nWord it however you like, but keep the {n} - squad members running this\naddon sync their overlay off that number. Messages are automatically\ntagged with [PullSync] so ordinary chat is never mistaken for a countdown.",
    );

    let mut pull_text = state::SETTINGS.lock().unwrap().chat_pull_text.clone();
    if InputText::new(ui, "Pull text", &mut pull_text).build() {
        state::SETTINGS.lock().unwrap().chat_pull_text = pull_text;
        settings::save();
    }
    tooltip(ui, "Sent to chat instead of the message above once the count reaches 0.");

    let mut use_broadcast = state::SETTINGS.lock().unwrap().use_squad_broadcast;
    if ui.checkbox("Use squad broadcast", &mut use_broadcast) {
        state::SETTINGS.lock().unwrap().use_squad_broadcast = use_broadcast;
        settings::save();
    }
    tooltip(
        ui,
        "Off (default): posts to normal squad/party chat.\nOn: the upfront message and the final \"pull now\" line post through GW2's squad broadcast\ninstead - more attention-grabbing, good for PUGs without a dedicated timer. Any per-second\nchat countdown ticks in between always use normal chat regardless of this, since broadcasts\nstay on screen for several seconds and queue up (getting out of sync) if sent that often.\nSquad-only; falls back to normal chat automatically when you're only in a party.",
    );

    ui.separator();
    ui.text("Overlay appearance");

    let mut locked = state::SETTINGS.lock().unwrap().overlay_locked;
    if ui.checkbox("Lock position", &mut locked) {
        state::SETTINGS.lock().unwrap().overlay_locked = locked;
        settings::save();
    }
    tooltip(
        ui,
        "Uncheck to drag the overlay to a new spot - a preview appears on screen\neven with no countdown running. Check it again to lock it back in place.",
    );
    if !locked {
        ui.text_disabled("A preview is shown on screen - drag it, then lock it back.");
    }

    let mut font_size = state::SETTINGS.lock().unwrap().overlay_font_size;
    if Slider::new("Size", 24.0f32, 200.0).build(ui, &mut font_size) {
        state::SETTINGS.lock().unwrap().overlay_font_size = font_size;
        overlay_font::resize(font_size);
        settings::save();
    }
    tooltip(ui, "Point size of the on-screen countdown text.");

    let mut color = state::SETTINGS.lock().unwrap().overlay_color;
    if ColorEdit::new("Color", &mut color).build(ui) {
        state::SETTINGS.lock().unwrap().overlay_color = color;
        settings::save();
    }
    tooltip(ui, "Color of the counting-down numbers (the final \"PULL!\" flash always stays red).");

    if ui.button("Reset appearance") {
        let defaults = settings::Settings::const_default();
        {
            let mut s = state::SETTINGS.lock().unwrap();
            s.overlay_pos_frac = defaults.overlay_pos_frac;
            s.overlay_font_size = defaults.overlay_font_size;
            s.overlay_color = defaults.overlay_color;
        }
        overlay_font::resize(defaults.overlay_font_size);
        settings::save();
    }
    tooltip(ui, "Resets position, size, and color back to defaults.");

    ui.separator();
    ui.text("Sound");
    ui.text_disabled("Both are local only - nothing is played on anyone else's client.");

    let mut sound_countdown = state::SETTINGS.lock().unwrap().sound_countdown_enabled;
    if ui.checkbox("Countdown sound (last 5s)", &mut sound_countdown) {
        state::SETTINGS.lock().unwrap().sound_countdown_enabled = sound_countdown;
        settings::save();
    }
    tooltip(
        ui,
        "Beeps down the final five seconds, in time with the on-screen numbers.\nThe sound is a fixed 5 seconds long, so it's skipped entirely when the\nstarting number is below 5.",
    );

    let mut sound_pull = state::SETTINGS.lock().unwrap().sound_pull_enabled;
    if ui.checkbox("Pull sound", &mut sound_pull) {
        state::SETTINGS.lock().unwrap().sound_pull_enabled = sound_pull;
        settings::save();
    }
    tooltip(ui, "Plays the moment the count reaches \"PULL!\".");

    ui.separator();
    ui.text("Automation");

    let mut auto_pull = state::SETTINGS.lock().unwrap().auto_pull_after_ready_check;
    if ui.checkbox("Auto-pull after ready check", &mut auto_pull) {
        state::SETTINGS.lock().unwrap().auto_pull_after_ready_check = auto_pull;
        settings::save();
    }
    tooltip(
        ui,
        "Automatically triggers a pull the moment a squad ready check finishes with\neveryone ready. Requires arcdps + Unofficial Extras to detect the ready check at all.",
    );
}
