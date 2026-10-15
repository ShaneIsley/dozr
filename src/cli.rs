use crate::conditions::{self, WaitCondition};
use chrono::{DateTime, Duration as ChronoDuration, LocalResult, NaiveTime, Offset, TimeZone};
use clap::{Parser, Subcommand};
use std::time::Duration;

/// Time from `now` until the next occurrence of the wall-clock time `target`
/// in `now`'s time zone.
///
/// - A target equal to `now` returns zero; an earlier target rolls to tomorrow.
/// - A target inside a daylight-saving gap (a local time that does not exist)
///   is moved forward by the length of the gap, so 02:30 on a spring-forward
///   night becomes 03:30.
/// - A target that occurs twice (fall back) uses the earliest occurrence that
///   is not in the past.
pub fn duration_until<Tz: TimeZone>(
    now: &DateTime<Tz>,
    target: NaiveTime,
) -> Result<Duration, String> {
    let tz = now.timezone();
    let mut date = now.date_naive();

    // Two days is always enough: today, or failing that tomorrow. The third
    // iteration only guards against a zone that skips an entire day.
    for _ in 0..3 {
        let naive = date.and_time(target);
        let candidate = match tz.from_local_datetime(&naive) {
            LocalResult::Single(dt) => (dt >= *now).then_some(dt),
            LocalResult::Ambiguous(earliest, latest) => {
                if earliest >= *now {
                    Some(earliest)
                } else if latest >= *now {
                    Some(latest)
                } else {
                    None
                }
            }
            LocalResult::None => {
                // Interpret the wall time using the offset in force before the
                // gap, which lands just after it.
                let before = tz.offset_from_utc_datetime(&(naive - ChronoDuration::days(1)));
                let utc =
                    naive - ChronoDuration::seconds(i64::from(before.fix().local_minus_utc()));
                let dt = tz.from_utc_datetime(&utc);
                (dt >= *now).then_some(dt)
            }
        };

        if let Some(target_dt) = candidate {
            return target_dt
                .signed_duration_since(now.clone())
                .to_std()
                .map_err(|e| e.to_string());
        }
        date = date
            .succ_opt()
            .ok_or_else(|| "date out of range".to_string())?;
    }
    Err(format!("could not resolve {target} to a future time"))
}

fn parse_time_until(s: &str) -> Result<Duration, String> {
    let parsed_time = NaiveTime::parse_from_str(s, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(s, "%H:%M:%S"))
        .map_err(|_| format!("Invalid time format. Expected HH:MM or HH:MM:SS: {s}"))?;
    duration_until(&chrono::Local::now(), parsed_time)
}

fn parse_finite(s: &str) -> Result<f64, String> {
    let v: f64 = s.parse().map_err(|_| format!("'{s}' is not a number"))?;
    if v.is_finite() {
        Ok(v)
    } else {
        Err("must be a finite number".to_string())
    }
}

fn parse_positive(s: &str) -> Result<f64, String> {
    let v = parse_finite(s)?;
    if v > 0.0 {
        Ok(v)
    } else {
        Err(format!("must be greater than 0, got {v}"))
    }
}

fn parse_non_negative(s: &str) -> Result<f64, String> {
    let v = parse_finite(s)?;
    if v >= 0.0 {
        Ok(v)
    } else {
        Err(format!("must be 0 or greater, got {v}"))
    }
}

fn parse_probability(s: &str) -> Result<f64, String> {
    let v = parse_finite(s)?;
    if (0.0..=1.0).contains(&v) {
        Ok(v)
    } else {
        Err(format!("must be between 0.0 and 1.0, got {v}"))
    }
}

