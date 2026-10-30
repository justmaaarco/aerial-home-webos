# Windows aerial wallpaper and screensaver

The separate **Aerial Desktop** package brings Classic, Minimal, Liquid Glass and Immersive clocks to a Windows wallpaper. It runs behind the normal Windows desktop using Wallpaper Engine, and the same wallpaper can run as a screensaver. It does not install the webOS launcher or its TV integrations on Windows.

## Current development setup

The Windows development PC uses Wallpaper Engine 2.8.42. Aerial Desktop is applied to its primary display. Three full-length local aerials are prepared: Iceland Coast, Iceland Snow Caps and Bumpheads, totaling about 27 minutes. Each prepared file is verified as 3840×2160, AV1, 10-bit SDR. Their combined size is about 7.8 GiB. The local preparation cache also retains the original source files.

The clock's **Clock style**, **Time format**, **Clock position**, **Show date**, **Shuffle aerials** and **OLED clock care** controls appear in Wallpaper Engine's Properties sidebar when Aerial Desktop is selected. Choices persist in the host. Glass is the desktop default; Immersive provides larger numerals. The desktop package has no app grid or HDMI controls.

OLED clock care reuses the bounded minute-based shifts, two-minute fade and five-minute hide from Aerial Home. Pointer activity restores the clock and location caption. Wallpaper Engine's pause/resume callback pauses video and resets clock activity on return. These protections cover the wallpaper's own text, not Windows icons, taskbar or other apps. They cannot guarantee protection against panel wear.

The existing host rules pause wallpapers for fullscreen and maximized applications, and startup is enabled. Runtime playback uses local files with no server, download loop, telemetry or sound. The shared clock tick runs every ten seconds; there is no per-frame JavaScript or live backdrop blur. Gaming performance has not been benchmarked.

Windows screensaver configuration uses the existing official Wallpaper Engine screensaver module, with a five-minute idle timeout. Display power-off remains at the PC's pre-existing 15-minute AC setting. Windows manages screensaver timing; a wallpaper is separate from the Windows lock screen. Previous wallpaper and screensaver settings are backed up locally before applying the setup.

## Build and prepare

Requires Node.js 22+, FFmpeg/ffprobe with `av1_nvenc`, an NVIDIA GPU that supports AV1 encoding, and Wallpaper Engine for desktop integration. The host is not bundled.

```sh
node tools/build-desktop.cjs
node tools/prepare-desktop-media.cjs
```

The dedicated package is written to `dist/aerial-desktop/`. Preparation uses three normal-speed 4K SDR sources from the existing Apple catalogue. It creates high-quality local AV1 WebM encodes for the web wallpaper player. This is a **lossy compatibility conversion** that retains source resolution, normal playback rate and 10-bit SDR; it is not lossless and does not enable HDR or Dolby Vision. Source and prepared stream metadata, duration, file size and a streaming SHA-256 checksum are recorded in the generated `media-manifest.json`.

To prepare a different selection, pass catalogue IDs separated by commas. Sources without a normal-speed 4K SDR variant are rejected; this preparation tool does not retime 240 fps Mac assets.

```sh
node tools/prepare-desktop-media.cjs --ids CATALOGUE-ID-ONE,CATALOGUE-ID-TWO
```

An explicit `--dest` may select a different dedicated desktop package; `--cache` selects the source cache directory. Rebuilding a package preserves its videos and playlist. The builder rejects nonempty destinations that are not identified as Aerial Desktop packages.

Import the dedicated HTML package through Wallpaper Engine's web-project workflow, or place it in a local `projects/myprojects/` directory with its `project.json`. The PC development setup uses hard links for local video copies on the same volume to avoid storing identical multi-gigabyte files repeatedly. That is a local installation choice, not required by the portable code.

Optional local browser preview:

```sh
node tools/serve-desktop.cjs
# http://127.0.0.1:5190/?preview=1
```

This development server binds to loopback and supports byte-range requests without loading entire videos into memory. It is not needed by the installed wallpaper.

## Validation and assets

The combined suite has 46 passing tests. Desktop checks cover property validation, clock formatting and OLED behavior, host pause/resume, local-path restrictions, bounded playback failures and shuffle/sequential transitions. Native Wallpaper Engine playback was visually checked with moving footage, Glass and Immersive clocks, live style changes and a manual screensaver start. Host customization controls are present in the native library. Local preview byte-range, invalid-range and traversal requests were checked. Full idle timing and gaming performance require separate observation.

Apple footage, source caches, local settings, private screen captures and generated packages stay outside tracked source. The repository's MIT license covers its code; it does not relicense Apple's media. Prepared videos are local runtime assets, not included in the public repository or TV package.

References: [Wallpaper Engine web guide](https://docs.wallpaperengine.io/en/web/overview.html), [user properties](https://docs.wallpaperengine.io/en/web/customization/properties.html), [pause callback](https://docs.wallpaperengine.io/en/web/api/propertylistener.html), [screensaver setup](https://help.wallpaperengine.io/en/functionality/screensaver.html), [command-line controls](https://help.wallpaperengine.io/en/functionality/cli.html).
