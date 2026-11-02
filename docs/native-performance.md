# Native performance engineering

Evidence reviewed on 2026-11-02. The saved measurements describe their recorded packages, not subsequent untested changes.

Optimize the measured build and its lifecycle, renderer and data flow. A web-versus-native comparison does not isolate a Rust-language effect: the two implementations have different scheduling and rendering work. Distinguish first branded tiles from fully loaded artwork, visible aerial capture from playback-confirmation checks, and app foreground from loaded third-party content.

## Confirmed target, bounded claims

The development TV has run 32-bit little-endian ARMv7 EABI native builds using `armv7-unknown-linux-gnueabi`. Preserve that calling convention and the matching webOS SDK linker. Rust's [Arm/Linux guide](https://doc.rust-lang.org/stable/rustc/platform-support/arm-linux.html) explains why gnueabi and gnueabihf are different ABIs; a CPU with floating-point hardware does not make them interchangeable.

Saved [native measurements](../native/measurements-v0.2.json) report an accelerated SDL `opengles2` renderer at 1920×1080 logical resolution and independent hardware 4K Dolby Vision playback. One UI RSS observation is approximately 27MB as recorded. It is not peak memory, total decoder/GPU memory or the TV's installed RAM. The renderer name does not identify the GPU model.

An initial standby metadata attempt was unavailable. During a later user-authorized deployment wake, read-only device inspection established these facts without initializing SDL or changing settings:

| Device evidence | Observed result | Limit |
|---|---|---|
| CPU identification | Four online ARMv8 CPUs, implementer `0x41`, part `0xD09`: Cortex-A73 | This identifies the physical CPU family; the application ABI remains 32-bit EABI. |
| CPU frequency reporting | All four cores reported a 1.8GHz current/max frequency, 540MHz minimum and the `performance` governor | A snapshot of firmware reporting, not proof of sustained clocks or thermal headroom. No governor was changed. |
| Device tree | `realtek,rtd2885p` / `rtk,rtd2885p` | A platform identifier, not a claim about a marketing processor name. |
| GPU runtime sysfs | `Mali-G57 1 cores r0p1 0x9091`; Mali kernel driver `r41p0-01eac0` | One GPU core as reported by the driver. Maximum texture size, exposed GLES version and sustained throughput were not measured. |
| Linux memory | `MemTotal` 1,727,856KiB, approximately 1.65GiB | Linux-reported memory, not physical installed DRAM capacity. Available memory varies with workloads. |