fn parse_positive_duration(s: &str) -> Result<Duration, String> {
    let d = humantime::parse_duration(s).map_err(|e| e.to_string())?;
    if d.is_zero() {
        Err("must be greater than 0".to_string())
    } else {
        Ok(d)
    }
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
    #[arg(short, long, global = true, value_parser = parse_probability, allow_negative_numbers = true)]
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
        /// Standard deviation in seconds (e.g., "0.1"). Must be 0 or greater.
        #[arg(value_parser = parse_non_negative, allow_negative_numbers = true)]
        std_dev: f64,
    },
    /// Wait using an exponential distribution
    #[command(alias = "e")]
    Exponential {
        /// Rate in events per second (e.g., "0.5" gives a 2s mean). Must be greater than 0.
        #[arg(value_parser = parse_positive, allow_negative_numbers = true)]
        lambda: f64,
    },
    /// Wait using a log-normal distribution
    #[command(alias = "ln")]
    LogNormal {
        /// Mean of the sampled waits (e.g., "1s"). Must be greater than 0.
        #[arg(value_parser = parse_positive_duration)]
        mean: Duration,
        /// Standard deviation of the sampled waits, in seconds (e.g., "0.1").
        #[arg(value_parser = parse_non_negative, allow_negative_numbers = true)]
        std_dev: f64,
    },
    /// Wait using a Pareto distribution
    #[command(alias = "par")]
    Pareto {
        /// Scale (minimum wait) in seconds (e.g., "1.0"). Must be greater than 0.
        #[arg(value_parser = parse_positive, allow_negative_numbers = true)]
        scale: f64,
        /// Shape parameter (e.g., "1.5"). Must be greater than 0.
        #[arg(value_parser = parse_positive, allow_negative_numbers = true)]
        shape: f64,
    },
    /// Wait using a uniform distribution
    #[command(alias = "u")]
    Uniform {
        /// Minimum value for the Uniform distribution (e.g., "1s").
        #[arg(value_parser = humantime::parse_duration)]
        min: Duration,
        /// Maximum value for the Uniform distribution (e.g., "5s"). Must be greater than MIN.
        #[arg(value_parser = humantime::parse_duration)]
        max: Duration,
    },
    /// Wait using a triangular distribution
    #[command(alias = "t")]
    Triangular {
        /// Minimum value in seconds (e.g., "0.0").
        #[arg(value_parser = parse_finite, allow_negative_numbers = true)]
        min: f64,
        /// Maximum value in seconds (e.g., "1.0"). Must be greater than MIN.
        #[arg(value_parser = parse_finite, allow_negative_numbers = true)]
        max: f64,
        /// Most likely value in seconds (e.g., "0.5"). Must be between MIN and MAX.
        #[arg(value_parser = parse_finite, allow_negative_numbers = true)]
        mode: f64,
    },
    /// Wait using a gamma distribution
    #[command(alias = "g")]
    Gamma {
        /// Shape parameter (e.g., "2.0"). Must be greater than 0.
        #[arg(value_parser = parse_positive, allow_negative_numbers = true)]
        shape: f64,
        /// Scale parameter in seconds (e.g., "1.0"). Must be greater than 0.
        #[arg(value_parser = parse_positive, allow_negative_numbers = true)]
        scale: f64,
    },
    /// Align the wait to the next even interval
    #[command(aliases = &["a", "ali"])]
    Align {
        /// The interval to align to (e.g., "1m", "30s"). Must be greater than 0.
        #[arg(value_parser = parse_positive_duration)]
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
    /// Checks constraints that span more than one argument.
    ///
    /// Single-argument constraints are enforced by clap value parsers.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Commands::Uniform { min, max } if min >= max => Err(format!(
                "uniform <MIN> must be less than <MAX>, got {} and {}",
                humantime::format_duration(*min),
                humantime::format_duration(*max)
            )),
            Commands::Triangular { min, max, .. } if min >= max => Err(format!(
                "triangular <MIN> must be less than <MAX>, got {min} and {max}"
            )),
            Commands::Triangular { min, max, mode } if mode < min || mode > max => Err(format!(
                "triangular <MODE> must be between <MIN> and <MAX>, got {mode} outside {min}..{max}"
            )),
            _ => Ok(()),
        }
    }

    /// Builds the wait condition for this command.
    ///
    /// Jitter applies to every command except `align` and `at`, whose purpose
    /// is to end at an exact time. Probability applies to every command.
    pub fn into_wait_condition(
        self,
        jitter: Option<Duration>,
        verbose: Option<Duration>,
        probability: Option<f64>,
    ) -> Box<dyn WaitCondition> {
        let base: Box<dyn WaitCondition> = match self {
            Commands::Duration { time } => Box::new(conditions::DurationWait {
                duration: time,
                jitter,
                verbose,
            }),
            Commands::Normal { mean, std_dev } => Box::new(conditions::NormalWait {
                mean,
                std_dev,
                verbose,
                jitter,
            }),
            Commands::Exponential { lambda } => Box::new(conditions::ExponentialWait {
                lambda,
                verbose,
                jitter,
            }),
            Commands::LogNormal { mean, std_dev } => Box::new(conditions::LogNormalWait {
                mean,
                std_dev,
                verbose,
                jitter,
            }),
            Commands::Pareto { scale, shape } => Box::new(conditions::ParetoWait {
                scale,
                shape,
                verbose,
                jitter,
            }),
            Commands::Triangular { min, max, mode } => Box::new(conditions::TriangularWait {
                min,
                max,
                mode,
                verbose,
                jitter,
            }),
            Commands::Align { interval } => Box::new(conditions::TimeAlignWait {
                align_interval: interval,
                verbose,
            }),
            Commands::Uniform { min, max } => Box::new(conditions::UniformWait {
                min,
                max,
                verbose,
                jitter,
            }),
            Commands::At { time } => Box::new(conditions::UntilTimeWait {
                sleep_duration: time,
                verbose,
            }),
            Commands::Gamma { shape, scale } => Box::new(conditions::GammaWait {
                shape,
                scale,
                verbose,
                jitter,
            }),
        };

        match probability {
            Some(probability) => Box::new(conditions::ProbabilityGate {
                inner: base,
                probability,
                verbose,
            }),
            None => base,
        }
    }
}

