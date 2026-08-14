# PullSync

A commander-only "pull countdown" addon for [Raidcore Nexus](https://raidcore.gg/Nexus).

There's a single icon in Nexus's quick access toolbar (default keybind `ALT+SHIFT+P`, rebindable
in Nexus's own Keybinds panel). Clicking it (or pressing the keybind) starts the pull, using
whatever starting number is currently configured; clicking it again while a countdown is running
cancels it instead (and posts a "Pull cancelled" chat message). Starting a pull:

- immediately shows a big, customizable on-screen countdown overlay, counting down to `PULL!`,
  and
- sends a chat message into the current squad/party chat (squad if it can't be determined - see
  Configuring below), so anyone without the addon still sees a plain-text heads-up. By default
  this is a single message (e.g. "Pulling in 10..."); optionally it can instead count down in
  chat too, once per second, for the last few seconds. It can also go through GW2's squad
  broadcast instead of normal chat, for more attention - see Configuring.

The icon itself is always visible to everyone (Nexus has no way to hide it conditionally), but
only the actual squad/party commander can trigger or cancel a pull with it - anyone else gets a
small "only the squad commander can control the pull" alert instead. There's also an option to
trigger a pull automatically once a squad ready check finishes with everyone ready.

Other squad members running this addon **and** [arcdps](https://www.deltaconnected.com/arcdps/)
+ [Unofficial Extras](https://github.com/Krappa322/arcdps_unofficial_extras_releases) will have
that chat message automatically detected and mirror the same countdown overlay locally, in sync
with the commander - no networking or server involved, the chat message itself is the sync
signal. Everyone else just sees the plain chat text. Detection just needs the word "pull"
somewhere in the message alongside a number, so the chat text can be customized freely as long
as it keeps saying "pull" somewhere - see Configuring below.

## Configuring

Open Nexus's addon list (`Ctrl+O` by default), find PullSync, and click **Configure**. This is
where everything lives - there's no separate always-open panel. A reference block at the top of
that screen summarizes how the addon works and what other people need for it to sync to them
(the same points covered above), so it's there whenever you or a squadmate need a reminder.
Below that:

- **Starting number** - what the overlay (and, by default, the chat message) counts down from.
- **Chat message** - "Countdown in chat" (off by default) adds a per-second countdown in chat for
  the last few seconds ("Chat countdown start"), on top of the upfront heads-up message; "Chat
  message" and "Pull text" customize the wording (`{n}` is replaced by the count; keep the word
  "pull" in there for the cross-client sync to keep working - see above). "Use squad broadcast"
  (off by default) sends through GW2's squad broadcast instead of normal chat - more
  attention-grabbing, but squad-only (falls back to normal chat in a plain party), and can feel
  redundant for a group that already runs on a shared timer.
- **Overlay appearance** - position, size, and color. To reposition, uncheck "Lock position": a
  draggable preview appears on screen, and re-checking the box locks it back in place.
- **Automation** - "Auto-pull after ready check" triggers a pull automatically once a squad
  ready check finishes with everyone ready (needs arcdps + Unofficial Extras to detect it).

Every setting has a tooltip - hover over its label for details.

## Requirements

- [Nexus](https://raidcore.gg/Nexus) installed.
- Optional, for the overlay to mirror to other squad members: [arcdps](https://www.deltaconnected.com/arcdps/)
  + [Unofficial Extras](https://github.com/Krappa322/arcdps_unofficial_extras_releases) also
  installed on their end.
- Optional, to restrict who can actually trigger a pull to the real commander: "Real-time Data
  API" enabled in GW2 under Options > General. Without it, anyone can trigger a pull
  (fail-open).

## Building

The addon is a Rust `cdylib` (a Windows DLL loaded by Nexus). Building it requires a Windows
target, which can be cross-compiled from Linux/macOS using [`cargo-xwin`](https://github.com/rust-cross/cargo-xwin)
(downloads the MSVC CRT/Windows SDK itself, no system mingw or Windows machine needed):

```sh
rustup target add x86_64-pc-windows-msvc
cargo install cargo-xwin
cargo xwin build --release --target x86_64-pc-windows-msvc
```

The resulting DLL is at `target/x86_64-pc-windows-msvc/release/gw2_pull_countdown_plugin.dll`.
Copy it into `<Guild Wars 2 install>/addons/`.

`assets/` bundles a dedicated font used only for the countdown overlay text (`Roboto.ttf`,
SIL Open Font License - see `assets/Roboto-OFL.txt`), rebaked at whatever size is configured so
it stays crisp instead of blurring like a scaled-up bitmap font would, plus the quick access
toolbar icon (`quick_access_icon.png` / `_hover.png`).

If you have a real Windows machine/CI runner available, a native build works too:

```sh
cargo build --release --target x86_64-pc-windows-msvc
```

## Manual verification checklist

There's no headless test harness for a live game overlay, so verification is manual, in-game:

1. Load sanity - DLL loads in Nexus with no `NotLoadedIncompatible` error and no immediate panic
   in the Nexus log; the quick access icon appears in the toolbar.
2. RTAPI off - clicking the icon still starts the local countdown and still sends the chat
   message (fail-open, since who's commander can't be confirmed).
3. RTAPI on, non-commander in a squad - clicking the icon shows the "only the squad commander"
   alert and does nothing else; the overlay still appears when someone else sends a `pull N`
   message.
4. RTAPI on, commander, leading an actual squad (not just a party) - clicking the icon lands the
   message in squad chat (whole squad, not just your subgroup), the local overlay starts
   immediately without waiting for the chat round-trip, and it does not re-trigger when the
   echoed message arrives back.
5. Party-only (no squad) - confirm whether `is_commander` ever reads true here; if triggering
   never works in a plain party, confirm that's acceptable.
6. Cross-client mirror - a second account/friend with Nexus + arcdps + Unofficial Extras (no
   manual interaction needed on their end) sees the same countdown start from the same N shortly
   after the commander triggers a pull.
7. Graceful fallback - a squad member on plain GW2 (no addons at all) just sees the literal
   `squad pull 10` line in chat; nothing else breaks for them.
8. Focus-stealing check - as commander, open your own chat box and start typing something, then
   trigger a pull: confirm the pre-flight textbox-focus check skips auto-typing and shows a
   warning instead of corrupting your draft. Separately, confirm a mirroring client's chat box is
   never touched by receiving a countdown (only the sender types anything).
9. Persisted settings - change the starting number and overlay appearance in Nexus's Configure
   panel, restart GW2, confirm they're remembered (and check
   `<GW2 install>/addons/pull_countdown/settings.json` directly too).
10. Overlay appearance - uncheck "Lock position", confirm a draggable preview appears and follows
    the mouse while dragging, adjust the Size slider and confirm the text stays sharp (not
    blurry/pixelated) at large sizes, adjust the Color picker, then lock it back and confirm the
    real countdown uses the new position/size/color.
11. Hot-reload mid-squad - with an active squad already formed, reload the addon and re-check
    commander detection recovers.
12. Chat customization - edit "Chat message" and "Pull text" in Configure, trigger a pull, and
    confirm the edited wording shows up in chat with `{n}` replaced correctly.
13. Chat countdown - enable "Countdown in chat", set "Chat countdown start" lower than the
    starting number, trigger a pull, and confirm chat immediately gets the upfront "Pulling in
    10..." heads-up, then stays quiet until the overlay reaches the chat countdown number, then
    posts one line per second down to the pull text - in sync with the overlay, not just
    "eventually".
14. Ready check automation - enable "Auto-pull after ready check", start a squad ready check as
    commander, get everyone (including yourself) to ready up, and confirm a pull triggers
    automatically without touching the icon; then start a ready check that's cancelled/times out
    before everyone readies, and confirm nothing triggers.
15. Cancel - trigger a pull, then click the icon again mid-countdown: confirm the overlay
    disappears immediately, a "Pull cancelled" chat line goes out, and (with chat countdown on)
    any remaining tail ticks stop rather than continuing to post after the cancel.
16. Squad broadcast - enable "Use squad broadcast" while leading a squad, trigger a pull, and
    confirm the message appears as a squad broadcast rather than a normal chat line; then try it
    from a plain party (no squad) and confirm it falls back to a normal chat message instead.