The Cortex-A73 interpretation is supported by [Linux's primary CPU ID definitions](https://github.com/torvalds/linux/blob/master/arch/arm64/include/asm/cputype.h) and [Arm's Cortex-A73 technical reference](https://documentation-service.arm.com/static/5e7b6c837158f500bd5c03fc). No new CPU, FPU or SIMD flags follow from this inventory. The cross-compiler's defaults and the build PC/Pi architecture still do not define the TV's ABI. Actual thermal limits, peak application memory and long-running resource pressure remain unmeasured.

## Build configuration

Use the relocated [community webOS SDK](https://github.com/webosbrew/native-toolchain) and the existing `native/build-tv.sh`. The release profile already uses:

```toml
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

Cargo additionally defaults release builds to opt-level 3 with incremental compilation disabled. These are an existing baseline, not a newly measured improvement. [Cargo's profile documentation](https://doc.rust-lang.org/cargo/reference/profiles.html) recommends comparing optimization choices empirically; opt-level 3 is not guaranteed to beat 2, and ThinLTO is not a universal runtime improvement.

Do not use `-C target-cpu=native` for a Windows/Pi cross-build: Rust defines it using the compiler host's CPU. Keep the explicit target; introduce device-specific CPU/features only after confirming hardware support and preserving the ABI. [Rust code-generation options](https://doc.rust-lang.org/rustc/codegen-options/#target-cpu)

The current upstream ARMv7 gnueabi target disables NEON and defaults to software floating-point code generation. An offline probe with the project's compiler emits software helper calls for ordinary `f32` multiply/add. The SDK's C softfp defaults do not change Rust's defaults. Avoid assuming host raster microbenchmarks represent this target. [Upstream target specification](https://github.com/rust-lang/rust/blob/main/compiler/rustc_target/src/spec/targets/armv7_unknown_linux_gnueabi.rs)

## Work that can be reduced safely

- Keep service calls, artwork reads, decoding and preparation away from the UI loop. Bound queues, merge replaceable updates and discard stale results after a Home visit changes.
- Reuse text, rounded surfaces, icon rasters and textures. Do not re-rasterize unchanged state. Prefer packed fills/copies and hoisted geometry over repeated floating-point pixel work; verify visual equivalence before replacing a raster algorithm.
- Keep SDL texture/window operations on their owning render thread. Workers should return bounded CPU data, not mutate renderer objects.
- Track cache cost in bytes. A 1920×1080 RGBA buffer costs 7.91MiB; a 3840×2160 buffer costs 31.64MiB, before extra copies and GPU storage. Retain the separate hardware video path instead of copying aerial frames through CPU UI buffers.
- Bound artwork dimensions and checked allocation arithmetic. A count-limited cache alone does not bound memory when entries have different dimensions.
- Sleep when nothing needs redrawing; use deadlines for input, animations, clock updates and pending UI work. Preserve responsive input and lifecycle processing while hidden.

These are engineering priorities, not claims of an unmeasured speedup. The actual supported native APIs are community maintained; [webOS Homebrew's native-development notes](https://www.webosbrew.org/pages/native-development) should not be confused with an official LG native-TV API contract. SDK headers and cached SDK libraries do not establish runtime compatibility. A read-only string scan of the installed `libSDL2-2.0.so.0.14.0` found `SDL_HINT_WEBOS_REGISTER_APP`, `nativeLifeCycleInterfaceVersion` and both `registerNativeApp`/`registerApp` service paths. This supports compatibility handling for that library; strings alone do not validate the registration handshake.

## Current source improvements

The event loop previously used a zero SDL wait whenever a redraw was pending, even when the window was hidden and could not present that redraw. Hidden/background events themselves mark the UI dirty, so that condition could spin indefinitely. It now waits up to the existing 100ms service/Home-IPC polling interval while hidden; SDL input/window events still interrupt the wait. New artwork requests are deferred until visible.

An unchanged scene no longer redraws once per second. Explicit UI changes still invalidate the frame; the time key additionally tracks the displayed minute/date, appearance-hour changes, OLED shifts/fade/hide/dimming and toast expiry. A regression simulation produces one composition for an otherwise unchanged minute rather than 60. Actual redraw frequency also depends on input, inventory/artwork replies and video state changes. The separate hardware video plane continues presenting aerials independently of UI redraws.

Text uses borrowed cache lookups and a bounded successful-width cache. UTF-8 truncation and tracking reuse string slices instead of allocating prefixes/glyph strings. Font, theme, clipping, colour and raster calculations are retained. The platform worker blocks on its own GLib context with an owned timeout rather than polling every 2ms; see the [platform note](native-platform-research.md) for ownership and lifecycle details.

A counterbalanced Pi AArch64 software-SDL component test of 2,000 warm label passes observed medians of 5,448ms before and 1,557ms after the text-cache changes. Rust heap allocations over 100 warm passes fell from 28,500 to zero. Baseline/candidate 1920×1080 RGB output was byte-identical, including after cache eviction, and Unicode, combining characters, clipping, theme/alpha and NUL-cleaning checks matched. These are component measurements on a different CPU/ABI, not TV frame or startup timings.

An isolated events-only SDL policy probe on the Pi host observed about 588–594ms thread CPU per 600ms busy-wait sample with the old hidden policy, versus about 12–15ms with the bounded wait. A posted event interrupted the latter wait about 0.53ms after posting. This is a short host test of the wait policy, not launcher CPU usage, physical remote latency or TV startup performance.

These changes were subsequently installed in the separate native app during a user-authorized deployment on 2026-11-02. Preserve the earlier package-specific results and run a new matched hardware comparison before assigning startup or app-handoff improvements to this candidate.

Local integration validation passed 56 headless Rust/SDL tests (including the 52 default Rust tests) and 61 Node tests, plus formatting/diff and public-file checks. The extra SDL tests exercise late preference, capability, favourite and colour replies without a service worker, video player or window. The ARMv7 EABI release cross-build and isolated native packaging succeeded. The candidate package SHA-256 is `90990b55dcfc75015b42deb075f02f3cb29033ac24662099d736639f6006a1b1`; it is distinct from the earlier benchmark package. The TV was not woken for the initial source/host validation pass.

The later deployment verified installed binary SHA-256 `62b0b9c9a9d796bda30d71fccaec294f810c1500ba38ae730770409ba2f7b1bd`, normal native foreground, real app artwork and an owned 3840×2160 Dolby Vision pipeline at 29.97fps. A short sample after returning to the original web Home recorded three CPU ticks over 5.74 seconds at 100 ticks/second: approximately 0.52% of one core for the hidden native process. This supports the absence of a continuous busy loop in that sample; it is not a full startup comparison or proof of the app-handoff cause. The native app was stopped, original preferences and permissions restored exactly, Home/power/CEC settings verified unchanged, and TV standby confirmed afterward.

## Validation

Record package hash, target/compiler/SDK, renderer, lifecycle state and process identity. Separate process entry→presentation, wake→visible capture, Enter→SAM dispatch and SAM dispatch→target window. Keep scene/quality, app order, standby method and delay settings matched, while documenting differing scheduling logic. Use medians/ranges with repeated trials; retain failed observer attempts separately.

Observe owned-process RSS/HWM/PSS and resource pressure over time, not one snapshot. [LG Resource Monitor](https://webostv.developer.lge.com/develop/tools/resource-monitor-introduction) supports app/system resource analysis. Keep private inventories, addresses, captures and device logs out of public source. Do not claim a fix for a post-dispatch window delay from CPU compiler flags alone.
