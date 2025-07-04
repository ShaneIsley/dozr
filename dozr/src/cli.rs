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

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Set the update period for verbose messages (e.g., "1s", "500ms")
    #[arg(long, value_parser = parse_duration)]
    pub update_period: Option<Duration>,
}
