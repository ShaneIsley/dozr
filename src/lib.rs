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

    let condition: Box<dyn conditions::WaitCondition> = match (args.duration, args.align) {
        (Some(duration), None) => {
            if let Some(probability) = args.probability {
                Box::new(conditions::ProbabilisticWait {
                    duration,
                    probability,
                    verbose: args.verbose,
                })
            } else {
                Box::new(conditions::DurationWait {
                    duration,
                    jitter: args.jitter,
                    verbose: args.verbose,
                })
            }
        }
        (None, Some(align_to)) => Box::new(conditions::TimeAlignWait {
            align_interval: align_to,
            verbose: args.verbose,
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
pub fn verbose_wait(total_wait: Duration, update_period: Duration) {
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
            println!("Waiting... {eta:.2}s remaining (ETA)");
        }
    }
    println!("Wait complete.");
}
