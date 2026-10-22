# Version 0.1.0 validation and scope

This alpha was developed on 22 October 2026. tvOS 26 is the selected visual reference, not a promise of feature or pixel identity.

## Verified

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
| Aerials | Three direct Apple city streams, no downloads/cache/category catalogue |
| Profiles | Local display name only; not multiple layouts or streaming account switching |
| App switcher / Top Shelf | Not implemented; no invented watch recommendations |
| Control Center | Launcher panel; cannot draw over another running LG app |
| Home / Power | Installation does not remap either button; optional separate integration required |
| Siri, FaceTime, HomeKit, Sing | Not implemented; no private Apple services |
| AirPlay | No new AirPlay implementation; TV's existing functionality is separate |
| Rendering | 1080p UI plane, 4K video plane on UHD webOS TVs |

Reference: [Apple's tvOS 26 introduction](https://www.apple.com/newsroom/2025/06/apple-tv-brings-a-beautiful-redesign-and-enhanced-home-entertainment-experience/) and [LG's app resolution specification](https://webostv.developer.lge.com/develop/specifications/app-resolution).

Future work includes measured layout comparisons against a matching tvOS 26 reference, more aerial categories, optional local media hosting/caching, richer local profiles and broader device testing. Those items are not advertised as already working.
