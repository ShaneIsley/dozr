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
}
