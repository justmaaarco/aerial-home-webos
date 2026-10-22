# OLED dimming

Open **Settings → OLED dimming**. The option is on by default, including after an upgrade, and can be turned off. **Preview dimming** shows the effect immediately.

After 30 minutes without remote, keyboard or pointer interaction, a 97% black overlay covers the complete launcher, including open menus, captions and the screen saver. The existing five-second idle check triggers it within five seconds of the deadline. Aerial playback continues underneath. The overlay uses a simple composite without extra blur effects.

The first navigation/OK press wakes the interface without activating the focused app or input. Holding that wake key does not navigate or launch anything until it is released. Pointer movement also wakes the interface; a click that wakes it is consumed. The menu and focus stay in place when waking from the dimmer alone. Waking the screen saver returns to Home as before.

The timer resets when the app returns to the foreground or receives a Home relaunch. It runs while Aerial Home is visible, even with video backgrounds or the screen saver disabled. It does not dim other TV apps or HDMI devices, set a power-off timer, or alter LG OLED care settings. Keep LG's built-in OLED care enabled: this reduces light output but cannot guarantee protection against burn-in.

Automated clock-driven tests cover the 30-minute boundary, activity reset, disabled mode, open menus, screen saver, foreground return, Home relaunch, held wake keys, and clicks without accidental app/input launches. Browser preview checks cover full-screen overlay and first-OK wake behavior.

On an LG B4 with webOS 9.2.4, the 0.2.3 preview dimmed the interface and native Dolby Vision aerial together. One injected OK press restored the same menu/focus without triggering the Preview control again. The video pipeline remained playing. The full 30-minute wait has not been repeated on hardware.
