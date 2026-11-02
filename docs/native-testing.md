# Aerial Home Native validation checklist

This checklist covers the separate **0.2.0 native candidate**, `io.github.justmaaarco.aerialhome.nativeprobe`. Record the package hash, firmware, conditions and evidence for each check. Unit tests, software previews and accelerated hardware trials establish different things; do not substitute one for another.

The original web launcher remains the Home-button and power-on target. No native Home/power migration has been applied. The already confirmed PS5 power-link configuration is unchanged.

## Current evidence

| Evidence | Scope |
| --- | --- |
| 41 passing Rust tests, 14 library + 27 binary | Model, service boundaries, video state, persistence/cache and raster behavior |
| 61 passing Node tests | Existing web behavior and packaging checks |
| Five offline software previews | Home, Settings, Control Center, Glass and Immersive clocks; no video/window initialization, TV service requests or foreground change |
| Normal app-manager launch | Native jail and foreground identity verified |
| Live inventory | 31 app/input entries |
| Accelerated visual-port trial | First UI 454.558 ms; grid 502.049 ms; first available artwork frame 668.951 ms with one icon |
| Scripted presentation samples | 207 samples, median 2 ms, p95 4 ms; not remote-to-photon |
| Memory sample | Approximately 27 MB process RSS |
| Hardware video | Moving 3840×2160 New York Skyline Dolby Vision, `dvh1` at 29.97 fps, owned VDEC/Dolby processing/main scaler resources and advancing time |
| End-of-clip handling | A near-end seek led to a different URI/media ID and another verified 4K Dolby Vision pipeline; repeat on the final package |

The recorded accelerated trial used package SHA-256 `6842489f2c16adf3dc55f8a8b38a072024a563a75a3e28f0c78234f90bdf87a6`. Its initialization timer starts before SDL setup, after local Home IPC setup, and stops at the relevant `SDL_RenderPresent` return. TV wake, app-manager startup and jail setup are excluded. See [native architecture and measurement limits](../native/README.md) and [per-artifact records](../native/measurements-v0.2.json).

**Renderer gate passed:** the final package cross-built and passed the accelerated GPU recheck. The transparent-letterbox correction removed the observed artwork seams, and the `4K · DOLBY VISION` badge agrees with media inspection. No matched native/web speedup or broad TV-model compatibility has been established.

The catalogue has 216 entries; 164 have an eligible source of at least 1440p under the tested capabilities/helper configuration. This is source eligibility, not a successful playback count. The [0.1.0 placeholder probe](../native/measurements.json) is historical and is not part of the candidate's performance evidence.

## Package and service boundaries

- [ ] Verify the final IPK hash, ARM ELF architecture, executable permissions, separate app ID, fonts and license notices.
- [x] Launch through the TV's normal app manager under its native jail and verify the native foreground identity.
- [x] Discover real app/input inventory and display it in the native UI.
- [ ] Exercise installed-app LS2 inventory, input, capability, artwork and control requests. Confirm bounded failures with a navigable recovery path; a root-side diagnostic alone is insufficient.
- [ ] Exit and relaunch; check for orphaned app/media workers, stale sockets and compositor surfaces.
- [ ] Verify that candidate tests leave the web app's files, preferences and Home/power bindings unchanged.

## Visual parity and navigation

- [x] Render Home, Settings, Control Center, Glass and Immersive clock fixtures through the shared renderer without initializing video or creating a window.
- [ ] Compare the final accelerated UI with the existing web layout at the same logical resolution, preferences, artwork and background. Check Inter weights, tile geometry, contain/crop behavior, focus, toolbar, captions and compact/wide panels.
- [ ] Verify all Home logos and inspect transparent letterbox edges over both light and dark moving footage. There must be no coloured vertical fringe or opaque frame introduced by resampling.
- [ ] Navigate with the physical remote. Focus must never launch an app or select an input; OK must activate the intended item.
- [ ] Scroll Home, All apps, Search and long panels. Move the pointer over visible items, then click without moving it: content and the target must remain stationary. Keyboard movement should reveal only focus that leaves the viewport.
- [ ] Check Back, Options, panel close, Search input focus, keyboard navigation and returning to Home. Home must remain behind panels, and hidden controls must not accept clicks.
- [ ] Inspect every clock/icon style, labels, light/dark/automatic appearance and Reduce motion. Verify 12/24-hour formatting, date visibility, midnight/noon and fullscreen clock entry/exit.

## Apps, organisation and saved data

