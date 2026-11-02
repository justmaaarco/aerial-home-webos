# Native Home and power routing

The development TV now uses the Rust launcher for Home and normal power-on. The legacy TV browser app `io.github.justmaaarco.aerialhome` has been uninstalled. The native app retains its established ID, `io.github.justmaaarco.aerialhome.nativeprobe`, so its persistent data and installation identity remain stable. LG's built-in Home is retained.

The UI, navigation and video control run in Rust/SDL. Two small Node programs integrate that app with the existing rooted-TV input/power hooks; they do not render a browser UI:

- [`open-native.cjs`](../tools/native-home/open-native.cjs) sends the native binary's `--home` IPC reset, then asks SAM to foreground the native app. A failed reset or cancelled power transition does not launch an input or another launcher.
- [`watch.cjs`](../tools/native-home/watch.cjs) subscribes to TV power state. A real standby/off-to-Active transition calls the same helper. Screen-off and screen-saver events do not count as new power-ons.

The validated installation stores these as `/var/lib/luma-power/open-native.cjs` and `/var/lib/luma-power/watch.cjs`, using the existing power hook. Home key 773 invokes the helper through Input Hook. The legacy directory name is retained for the existing owned hook; it does not run the removed web launcher. Neither program selects HDMI or changes CEC settings.

This is a tested integration for the development TV's webOS9.2.4-48 firmware and native0.2.0 installation, not a universal root installer. The source programs check those prerequisites. Installing the IPK alone does not apply Home/power changes or uninstall another app. The private migration transport and backups contain device connection details and are not published.

## Migration and recovery

Current browser preferences were copied into native storage, including Home columns, selection/order/usage, hidden apps, clocks/icons, aerial options, helper settings and OLED care. Web `legacyHD` maps to native `legacyHd`. The browser's in-memory shuffle sequence and playhead do not have a persistent queue to transfer.

An offline verified web IPK, exact web preference export, native preferences and previous startup/key files were archived before removal. To roll back this particular migration, restore the archived web package and preferences **before** restoring startup routes that name its app ID. No web launcher needs to remain installed for normal native operation.

The Windows wallpaper/screensaver and shared catalogue, media, core and font assets remain. Repository browser previews are legacy/reference material, not the installed TV launcher. CI checks native and shared/desktop behavior and no longer publishes a web-TV IPK automatically. Build native with the matching SDK as described in the [native guide](../native/README.md); `npm run build` packages that ARM binary.

## Verification scope

The native Home helper was checked from a native control panel and from Netflix after actual remote-style OK activation. Normal standby/power-on reached the native app. The exact legacy web app was removed through Homebrew Channel's app installer; native installation and the built-in LG Home remain. These checks do not establish every item in the [native validation checklist](native-testing.md), and HDMI-CEC devices can still request their own input independently of the launcher.
