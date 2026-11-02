use aerial_native_probe::model::ClockParts;
#[repr(C)]
struct Tm {
    sec: i32,
    min: i32,
    hour: i32,
    day: i32,
    month: i32,
    year: i32,
    weekday: i32,
    yearday: i32,
    dst: i32,
    offset: isize,
    zone: *const std::ffi::c_char,
}
#[cfg(target_pointer_width = "32")]
type Time = i32;
#[cfg(not(target_pointer_width = "32"))]
type Time = i64;
extern "C" {
    fn localtime_r(time: *const Time, out: *mut Tm) -> *mut Tm;
}
pub fn parts() -> ClockParts {
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as Time;
    let mut tm: Tm = unsafe { std::mem::zeroed() };
    unsafe {
        localtime_r(&time, &mut tm);
    };
    ClockParts {
        hour: tm.hour.clamp(0, 23) as u8,
        minute: tm.min.clamp(0, 59) as u8,
        weekday: tm.weekday.clamp(0, 6) as u8,
        month: (tm.month + 1).clamp(1, 12) as u8,
        day: tm.day.clamp(1, 31) as u8,
    }
}
