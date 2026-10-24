# Icon styles

Prepared in the local 0.2.5 candidate alongside clock styles. These additions have not been installed on the development TV or publicly released. Existing installations default to Classic and labels on focus.

Open **Control Center → Settings → Icon style**. Clock and icon choices are saved independently.

| Style | Treatment |
| --- | --- |
| Classic | Original app artwork, tile backgrounds and focus treatment |
| Liquid Glass | Inset artwork inside translucent tiles with a static highlight and fine edge |
| Minimal | Inset artwork, quieter surfaces and a simple focus outline |
| Monochrome | Grayscale artwork and glyphs on neutral tiles |

Styles apply to app tiles on Home, All apps and folder contents, folder tiles, and HDMI/input tiles. Toolbar and settings control icons keep their existing appearance. **Home labels** switches between On focus and Always visible; All apps, folder contents and inputs keep their existing visible labels. App buttons retain their accessible names in every style.

The styles use existing artwork files and cache entries. Opaque backgrounds within an app's supplied image remain visible; no background removal or replacement icon pack is applied. Tile geometry and remote navigation remain unchanged. Selecting a style only updates the saved appearance and CSS; it does not rebuild the Home tiles or refetch their images. Discovery changes pending behind a menu still refresh normally when it closes.

Glass uses static CSS gradients, borders and shadows, with no live backdrop blur. Monochrome applies grayscale only to the small artwork/glyph surfaces; it never filters the aerial or complete UI plane. No additional network requests, fonts, animation loop or timer are added. The existing OLED UI plane continues to shift, fade and hide all tiles and labels. Native TV rendering cost remains unmeasured for these styles.

Automated tests verify migration, valid/invalid preferences, independent saved clock/icon settings, focused selection, retained artwork nodes, unchanged Home tiles, label preferences, input navigation and absence of unintended launches during selection. Local browser checks cover Glass, Minimal and Monochrome appearance, All apps, the smaller input row, label visibility and persistence after reload. Classic retains the baseline appearance. Actual installed-app artwork and native video composition still need physical-TV validation before deployment.
