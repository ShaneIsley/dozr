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
#[command(group = ArgGroup::new("wait_type").required(true).multiple(false).args([
    "duration",
    "normal",
    "exponential",
    "log_normal",
    "pareto",
    "weibull",
    "uniform",
    "triangular",
    "align",
    "until",
]))]
pub struct Cli {
    /// The base duration to wait (e.g., "1s", "500ms").
    #[arg(long, value_parser = humantime::parse_duration, group = "wait_type")]
    pub duration: Option<Duration>,

    /// Use a Normal distribution for the wait duration.
    #[arg(long, group = "wait_type")]
    pub normal: bool,

    /// Mean of the Normal distribution (e.g., "1s").
    #[arg(long, value_parser = humantime::parse_duration, required_if_eq("normal", "true"))]
    pub normal_mean: Option<Duration>,

    /// Standard deviation of the Normal distribution (e.g., "0.1").
    #[arg(long, value_parser = clap::value_parser!(f64), required_if_eq("normal", "true"))]
    pub normal_std_dev: Option<f64>,

    /// Use an Exponential distribution for the wait duration.
    #[arg(long, group = "wait_type")]
    pub exponential: bool,

    /// Lambda (rate parameter) of the Exponential distribution (e.g., "0.5").
    #[arg(long, value_parser = clap::value_parser!(f64), required_if_eq("exponential", "true"))]
    pub exponential_lambda: Option<f64>,

    /// Use a Log-Normal distribution for the wait duration.
    #[arg(long, group = "wait_type")]
    pub log_normal: bool,

    /// Mean of the Log-Normal distribution (e.g., "1s").
    #[arg(long, value_parser = humantime::parse_duration, required_if_eq("log_normal", "true"))]
    pub log_normal_mean: Option<Duration>,

    /// Standard deviation of the Log-Normal distribution (e.g., "0.1").
    #[arg(long, value_parser = clap::value_parser!(f64), required_if_eq("log_normal", "true"))]
    pub log_normal_std_dev: Option<f64>,

    /// Use a Pareto distribution for the wait duration.
    #[arg(long, group = "wait_type")]
    pub pareto: bool,

    /// Scale parameter of the Pareto distribution (e.g., "1.0").
    #[arg(long, value_parser = clap::value_parser!(f64), required_if_eq("pareto", "true"))]
    pub pareto_scale: Option<f64>,

    /// Shape parameter of the Pareto distribution (e.g., "1.5").
    #[arg(long, value_parser = clap::value_parser!(f64), required_if_eq("pareto", "true"))]
    pub pareto_shape: Option<f64>,

    /// Use a Weibull distribution for the wait duration.
    #[arg(long, group = "wait_type")]
    pub weibull: bool,

    /// Shape parameter of the Weibull distribution (e.g., "1.5").
    #[arg(long, value_parser = clap::value_parser!(f64), required_if_eq("weibull", "true"))]
    pub weibull_shape: Option<f64>,

    /// Scale parameter of the Weibull distribution (e.g., "1.0").
    #[arg(long, value_parser = clap::value_parser!(f64), required_if_eq("weibull", "true"))]
    pub weibull_scale: Option<f64>,

    /// Use a Uniform distribution for the wait duration.
    #[arg(long, group = "wait_type")]
    pub uniform: bool,

    /// Minimum value for the Uniform distribution (e.g., "1s").
    #[arg(long, value_parser = humantime::parse_duration, required_if_eq("uniform", "true"))]
    pub uniform_min: Option<Duration>,

    /// Maximum value for the Uniform distribution (e.g., "5s").
    #[arg(long, value_parser = humantime::parse_duration, required_if_eq("uniform", "true"))]
    pub uniform_max: Option<Duration>,

    /// Use a Triangular distribution for the wait duration.
    #[arg(long, group = "wait_type")]
    pub triangular: bool,

    /// Minimum value for the Triangular distribution (e.g., "0.0").
    #[arg(long, value_parser = clap::value_parser!(f64), required_if_eq("triangular", "true"))]
    pub triangular_min: Option<f64>,

    /// Maximum value for the Triangular distribution (e.g., "1.0").
    #[arg(long, value_parser = clap::value_parser!(f64), required_if_eq("triangular", "true"))]
    pub triangular_max: Option<f64>,

    /// Mode (most likely value) for the Triangular distribution (e.g., "0.5").
    #[arg(long, value_parser = clap::value_parser!(f64), required_if_eq("triangular", "true"))]
    pub triangular_mode: Option<f64>,

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

pub enum WaitType {
    Duration(Duration),
    Normal { mean: Duration, std_dev: f64 },
    Exponential { lambda: f64 },
    LogNormal { mean: Duration, std_dev: f64 },
    Pareto { scale: f64, shape: f64 },
    Weibull { shape: f64, scale: f64 },
    Uniform { min: Duration, max: Duration },
    Triangular { min: f64, max: f64, mode: f64 },
    Align(Duration),
    Until(Duration),
}

impl Cli {
    pub fn get_wait_type(&self) -> WaitType {
        if let Some(duration) = self.duration {
            WaitType::Duration(duration)
        } else if self.normal {
            // These unwraps are safe because of requires_all in clap
            WaitType::Normal {
                mean: self.normal_mean.unwrap(),
                std_dev: self.normal_std_dev.unwrap(),
            }
        } else if self.exponential {
            // This unwrap is safe because of requires in clap
            WaitType::Exponential {
                lambda: self.exponential_lambda.unwrap(),
            }
        } else if self.log_normal {
            // These unwraps are safe because of requires_all in clap
            WaitType::LogNormal {
                mean: self.log_normal_mean.unwrap(),
                std_dev: self.log_normal_std_dev.unwrap(),
            }
        } else if self.pareto {
            // These unwraps are safe because of requires_all in clap
            WaitType::Pareto {
                scale: self.pareto_scale.unwrap(),
                shape: self.pareto_shape.unwrap(),
            }
        } else if self.weibull {
            // These unwraps are safe because of requires_all in clap
            WaitType::Weibull {
                shape: self.weibull_shape.unwrap(),
                scale: self.weibull_scale.unwrap(),
            }
        } else if self.uniform {
            // These unwraps are safe because of requires_all in clap
            WaitType::Uniform {
                min: self.uniform_min.unwrap(),
                max: self.uniform_max.unwrap(),
            }
        } else if self.triangular {
            // These unwraps are safe because of requires_all in clap
            WaitType::Triangular {
                min: self.triangular_min.unwrap(),
                max: self.triangular_max.unwrap(),
                mode: self.triangular_mode.unwrap(),
            }
        } else if let Some(align) = self.align {
            WaitType::Align(align)
        } else if let Some(until) = self.until {
            WaitType::Until(until)
        } else {
            // This case should ideally not be reached due to clap's required group
            // but as a fallback, we can default to a 0 duration.
            WaitType::Duration(Duration::new(0, 0))
        }
    }

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