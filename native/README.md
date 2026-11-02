# Aerial Home Native — 0.2.0 alpha

Aerial Home Native is the primary TV implementation, using Rust, SDL2 and OpenGL ES for rooted LG webOS TVs. Its existing app ID `io.github.justmaaarco.aerialhome.nativeprobe` is retained for saved-data and upgrade compatibility; the installed title is **Aerial Home Native**. The development TV's separately configured Home/power integration now targets native, and the legacy web TV package was removed. Package installation itself does not configure those bindings or change HDMI power-link settings.

This is an alpha implementation with outstanding validation on individual flows and other TV models. It follows Aerial Home's existing design and does not promise pixel identity or feature equivalence with tvOS. An earlier 0.2.0 build has a matched comparison on one LG B4; subsequent source optimizations still need a fresh comparison before claiming their startup or handoff benefit.

The current performance pass fixes hidden-window busy polling, defers new artwork work while hidden, redraws time-dependent UI only when visible state changes, caches text measurements and uses bounded event-driven service waits. Its package was deployed and smoke-checked on the development TV on 2026-11-02, followed by the user-requested native-default migration. See [performance engineering](../docs/native-performance.md), [platform research](../docs/native-platform-research.md) and the [migration record](../docs/native-testing.md#development-tv-migration) for evidence, limits and validation.

## Architecture and interface

The UI is a native Rust process rather than a Chromium page. It follows the retained legacy `app/style.css`: the Home identity, three icon controls, Inter typography, brand-colour artwork tiles, focus treatment, input row, compact panels and clock styles. Bundled static Inter faces come from the shared variable font and retain its OFL license. Shared browser assets remain available for the PC wallpaper/screensaver and native packaging.

SDL textures cache text, rounded surfaces, static glass highlights and artwork. Icons preserve aspect ratio, clip rounded edges and use app metadata for preferred artwork and brand colours. Home, library, search and panel scrolling retain their offsets when pointer focus changes. No effect samples or blurs the live video plane.

In-process Luna Service 2 requests supply TV integration. Inventory, controls, artwork and optional helper preparation run away from the rendering thread. Aerials use LG's hardware uMedia pipeline and an exported compositor window behind the UI. Rust dependencies are pinned in `Cargo.lock`; Node.js packages the app but is not its UI runtime.

## Implemented features

- Real apps and artwork, cached inventory, most-used or favourite Home selection, All apps and search.
- Favourites, hidden apps, ordering, folders with membership/rename/delete controls, and a local profile name.
- A smaller input row revealed with Down, plus an input panel. Moving focus does not select an input; OK does.
- A remote keyboard with case switching and URL punctuation for search, naming and helper configuration.
- Classic, Minimal, Liquid Glass and Immersive clocks, 12/24-hour formatting, date controls, a fullscreen clock and automatic screen saver.
- Classic, Glass, Minimal and Monochrome icon treatments, label preferences, light/dark/automatic appearance and Reduce motion.
- Aerial selection, categories, saved shuffle progress, capability-aware SDR/HDR/Dolby selection and source fallback. Sources below 1440p are excluded. Mac 240 fps aerials require the optional LAN helper and a successful prepared-media response.
- TV volume/mute, a sleep timer and access to LG Settings.
- OLED care with bounded shifting, two-minute UI fading, five-minute hiding and a 97% dark overlay after thirty idle minutes. The first waking input is consumed, including held-key repeats.

These are source capabilities. Physical remote operation, persistence, lifecycle and the complete catalogue still have the separate [validation checklist](../docs/native-testing.md).

## Recorded validation

The development TV is an LG B4 with 32-bit ARM soft-float application userspace. The following historical visual-port record uses package SHA-256 `6842489f2c16adf3dc55f8a8b38a072024a563a75a3e28f0c78234f90bdf87a6`.

| Check | Recorded result |
| --- | --- |
| Native lifecycle | Normal app-manager launch verified separately; final scripted trial ran under the native app jail and exited cleanly |
| Inventory | 31 app/input entries |
| First UI presentation | 454.558 ms |
| First app grid | 502.049 ms |
| First available artwork frame | 668.951 ms; one icon available at that frame |
| Scripted update to presentation return | 207 samples; median 2 ms, p95 4 ms |
| Process RSS | Approximately 27 MB in one sample |
| Native video | Moving 3840×2160 New York Skyline aerial; Dolby Vision `dvh1`, 29.97 fps |
| Media ownership/progress | VDEC, Dolby processing and main scaler resources; playback time advanced |
| Offline visual checks | Home, Settings, Control Center, Glass clock and Immersive clock passed |

The initialization timer starts before SDL initialization, after local Home IPC setup, and ends when `SDL_RenderPresent` returns for the corresponding frame. It excludes TV wake, app-manager launch and jail setup. Scripted samples are not physical remote-to-photon measurements. One run and one RSS sample do not establish sustained performance or a native/web speedup. Per-artifact records are kept in [measurements-v0.2.json](measurements-v0.2.json).

That visual-port build passed **41 Rust tests**, 14 library and 27 binary, alongside **61 Node tests**. The transparent-letterbox correction passed seven raster tests and an accelerated GPU recheck: app artwork edges were clean and the Dolby Vision badge matched the independently verified pipeline. The optimized package passed **56 headless Rust/SDL tests**, 23 library and 33 binary, including the 52 default Rust tests, plus 61 Node tests. Four later Home/power integration tests bring the Node suite to 65. Do not transfer the historical table's timings to a different package hash.

The catalogue contains 216 entries; 164 have a source of at least 1440p eligible under the tested capabilities/helper configuration. Eligibility does not establish successful playback of every URL or format. An end-of-clip transition to another verified 4K Dolby Vision pipeline has been exercised separately; a full shuffle cycle and final-package lifecycle checks remain outstanding.

## Offline previews

The same drawing code can render local fixture data into an owned 1920×1080 software surface. Preview initializes no SDL video subsystem, window, native video player or exported window. It uses an idle worker, makes no TV service requests and does not read or write the installed launcher's preferences. Video-initialization assertions run around rendering and capture.

```sh
./aerial-native-probe --preview /path/to/preview/home.ppm \
  --fixture /path/to/preview/fixture.json --screen home \
  --clock glass --icons classic --appearance dark \
  --background /path/to/preview/background.png
```

The fixture contains `apps`, an `icons` path map and `favorites`. JSON is limited to 2 MiB and 260 apps. IDs, titles, colours and favourites are validated; icons must be PNG/JPEG, at most 4 MiB, with relative paths that resolve inside the fixture directory without symlinks or traversal. Keep fixtures, real inventories, artwork and captures private. Software previews verify layout and raster output; they do not measure the accelerated TV renderer or HDR output.

## Native storage and Home reset

Persistent data lives outside the installed payload in an app-owned directory. On TV, the resolver considers `/media/developer/io.github.justmaaarco.aerialhome.nativeprobe`, the corresponding path under `/media/developer/temp`, and `/media/internal/.io.github.justmaaarco.aerialhome.nativeprobe`. Ownership and write access determine the usable location. Temporary storage is a fallback, not a persistence guarantee.

The directory contains `preferences.json`, a separately revalidated `apps.json`, bounded `icons/` artwork and the owner-only `home.sock`. App and input inventory groups refresh independently. Keep this directory during upgrades; verify reads and writes through the jailed app rather than a root shell. Development hosts use `XDG_CONFIG_HOME` or the user's `.config` directory under the native app ID. Private inventories, helper details and saved data do not belong in public source.

The binary's `--home` mode sends a best-effort local reset and exits. A running launcher returns from panels to Home without activating an app or input. This command does not foreground or launch the app; a Home integration must request it separately through SAM. Root-side clients reuse an existing app-owned directory rather than creating one. The development TV's Home and power-on targets now use native. These integrations remain separate from the app package; see the [routing helpers and verification scope](../docs/native-home.md).

Native startup does not select an HDMI input. HDMI power linking is separate from launcher rendering: a later benchmark observed PS5 CEC input takeover, so the earlier console-side setting is not a guarantee of current behavior. SIMPLINK was temporarily isolated for that comparison and restored afterward. Preserve the user's other devices' power linking when diagnosing the console separately.

## Build and package

Use the [webOS native toolchain](https://github.com/webosbrew/native-toolchain), a relocated SDK matching the build host and Rust's `armv7-unknown-linux-gnueabi` target. A 64-bit TV kernel does not imply 64-bit application userspace.

```sh
rustup target add armv7-unknown-linux-gnueabi
export WEBOS_SDK=/path/to/relocated/arm-webos-linux-gnueabi_sdk-buildroot
sh native/build-tv.sh
node tools/package-native.cjs native/target/armv7-unknown-linux-gnueabi/release/aerial-native-probe
```

The build script runs locked host tests, cross-compiles the SDL build and prints the ELF header. The packager checks 32-bit ARM ELF architecture and emits a separate native IPK with executable permissions, catalogue metadata, fonts, project license and third-party notices. It includes no startup scripts, Apple video files or installed-app artwork. Packages and build directories are ignored by Git. Font regeneration is documented in [fonts/README.md](fonts/README.md).

The SDK lacks a `getauxval` link stub used by Rust, although the tested TV provides the underlying functionality. The compatibility implementation reads kernel auxiliary records and is covered by host tests; [Luma documents the same SDK limitation](https://github.com/ecklf/lg-luma/blob/main/src/webos-compat.rs). Native development uses community APIs; see [webOS Homebrew native development](https://www.webosbrew.org/pages/native-development).

Normal launches have no fixed timeout. `--benchmark` performs scripted navigation for twenty seconds and exits. The legacy web TV app is no longer installed on the development TV; recovery uses a private offline package/preferences/integration archive and the retained LG Home. See [installation and recovery](../docs/install.md). The user-requested default switch does not complete the remaining [hardware checks](../docs/native-testing.md) or establish a new performance result.

## Historical probe

The initial 0.1.0 experiment used ten placeholder tiles and had no aerial video, real artwork or TV integration. Its [historical measurements](measurements.json) establish that native accelerated rendering ran on the TV; they are not results for the current native candidate.
