//! Progress reporting while waiting.
//!
//! The update loops take a [`Clock`] so tests can drive them with simulated
//! time instead of sleeping.

use std::time::{Duration, Instant};

/// How progress is reported while waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Verbose {
    /// No progress output.
    #[default]
    Off,
    /// Update less often as the remaining time grows.
    Adaptive,
    /// Update on a fixed period. Must be at least [`MIN_VERBOSE_PERIOD`].
    Every(Duration),
}

impl Verbose {
    /// Whether any progress output is produced.
    pub fn is_on(&self) -> bool {
        *self != Verbose::Off
    }
}

/// Shortest fixed update period accepted for [`Verbose::Every`].
pub const MIN_VERBOSE_PERIOD: Duration = Duration::from_millis(1);

/// A monotonic time source that can also sleep.
pub trait Clock {
    /// Time since an arbitrary fixed origin.
    fn now(&self) -> Duration;
    /// Blocks (or, for a simulated clock, advances) for `duration`.
    fn sleep(&mut self, duration: Duration);
}

/// The real [`Clock`]: `Instant` for time and `thread::sleep` for sleeping.
#[derive(Debug, Clone, Copy)]
pub struct SystemClock {
    origin: Instant,
}

impl SystemClock {
    pub fn new() -> Self {
        SystemClock {
            origin: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }

    fn sleep(&mut self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

/// Sleeps for `total_wait`, calling `display_fn` every `update_period`.
///
/// `display_fn` receives the exact remaining time on every tick, and
/// `Duration::ZERO` once when the wait is over. The wait never runs past
/// `total_wait`, however long `update_period` is.
pub fn verbose_wait<F>(total_wait: Duration, update_period: Duration, display_fn: F)
where
    F: FnMut(Duration),
{
    verbose_wait_with(
        &mut SystemClock::new(),
        total_wait,
        update_period,
        display_fn,
    );
}

/// [`verbose_wait`] driven by the supplied clock.
pub fn verbose_wait_with<F>(
    clock: &mut dyn Clock,
    total_wait: Duration,
    update_period: Duration,
    mut display_fn: F,
) where
    F: FnMut(Duration),
{
    let update_period = update_period.max(MIN_VERBOSE_PERIOD);
    let start = clock.now();

    loop {
        let remaining = total_wait.saturating_sub(clock.now().saturating_sub(start));
        if remaining.is_zero() {
            display_fn(Duration::ZERO);
            break;
        }
        display_fn(remaining);
        clock.sleep(update_period.min(remaining));
    }
}

/// Sleeps for `total_wait`, calling `display_fn` less often as the remaining
/// time grows.
///
/// `display_fn` receives the remaining time rounded to whole seconds, only
/// when that value changes, and `Duration::ZERO` once at the end. Updates
/// land on round numbers: every minute above 10 minutes, then every 15
/// seconds, 10 seconds, 5 seconds, and finally every second below 20 seconds.
pub fn adaptive_verbose_wait<F>(total_wait: Duration, display_fn: F)
where
    F: FnMut(Duration),
{
    adaptive_verbose_wait_with(&mut SystemClock::new(), total_wait, display_fn);
}

/// [`adaptive_verbose_wait`] driven by the supplied clock.
pub fn adaptive_verbose_wait_with<F>(clock: &mut dyn Clock, total_wait: Duration, mut display_fn: F)
where
    F: FnMut(Duration),
{
    let start = clock.now();
    let mut last_displayed_eta: Option<u64> = None;

    loop {
        let remaining = total_wait.saturating_sub(clock.now().saturating_sub(start));
        if remaining.is_zero() {
            display_fn(Duration::ZERO);
            break;
        }

        let rounded_eta = remaining.as_secs_f64().round() as u64;
        if last_displayed_eta != Some(rounded_eta) {
            display_fn(Duration::from_secs(rounded_eta));
            last_displayed_eta = Some(rounded_eta);
        }

        clock.sleep(adaptive_sleep(remaining));
    }
}

/// Time until the next adaptive update: the next round-number marker for the
/// current period, or the next threshold where the period changes, whichever
/// comes first. Never less than 1ms, so the loop cannot spin.
fn adaptive_sleep(remaining: Duration) -> Duration {
    let period = adaptive_update_period(remaining);
    let remaining_secs = remaining.as_secs();

    let marker_secs = (remaining_secs / period.as_secs()) * period.as_secs();
    let to_next_marker = remaining.saturating_sub(Duration::from_secs(marker_secs));

    let threshold = [600, 300, 60, 20]
        .into_iter()
        .find(|&t| remaining_secs > t)
        .unwrap_or(0);
    let to_next_threshold = remaining.saturating_sub(Duration::from_secs(threshold));

    period
        .min(to_next_threshold)
        .min(to_next_marker)
        .max(MIN_VERBOSE_PERIOD)
}

/// Update period for adaptive progress, by remaining time.
pub(crate) fn adaptive_update_period(remaining: Duration) -> Duration {
    let secs = match remaining.as_secs() {
        0..=20 => 1,
        21..=60 => 5,
        61..=300 => 10,
        301..=600 => 15,
        _ => 60,
    };
    Duration::from_secs(secs)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A clock that only moves when slept on.
    #[derive(Debug, Default)]
    pub(crate) struct FakeClock {
        pub(crate) now: Duration,
        pub(crate) sleeps: Vec<Duration>,
    }

    impl Clock for FakeClock {
        fn now(&self) -> Duration {
            self.now
        }

        fn sleep(&mut self, duration: Duration) {
            self.sleeps.push(duration);
            self.now += duration;
        }
    }

    fn s(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn run_fixed(total: Duration, period: Duration) -> (Vec<Duration>, FakeClock) {
        let mut clock = FakeClock::default();
        let mut shown = Vec::new();
        verbose_wait_with(&mut clock, total, period, |d| shown.push(d));
        (shown, clock)
    }

    fn run_adaptive(total: Duration) -> (Vec<u64>, FakeClock) {
        let mut clock = FakeClock::default();
        let mut shown = Vec::new();
        adaptive_verbose_wait_with(&mut clock, total, |d| shown.push(d.as_secs()));
        (shown, clock)
    }

    #[test]
    fn test_verbose_is_on() {
        assert!(!Verbose::Off.is_on());
        assert!(Verbose::Adaptive.is_on());
        assert!(Verbose::Every(s(1)).is_on());
    }

    #[test]
    fn test_fixed_period_reports_every_tick() {
        let (shown, clock) = run_fixed(s(3), s(1));
        assert_eq!(shown, vec![s(3), s(2), s(1), Duration::ZERO]);
        assert_eq!(clock.now, s(3));
    }

    #[test]
    fn test_fixed_period_sub_second() {
        let (shown, _) = run_fixed(s(1), ms(250));
        assert_eq!(shown, vec![s(1), ms(750), ms(500), ms(250), Duration::ZERO]);
    }

    #[test]
    fn test_fixed_period_longer_than_wait_does_not_overshoot() {
        let (shown, clock) = run_fixed(ms(100), s(10));
        assert_eq!(shown, vec![ms(100), Duration::ZERO]);
        assert_eq!(clock.sleeps, vec![ms(100)]);
    }

    #[test]
    fn test_fixed_period_uneven_final_tick() {
        let (_, clock) = run_fixed(ms(2_500), s(1));
        assert_eq!(clock.sleeps, vec![s(1), s(1), ms(500)]);
    }

    #[test]
    fn test_fixed_period_zero_is_raised_to_minimum() {
        let (_, clock) = run_fixed(ms(3), Duration::ZERO);
        assert_eq!(clock.sleeps, vec![ms(1), ms(1), ms(1)]);
    }

    #[test]
    fn test_zero_wait_reports_only_completion() {
        let (shown, clock) = run_fixed(Duration::ZERO, s(1));
        assert_eq!(shown, vec![Duration::ZERO]);
        assert!(clock.sleeps.is_empty());
        let (shown, _) = run_adaptive(Duration::ZERO);
        assert_eq!(shown, vec![0]);
    }

    #[test]
    fn test_adaptive_short_wait_counts_every_second() {
        let (shown, clock) = run_adaptive(s(5));
        assert_eq!(shown, vec![5, 4, 3, 2, 1, 0]);
        assert_eq!(clock.now, s(5));
    }

    #[test]
    fn test_adaptive_long_wait_schedule() {
        // 25 minutes: minute marks down to 10 minutes, then 15s, 10s, 5s and
        // 1s steps as each threshold is crossed.
        let (shown, clock) = run_adaptive(s(25 * 60));
        assert_eq!(clock.now, s(25 * 60));

        let expect_steps = |from: u64, to: u64, step: u64| -> Vec<u64> {
            (to..=from).rev().step_by(step as usize).collect()
        };
        let mut expected = expect_steps(1500, 660, 60);
        expected.extend(expect_steps(600, 315, 15));
        expected.extend(expect_steps(300, 70, 10));
        expected.extend(expect_steps(60, 25, 5));
        expected.extend(expect_steps(20, 0, 1));
        assert_eq!(shown, expected);
    }

    #[test]
    fn test_adaptive_unaligned_start_snaps_to_markers() {
        // 67s: first update at 67, then the next 10s marker (60), then 5s
        // steps below the 60s threshold.
        let (shown, _) = run_adaptive(s(67));
        assert_eq!(&shown[..4], &[67, 60, 55, 50]);
    }

    #[test]
    fn test_adaptive_update_period() {
        let cases = [
            (0, 1),
            (10, 1),
            (20, 1),
            (21, 5),
            (60, 5),
            (61, 10),
            (300, 10),
            (301, 15),
            (600, 15),
            (601, 60),
            (1000, 60),
        ];
        for (remaining, period) in cases {
            assert_eq!(
                adaptive_update_period(s(remaining)),
                s(period),
                "{remaining}s"
            );
        }
    }

    #[test]
    fn test_system_clock_really_sleeps() {
        let mut clock = SystemClock::new();
        let before = clock.now();
        clock.sleep(ms(20));
        assert!(clock.now() - before >= ms(20));
    }
}
