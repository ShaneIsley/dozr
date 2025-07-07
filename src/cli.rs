use chrono::{Duration as ChronoDuration, Local, NaiveTime, Timelike};
use clap::{ArgGroup, Parser};
use std::time::Duration;

fn parse_time_until(s: &str) -> Result<Duration, String> {
    let now = Local::now();
    let parsed_time = NaiveTime::parse_from_str(s, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(s, "%H:%M:%S"))
        .map_err(|_| format!("Invalid time format. Expected HH:MM or HH:MM:SS: {s}"))?;

    let mut target_datetime = now
        .with_hour(parsed_time.hour())
        .and_then(|dt| dt.with_minute(parsed_time.minute()))
        .and_then(|dt| dt.with_second(parsed_time.second()))
        .and_then(|dt| dt.with_nanosecond(parsed_time.nanosecond()))
        .unwrap(); // These unwraps are safe as we are setting valid time components

    // If the target time has already passed today, set it for tomorrow
    if target_datetime < now {
        target_datetime += ChronoDuration::days(1);
    }

    let duration_until = target_datetime.signed_duration_since(now);

    // Convert chrono::Duration to std::time::Duration
    duration_until.to_std().map_err(|e| e.to_string())
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(group = ArgGroup::new("wait_type").required(true).multiple(false))]
pub struct Cli {
    /// The base duration to wait (e.g., "1s", "500ms").
    #[arg(long, value_parser = humantime::parse_duration, group = "wait_type")]
    pub duration: Option<Duration>,

    /// Add a random duration of jitter (e.g., "100ms").
    #[arg(short, long, value_parser = humantime::parse_duration)]
    pub jitter: Option<Duration>,

    /// Align the wait to the next even interval of the given duration (e.g., "1m", "30s").
    #[arg(short, long, value_parser = humantime::parse_duration, group = "wait_type")]
    pub align: Option<Duration>,

    /// Enable verbose output, with an optional update period (e.g., "250ms").
    /// If no update period is specified, adaptive verbose output is used.
    #[arg(short, long, value_name = "UPDATE_PERIOD", value_parser = humantime::parse_duration, num_args = 0..=1, default_missing_value = "1ns")]
    pub verbose: Option<Duration>,

    /// Wait only with a certain probability (0.0 to 1.0).
    #[arg(short, long)]
    pub probability: Option<f64>,

    /// Wait until a specific time of day (HH:MM or HH:MM:SS). Rolls over to next day if time has passed.
    #[arg(long, value_parser = parse_time_until, group = "wait_type")]
    pub until: Option<Duration>,
}

impl Cli {
    pub fn is_adaptive_verbose(&self) -> bool {
        self.verbose == Some(Duration::from_nanos(1))
    }

    pub fn verbose_period(&self) -> Option<Duration> {
        if self.is_adaptive_verbose() {
            None
        } else {
            self.verbose
        }
    }
}
