# Clock styles

Prepared in the local 0.2.5 candidate. These additions have not been installed on the development TV or publicly released. Existing installations keep the Classic appearance when updated.

Open **Control Center → Settings → Clock style**.

| Style | Home | Full screen |
| --- | --- | --- |
| Classic | Original corner clock and time-of-day greeting | Compact clock at the lower left |
| Minimal | Large, light numerals above the apps | Centered, light numerals |
| Liquid Glass | Translucent highlighted numerals above the apps | A larger centered glass face |
| Immersive | Oversized centered clock above the apps | Expansive centered numerals over the background |

**Time format** cycles through System, 12-hour and 24-hour. The explicit 24-hour choice uses 00 at midnight. Time and date follow the runtime's locale and time zone; the app does not change LG's system clock. **Show date** affects the large Home clocks and all full-screen faces. Classic keeps its original Home greeting rather than adding a date there.

**View full-screen clock** hides the apps and toolbar. It works with video disabled, using the existing background, and does not itself start or restart video. The first key or pointer activity returns Home and is consumed so it cannot also launch an app or select an input. The normal automatic aerial screen saver uses the chosen clock style too.

All faces stay inside the existing OLED UI plane. Enabled OLED care still shifts them, fades the complete interface after two idle minutes and hides it after five; the independent 30-minute full-screen dimming remains in place. A full-screen clock does not bypass these safeguards. See [OLED care](oled-dimming.md) for limitations.

The glass face is an independent CSS approximation using a static gradient, fine edge and text shadow. It does not implement Apple's optical refraction or depth-aware subject masking. It adds no live backdrop blur, per-frame JavaScript, seconds animation, extra font or network request. It reuses the existing ten-second clock tick and cached locale formatters. Actual TV rendering cost has not been measured for these styles.

Local validation covers preference migration and rejection of invalid values, locale/12/24-hour formatting, midnight/noon, date visibility, matching Home/full-screen faces, first-key wake consumption and OLED fade/hide integration. Browser checks cover visual layout, saved preferences and full-screen entry/exit. Physical-TV readability, native video composition and performance remain to be checked before deployment.
