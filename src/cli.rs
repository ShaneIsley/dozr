use clap::Parser;
use humantime::parse_duration;
use std::time::Duration;

fn parse_positive_duration(s: &str) -> Result<Duration, String> {
    parse_duration(s).map_err(|e| e.to_string())
}

fn parse_probability(s: &str) -> Result<f64, String> {
    let prob: f64 = s.parse().map_err(|_| format!("Invalid float value: {}", s))?;
    if prob < 0.0 || prob > 1.0 {
        Err(format!("Probability must be between 0.0 and 1.0, inclusive: {}", s))
    } else {
        Ok(prob)
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// The duration to wait (e.g., "5s", "1m30s"). Must be a non-negative duration.
    #[arg(value_parser = parse_positive_duration, group = "wait_type")]
    pub duration: Option<Duration>,

    /// An optional jitter to add to the duration (e.g., "1s", "500ms"). Must be a non-negative duration.
    #[arg(long, value_parser = parse_positive_duration)]
    pub jitter: Option<Duration>,

    /// Enable verbose output. Optionally specify update period (e.g., "500ms").
    /// If no value is given, defaults to 1 second (or adaptive for short waits).
    /// Must be a non-negative duration.
    #[arg(long, short, value_parser = parse_positive_duration, num_args = 0..=1, default_missing_value = "1s")]
    pub verbose: Option<Duration>,

    /// Align the wait to the next even interval (e.g., "5m", "1h"). Must be a non-negative duration.
    #[arg(long, value_parser = parse_positive_duration, group = "wait_type")]
    pub align: Option<Duration>,

    /// The probability (0.0-1.0) that the wait will occur. Only applicable with DURATION.
    #[arg(long, value_parser = parse_probability, requires = "duration")]
    pub probability: Option<f64>,
}
