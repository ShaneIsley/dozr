use anyhow::Result;
use chrono::{DateTime, Duration as ChronoDuration, Local};
use clap::Parser;
use rand::Rng;
use std::thread;
use std::time::Duration;

pub mod cli;
pub mod conditions;

pub fn run() -> Result<()> {
    let args = cli::Cli::parse();

    let condition: Box<dyn conditions::WaitCondition> =
        match (args.duration, args.align, args.until) {
            (Some(duration), None, None) => {
                if let Some(probability) = args.probability {
                    Box::new(conditions::ProbabilisticWait {
                        duration,
                        probability,
                        verbose: args.verbose_period(),
                    })
                } else {
                    Box::new(conditions::DurationWait {
                        duration,
                        jitter: args.jitter,
                        verbose: args.verbose_period(),
                    })
                }
            }
            (None, Some(align_to), None) => Box::new(conditions::TimeAlignWait {
                align_interval: align_to,
                verbose: args.verbose_period(),
            }),
            (None, None, Some(until_duration)) => Box::new(conditions::UntilTimeWait {
                sleep_duration: until_duration,
                verbose: args.verbose_period(),
            }),
            // This case is now unreachable because of the clap group validation
            _ => unreachable!(),
        };

    condition.wait()
}

/// Calculates the total wait duration, including a base duration and optional jitter.
pub fn get_total_wait_duration(
    base_duration: Duration,
    jitter: Option<Duration>,
) -> Result<Duration> {
    let jitter_duration = if let Some(j) = jitter {
        let j_nanos = j.as_nanos();
        if j_nanos == 0 {
            Duration::new(0, 0)
        } else {
            let random_jitter = rand::rng().random_range(0..=j_nanos);
            Duration::from_nanos(random_jitter as u64)
        }
    } else {
        Duration::new(0, 0)
    };

    Ok(base_duration + jitter_duration)
}

/// Calculates the duration to wait until the next alignment interval.
pub fn get_alignment_duration(align_to: Duration) -> Result<Duration> {
    let now: DateTime<Local> = Local::now();
    let align_to_chrono = ChronoDuration::from_std(align_to)?;

    let since_epoch = now.timestamp_nanos_opt().unwrap_or_default();
    let align_to_nanos = align_to_chrono.num_nanoseconds().unwrap_or_default();

    if align_to_nanos == 0 {
        return Ok(Duration::new(0, 0));
    }

    let remainder = since_epoch % align_to_nanos;
    let wait_nanos = if remainder == 0 {
        0
    } else {
        align_to_nanos - remainder
    };

    Ok(Duration::from_nanos(wait_nanos as u64))
}

/// Performs the wait with verbose progress updates.
pub fn verbose_wait<F>(total_wait: Duration, update_period: Duration, mut display_fn: F)
where
    F: FnMut(Duration),
{
    let start = std::time::Instant::now();
    let mut remaining = total_wait;

    while remaining > Duration::ZERO {
        let elapsed = start.elapsed();
        let current_wait = if remaining < update_period {
            remaining
        } else {
            update_period
        };

        thread::sleep(current_wait);

        remaining = total_wait.saturating_sub(elapsed);
        let eta = remaining.as_secs_f64();

        if eta > 0.0 {
            display_fn(Duration::from_secs(eta.round() as u64));
        }
    }
    display_fn(Duration::ZERO);
}

/// Performs the wait with adaptive verbose progress updates.
pub fn adaptive_verbose_wait<F>(total_wait: Duration, mut display_fn: F)
where
    F: FnMut(Duration),
{
    let start = std::time::Instant::now();
    let mut remaining = total_wait;

    while remaining > Duration::ZERO {
        let elapsed = start.elapsed();
        let update_period = get_adaptive_update_period(remaining);
        let current_wait = if remaining < update_period {
            remaining
        } else {
            update_period
        };

        thread::sleep(current_wait);

        remaining = total_wait.saturating_sub(elapsed);
        let eta = remaining.as_secs_f64();

        if eta > 0.0 {
            display_fn(Duration::from_secs(eta.round() as u64));
        }
    }
    display_fn(Duration::ZERO);
}

fn get_adaptive_update_period(remaining: Duration) -> Duration {
    let remaining_secs = remaining.as_secs();

    if remaining_secs <= 20 {
        Duration::from_secs(1) // 0-20s: 1s
    } else if remaining_secs <= 60 {
        Duration::from_secs(5) // 21-60s: 5s
    } else if remaining_secs <= 300 {
        // 5 minutes
        Duration::from_secs(10) // 1-5m: 10s
    } else if remaining_secs <= 600 {
        // 10 minutes
        Duration::from_secs(15) // 6-10m: 15s
    } else {
        Duration::from_secs(60) // 10m+: 1m
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_get_total_wait_duration_no_jitter() {
        let base_duration = Duration::from_secs(1);
        let total_wait = get_total_wait_duration(base_duration, None).unwrap();
        assert_eq!(total_wait, base_duration);
    }

    #[test]
    fn test_get_total_wait_duration_with_jitter() {
        let base_duration = Duration::from_secs(1);
        let jitter = Duration::from_millis(500);
        let total_wait = get_total_wait_duration(base_duration, Some(jitter)).unwrap();
        assert!(total_wait >= base_duration);
        assert!(total_wait <= base_duration + jitter);
    }

    #[test]
    fn test_get_alignment_duration() {
        // This test is sensitive to the current time, so we can't assert a specific value.
        // Instead, we'll just check that the function returns a duration.
        let align_to = Duration::from_secs(10);
        let wait_duration = get_alignment_duration(align_to).unwrap();
        assert!(wait_duration <= align_to);
    }

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
}
