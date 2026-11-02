# Native Rust rendering experiment

This is a separate, installed hardware probe, not a replacement launcher. Its app ID is `io.github.justmaaarco.aerialhome.nativeprobe`; production remains `io.github.justmaaarco.aerialhome`. The probe does not change startup, Home-button bindings, TV settings or HDMI power behavior.

It draws a ten-tile placeholder menu using the TV's SDL2/OpenGL ES renderer. Arrows move focus, OK highlights a placeholder, and Back or Exit closes it. It never launches a streaming app or HDMI input. The manual test exits after five minutes; `--benchmark` exits after twenty seconds. It has no aerial decoder, real app artwork, clock styles, settings persistence, app inventory, or feature parity with Aerial Home. These must be integrated and measured before considering migration.

## Build

Use the [webOS native toolchain](https://github.com/webosbrew/native-toolchain), an SDK matching the build host and Rust's `armv7-unknown-linux-gnueabi` target. The development B4 has a 64-bit kernel but 32-bit ARM soft-float applications. Do not build an aarch64 or hard-float TV executable based only on `uname`.

```sh
rustup target add armv7-unknown-linux-gnueabi
export WEBOS_SDK=/path/to/relocated/arm-webos-linux-gnueabi_sdk-buildroot
sh native/build-tv.sh
node tools/package-native.cjs native/target/armv7-unknown-linux-gnueabi/release/aerial-native-probe
```

`build-tv.sh` runs host unit tests and cross-compiles the native executable. `package-native.cjs` checks the ELF architecture and emits a separate ARM IPK with executable permissions. It contains no install/startup scripts or Apple media. Generated packages and target directories are ignored by Git. No external Rust crates are required.

The SDK's older libc link stubs lack `getauxval`, even though the development TV has glibc 2.35. The compatibility implementation reads actual kernel auxiliary records, preserves successful-call errno, and reports a missing key correctly. It does not invent CPU capabilities. A host-side test checks page size and errno behavior. [Luma's compatibility implementation](https://github.com/ecklf/lg-luma/blob/main/src/webos-compat.rs) documents the same SDK limitation.

## First hardware result — 2 November 2026

Built with Rust 1.99.1 and the `2026.08-webos` aarch64-hosted SDK. Tested on the development LG B4, webOS 9.2.4, with the `opengles2` accelerated renderer and a 1920×1080 logical UI. The package was installed separately, run once through the native jail, allowed to exit normally, and Aerial Home was restored afterward.

| Measurement | First run |
| --- | ---: |
| Native program entry to first presentation return | 275.272 ms |
| Scripted navigation/presentation samples | 222 |
| Median scripted update to presentation return | 7 ms |
| p95 scripted update to presentation return | 9 ms |
| Process RSS sampled about three seconds after launch | 23,348 KiB |
| Release ARM executable size | 339,036 bytes |

The script generates updates in the native event loop; these are **not physical remote-to-photon measurements**. Timing stops when `SDL_RenderPresent` returns, not when the display emits light. First presentation excludes TV wake, app-manager startup and jail setup. This simple bitmap-text interface has no video or expensive effects. Its numbers are not directly comparable to the existing web launcher's complete layout, artwork or earlier two-frame scheduling tests. One successful native run establishes feasibility, not a demonstrated full-launcher speedup. Three Rust tests cover navigation, timing summaries and auxiliary-value compatibility; the Node suite separately checks package isolation and executable mode.

## Next comparison

Measure a matched native/web menu with the same artwork, dimensions and effects, both without video and with the same hardware-decoded aerial. Collect repeated fresh starts, foreground returns, physical remote navigation, p95/p99 scheduling, idle CPU and memory. Test native hardware video composition and HDR/Dolby behavior before porting the full design. Rust WebAssembly inside the current web page would retain that web rendering/composition path and would not establish the benefit of a native renderer.

Home and power-on should continue to target the existing launcher until the native candidate has app launching, inputs, video, settings, lifecycle and OLED behavior validated on the TV. Native development uses community APIs rather than LG's supported web-app workflow; see [webOS Homebrew native development](https://www.webosbrew.org/pages/native-development).
