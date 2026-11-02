//! Deterministic menu model shared by the probe and its host-side tests.
pub mod catalogue;
pub mod model;
pub mod redraw;
pub mod text_cache;
pub const LABELS: [&str; 10] = [
    "NETFLIX",
    "YOUTUBE",
    "APPLE TV",
    "PRIME VIDEO",
    "SPOTIFY",
    "ALL APPS",
    "CLOCK",
    "SETTINGS",
    "INPUTS",
    "EXIT",
];

#[derive(Clone, Copy)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

pub fn navigate(index: usize, count: usize, columns: usize, direction: Direction) -> usize {
    if count == 0 || columns == 0 {
        return 0;
    }
    let index = index.min(count - 1);
    match direction {
        Direction::Left if index % columns > 0 => index - 1,
        Direction::Right if index % columns + 1 < columns && index + 1 < count => index + 1,
        Direction::Up if index >= columns => index - columns,
        Direction::Down if index + columns < count => index + columns,
        _ => index,
    }
}

pub fn percentile(values: &[f64], fraction: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    Some(sorted[((sorted.len() - 1) as f64 * fraction.clamp(0.0, 1.0)).round() as usize])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn navigation_does_not_wrap_or_enter_a_missing_tile() {
        assert_eq!(navigate(0, 8, 5, Direction::Left), 0);
        assert_eq!(navigate(4, 8, 5, Direction::Right), 4);
        assert_eq!(navigate(4, 8, 5, Direction::Down), 4);
        assert_eq!(navigate(2, 8, 5, Direction::Down), 7);
        assert_eq!(navigate(7, 8, 5, Direction::Up), 2);
        assert_eq!(navigate(99, 0, 5, Direction::Right), 0);
    }
    #[test]
    fn timings_report_observed_samples_without_inventing_empty_results() {
        assert_eq!(percentile(&[], 0.5), None);
        assert_eq!(percentile(&[5.0, 1.0, 3.0], 0.5), Some(3.0));
        assert_eq!(percentile(&[5.0, 1.0, 3.0], 0.95), Some(5.0));
    }
}
