# PullSync

A commander-only "pull countdown" addon for [Raidcore Nexus](https://raidcore.gg/Nexus).

There's a single icon in Nexus's quick access toolbar (default keybind `ALT+SHIFT+P`, rebindable
in Nexus's own Keybinds panel). Clicking it (or pressing the keybind) starts the pull, using
whatever starting number is currently configured; clicking it again while a countdown is running
cancels it instead (and posts a "Pull cancelled" chat message). Starting a pull:

- immediately shows a big, customizable on-screen countdown overlay, counting down to `PULL!`
  (with local countdown/pull sounds, each individually toggleable and independently
  volume-adjustable), and
- sends a chat message into the current squad/party chat (squad or party is detected
  automatically - see Configuring below), so anyone without the addon still sees a plain-text
  heads-up. By default that's two messages - an upfront heads-up (e.g. "Pulling in 10...") and a
  final "Pull!" line when the timer completes; optionally it can instead count down in chat too,
  once per second, for the last few seconds. It can also go through GW2's squad broadcast instead
  of normal chat, for more attention - see Configuring.

The icon itself is always visible to everyone (Nexus has no way to hide it conditionally), but
triggering or cancelling a pull with it follows the same rule as who's obeyed on the receiving
end (see "Who can start a countdown on your screen" below): in a squad, only the commander or a
lieutenant may; in a plain party, anyone may. Anyone else gets a small "only the squad commander
or a lieutenant can control the pull" alert instead. Without [RTAPI](#requirements) installed,
lieutenant status specifically can't be confirmed (there's no addon-independent signal for it),
so only a confirmed commander (detected from your character's commander tag, which needs no
extra addon) is allowed - a lieutenant without RTAPI gets the alert too, rather than silently
starting a pull that would only ever show up on their own screen.

There's also an option to trigger a pull automatically once a squad ready check finishes with
everyone ready. Only the commander's client ever sends the chat message for this - unlike a
manual click, the ready-check-complete event fires identically on every eligible client at once,
so if lieutenants could also auto-trigger, several people's clients would each open chat and type
their own copy of the message at the same time. A lieutenant can still always trigger manually.

Other squad members running this addon **and** [arcdps](https://www.deltaconnected.com/arcdps/)
+ [Unofficial Extras](https://github.com/Krappa322/arcdps_unofficial_extras_releases) will have
that chat message automatically detected and mirror the same countdown overlay locally, in sync
with the commander - no networking or server involved, the chat message itself is the sync
signal. Everyone else just sees the plain chat text.

### Who can start a countdown on your screen

Since the transport is ordinary chat, anything typed in chat could in principle drive everyone's
overlay. Two rules prevent that:

- **Messages must carry the `[PullSync]` tag**, which the addon adds automatically and a human
  would never type by hand. Ordinary conversation is therefore never mistaken for a countdown -
  saying "we pull at 10%" or "cancel that" does nothing, no matter who says it.
- **In a squad, only the commander or a lieutenant is obeyed.** A tagged message from any other
  member is ignored, so a squadmate can't start or cancel pulls for everyone. In a plain party
  there is no leader to check against, so anyone in the party may.

Both checks happen on the *receiving* client, so they hold even if someone modifies their own
copy of the addon.

## Configuring

Open Nexus's addon list (`Ctrl+O` by default), find PullSync, and click **Configure**. This is
where everything lives - there's no separate always-open panel. A reference block at the top of
that screen summarizes how the addon works and what other people need for it to sync to them
(the same points covered above), so it's there whenever you or a squadmate need a reminder.
Below that:

- **Profile** - everything else on this screen belongs to whichever profile is selected here.
  Switch between saved profiles with the dropdown, create a new one (starts as a copy of the
  current one, so you tweak instead of starting from scratch), rename the active one, or delete
  it (refused if it's the only one left). Useful for e.g. a "Raid" profile vs. a "Fractals"
  profile with different wording or a different starting number.
- **Starting number** - what the overlay (and, by default, the chat message) counts down from.
- **Chat message** - by default, exactly two lines go out: the upfront heads-up and a final
  "Pull!" line once the timer completes. "Countdown in chat" additionally adds a per-second
  countdown for the last few seconds ("Chat countdown start"), in between those two. "Chat
  message" and "Pull text" customize the wording - word them however you like, but keep the
  `{n}` in the countdown message, since receiving clients read the count off that number (the
  `[PullSync]` tag is added automatically and isn't part of your wording). "Use squad broadcast"
  (off by default) sends the upfront message and the final "pull now" line through GW2's squad
  broadcast instead of normal chat - more attention-grabbing, but squad-only (falls back to
  normal chat in a plain party). Only those two lines ever broadcast, even with chat countdown
  on - broadcasts stay on screen for several seconds and queue up (falling out of sync with the
  overlay) if sent every second, so the ticks in between always use normal chat regardless. When
  a pull is auto-triggered by a ready check, the *upfront* message always uses normal chat
  instead, even with broadcast on - GW2 posts its own broadcast-style notice the moment a ready
  check completes, and our own would otherwise queue up behind it and land late. The final "pull
  now" line still broadcasts normally, since by then GW2's own notice has long since cleared.
- **Overlay appearance** - position, size, and color. To reposition, uncheck "Lock position": a
  draggable preview appears on screen, and re-checking the box locks it back in place.
- **Sound** - two independent toggles with their own volume sliders, both on by default at full
  volume (1.0, i.e. the bundled recording played as-is) and both local only (nothing is played on
  anyone else's client). The sliders go up to 8x, since the recordings themselves are quiet - feel
  free to push them well past 1.0 if they're hard to hear. The countdown sound beeps down the
  final five seconds in time with the on-screen numbers; because it's a fixed five seconds long,
  it's skipped entirely when the starting number is below 5. The pull sound fires as the count
  reaches "PULL!". Cancelling a pull stops the countdown sound immediately.
- **Automation** - "Auto-pull after ready check" triggers a pull automatically once a squad
  ready check finishes with everyone ready (needs arcdps + Unofficial Extras to detect it).

Every setting has a tooltip - hover over its label for details.

## Requirements

- [Nexus](https://raidcore.gg/Nexus) installed.
- Optional, for the overlay to mirror to other squad members: [arcdps](https://www.deltaconnected.com/arcdps/)
  + [Unofficial Extras](https://github.com/Krappa322/arcdps_unofficial_extras_releases) also
  installed on their end.
- Optional, to also confirm lieutenant status specifically:
  [RTAPI](https://github.com/RaidcoreGG/GW2-RealTime-API-Releases) installed as its own Nexus
  addon (search "RTAPI" in Nexus's Library tab, or download it from that link and drop it in the
  same `addons` folder as this one) - it's a separate addon by the Raidcore team, not a GW2
  setting. The commander themselves needs no extra addon for this - that's read straight from
  your character's commander tag - but there's no addon-independent way to tell a lieutenant
  apart from a regular member, so without RTAPI only the commander is allowed to trigger a pull
  in a squad (anyone may in a plain party either way).

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
it stays crisp instead of blurring like a scaled-up bitmap font would, the quick access toolbar
icon (`quick_access_icon.png` / `_hover.png`), and the two alert sounds (`countdown.wav`,
`pull.wav`).

The sounds are 48kHz 16-bit mono PCM WAV, which is not incidental: they're played with Win32
`PlaySound`, which handles **WAV only** (no MP3), and mono keeps the embedded size sensible
since both are compiled into the DLL. `countdown.wav` must stay exactly 5 seconds with its
beats one second apart - the code starts it precisely 5 seconds from zero so each beat lands on
a digit, so replacing it with a different length would desynchronise it from the overlay.

## Releasing a new version

The addon declares itself as GitHub-updatable (`provider` in `src/lib.rs`, pointing at
Cargo.toml's `repository`), so anyone with it installed - whether or not it's ever listed in
Nexus's official Library - gets notified of new versions automatically, as long as releases are
published a specific way:

1. Bump `version` in `Cargo.toml` (e.g. `0.1.0` -> `0.1.1`), then run a build so `Cargo.lock`
   picks up the new version too, and commit both.
2. Tag the commit `v<version>` (matching Cargo.toml exactly, e.g. `v0.1.1`) and push both the
   commit and the tag.
3. `.github/workflows/release.yml` picks up the tag push, verifies the tag matches Cargo.toml,
   builds a native Windows DLL on a `windows-latest` runner, and publishes it as a GitHub
   Release with the `.dll` attached - nothing to do manually beyond pushing the tag.

Nexus's update check (verified against its own source) fetches every release from this repo,
parses each `tag_name`, and picks the **highest-versioned** release with a `.dll` attached - not
the most recent one, so re-publishing an older version to roll users back does not work. Things
that make a release silently invisible to it:

- **Tags with fewer than three components.** `v0.2` passes its regex but then throws while
  parsing, and the release is skipped. Always `vX.Y.Z`.
- **Tags with a suffix**, e.g. `v0.2.0-rc.1` - same silent skip. (The workflow's tag filter
  only matches plain `vX.Y.Z` for this reason, so these never get published in the first place.)
- **Bumping only a fourth "revision" component**, e.g. `v0.1.0.1`. `nexus-rs` encodes a stable
  release's revision as `-1`, which the Nexus host reads back as `65535`, so no real revision
  number can ever exceed it. Only major/minor/build bumps actually trigger an update.

Releases marked as **pre-release** on GitHub are skipped for most users, but not all: Nexus has
a per-addon "allow pre-releases" toggle that's available for GitHub-hosted addons, so those
users do receive them.

If you have a real Windows machine/CI runner available, a native build works too:

```sh
cargo build --release --target x86_64-pc-windows-msvc
```

## Manual verification checklist

There's no headless test harness for a live game overlay, so verification is manual, in-game:

1. Load sanity - DLL loads in Nexus with no `NotLoadedIncompatible` error and no immediate panic
   in the Nexus log; the quick access icon appears in the toolbar.
2. RTAPI off, leading a squad with your commander tag active - clicking the icon still starts the
   countdown and still sends the chat message (confirmed via the commander tag, not fail-open).
3. RTAPI off, plain squad member (no commander tag) - clicking the icon shows the "only the squad
   commander or a lieutenant" alert and does nothing else, even if you're actually a lieutenant
   (can't be confirmed without RTAPI).
4. RTAPI on, plain member (not commander/lieutenant) in a squad - clicking the icon shows the
   "only the squad commander or a lieutenant" alert and does nothing else; the overlay still
   appears when the commander or a lieutenant sends a `pull N` message. A lieutenant should be
   able to trigger/cancel just like the commander (this specifically requires RTAPI - see 3).
5. RTAPI on, commander, leading an actual squad (not just a party) - clicking the icon lands the
   message in squad chat (whole squad, not just your subgroup), the local overlay starts
   immediately without waiting for the chat round-trip, and it does not re-trigger when the
   echoed message arrives back.
6. Party-only (no squad) - any party member (not just whoever is "commander") can trigger/cancel
   regardless of RTAPI, and the chat message lands as `party ...` rather than `squad ...`.
7. Cross-client mirror - a second account/friend with Nexus + arcdps + Unofficial Extras (no
   manual interaction needed on their end) sees the same countdown start from the same N shortly
   after the commander triggers a pull.
8. Graceful fallback - a squad member on plain GW2 (no addons at all) just sees the literal
   `squad pull 10` line in chat; nothing else breaks for them.
9. Focus-stealing check - as commander, open your own chat box and start typing something, then
   trigger a pull: confirm the pre-flight textbox-focus check skips auto-typing and shows a
   warning instead of corrupting your draft. Separately, confirm a mirroring client's chat box is
   never touched by receiving a countdown (only the sender types anything).
10. Persisted settings - change the starting number and overlay appearance in Nexus's Configure
    panel, restart GW2, confirm they're remembered (and check
    `<GW2 install>/addons/pull_countdown/settings.json` directly too - it now holds every profile,
    not just one flat settings object).
11. Overlay appearance - uncheck "Lock position", confirm a draggable preview appears and follows
    the mouse while dragging, adjust the Size slider and confirm the text stays sharp (not
    blurry/pixelated) at large sizes, adjust the Color picker, then lock it back and confirm the
    real countdown uses the new position/size/color. Let a countdown run all the way to "PULL!"
    and confirm it stays centered on the same spot as the numbers, rather than drifting to one
    side since "PULL!" is wider text than a single digit.
12. Hot-reload mid-squad - with an active squad already formed, reload the addon and re-check
    commander detection recovers.
13. Chat customization - edit "Chat message" and "Pull text" in Configure, trigger a pull, and
    confirm the edited wording shows up in chat with `{n}` replaced correctly.
14. Chat countdown - enable "Countdown in chat", set "Chat countdown start" lower than the
    starting number, trigger a pull, and confirm chat immediately gets the upfront "Pulling in
    10..." heads-up, then stays quiet until the overlay reaches the chat countdown number, then
    posts one line per second down to the pull text - in sync with the overlay, not just
    "eventually".
15. Ready check automation, single sender - with a squad that has both a commander and at least
    one lieutenant, and both having "Auto-pull after ready check" enabled, get everyone ready and
    confirm only the commander's client opens chat and types the message - the lieutenant's
    client should just receive/mirror it like a regular member, not also send its own copy. Then
    start a ready check that's cancelled/times out before everyone readies, and confirm nothing
    triggers.
16. Ready check without RTAPI - with RTAPI not installed/active, enable "Auto-pull after ready
    check", start a squad ready check as commander, get everyone ready, and confirm it still
    auto-triggers for everyone (this used to silently never fire without RTAPI).
17. Cancel - trigger a pull, then click the icon again mid-countdown: confirm the overlay
    disappears immediately, a "Pull cancelled" chat line goes out, and (with chat countdown on)
    any remaining tail ticks stop rather than continuing to post after the cancel. Specifically
    try cancelling right as a tail tick number appears in chat (the race that used to make the
    cancel message silently not send at all) and confirm it now always goes out, even if a
    stray tick number occasionally lands just before it.
18. Squad broadcast - enable "Use squad broadcast" while leading a squad, trigger a pull manually
    (not via ready check), and confirm the *upfront* message appears as a squad broadcast rather
    than a normal chat line; then try it from a plain party (no squad) and confirm it falls back
    to a normal chat message instead.
19. Broadcast vs. ready-check notice - enable "Use squad broadcast", enable "Auto-pull after ready
    check", and trigger a pull via a completed ready check: confirm the upfront message goes out
    as a *normal* chat line (not a broadcast, even though the setting is on) so it doesn't queue
    up behind GW2's own ready-check-complete broadcast notice; the final "pull now" line should
    still broadcast normally.
20. Profiles - change a few settings, create a new profile (confirm it starts as a copy), change
    something in the new one, switch back to the original and confirm its own settings (not the
    new profile's) are what's active, then rename and delete the new profile and confirm it can't
    delete the last remaining profile.
21. Sound - with a starting number above 5, confirm the countdown sound starts exactly as the
    overlay hits 5 and its beats land on 5/4/3/2/1, and the pull sound fires as it reaches
    "PULL!". Each plays once (not once per frame during the linger, and not for the "10" preview
    shown while repositioning the overlay). Then check: a starting number below 5 plays no
    countdown sound at all but still plays the pull sound; a starting number of exactly 5 starts
    the countdown sound immediately; cancelling mid-countdown cuts the sound off; and each
    checkbox silences its own sound independently. Also drag each volume slider well past 1.0
    (up toward the 8x max) and confirm the sound gets noticeably louder rather than distorting or
    clipping harshly.
22. Broadcast timing - with both "Countdown in chat" and "Use squad broadcast" on (manual
    trigger), trigger a pull and confirm only the upfront message and the final "pull now" line
    show up as squad broadcasts, while every tick in between (5, 4, 3...) lands in normal chat
    instead - and that the ticks stay in sync with the overlay rather than queuing up behind a
    broadcast.
23. Broadcast without chat countdown - with "Countdown in chat" off and "Use squad broadcast" on
    (manual trigger), trigger a pull and confirm *both* the upfront message and the final "Pull!"
    line show up as squad broadcasts (not just the first one).
