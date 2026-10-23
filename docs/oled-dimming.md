# OLED care

The static-element additions in 0.2.4 are installed on the development TV but not yet publicly released. Physical-TV previews verified fade, hide, dim and wake behavior. Version 0.2.3 retains its original 30-minute dimming option under **Settings → OLED dimming**.

In 0.2.4, open **Settings → OLED care**. **Protect static elements** and **Dim after 30 minutes idle** are independent switches, both on by default. Each idle state has a preview that lasts until interaction.

## Static elements

The entire interface moves together through nine positions once per minute, bounded to four CSS pixels in each direction. This includes installed app tiles, focus highlights, greetings, the toolbar and clock, inputs, folders, menus, toasts, the footer, and screen-saver captions. The fixed video background keeps its full-screen coverage. On the TV's 1080p UI plane, these offsets correspond to UI pixels; the final physical pixel distance depends on display scaling.

After two idle minutes, the whole UI fades to 45% opacity. After five, it becomes invisible so only the aerial/background remains. These timers also cover open menus and work with video backgrounds and the screen saver disabled. The first navigation/OK press restores the interface and is consumed, including repeats until release. Pointer movement wakes it too; a waking click or wheel event is consumed. Menus retain their state and focus unless waking the screen saver, which already returns to Home.

Pixel shifting continues on its minute schedule while the interface is in use. It does not restart with each interaction, so regular remote activity does not leave the interface at its initial position. **Reduce motion** disables shifting and animated fades while retaining the idle fade/hide states. The protection uses the existing five-second idle check, with no continuous movement loop or full-screen filter.

## Full-screen dimming

After 30 minutes without remote, keyboard or pointer interaction, a 97% black overlay covers the complete launcher, including open menus, captions and the screen saver. The existing five-second idle check triggers it within five seconds of the deadline. Aerial playback continues underneath. The overlay uses a simple composite without extra blur effects.

The timer resets when the app returns to the foreground or receives a Home relaunch. It runs while Aerial Home is visible, even with video backgrounds or the screen saver disabled. It does not dim other TV apps or HDMI devices, set a power-off timer, or alter LG OLED care settings. Keep LG's built-in OLED care enabled: this reduces light output but cannot guarantee protection against burn-in.

## Validation

Automated clock-driven tests cover the 2-, 5- and 30-minute boundaries, bounded pixel-shift cycles, Reduce motion, activity reset, disabled modes, open menus, screen saver, foreground return, Home relaunch, held wake keys, and clicks/wheel events without accidental app/input launches. No protection against physical panel wear is claimed measured or guaranteed.

On an LG B4 with webOS 9.2.4, the 0.2.4 previews visibly faded the whole UI, hid it completely while retaining the aerial, and dimmed both UI and native Dolby Vision video. One injected OK press restored the same menu/focus without triggering the Preview control again. The video pipeline remained playing. Pixel-shift cycles and the full 2-, 5- and 30-minute automatic waits have not been repeated on hardware; local browser inspection and clock-driven tests cover those paths.
