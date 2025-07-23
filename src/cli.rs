use chrono::{Duration as ChronoDuration, Local, NaiveTime, Timelike};
use crate::conditions::{self, WaitCondition};
use clap::{Parser, Subcommand};
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
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Add a random duration of jitter (e.g., "100ms").
    #[arg(short, long, value_parser = humantime::parse_duration, global = true)]
    pub jitter: Option<Duration>,

    /// Enable verbose output, with an optional update period (e.g., "250ms").
    /// If no update period is specified, adaptive verbose output is used.
    #[arg(short, long, value_name = "UPDATE_PERIOD", value_parser = humantime::parse_duration, num_args = 0..=1, default_missing_value = "1ns", global = true)]
    pub verbose: Option<Duration>,

    /// Wait only with a certain probability (0.0 to 1.0).
    #[arg(short, long, global = true)]
    pub probability: Option<f64>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Wait for a fixed duration
    #[command(alias = "d")]
    Duration {
        /// The base duration to wait (e.g., "1s", "500ms").
        #[arg(value_parser = humantime::parse_duration)]
        time: Duration,
    },
    /// Wait using a normal distribution
    #[command(alias = "n")]
    Normal {
        /// Mean of the Normal distribution (e.g., "1s").
        #[arg(value_parser = humantime::parse_duration)]
        mean: Duration,
        /// Standard deviation of the Normal distribution (e.g., "0.1").
        std_dev: f64,
    },
    /// Wait using an exponential distribution
    #[command(alias = "e")]
    Exponential {
        /// Lambda (rate parameter) of the Exponential distribution (e.g., "0.5").
        lambda: f64,
    },
    /// Wait using a log-normal distribution
    #[command(alias = "ln")]
    LogNormal {
        /// Mean of the Log-Normal distribution (e.g., "1s").
        #[arg(value_parser = humantime::parse_duration)]
        mean: Duration,
        /// Standard deviation of the Log-Normal distribution (e.g., "0.1").
        std_dev: f64,
    },
    /// Wait using a Pareto distribution
    #[command(alias = "par")]
    Pareto {
        /// Scale parameter of the Pareto distribution (e.g., "1.0").
        scale: f64,
        /// Shape parameter of the Pareto distribution (e.g., "1.5").
        shape: f64,
    },
    /// Wait using a uniform distribution
    #[command(alias = "u")]
    Uniform {
        /// Minimum value for the Uniform distribution (e.g., "1s").
        #[arg(value_parser = humantime::parse_duration)]
        min: Duration,
        /// Maximum value for the Uniform distribution (e.g., "5s").
        #[arg(value_parser = humantime::parse_duration)]
        max: Duration,
    },
    /// Wait using a triangular distribution
    #[command(alias = "t")]
    Triangular {
        /// Minimum value for the Triangular distribution (e.g., "0.0").
        min: f64,
        /// Maximum value for the Triangular distribution (e.g., "1.0").
        max: f64,
        /// Mode (most likely value) for the Triangular distribution (e.g., "0.5").
        mode: f64,
    },
    /// Wait using a gamma distribution
    #[command(alias = "g")]
    Gamma {
        /// Shape parameter of the Gamma distribution (e.g., "2.0").
        shape: f64,
        /// Scale parameter of the Gamma distribution (e.g., "1.0").
        scale: f64,
    },
    /// Align the wait to the next even interval
    #[command(aliases = &["a", "ali"])]
    Align {
        /// The interval to align to (e.g., "1m", "30s").
        #[arg(value_parser = humantime::parse_duration)]
        interval: Duration,
    },
    /// Wait until a specific time of day
    #[command()]
    At {
        /// The time to wait until (HH:MM or HH:MM:SS).
        #[arg(value_parser = parse_time_until)]
        time: Duration,
    },
}

