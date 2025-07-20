use anyhow::Result;
use clap::Parser;


pub mod cli;
pub mod conditions;

/// The main entry point for the dozr application.
///
/// This function parses command-line arguments, determines the appropriate
/// wait condition, and then executes the wait.
pub fn run() -> Result<()> {
    let args = cli::Cli::parse();
    run_with_args(args)
}

/// The main logic of the application, accepting a Cli object.
fn run_with_args(args: cli::Cli) -> Result<()> {
    let condition: Box<dyn conditions::WaitCondition> = match args.command {
        cli::Commands::Duration { time } => {
            if let Some(probability) = args.probability {
                Box::new(conditions::ProbabilisticWait {
                    duration: time,
                    probability,
                    verbose: args.verbose_period(),
                })
            } else {
                Box::new(conditions::DurationWait {
                    duration: time,
                    jitter: args.jitter,
                    verbose: args.verbose_period(),
                })
            }
        }
        cli::Commands::Normal { mean, std_dev } => Box::new(conditions::NormalWait {
            mean,
            std_dev,
            verbose: args.verbose_period(),
            jitter: args.jitter,
        }),
        cli::Commands::Exponential { lambda } => Box::new(conditions::ExponentialWait {
            lambda,
            verbose: args.verbose_period(),
            jitter: args.jitter,
        }),
        cli::Commands::LogNormal { mean, std_dev } => Box::new(conditions::LogNormalWait {
            mean,
            std_dev,
            verbose: args.verbose_period(),
            jitter: args.jitter,
        }),
        cli::Commands::Pareto { scale, shape } => Box::new(conditions::ParetoWait {
            scale,
            shape,
            verbose: args.verbose_period(),
            jitter: args.jitter,
        }),
        cli::Commands::Triangular { min, max, mode } => Box::new(conditions::TriangularWait {
            min,
            max,
            mode,
            verbose: args.verbose_period(),
            jitter: args.jitter,
        }),
        cli::Commands::Align { interval } => Box::new(conditions::TimeAlignWait {
            align_interval: interval,
            verbose: args.verbose_period(),
        }),
        cli::Commands::Uniform { min, max } => Box::new(conditions::UniformWait {
            min,
            max,
            verbose: args.verbose_period(),
            jitter: args.jitter,
        }),
        cli::Commands::At { time } => Box::new(conditions::UntilTimeWait {
            sleep_duration: time,
            verbose: args.verbose_period(),
        }),
        cli::Commands::Gamma { shape, scale } => Box::new(conditions::GammaWait {
            shape,
            scale,
            verbose: args.verbose_period(),
            jitter: args.jitter,
        }),
    };

    condition.wait()
}

/// Performs the wait with verbose progress updates.
pub fn verbose_wait<F>(total_wait: std::time::Duration, update_period: std::time::Duration, mut display_fn: F)
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
        if last_displayed_eta.map_or(true, |last_eta| last_eta != rounded_eta) {
            display_fn(std::time::Duration::from_secs(rounded_eta));
            last_displayed_eta = Some(rounded_eta);
        }

        let next_update_time = elapsed + update_period;
        let sleep_duration = next_update_time.saturating_sub(elapsed);

        if sleep_duration > std::time::Duration::ZERO {
            std::thread::sleep(sleep_duration);
        } else if remaining > std::time::Duration::ZERO {
            // If sleep_duration is zero or negative, but there's still time remaining,
            // yield to ensure other threads can run and prevent busy-waiting.
            std::thread::yield_now();
        } else {
            // If no time remaining, break the loop
            break;
        }
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
        if last_displayed_eta.map_or(true, |last_eta| last_eta != rounded_eta) {
            display_fn(std::time::Duration::from_secs(rounded_eta));
            last_displayed_eta = Some(rounded_eta);
        }

        let current_update_period = get_adaptive_update_period(remaining);

        let remaining_secs = remaining.as_secs();

        let time_to_next_marker = if current_update_period.as_secs() == 0 {
            remaining
        } else {
            let target_marker_secs = (remaining_secs / current_update_period.as_secs()) * current_update_period.as_secs();
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

        let sleep_duration = std::cmp::min(current_update_period, std::cmp::min(time_to_next_threshold, time_to_next_marker));
        let sleep_duration = sleep_duration.max(std::time::Duration::from_millis(1)); // Ensure at least 1ms sleep to avoid busy-waiting

        if sleep_duration > std::time::Duration::ZERO {
            std::thread::sleep(sleep_duration);
        } else if remaining > std::time::Duration::ZERO {
            // If sleep_duration is zero or negative, but there's still time remaining,
            // yield to ensure other threads can run and prevent busy-waiting.
            std::thread::yield_now();
        } else {
            // If no time remaining, break the loop
            break;
        }
    }
}