- [ ] Compare All apps with the installed inventory, including removal and unavailable-launch errors. Confirm usable fallback artwork on later pages.
- [ ] Verify curated/favourite selection and successful-launch usage. Inputs and utilities must not crowd the most-used row.
- [ ] Test favourites, hiding/unhiding, ordering and folders: create, add/remove members, rename and delete without removing installed apps.
- [ ] Exercise search, profile/folder names and helper URL entry through the remote keyboard. Check case, punctuation, spaces, delete, Done and Cancel.
- [ ] Relaunch and verify preferences, order, folders, hidden apps, usage and shuffle progress. Corrupt or unsupported saved data must recover safely.
- [ ] Check the selected [app-owned data directory](../native/README.md#native-storage-and-home-reset) through the jailed app, including write ownership and upgrade retention. A root-created file is insufficient evidence.
- [ ] Refresh inventory and input groups independently. Remove stale members from the refreshed group without discarding the other group; verify that the warm cache converges to the TV's current state.

## Inputs and linked-device power

- [ ] Confirm that Down reveals the smaller input row and Up/Back hides it while restoring app focus.
- [ ] Move focus across HDMI entries without changing inputs or waking devices.
- [ ] Select an intended input with OK, verify the switch and return to a usable Home with the configured aerial delay.
- [x] Preserve the confirmed configuration: PS5 HDMI Device Link off, LG SIMPLINK enabled for other devices; normal TV off/on previously left the PS5 off.
- [ ] Repeat the power test after any future native integration change. Record explicit input-selection wake behavior separately from startup behavior.

## Aerials and lifecycle

- [x] Verify visibly moving video behind the UI together with the owned pipeline URL, resolution, codec, resources and advancing playback time for the recorded 4K Dolby Vision trial.
- [ ] Recheck the final package's SDR, HDR and Dolby sources, colour, speed, aspect ratio, clipping and silent playback. Do not present an upscaled HD fallback as native 4K.
- [ ] Test Next, explicit scene selection, filters, quality preferences and complete shuffle/sequential cycles. Failed sources must advance after bounded retries without resetting unplayed progress.
- [ ] Verify an end-of-clip transition on the final artifact, including a new scene, URI/media ID and valid playing resources.
- [ ] Enter a valid LAN helper address; reject credentials, public addresses and unrelated paths. Prepare a Mac 240 fps aerial and verify the successful returned media URL, normal speed and source resolution.
- [ ] Disable moving aerials during preparation/playback and change scenes rapidly. Late replies must not restart superseded media.
- [ ] Leave for another app, LG Settings, screen-off and standby, then return. Check focus, navigation, fresh inventory, startup delay and decoder/compositor ownership; no stale video or permanently stuck loading state.

## OLED care, sound and sleep

- [ ] Confirm that static labels, clocks, panels, captions and keyboards participate in bounded shifting and fade/hide while video remains independently composed.
- [ ] Exercise fade/hide/dim previews. The first key or click must restore the UI without activation; held repeats and pointer motion followed by clicking must be safe.
- [ ] Measure automatic two-minute fade, five-minute hide and thirty-minute dimming on hardware. Previews and simulated clocks do not establish elapsed-time behavior.
- [ ] Verify screen-saver entry, fullscreen clock wake and background/foreground idle reset. Keep LG's built-in OLED care enabled; launcher controls do not guarantee prevention of panel wear.
- [ ] Read and change real volume/mute, then restore the original state.
- [ ] Set and verify a supported sleep timer, cancel it immediately and verify cancellation. Do not leave a test timer active.

## Performance and default-switch gate

- [ ] Record repeated native/web fresh starts with matched app counts, artwork, effects and cache conditions. Report TV wake/app-manager delay separately from UI initialization.
- [ ] Compare navigation and settings without video, then with the same hardware-decoded clip and picture/power settings.
- [ ] Measure physical remote response alongside scripted samples, p95/p99, idle CPU, memory and video resources. State each timing boundary and sample count.
- [ ] Repeat foreground returns and normal off/on cycles; check memory growth, long pauses, duplicate workers and retained resources.
- [ ] Test `--home` reset followed by a SAM foreground request from a native panel, another app and a stopped native process. It must not select an input or create duplicate instances/root-owned saved data.
- [ ] Complete app launching, input safety, persistence, OLED wake, lifecycle and recovery checks before changing Home or startup targets.
- [ ] Keep the web launcher installed for rollback. After any deliberate default switch, separately verify the physical Home button and TV power-on path, including the PS5 arrangement.

Publish artifact-specific outcomes and remaining limits. Unit-test totals, software previews and the historical rendering probe do not establish a complete replacement or a matched performance improvement.
