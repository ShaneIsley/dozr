use crate::conditions::{self, WaitCondition};
use crate::{MIN_VERBOSE_PERIOD, Verbose};
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

fn parse_probability(s: &str) -> Result<f64, String> {
    let v = parse_finite(s)?;
    if (0.0..=1.0).contains(&v) {
        Ok(v)
    } else {
        Err(format!("must be between 0.0 and 1.0, got {v}"))
    }
}

/// Parses a length of time such as `500ms` or `1m30s`.
///
/// A bare number, even `0`, is rejected with a hint to add a unit, rather
/// than humantime's terse "invalid character" or "time unit needed".
fn parse_time(s: &str) -> Result<Duration, String> {
    if s.parse::<f64>().is_ok() {
        return Err(format!("a unit is required, for example {s}s or {s}ms"));
    }
    humantime::parse_duration(s).map_err(|e| e.to_string())
}

/// Parses the `--verbose` value: `adaptive`, `off`, or an update period of
/// at least [`MIN_VERBOSE_PERIOD`].
fn parse_verbose(s: &str) -> Result<Verbose, String> {
    match s {
        "adaptive" => Ok(Verbose::Adaptive),
        "off" => Ok(Verbose::Off),
        _ => {
            let period = parse_time(s)?;
            if period < MIN_VERBOSE_PERIOD {
                Err(format!(
                    "update period must be at least {}",
                    humantime::format_duration(MIN_VERBOSE_PERIOD)
                ))
            } else {
                Ok(Verbose::Every(period))
            }
        }
    }
}

fn parse_positive_duration(s: &str) -> Result<Duration, String> {
    let d = parse_time(s)?;
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
    #[arg(short, long, value_parser = parse_time, global = true)]
    pub jitter: Option<Duration>,

    /// Show progress on stderr. With no value, updates come less often as the
    /// remaining time grows; with a period (at least 1ms, e.g. "250ms"), they
    /// come on that period.
    #[arg(
        short,
        long,
        value_name = "UPDATE_PERIOD",
        value_parser = parse_verbose,
        num_args = 0..=1,
        default_value = "off",
        hide_default_value = true,
        default_missing_value = "adaptive",
        global = true
    )]
    pub verbose: Verbose,

    /// Wait only with a certain probability (0.0 to 1.0).
    #[arg(short, long, global = true, value_parser = parse_probability, allow_negative_numbers = true)]
    pub probability: Option<f64>,
}

