# Version 0.2.0 validation and scope

This alpha was developed on 22 October 2026. tvOS 26 is the selected visual reference, not a promise of feature or pixel identity.

## Verified

- Thirteen automated tests cover core behavior, input sanitization, adaptive app selection, greeting boundaries, shuffle cycles, capability-based Dolby selection, catalogue provenance and the helper's HTTP/file-access boundaries.
- Catalogue snapshot: 216 merged entries, 831 variants across eight official Apple manifests. Metadata probes found 114 reachable 4K Dolby Vision profile 5 URLs and 13 removed legacy HDR URLs. This does not mean every catalogue video was watched on every TV model.
- Physical TV: native 4K Dolby Vision footage reached `play` with hardware VDEC and Dolby DM resources; the reviewed Bumpheads display capture had correct-looking colour.
- Physical TV: newer Goa Beaches footage played from the Pi helper. ffprobe verified the prepared file retained 3840×2160, HEVC, 10-bit SDR and approximately 29.97 fps. The TV allocated hardware VDEC and displayed moving footage.
- Greetings and the compact home selection appeared on the TV. Existing five-column layout and hidden apps were preserved.
- Browser: All apps, catalogue count, shuffle toggling and immediate menu OK handling checked. A single OK press toggled shuffle once without also toggling video background.

## Remaining menu lag

The user still reports lag in both Aerial Home and LG Settings. Removing repeated grid rebuilds and live TV backdrop blur did not resolve their experience. Instrumentation measured roughly 165–210 ms to the second animation frame while video played, versus 36–44 ms with video already paused/unloaded. Those are software scheduling measurements, not a full input-to-photon measurement.

Pausing video behind both launcher menus and LG Quick Settings was tested. The TV confirmed play → pause → play for the LG overlay, but the user reported no improvement and requested removal of the static aerial. **The pause-on-menu behavior was removed. Aerials continue moving behind menus. Menu lag is not claimed fixed.** Other launchers were checked and were not running alongside Aerial Home. Normal release of video on app backgrounding remains enabled.

## Earlier validation retained

- Core tests: malformed saved preferences; ordering, folders, hidden-app search; partial grid navigation.
- Adapter tests: permission-error propagation and bridge cleanup; Homebrew fallback and input discovery; rejection of unsafe artwork paths.
- IPK structure: independent ar/tar parsing, no path traversal/symlinks/install scripts; transfer hash comparison and installed event.
- Physical LG B4 / webOS 9.2.4: application foreground; installed application artwork and titles visible; Apple New York aerial in `play` state with allocated hardware VDEC; 1920×1080 display capture reviewed.
- Source video dimensions and codec were verified using ffprobe: 3840×2160, HEVC, 10-bit SDR, 29.97 fps.
- Browser UI: folder creation, membership editing, persistence after reload, removal, Control Center navigation and input picker.
- Physical TV: Control Center opened through a launch parameter and displayed the real volume returned by the TV.
- Remote key injection on the physical TV: Back closed Control Center, arrows selected Apple TV, and OK launched `com.apple.appletv`. The launcher then had no remaining video pipeline. Relaunching Aerial Home restored its interface.

## Manual validation still needed

Physical-remote feel, pointer long-press behavior, folder/reorder persistence across TV restarts, audio output combinations and native sleep-timer expiration need broader testing. A working service response is not the same as listening to the speakers or waiting for a sleep timer to expire.

## Scope limits

| Feature | Current behavior |
| --- | --- |
| Home grid and focus | Independent layout and CSS approximation; not measured pixel parity |
| Liquid Glass | CSS transparency, blur, highlight and shadows; no Apple's renderer or optical refraction shader |
| Aerials | Eight-manifest snapshot; per-scene maximum compatible quality; some Apple files have been removed |
| New Mac aerials | Optional LAN helper for lossless 240→29.97 fps retiming; initial preparation and local storage required |
| Most used | Starts with Luma favourites/curated brands and learns launches from Aerial Home, not private historical LG analytics |
| Profiles | Local display name only; not multiple layouts or streaming account switching |
| App switcher / Top Shelf | Not implemented; no invented watch recommendations |
| Control Center | Launcher panel; cannot draw over another running LG app |
| Home / Power | Installation does not remap either button; optional separate integration required |
| Siri, FaceTime, HomeKit, Sing | Not implemented; no private Apple services |
| AirPlay | No new AirPlay implementation; TV's existing functionality is separate |
| Rendering | 1080p UI plane, 4K video plane on UHD webOS TVs |

Reference: [Apple's tvOS 26 introduction](https://www.apple.com/newsroom/2025/06/apple-tv-brings-a-beautiful-redesign-and-enhanced-home-entertainment-experience/) and [LG's app resolution specification](https://webostv.developer.lge.com/develop/specifications/app-resolution).

Future work includes resolving the reported menu latency, measured visual comparison against a matching tvOS 26 reference, richer local profiles and broader device testing. Those items are not advertised as already working.
