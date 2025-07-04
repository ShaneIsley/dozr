use clap::Parser;
use humantime::parse_duration;
use std::time::Duration;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// The duration to wait (e.g., "5s", "1m30s")
    #[arg(value_parser = parse_duration)]
    pub duration: Duration,

    /// An optional jitter to add to the duration
    #[arg(long, value_parser = parse_duration)]
    pub jitter: Option<Duration>,

    /// Enable verbose output. Optionally specify update period (e.g., "500ms").
    /// If no value is given, defaults to 1 second (or adaptive for short waits).
    #[arg(long, short, value_parser = parse_duration, num_args = 0..=1, default_missing_value = "1s")]
    pub verbose: Option<Duration>,

    /// Align the wait to the next even interval (e.g., "5m", "1h").
    #[arg(long, value_parser = parse_duration)]
    pub align: Option<Duration>,
}