/// Every argument that is a length of time takes a value such as `500ms`,
/// `2s`, or `1m30s`. Bare numbers are only used for rates and shape
/// parameters, which have no unit.
///
/// Distributions that can produce a negative value (only `normal`, in
/// practice) wait for zero instead.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Wait for a fixed duration
    #[command(alias = "d")]
    Duration {
        /// The base duration to wait (e.g., "1s", "500ms").
        #[arg(value_parser = parse_time)]
        time: Duration,
    },
    /// Wait using a normal distribution. Negative samples wait for zero.
    #[command(alias = "n")]
    Normal {
        /// Mean wait (e.g., "1s").
        #[arg(value_parser = parse_time)]
        mean: Duration,
        /// Standard deviation (e.g., "100ms").
        #[arg(value_parser = parse_time)]
        std_dev: Duration,
    },
    /// Wait using an exponential distribution
    #[command(alias = "e")]
    Exponential {
        /// Rate in events per second, a bare number (e.g., "0.5" gives a 2s mean). Must be greater than 0.
        #[arg(value_parser = parse_positive, allow_negative_numbers = true)]
        lambda: f64,
    },
    /// Wait using a log-normal distribution
    #[command(alias = "ln")]
    LogNormal {
        /// Mean wait (e.g., "1s"). Must be greater than 0.
        #[arg(value_parser = parse_positive_duration)]
        mean: Duration,
        /// Standard deviation of the waits (e.g., "250ms").
        #[arg(value_parser = parse_time)]
        std_dev: Duration,
    },
    /// Wait using a Pareto distribution
    #[command(alias = "par")]
    Pareto {
        /// Minimum wait (e.g., "1s"). Must be greater than 0.
        #[arg(value_parser = parse_positive_duration)]
        scale: Duration,
        /// Shape, a bare number (e.g., "1.5"); smaller means a heavier tail. Must be greater than 0.
        #[arg(value_parser = parse_positive, allow_negative_numbers = true)]
        shape: f64,
    },
    /// Wait using a uniform distribution
    #[command(alias = "u")]
    Uniform {
        /// Minimum wait (e.g., "1s").
        #[arg(value_parser = parse_time)]
        min: Duration,
        /// Maximum wait (e.g., "5s"). Must be greater than MIN.
        #[arg(value_parser = parse_time)]
        max: Duration,
    },
    /// Wait using a triangular distribution
    #[command(alias = "t")]
    Triangular {
        /// Minimum wait (e.g., "1s").
        #[arg(value_parser = parse_time)]
        min: Duration,
        /// Maximum wait (e.g., "5s"). Must be greater than MIN.
        #[arg(value_parser = parse_time)]
        max: Duration,
        /// Most likely wait (e.g., "2s"). Must be between MIN and MAX.
        #[arg(value_parser = parse_time)]
        mode: Duration,
    },
    /// Wait using a gamma distribution
    #[command(alias = "g")]
    Gamma {
        /// Shape, a bare number (e.g., "2.0"). Must be greater than 0.
        #[arg(value_parser = parse_positive, allow_negative_numbers = true)]
        shape: f64,
        /// Scale (e.g., "1s"). The mean wait is SHAPE times SCALE. Must be greater than 0.
        #[arg(value_parser = parse_positive_duration)]
        scale: Duration,
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
                "triangular <MIN> must be less than <MAX>, got {} and {}",
                humantime::format_duration(*min),
                humantime::format_duration(*max)
            )),
            Commands::Triangular { min, max, mode } if mode < min || mode > max => Err(format!(
                "triangular <MODE> must be between <MIN> and <MAX>, got {} outside {}..{}",
                humantime::format_duration(*mode),
                humantime::format_duration(*min),
                humantime::format_duration(*max)
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
        verbose: Verbose,
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
        assert!(parse_positive("0").is_err());
        assert!(parse_positive("inf").is_err());
        assert!(parse_positive_duration("0s").is_err());
        assert_eq!(parse_time("0s"), Ok(Duration::ZERO));
        assert!(parse_time("0").unwrap_err().contains("a unit is required"));
        assert_eq!(parse_time("1m30s"), Ok(Duration::from_secs(90)));
        assert!(
            parse_time("0.1")
                .unwrap_err()
                .contains("a unit is required")
        );
        assert!(parse_time("2").unwrap_err().contains("a unit is required"));
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

        let tri = |min, max, mode| Commands::Triangular {
            min: Duration::from_millis(min),
            max: Duration::from_millis(max),
            mode: Duration::from_millis(mode),
        };
        assert!(tri(0, 1000, 500).validate().is_ok());
        assert!(tri(0, 1000, 0).validate().is_ok());
        assert!(tri(0, 1000, 1000).validate().is_ok());
        assert!(tri(1000, 1000, 1000).validate().is_err());
        assert!(tri(0, 1000, 1500).validate().is_err());
        assert!(tri(500, 1000, 100).validate().is_err());
    }

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("dozr").chain(args.iter().copied()))
    }

    #[test]
    fn test_verbose_parsing() {
        assert_eq!(parse(&["d", "1s"]).unwrap().verbose, Verbose::Off);
        assert_eq!(
            parse(&["d", "1s", "-v"]).unwrap().verbose,
            Verbose::Adaptive
        );
        assert_eq!(
            parse(&["d", "1s", "-v", "500ms"]).unwrap().verbose,
            Verbose::Every(Duration::from_millis(500))
        );
        // Global flag given before the subcommand.
        assert_eq!(
            parse(&["-v", "500ms", "d", "1s"]).unwrap().verbose,
            Verbose::Every(Duration::from_millis(500))
        );
        // A bare `-v` directly before the subcommand would take the subcommand
        // name as its value, so it must come after the subcommand instead.
        assert_eq!(
            parse(&["d", "1s", "-v", "1ms"]).unwrap().verbose,
            Verbose::Every(Duration::from_millis(1))
        );
        assert!(parse(&["d", "1s", "-v", "1ns"]).is_err());
        assert!(parse(&["d", "1s", "-v", "999us"]).is_err());
        assert!(parse(&["d", "1s", "-v", "0s"]).is_err());
    }

    #[test]
    fn test_time_arguments_require_units() {
        // Bare numbers are rejected wherever a length of time is expected.
        for args in [
            &["n", "1s", "0.1"][..],
            &["ln", "1s", "0.1"],
            &["par", "1", "2"],
            &["u", "1", "2"],
            &["t", "1", "3", "2"],
            &["g", "2", "1.5"],
        ] {
            assert!(parse(args).is_err(), "{args:?} should be rejected");
        }
        // The same commands with units are accepted.
        for args in [
            &["n", "1s", "100ms"][..],
            &["ln", "1s", "100ms"],
            &["par", "1s", "2"],
            &["u", "1s", "2s"],
            &["t", "1s", "3s", "2s"],
            &["g", "2", "1500ms"],
        ] {
            assert!(parse(args).is_ok(), "{args:?} should be accepted");
        }
    }
}
