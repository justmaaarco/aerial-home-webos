# Windows aerial wallpaper and screensaver

The separate **Aerial Desktop** package brings Classic, Minimal, Liquid Glass and Immersive clocks to a Windows wallpaper. It runs behind the normal Windows desktop using Wallpaper Engine, and the same wallpaper can run as a screensaver. It does not install the webOS launcher or its TV integrations on Windows.

## Current development setup

The Windows development PC uses Wallpaper Engine 2.8.42. Aerial Desktop is applied to both displays. The full-catalogue mode covers all 216 entries in the repository's eight-manifest Apple snapshot. Each entry is attempted once per shuffled cycle; removed or unplayable Apple files are skipped. Shuffle progress is saved when browser storage is available. Each new start selects the next unplayed entry in that shuffled cycle, without favouring a local clip or restoring the previous clip. Ordinary pause/resume continues the current aerial when it is available. If another display owns the preparation worker, a local or cached aerial plays instead and the blocked scene is deferred. Ready aerials can repeat when all ready entries have already been watched. A failed URL does not reset the cycle. Turning Shuffle off uses catalogue order, except when a ready fallback is needed.

Three full-length local aerials remain available: Iceland Coast, Iceland Snow Caps and Bumpheads, totaling about 27 minutes. They participate in the same shuffle as the rest of the catalogue. Each prepared file is verified as 3840×2160, AV1, 10-bit SDR. Their combined size is about 7.8 GiB. The local preparation cache also retains the original source files.

The desktop wallpaper hides the clock and date. A separate **Aerial Desktop Screensaver** package shows them, with **Clock style**, **Time format**, **Clock position**, **Show date**, **Shuffle aerials** and **OLED clock care** controls in Wallpaper Engine's Properties sidebar. Choices persist in the host. Glass is the screensaver default; Immersive provides larger numerals. Neither package has an app grid or HDMI controls.

Both packages have **Next aerial (toggle to skip)** in their Properties sidebar. Each checkbox change advances once; the initial complete settings batch does not skip. The first click also works when the host omits the initial Next value. Advancing cancels the previous online stream before starting another, including when the wallpaper is paused behind an application. Updated packages use resource hashes so the browser loads the current player code.

Build the clock-enabled screensaver with `node tools/build-desktop.cjs dist/aerial-screensaver --catalogue --screensaver`. Configure Wallpaper Engine's screensaver separately and select that package, retaining Aerial Desktop for the wallpaper. Both packages use the same catalogue helper. The development PC shares the existing local videos through hard links, without duplicating their storage. Browser preview (`?preview=1`) also shows the clock for customization.

OLED clock care reuses the bounded minute-based shifts, two-minute fade and five-minute hide from Aerial Home. Pointer activity restores the clock and location caption. Wallpaper Engine's pause/resume callback pauses video and resets clock activity on return. These protections cover the wallpaper's own text, not Windows icons, taskbar or other apps. They cannot guarantee protection against panel wear.

The existing host rules pause wallpapers for fullscreen and maximized applications. A separate per-user task starts Wallpaper Engine 64-bit quietly ten seconds after sign-in. Local-only mode needs no server. Full-catalogue mode uses a read-only loopback helper and on-demand NVIDIA decoding/AV1 encoding, paced slightly ahead of playback. Pausing releases the video decoder, aborts pending info requests and cancels online encoding; resuming reloads playback at its saved position with a fresh loading timeout. Host pause and document visibility are tracked independently, so a visibility event cannot override the gaming pause. Info requests have an eight-second timeout and can fall back to local footage. A five-second health check retries playback after twenty seconds without progress, then skips a persistently frozen scene. There is no background bulk download, external telemetry or sound. The shared clock tick runs every ten seconds; there is no per-frame JavaScript or live backdrop blur. Gaming performance has not been benchmarked.

Windows screensaver configuration uses the existing official Wallpaper Engine screensaver module, with a five-minute idle timeout. Display power-off remains at the PC's pre-existing 15-minute AC setting. Windows manages screensaver timing; a wallpaper is separate from the Windows lock screen. Previous wallpaper and screensaver settings are backed up locally before applying the setup.

## Full catalogue

The helper selects the highest available SDR source for each scene, preserving source resolution and bit depth. The snapshot includes 126 normal-speed 4K scenes, another 38 requiring Mac 240→29.97 fps timing correction, and 52 older HD-only scenes. HD is not upscaled. Some Apple URLs no longer exist; a catalogue record does not guarantee a reachable video. Compatibility conversion to AV1 is lossy. Desktop playback remains SDR, without HDR or Dolby Vision.

Prepare local startup clips first if desired, then enable the catalogue and install its Windows helper:

```powershell
node tools/build-desktop.cjs --catalogue
./tools/install-desktop-helper.ps1
```