impl Cli {
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
    use chrono::{FixedOffset, Utc};
    use chrono_tz::America::New_York;

    fn hms(h: u32, m: u32, s: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, s).unwrap()
    }

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    #[test]
    fn test_duration_until_later_today() {
        let now = Utc.with_ymd_and_hms(2025, 6, 1, 10, 0, 0).unwrap();
        assert_eq!(duration_until(&now, hms(10, 1, 0)), Ok(secs(60)));
    }

    #[test]
    fn test_duration_until_exactly_now_is_zero() {
        let now = Utc.with_ymd_and_hms(2025, 6, 1, 10, 0, 0).unwrap();
        assert_eq!(duration_until(&now, hms(10, 0, 0)), Ok(Duration::ZERO));
    }

    #[test]
    fn test_duration_until_past_rolls_to_tomorrow() {
        let now = Utc.with_ymd_and_hms(2025, 6, 1, 10, 0, 0).unwrap();
        assert_eq!(
            duration_until(&now, hms(9, 59, 0)),
            Ok(secs(24 * 3600 - 60))
        );
    }

    #[test]
    fn test_duration_until_across_midnight() {
        let now = Utc.with_ymd_and_hms(2025, 6, 1, 23, 59, 58).unwrap();
        assert_eq!(duration_until(&now, hms(0, 0, 3)), Ok(secs(5)));
    }

    #[test]
    fn test_duration_until_respects_fixed_offset() {
        let tz = FixedOffset::east_opt(5 * 3600 + 1800).unwrap();
        let now = tz.with_ymd_and_hms(2025, 6, 1, 8, 0, 0).unwrap();
        assert_eq!(duration_until(&now, hms(9, 30, 0)), Ok(secs(5400)));
    }

    #[test]
    fn test_duration_until_spring_forward_gap_does_not_panic() {
        // 2025-03-09: New York clocks jump from 02:00 EST to 03:00 EDT.
        let now = New_York.with_ymd_and_hms(2025, 3, 9, 0, 0, 0).unwrap();
        // 02:30 does not exist; it resolves to 03:30 EDT (07:30 UTC), which
        // is 2h30m after 00:00 EST (05:00 UTC).
        assert_eq!(duration_until(&now, hms(2, 30, 0)), Ok(secs(9000)));
    }

    #[test]
    fn test_duration_until_spring_forward_gap_already_past() {
        // At 04:00 EDT the gap has passed, so 02:30 means tomorrow at 02:30 EDT.
        let now = New_York.with_ymd_and_hms(2025, 3, 9, 4, 0, 0).unwrap();
        assert_eq!(
            duration_until(&now, hms(2, 30, 0)),
            Ok(secs(22 * 3600 + 1800))
        );
    }

    #[test]
    fn test_duration_until_fall_back_uses_earliest() {
        // 2025-11-02: New York clocks fall back from 02:00 EDT to 01:00 EST.
        let now = New_York.with_ymd_and_hms(2025, 11, 2, 0, 0, 0).unwrap();
        // 01:30 happens twice; the first is 01:30 EDT, 1h30m after 00:00 EDT.
        assert_eq!(duration_until(&now, hms(1, 30, 0)), Ok(secs(5400)));
    }

    #[test]
    fn test_duration_until_fall_back_uses_second_occurrence_when_first_passed() {
        // 01:45 EDT (05:45 UTC): the first 01:30 has passed, the second
        // (01:30 EST, 06:30 UTC) is 45 minutes away.
        let now = New_York.from_utc_datetime(
            &Utc.with_ymd_and_hms(2025, 11, 2, 5, 45, 0)
                .unwrap()
                .naive_utc(),
        );
        assert_eq!(duration_until(&now, hms(1, 30, 0)), Ok(secs(2700)));
    }

    #[test]
    fn test_parse_time_invalid_format() {
        assert!(parse_time_until("invalid-time").is_err());
        assert!(parse_time_until("25:00").is_err());
        assert!(parse_time_until("10:65").is_err());
    }

    #[test]
    fn test_number_parsers() {
        assert_eq!(parse_probability("0.0"), Ok(0.0));
        assert_eq!(parse_probability("1"), Ok(1.0));
        assert!(parse_probability("1.01").is_err());
        assert!(parse_probability("-0.1").is_err());
        assert!(parse_probability("NaN").is_err());
        assert_eq!(parse_non_negative("0"), Ok(0.0));
        assert!(parse_non_negative("-1").is_err());
        assert!(parse_positive("0").is_err());
        assert!(parse_positive("inf").is_err());
        assert!(parse_positive_duration("0s").is_err());
        assert_eq!(parse_positive_duration("1ms"), Ok(Duration::from_millis(1)));
    }

    #[test]
    fn test_validate_cross_argument_rules() {
        let uniform = |min, max| Commands::Uniform {
            min: Duration::from_secs(min),
            max: Duration::from_secs(max),
        };
        assert!(uniform(1, 2).validate().is_ok());
        assert!(uniform(2, 2).validate().is_err());
        assert!(uniform(5, 1).validate().is_err());

        let tri = |min, max, mode| Commands::Triangular { min, max, mode };
        assert!(tri(0.0, 1.0, 0.5).validate().is_ok());
        assert!(tri(0.0, 1.0, 0.0).validate().is_ok());
        assert!(tri(0.0, 1.0, 1.0).validate().is_ok());
        assert!(tri(1.0, 1.0, 1.0).validate().is_err());
        assert!(tri(0.0, 1.0, 1.5).validate().is_err());
        assert!(tri(0.0, 1.0, -0.5).validate().is_err());
    }
}