impl Commands {
    pub fn into_wait_condition(
        self,
        jitter: Option<Duration>,
        verbose: Option<Duration>,
        probability: Option<f64>,
    ) -> Box<dyn WaitCondition> {
        match self {
            Commands::Duration { time } => {
                if let Some(probability) = probability {
                    Box::new(conditions::ProbabilisticWait {
                        duration: time,
                        probability,
                        verbose: verbose,
                    })
                } else {
                    Box::new(conditions::DurationWait {
                        duration: time,
                        jitter: jitter,
                        verbose: verbose,
                    })
                }
            }
            Commands::Normal { mean, std_dev } => Box::new(conditions::NormalWait {
                mean,
                std_dev,
                verbose: verbose,
                jitter: jitter,
            }),
            Commands::Exponential { lambda } => Box::new(conditions::ExponentialWait {
                lambda,
                verbose: verbose,
                jitter: jitter,
            }),
            Commands::LogNormal { mean, std_dev } => Box::new(conditions::LogNormalWait {
                mean,
                std_dev,
                verbose: verbose,
                jitter: jitter,
            }),
            Commands::Pareto { scale, shape } => Box::new(conditions::ParetoWait {
                scale,
                shape,
                verbose: verbose,
                jitter: jitter,
            }),
            Commands::Triangular { min, max, mode } => Box::new(conditions::TriangularWait {
                min,
                max,
                mode,
                verbose: verbose,
                jitter: jitter,
            }),
            Commands::Align { interval } => Box::new(conditions::TimeAlignWait {
                align_interval: interval,
                verbose: verbose,
            }),
            Commands::Uniform { min, max } => Box::new(conditions::UniformWait {
                min,
                max,
                verbose: verbose,
                jitter: jitter,
            }),
            Commands::At { time } => Box::new(conditions::UntilTimeWait {
                sleep_duration: time,
                verbose: verbose,
            }),
            Commands::Gamma { shape, scale } => Box::new(conditions::GammaWait {
                shape,
                scale,
                verbose: verbose,
                jitter: jitter,
            }),
        }
    }
}

pub enum WaitType {
    Duration(Duration),
    Normal { mean: Duration, std_dev: f64 },
    Exponential { lambda: f64 },
    LogNormal { mean: Duration, std_dev: f64 },
    Pareto { scale: f64, shape: f64 },
    Uniform { min: Duration, max: Duration },
    Triangular { min: f64, max: f64, mode: f64 },
    Gamma { shape: f64, scale: f64 },
    Align(Duration),
    Until(Duration),
}

impl Cli {
    pub fn get_wait_type(&self) -> WaitType {
        match &self.command {
            Commands::Duration { time } => WaitType::Duration(*time),
            Commands::Normal { mean, std_dev } => WaitType::Normal {
                mean: *mean,
                std_dev: *std_dev,
            },
            Commands::Exponential { lambda } => WaitType::Exponential { lambda: *lambda },
            Commands::LogNormal { mean, std_dev } => WaitType::LogNormal {
                mean: *mean,
                std_dev: *std_dev,
            },
            Commands::Pareto { scale, shape } => WaitType::Pareto {
                scale: *scale,
                shape: *shape,
            },
            Commands::Uniform { min, max } => WaitType::Uniform {
                min: *min,
                max: *max,
            },
            Commands::Triangular { min, max, mode } => WaitType::Triangular {
                min: *min,
                max: *max,
                mode: *mode,
            },
            Commands::Gamma { shape, scale } => WaitType::Gamma {
                shape: *shape,
                scale: *scale,
            },
            Commands::Align { interval } => WaitType::Align(*interval),
            Commands::At { time } => WaitType::Until(*time),
        }
    }

    pub fn is_adaptive_verbose(&self) -> bool {
        self.verbose == Some(Duration::from_nanos(1))
    }

    /// Returns the verbose update period, if specified.
    ///
    /// If adaptive verbose output is enabled, this method returns `None`.
    /// Otherwise, it returns `Some(Duration)` with the specified update period.
    pub fn verbose_period(&self) -> Option<Duration> {
        if self.is_adaptive_verbose() {
            None
        } else {
            self.verbose
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;

    #[test]
    fn test_parse_time_until_in_future() {
        let now = Local::now();
        let future_time = now + ChronoDuration::minutes(1);
        let time_str = future_time.format("%H:%M:%S").to_string();
        let duration = parse_time_until(&time_str).unwrap();
        assert!(duration > Duration::from_secs(50) && duration <= Duration::from_secs(60));
    }

    #[test]
    fn test_parse_time_until_in_past_rolls_to_next_day() {
        let now = Local::now();
        let past_time = now - ChronoDuration::minutes(1);
        let time_str = past_time.format("%H:%M:%S").to_string();
        let duration = parse_time_until(&time_str).unwrap();
        // Should be almost 24 hours
        assert!(duration > Duration::from_secs(23 * 3600));
    }

    #[test]
    fn test_parse_time_invalid_format() {
        assert!(parse_time_until("invalid-time").is_err());
        assert!(parse_time_until("25:00").is_err());
        assert!(parse_time_until("10:65").is_err());
    }
}