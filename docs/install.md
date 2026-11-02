# Installation and recovery

## Native TV package

The primary TV implementation is **Aerial Home Native**, app ID `io.github.justmaaarco.aerialhome.nativeprobe`. The historical ID is retained for upgrades and saved-data compatibility. The legacy browser TV app has a different ID: `io.github.justmaaarco.aerialhome`.

Build the native ARM binary with the relocated webOS SDK, then package it:

```sh
rustup target add armv7-unknown-linux-gnueabi
export WEBOS_SDK=/path/to/relocated/arm-webos-linux-gnueabi_sdk-buildroot
sh native/build-tv.sh
node tools/package-native.cjs native/target/armv7-unknown-linux-gnueabi/release/aerial-native-probe
```

See [native build details](../native/README.md#build-and-package). The IPK and `.sha256` file are written to `dist/`; compare the checksum with the intended artifact before installation. Older web alpha downloads are legacy packages.

Use webOS Dev Manager's package installer with a TV connection that is already configured, or the LG CLI:

```sh
ares-install --device YOUR_TV dist/io.github.justmaaarco.aerialhome.nativeprobe_0.2.0_arm.ipk
ares-launch --device YOUR_TV io.github.justmaaarco.aerialhome.nativeprobe
```

TV inventory, artwork and control integration require an already-rooted TV and Homebrew Channel. Installation does not root the TV, enable SSH, install keys, replace LG Home or install boot hooks.

For a known rooted TV, Homebrew Channel can install a reachable IPK with its SHA-256 checksum through `org.webosbrew.hbchannel.service/install`. See the [Homebrew command reference](https://www.webosbrew.org/pages/commands-cheatsheet). Do not expose the TV's Luna/SSH services to the internet.

## Home/power migration and the legacy TV app

Installing the native IPK does not automatically configure Home or power-on. On the development TV, the separately managed integration now targets native for both, and the legacy web TV package was removed after checking native Home routing and normal power cycles. See the [routing helpers and verification scope](native-home.md). This was a user-requested migration; it does not mark the remaining [hardware checklist](native-testing.md) complete.

For another existing installation, keep a private backup of its package, compatible preferences and integration configuration. Import compatible settings into native, verify app launching and its Home/power routes, and only then retire the exact legacy package:

```sh
ares-install --device YOUR_TV --remove io.github.justmaaarco.aerialhome
```

This removes only the legacy web TV app. Retain `io.github.justmaaarco.aerialhome.nativeprobe`, LG's built-in Home and Homebrew Channel. The Windows wallpaper/screensaver is independent.

The repository's `app/` directory remains shared source, not an installed-package inventory: desktop builds use its core, media, catalogue and fonts, and native packaging uses its catalogue and icon. Keep those assets even when the old TV app is uninstalled. The browser UI, preview tools and explicit `node tools/package.cjs` build remain available as legacy material.

## Recovery and upgrades

Native preferences, inventory and artwork live in the [app-owned data directory](../native/README.md#native-storage-and-home-reset), outside the installed payload. Keep that directory during native upgrades and verify ownership through the jailed app. Legacy web preferences used the `aerial-home-v1` localStorage key; any migration backup belongs outside public source.

The development TV no longer keeps the web app installed for recovery. Use the private offline package/preferences/integration archive if a deliberate rollback is needed: reinstall the archived package, restore compatible data and repoint the separate Home/power integration. Repoint an integration before removing its target. Keep LG's built-in Home available.

To uninstall native itself after removing or repointing its integrations:

```sh
ares-install --device YOUR_TV --remove io.github.justmaaarco.aerialhome.nativeprobe
```

Apple TV aerials stream directly from Apple. Newer 240 fps Mac aerials need the optional [local media helper](media-helper.md). Native releases its video path when hidden. See the [native implementation](../native/README.md) for lifecycle and validation limits.

Legacy browser integrations could launch panels using parameters such as `{"screen":"controls"}`, plus `sceneId` and `mediaServer`. These browser launch parameters are not a native integration contract. Native's `--home` command only resets an existing native UI; foregrounding it requires a separate app-manager launch. Helper addresses and private device configuration stay local.