The installer copies the helper and catalogue to `AerialDesktop/helper` in the user's Documents folder and registers a per-user logon task. Documents is used so the Windows task can access the same files when the installer runs from a packaged desktop app. The hidden launcher waits for its Node process; the task uses the signed-in user's limited token, has no execution timeout and retries a failed launch up to three times. It requires Node.js and an FFmpeg build with CUDA decoding, the `realtime` filter and `av1_nvenc`, plus a compatible NVIDIA GPU. It does not elevate privileges or change execution policy. Import the rebuilt package into Wallpaper Engine. Restart the helper by rerunning the installer after source updates; rebuild the package after catalogue changes.

For a delayed host launch independent of Wallpaper Engine's normal Run entry:

```powershell
./tools/install-desktop-startup.ps1
# For a different Steam library, pass -WallpaperRoot '<Wallpaper Engine folder>'.
```

This registers `Aerial Desktop Wallpaper-…` for the current user's sign-in, using a limited interactive token and a ten-second delay. The hidden launcher waits up to fifteen seconds for the catalogue helper, then starts `wallpaper64.exe -silent` from its installation directory. It uses Wallpaper Engine's saved display assignments and avoids starting another copy if either architecture is already running from that installation. A failed launch can retry up to three times. The installer backs up the original normal startup command in `AerialDesktop/before-wallpaper-startup.json`, and removes only a matching `WallpaperEngine` Run entry after registering the task. No elevation, execution-policy or antivirus changes are made. The exact task name is saved in `AerialDesktop/helper/wallpaper-task-name.txt`. To use Wallpaper Engine's built-in startup again, disable this task and enable the app's normal startup option. On the development PC the task was tested with no host running: both saved displays reached aerial playback. Running it again returned success without changing the host PID or creating another copy. An actual reboot/sign-in remains to be observed.

The service binds only to `127.0.0.1:5194`. It accepts catalogue IDs and a bounded playback offset, never arbitrary URLs, paths, or commands. Source URLs are taken exclusively from the bundled Apple catalogue. `GET /api/status` reports catalogue size, cache usage, the active worker and up to eight recent player states. A bounded `POST /api/client` endpoint records playback selection and Next counters in memory for local diagnosis; it exposes no control commands and sends no diagnostics externally. There is one preparation worker. Source failures are held for 24 hours; an interrupted request is cancelled rather than treated as a missing source.

Completed, indexed cache files are capped at **4 GiB**, in addition to the existing local videos. A single cacheable online clip is limited to 2 GiB; larger clips still play but are not retained. Temporary output can add up to 2 GiB while streaming. Completed streams are losslessly remuxed to add WebM duration and seek metadata before entering the cache. Only owned cache filenames are pruned; local startup videos are retained. With the three existing startup clips, video storage peaks around 13.8 GiB, excluding the separate original-source preparation cache. A complete 216-entry cycle exceeds the cache, so videos will be fetched again on later cycles. Network access is required for uncached footage.

To return to local-only playback, use the backed-up package or build a separate fresh package without `--catalogue` and prepare its local playlist. Disable the `Aerial Desktop Catalogue-…` task in Windows Task Scheduler to stop automatic helper startup; its exact name is saved in `AerialDesktop/helper/task-name.txt` in Documents. The installer retires its older Startup shortcut after registering the task. The original wallpaper and screensaver settings are separately backed up on the development PC.

## Build and prepare local clips

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

The combined suite has 59 passing tests. Desktop checks cover property validation, clock formatting and OLED behavior, host pause/resume, source restrictions, bounded playback failures, complete 216-entry cycles across a failure and reload, unbiased startup selection, repeatable Next controls (including the first click and while paused), online skip/pause cancellation, bounded diagnostics, source selection/retiming, and helper HTTP boundaries. Regression checks include a simulated three-hour pause, restored decoder timeouts, independent host/visibility pauses, frozen playback recovery, a busy encoder with deferred shuffle progress, and aborted or timed-out info requests. Two consecutive Next changes were previously verified in the installed player. Both installed displays now reached playback after three native pause/play cycles with local footage, plus a fourth cycle with streamed Great Wall footage and a ready fallback on the other display. Pausing released the preparation worker and resuming reached playback on both displays. This exercises the host callback rather than a complete gaming session. No test popout was left running. Native Wallpaper Engine playback was previously visually checked with moving Mac aerial footage in 4K 10-bit SDR and the Glass clock. Legacy HD streaming, source dimensions/bit depth/frame rate, cancelled worker cleanup and WebM indexing were checked separately. Earlier checks covered Immersive clocks, native customization controls and a manual screensaver start. A complete multi-hour catalogue cycle, full idle timing and gaming performance require separate observation.

Apple footage, source caches, local settings, private screen captures and generated packages stay outside tracked source. The repository's MIT license covers its code; it does not relicense Apple's media. Prepared videos are local runtime assets, not included in the public repository or TV package.

References: [Wallpaper Engine web guide](https://docs.wallpaperengine.io/en/web/overview.html), [user properties](https://docs.wallpaperengine.io/en/web/customization/properties.html), [pause callback](https://docs.wallpaperengine.io/en/web/api/propertylistener.html), [screensaver setup](https://help.wallpaperengine.io/en/functionality/screensaver.html), [command-line controls](https://help.wallpaperengine.io/en/functionality/cli.html).
