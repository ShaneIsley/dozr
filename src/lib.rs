use anyhow::Result;
use clap::{CommandFactory, Parser};

pub mod cli;
pub mod conditions;

/// How progress is reported while waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Verbose {
    /// No progress output.
    #[default]
    Off,
    /// Update less often as the remaining time grows.
    Adaptive,
    /// Update on a fixed period. Must be at least [`MIN_VERBOSE_PERIOD`].
    Every(std::time::Duration),
}

impl Verbose {
    /// Whether any progress output is produced.
    pub fn is_on(&self) -> bool {
        *self != Verbose::Off
    }
}

/// Shortest fixed update period accepted for [`Verbose::Every`].
pub const MIN_VERBOSE_PERIOD: std::time::Duration = std::time::Duration::from_millis(1);

/// The main entry point for the dozr application.
///
/// This function parses command-line arguments, validates them, determines
/// the appropriate wait condition, and then executes the wait. Invalid
/// arguments print a usage error and exit with status 2.
pub fn run() -> Result<()> {
    let args = cli::Cli::parse();
    if let Err(message) = args.command.validate() {
        cli::Cli::command()
            .error(clap::error::ErrorKind::ValueValidation, message)
            .exit();
    }
    run_with_args(args)
}

/// The main logic of the application, accepting a Cli object.
fn run_with_args(args: cli::Cli) -> Result<()> {
    let condition = args
        .command
        .into_wait_condition(args.jitter, args.verbose, args.probability);
    condition.wait()
}

/// Performs the wait with verbose progress updates every `update_period`.
///
/// `display_fn` receives the exact remaining time on every tick, and
/// `Duration::ZERO` once when the wait is over. The wait never runs past
/// `total_wait`, however long `update_period` is.
pub fn verbose_wait<F>(
    total_wait: std::time::Duration,
    update_period: std::time::Duration,
    mut display_fn: F,
) where
    F: FnMut(std::time::Duration),
{
    let update_period = update_period.max(MIN_VERBOSE_PERIOD);
    let start = std::time::Instant::now();

    loop {
        let remaining = total_wait.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            display_fn(std::time::Duration::ZERO);
            break;
        }
        display_fn(remaining);
        std::thread::sleep(update_period.min(remaining));
    }
}

/// Performs the wait with adaptive verbose progress updates.
pub fn adaptive_verbose_wait<F>(total_wait: std::time::Duration, mut display_fn: F)
where
    F: FnMut(std::time::Duration),
{
    let start = std::time::Instant::now();
    let mut last_displayed_eta: Option<u64> = None;

    loop {
        let elapsed = start.elapsed();
        let remaining = total_wait.saturating_sub(elapsed);
        let eta = remaining.as_secs_f64();
        let rounded_eta = eta.round() as u64;

        if remaining == std::time::Duration::ZERO {
            display_fn(std::time::Duration::ZERO);
            break;
        }

        // Only display if ETA has changed or it's the very first display
        if last_displayed_eta != Some(rounded_eta) {
            display_fn(std::time::Duration::from_secs(rounded_eta));
            last_displayed_eta = Some(rounded_eta);
        }

        let current_update_period = get_adaptive_update_period(remaining);

        let remaining_secs = remaining.as_secs();

        let time_to_next_marker = if current_update_period.as_secs() == 0 {
            remaining
        } else {
            let target_marker_secs = (remaining_secs / current_update_period.as_secs())
                * current_update_period.as_secs();
            remaining.saturating_sub(std::time::Duration::from_secs(target_marker_secs))
        };

        let time_to_next_threshold = if remaining_secs > 600 {
            remaining.saturating_sub(std::time::Duration::from_secs(600))
        } else if remaining_secs > 300 {
            remaining.saturating_sub(std::time::Duration::from_secs(300))
        } else if remaining_secs > 60 {
            remaining.saturating_sub(std::time::Duration::from_secs(60))
        } else if remaining_secs > 20 {
            remaining.saturating_sub(std::time::Duration::from_secs(20))
        } else {
            remaining
        };

        let sleep_duration = std::cmp::min(
            current_update_period,
            std::cmp::min(time_to_next_threshold, time_to_next_marker),
        );
        // Ensure at least 1ms sleep to avoid busy-waiting
        let sleep_duration = sleep_duration.max(std::time::Duration::from_millis(1));
        std::thread::sleep(sleep_duration);
    }
}

