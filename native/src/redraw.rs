//! Time-driven UI invalidation. The hardware video plane presents independently;
//! an unchanged clock/overlay does not need another full SDL composition.
use crate::model::{ClockParts, OledState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedVisuals {
    clock: [u8; 5],
    oled: OledState,
    toast_visible: bool,
}
impl TimedVisuals {
    pub fn new(clock: ClockParts, oled: OledState, toast_visible: bool) -> Self {
        Self {
            clock: [
                clock.hour,
                clock.minute,
                clock.weekday,
                clock.month,
                clock.day,
            ],
            oled,
            toast_visible,
        }
    }
}

/// Hidden windows cannot consume a pending redraw. Polling without a wait in
/// that state spins forever, competing with the app that just took foreground.
/// SDL input still interrupts this wait; service replies/Home IPC retain the
/// existing maximum 100 ms polling interval.
pub fn event_wait_ms(dirty: bool, visible: bool) -> i32 {
    if dirty && visible {
        0
    } else {
        100
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{oled_state, Preferences};

    fn state(now: u64, minute: u8, toast_until: u64) -> TimedVisuals {
        TimedVisuals::new(
            ClockParts {
                hour: 12,
                minute,
                weekday: 1,
                month: 11,
                day: 2,
            },
            oled_state(&Preferences::default(), now, now, true),
            now < toast_until,
        )
    }

    #[test]
    fn background_with_an_unpresented_frame_never_busy_polls() {
        for dirty in [false, true] {
            assert_eq!(event_wait_ms(dirty, false), 100);
        }
        assert_eq!(event_wait_ms(false, true), 100);
        assert_eq!(event_wait_ms(true, true), 0);
    }

    #[test]
    fn unchanged_minute_does_not_schedule_repeated_compositions() {
        let mut previous = None;
        let mut compositions = 0;
        for now in (0..60_000).step_by(100) {
            let next = state(now, 0, 0);
            if previous != Some(next) {
                compositions += 1;
                previous = Some(next);
            }
        }
        assert_eq!(compositions, 1);
        assert_ne!(previous, Some(state(60_000, 1, 0)));
    }

    #[test]
    fn clock_date_and_daytime_changes_invalidate() {
        let clock = ClockParts {
            hour: 6,
            minute: 59,
            weekday: 1,
            month: 11,
            day: 2,
        };
        let oled = oled_state(&Preferences::default(), 0, 0, true);
        let initial = TimedVisuals::new(clock, oled, false);
        for changed in [
            ClockParts { hour: 7, ..clock },
            ClockParts { minute: 0, ..clock },
            ClockParts {
                day: 3,
                weekday: 2,
                ..clock
            },
            ClockParts { month: 12, ..clock },
        ] {
            assert_ne!(initial, TimedVisuals::new(changed, oled, false));
        }
    }

    #[test]
    fn oled_boundaries_and_toast_expiry_still_redraw() {
        for boundary in [60_000, 120_000, 300_000, 1_800_000] {
            assert_ne!(state(boundary - 1, 0, 0), state(boundary, 0, 0));
        }
        assert_ne!(state(4_999, 0, 5_000), state(5_000, 0, 5_000));
    }
}