fn get_adaptive_update_period(remaining: std::time::Duration) -> std::time::Duration {
    let remaining_secs = remaining.as_secs();

    if remaining_secs <= 20 {
        std::time::Duration::from_secs(1) // 0-20s: 1s
    } else if remaining_secs <= 60 {
        std::time::Duration::from_secs(5) // 21-60s: 5s
    } else if remaining_secs <= 300 {
        // 5 minutes
        std::time::Duration::from_secs(10) // 1-5m: 10s
    } else if remaining_secs <= 600 {
        // 10 minutes
        std::time::Duration::from_secs(15) // 6-10m: 15s
    } else {
        std::time::Duration::from_secs(60) // 10m+: 1m
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::cli::{Cli, Commands};

    #[test]
    fn test_verbose_wait() {
        let total_wait = Duration::from_millis(100);
        let update_period = Duration::from_millis(10);
        let mut call_count = 0;
        verbose_wait(total_wait, update_period, |_| {
            call_count += 1;
        });
        assert!(call_count > 0);
    }

    #[test]
    fn test_adaptive_verbose_wait() {
        let total_wait = Duration::from_secs(5);
        let mut call_count = 0;
        adaptive_verbose_wait(total_wait, |_| {
            call_count += 1;
        });
        assert!(call_count > 0);
    }

    #[test]
    fn test_get_adaptive_update_period() {
        // 0-20s: 1s
        assert_eq!(get_adaptive_update_period(Duration::from_secs(0)), Duration::from_secs(1));
        assert_eq!(get_adaptive_update_period(Duration::from_secs(10)), Duration::from_secs(1));
        assert_eq!(get_adaptive_update_period(Duration::from_secs(20)), Duration::from_secs(1));

        // 21-60s: 5s
        assert_eq!(get_adaptive_update_period(Duration::from_secs(21)), Duration::from_secs(5));
        assert_eq!(get_adaptive_update_period(Duration::from_secs(40)), Duration::from_secs(5));
        assert_eq!(get_adaptive_update_period(Duration::from_secs(60)), Duration::from_secs(5));

        // 1-5m (61-300s): 10s
        assert_eq!(get_adaptive_update_period(Duration::from_secs(61)), Duration::from_secs(10));
        assert_eq!(get_adaptive_update_period(Duration::from_secs(150)), Duration::from_secs(10));
        assert_eq!(get_adaptive_update_period(Duration::from_secs(300)), Duration::from_secs(10));

        // 6-10m (301-600s): 15s
        assert_eq!(get_adaptive_update_period(Duration::from_secs(301)), Duration::from_secs(15));
        assert_eq!(get_adaptive_update_period(Duration::from_secs(450)), Duration::from_secs(15));
        assert_eq!(get_adaptive_update_period(Duration::from_secs(600)), Duration::from_secs(15));

        // 10m+ (601s+): 1m
        assert_eq!(get_adaptive_update_period(Duration::from_secs(601)), Duration::from_secs(60));
        assert_eq!(get_adaptive_update_period(Duration::from_secs(1000)), Duration::from_secs(60));
    }

    #[test]
    fn test_run_with_args_duration() {
        let cli = Cli {
            command: Commands::Duration { time: Duration::from_secs(1) },
            jitter: None,
            verbose: None,
            probability: None,
        };
        assert!(run_with_args(cli).is_ok());
    }

    #[test]
    fn test_run_with_args_normal() {
        let cli = Cli {
            command: Commands::Normal { mean: Duration::from_secs(1), std_dev: 0.1 },
            jitter: None,
            verbose: None,
            probability: None,
        };
        assert!(run_with_args(cli).is_ok());
    }
}