fn get_adaptive_update_period(remaining: std::time::Duration) -> std::time::Duration {
    let remaining_secs = remaining.as_secs();

    if remaining_secs <= 20 {
        std::time::Duration::from_secs(1) // 0-20s: 1s
    } else if remaining_secs <= 60 {
        std::time::Duration::from_secs(5) // 21-60s: 5s
    } else if remaining_secs <= 300 {
        std::time::Duration::from_secs(10) // 1-5m: 10s
    } else if remaining_secs <= 600 {
        std::time::Duration::from_secs(15) // 6-10m: 15s
    } else {
        std::time::Duration::from_secs(60) // 10m+: 1m
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use std::time::{Duration, Instant};

    #[test]
    fn test_verbose_wait_reports_and_finishes_at_zero() {
        let mut calls = Vec::new();
        verbose_wait(Duration::from_millis(100), Duration::from_millis(10), |d| {
            calls.push(d)
        });
        assert!(!calls.is_empty());
        assert_eq!(calls.last(), Some(&Duration::ZERO));
    }

    #[test]
    fn test_verbose_wait_reports_every_tick() {
        // 100ms at a 20ms period is five ticks, then the final zero. Every
        // reported remaining time must be strictly decreasing.
        let mut calls = Vec::new();
        verbose_wait(Duration::from_millis(100), Duration::from_millis(20), |d| {
            calls.push(d)
        });
        assert!(calls.len() >= 5, "only {} updates: {calls:?}", calls.len());
        assert!(calls.windows(2).all(|w| w[0] > w[1]), "{calls:?}");
    }

    #[test]
    fn test_verbose_is_on() {
        assert!(!Verbose::Off.is_on());
        assert!(Verbose::Adaptive.is_on());
        assert!(Verbose::Every(Duration::from_secs(1)).is_on());
    }

    #[test]
    fn test_verbose_wait_period_longer_than_wait_does_not_overshoot() {
        let start = Instant::now();
        verbose_wait(Duration::from_millis(100), Duration::from_secs(10), |_| {});
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(100));
        assert!(elapsed < Duration::from_millis(200), "took {elapsed:?}");
    }

    #[test]
    fn test_verbose_wait_zero_period_terminates() {
        let start = Instant::now();
        verbose_wait(Duration::from_millis(20), Duration::ZERO, |_| {});
        assert!(start.elapsed() < Duration::from_millis(200));
    }

    #[test]
    fn test_adaptive_verbose_wait() {
        let start = Instant::now();
        let mut calls = Vec::new();
        adaptive_verbose_wait(Duration::from_millis(1_500), |d| calls.push(d));
        let elapsed = start.elapsed();
        assert!(calls.len() >= 2);
        assert_eq!(calls.last(), Some(&Duration::ZERO));
        assert!(elapsed >= Duration::from_millis(1_500));
        assert!(elapsed < Duration::from_millis(1_700), "took {elapsed:?}");
    }

    #[test]
    fn test_get_adaptive_update_period() {
        let s = Duration::from_secs;
        // 0-20s: 1s
        assert_eq!(get_adaptive_update_period(s(0)), s(1));
        assert_eq!(get_adaptive_update_period(s(10)), s(1));
        assert_eq!(get_adaptive_update_period(s(20)), s(1));
        // 21-60s: 5s
        assert_eq!(get_adaptive_update_period(s(21)), s(5));
        assert_eq!(get_adaptive_update_period(s(40)), s(5));
        assert_eq!(get_adaptive_update_period(s(60)), s(5));
        // 1-5m (61-300s): 10s
        assert_eq!(get_adaptive_update_period(s(61)), s(10));
        assert_eq!(get_adaptive_update_period(s(150)), s(10));
        assert_eq!(get_adaptive_update_period(s(300)), s(10));
        // 6-10m (301-600s): 15s
        assert_eq!(get_adaptive_update_period(s(301)), s(15));
        assert_eq!(get_adaptive_update_period(s(450)), s(15));
        assert_eq!(get_adaptive_update_period(s(600)), s(15));
        // 10m+ (601s+): 1m
        assert_eq!(get_adaptive_update_period(s(601)), s(60));
        assert_eq!(get_adaptive_update_period(s(1000)), s(60));
    }

    #[test]
    fn test_run_with_args_duration() {
        let cli = Cli {
            command: Commands::Duration {
                time: Duration::from_millis(50),
            },
            jitter: None,
            verbose: Verbose::Off,
            probability: None,
        };
        assert!(run_with_args(cli).is_ok());
    }

    #[test]
    fn test_run_with_args_normal() {
        let cli = Cli {
            command: Commands::Normal {
                mean: Duration::from_millis(50),
                std_dev: Duration::from_millis(10),
            },
            jitter: None,
            verbose: Verbose::Off,
            probability: None,
        };
        assert!(run_with_args(cli).is_ok());
    }
}
