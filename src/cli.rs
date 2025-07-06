use clap::{ArgGroup, Parser};
use std::time::Duration;

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
    #[arg(short, long, value_name = "UPDATE_PERIOD", value_parser = humantime::parse_duration, num_args = 0..=1, default_missing_value = "1s")]
    pub verbose: Option<Duration>,

    /// Wait only with a certain probability (0.0 to 1.0).
    #[arg(short, long)]
    pub probability: Option<f64>,
}
