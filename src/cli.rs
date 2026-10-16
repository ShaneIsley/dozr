//! Command-line parsing for `dozr`.

use crate::Verbose;
use crate::conditions::{Sampler, Wait};
use chrono::NaiveTime;
use clap::{Parser, Subcommand};
use std::time::Duration;

fn parse_time_of_day(s: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(s, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(s, "%H:%M:%S"))
        .map_err(|_| format!("Invalid time format. Expected HH:MM or HH:MM:SS: {s}"))
}

/// Parses a length of time such as `500ms` or `1m30s`.
///
/// A bare number is rejected with a hint the parser will accept. The parser
/// takes whole numbers of units only, so `0.1` is steered to `100ms` rather
/// than to `0.1s`.
fn parse_time(s: &str) -> Result<Duration, String> {
    if let Ok(n) = s.parse::<f64>() {
        return Err(unit_hint(n, s));
    }
    humantime::parse_duration(s).map_err(|e| e.to_string())
}

/// The error for a bare number where a length of time is expected.
fn unit_hint(n: f64, s: &str) -> String {
    const GENERIC: &str = "a whole number of units is required, for example 500ms or 2s";
    if !n.is_finite() || !(0.0..1e15).contains(&n) {
        return GENERIC.to_string();
    }
    if n.fract() == 0.0 {
        let n = n as u64;
        return format!("a unit is required, for example {n}s or {n}ms");
    }
    // A decimal: suggest the same amount of seconds in the largest unit that
    // expresses it exactly.
    for (unit, per_second) in [("ms", 1e3), ("us", 1e6), ("ns", 1e9)] {
        let v = n * per_second;
        if v >= 1.0 && (v - v.round()).abs() < 1e-6 {
            return format!(
                "a unit is required; if you meant {s} seconds, write {}{unit}",
                v.round() as u64
            );
        }
    }
    GENERIC.to_string()
}

/// Parses the `--verbose` value: `adaptive`, `off`, or an update period.
/// The minimum period is enforced by [`Wait::validate`].
fn parse_verbose(s: &str) -> Result<Verbose, String> {
    match s {
        "adaptive" => Ok(Verbose::Adaptive),
        "off" => Ok(Verbose::Off),
        _ => parse_time(s).map(Verbose::Every),
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
    #[arg(short, long, global = true, allow_negative_numbers = true)]
    pub probability: Option<f64>,

    /// Never wait less than this (e.g., "500ms"), after jitter.
    // Ids differ from the uniform and triangular <MIN>/<MAX> arguments,
    // which clap would otherwise reject as duplicates.
    #[arg(id = "floor", long = "min", value_name = "TIME", value_parser = parse_time, global = true)]
    pub min: Option<Duration>,

    /// Never wait more than this (e.g., "30s"), after jitter.
    #[arg(id = "ceiling", long = "max", value_name = "TIME", value_parser = parse_time, global = true)]
    pub max: Option<Duration>,

    /// Seed the random number generator, so the same arguments always pick
    /// the same duration.
    #[arg(long, value_name = "SEED", global = true)]
    pub seed: Option<u64>,

    /// Print the chosen duration (e.g., "1s 250ms") instead of waiting.
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Exit with this status when a --probability wait is skipped.
    /// Statuses 1 and 2 also mean errors, so 3 or above is clearest.
    #[arg(long, value_name = "STATUS", default_value_t = 0, global = true)]
    pub skip_exit_code: u8,
}

/// How to carry out a wait: everything on the command line that is not part
/// of the [`Wait`] itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunOptions {
    /// Seed for the random number generator; `None` means unpredictable.
    pub seed: Option<u64>,
    /// Print the chosen duration instead of waiting.
    pub dry_run: bool,
    /// Exit status when a probabilistic wait is skipped.
    pub skip_exit_code: u8,
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
        #[arg(allow_negative_numbers = true)]
        lambda: f64,
    },
    /// Wait using a log-normal distribution
    #[command(alias = "ln")]
    LogNormal {
        /// Mean wait (e.g., "1s"). Must be greater than 0.
        #[arg(value_parser = parse_time)]
        mean: Duration,
        /// Standard deviation of the waits (e.g., "250ms").
        #[arg(value_parser = parse_time)]
        std_dev: Duration,
    },
    /// Wait using a Pareto distribution
    #[command(alias = "par")]
    Pareto {
        /// Minimum wait (e.g., "1s"). Must be greater than 0.
        #[arg(value_parser = parse_time)]
        scale: Duration,
        /// Shape, a bare number (e.g., "1.5"); smaller means a heavier tail. Must be greater than 0.
        #[arg(allow_negative_numbers = true)]
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
        #[arg(allow_negative_numbers = true)]
        shape: f64,
        /// Scale (e.g., "1s"). The mean wait is SHAPE times SCALE. Must be greater than 0.
        #[arg(value_parser = parse_time)]
        scale: Duration,
    },
    /// Wait until the next boundary: local midnight plus whole multiples of the interval
    #[command(aliases = &["a", "ali"])]
    Align {
        /// The interval to align to (e.g., "1m", "30s"), measured from local midnight. Must be at least 1ms.
        #[arg(value_parser = parse_time)]
        interval: Duration,
    },
    /// Wait until a specific time of day
    #[command()]
    At {
        /// The time to wait until (HH:MM or HH:MM:SS), in local time.
        #[arg(value_parser = parse_time_of_day)]
        time: NaiveTime,
    },
}

impl Commands {
    /// The sampler this command describes.
    pub fn into_sampler(self) -> Sampler {
        match self {
            Commands::Duration { time } => Sampler::Fixed(time),
            Commands::Normal { mean, std_dev } => Sampler::Normal { mean, std_dev },
            Commands::Exponential { lambda } => Sampler::Exponential { lambda },
            Commands::LogNormal { mean, std_dev } => Sampler::LogNormal { mean, std_dev },
            Commands::Pareto { scale, shape } => Sampler::Pareto { scale, shape },
            Commands::Uniform { min, max } => Sampler::Uniform { min, max },
            Commands::Triangular { min, max, mode } => Sampler::Triangular { min, max, mode },
            Commands::Gamma { shape, scale } => Sampler::Gamma { shape, scale },
            Commands::Align { interval } => Sampler::Align(interval),
            Commands::At { time } => Sampler::At(time),
        }
    }
}

impl Cli {
    /// The wait these arguments describe. Call [`Wait::validate`] before use.
    pub fn into_wait(self) -> Wait {
        self.into_parts().0
    }

    /// The wait these arguments describe, and how to run it.
    pub fn into_parts(self) -> (Wait, RunOptions) {
        let wait = Wait {
            sampler: self.command.into_sampler(),
            jitter: self.jitter,
            probability: self.probability,
            min: self.min,
            max: self.max,
            verbose: self.verbose,
        };
        let options = RunOptions {
            seed: self.seed,
            dry_run: self.dry_run,
            skip_exit_code: self.skip_exit_code,
        };
        (wait, options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_time_of_day() {
        assert_eq!(
            parse_time_of_day("22:30"),
            Ok(NaiveTime::from_hms_opt(22, 30, 0).unwrap())
        );
        assert_eq!(
            parse_time_of_day("07:05:09"),
            Ok(NaiveTime::from_hms_opt(7, 5, 9).unwrap())
        );
        assert!(parse_time_of_day("invalid-time").is_err());
        assert!(parse_time_of_day("25:00").is_err());
        assert!(parse_time_of_day("10:65").is_err());
    }

    #[test]
    fn test_parse_time_and_unit_hint() {
        assert_eq!(parse_time("0s"), Ok(Duration::ZERO));
        assert_eq!(parse_time("1m30s"), Ok(Duration::from_secs(90)));
        assert_eq!(parse_time("1ms"), Ok(Duration::from_millis(1)));
        // Whole numbers get the plain hint.
        assert_eq!(
            parse_time("2").unwrap_err(),
            "a unit is required, for example 2s or 2ms"
        );
        assert_eq!(
            parse_time("1e3").unwrap_err(),
            "a unit is required, for example 1000s or 1000ms"
        );
        assert!(parse_time("0").unwrap_err().contains("0s or 0ms"));
        // Decimals are steered to a whole number of a smaller unit, since the
        // parser rejects "0.1s".
        assert_eq!(
            parse_time("0.1").unwrap_err(),
            "a unit is required; if you meant 0.1 seconds, write 100ms"
        );
        assert!(parse_time("0.0005").unwrap_err().ends_with("write 500us"));
        assert!(parse_time("1.5").unwrap_err().ends_with("write 1500ms"));
        // Anything else gets the generic hint.
        let generic = "a whole number of units is required, for example 500ms or 2s";
        assert_eq!(parse_time("inf").unwrap_err(), generic);
        assert_eq!(parse_time("NaN").unwrap_err(), generic);
        assert_eq!(parse_time("-1").unwrap_err(), generic);
        assert_eq!(parse_time("1e-12").unwrap_err(), generic);
        // Every suggestion in a hint is accepted by the parser.
        for hint in ["2s", "2ms", "100ms", "500us", "1500ms", "500ms"] {
            assert!(parse_time(hint).is_ok(), "{hint}");
        }
    }

    #[test]
    fn test_single_argument_rules_are_enforced_by_validate() {
        // These used to be rejected by clap value parsers; the library is
        // now the single source of truth, and `run` turns a validation error
        // into a clap usage error with exit status 2.
        let invalid = |args: &[&str]| {
            parse(args)
                .unwrap()
                .into_wait()
                .validate()
                .unwrap_err()
                .to_string()
        };
        assert!(invalid(&["d", "0s", "-p", "1.01"]).contains("--probability"));
        assert!(invalid(&["d", "0s", "-p", "-0.1"]).contains("--probability"));
        assert!(invalid(&["d", "0s", "-p", "NaN"]).contains("--probability"));
        assert!(invalid(&["e", "0"]).contains("<LAMBDA>"));
        assert!(invalid(&["e", "inf"]).contains("<LAMBDA>"));
        assert!(invalid(&["ln", "0s", "1ms"]).contains("<MEAN>"));
        assert!(invalid(&["par", "0s", "1"]).contains("<SCALE>"));
        assert!(invalid(&["g", "1", "0s"]).contains("<SCALE>"));
        assert!(invalid(&["a", "0s"]).contains("<INTERVAL>"));
        assert!(invalid(&["a", "999us"]).contains("<INTERVAL>"));
        assert!(invalid(&["d", "1s", "-v", "1ns"]).contains("--verbose"));
        assert!(invalid(&["d", "1s", "-v", "0s"]).contains("--verbose"));
        for ok in [
            &["d", "0s", "-p", "0.0"][..],
            &["d", "0s", "-p", "1.0"],
            &["a", "1ms"],
            &["d", "1s", "-v", "1ms"],
        ] {
            assert!(parse(ok).unwrap().into_wait().validate().is_ok(), "{ok:?}");
        }
    }

    #[test]
    fn test_validation_runs_through_wait() {
        let invalid = |args: &[&str]| parse(args).unwrap().into_wait().validate().is_err();
        assert!(invalid(&["u", "2s", "1s"]));
        assert!(invalid(&["u", "1s", "1s"]));
        assert!(invalid(&["t", "1s", "2s", "3s"]));
        assert!(invalid(&["t", "1s", "1s", "1s"]));
        assert!(!invalid(&["u", "1s", "2s"]));
        assert!(!invalid(&["t", "0s", "1s", "0s"]));
        assert!(!invalid(&["t", "0s", "1s", "1s"]));
    }

    #[test]
    fn test_into_wait_carries_global_options() {
        let wait = parse(&["n", "1s", "100ms", "-j", "50ms", "-p", "0.5", "-v", "250ms"])
            .unwrap()
            .into_wait();
        assert_eq!(
            wait,
            Wait {
                sampler: Sampler::Normal {
                    mean: Duration::from_secs(1),
                    std_dev: Duration::from_millis(100),
                },
                jitter: Some(Duration::from_millis(50)),
                probability: Some(0.5),
                min: None,
                max: None,
                verbose: Verbose::Every(Duration::from_millis(250)),
            }
        );
    }

    #[test]
    fn test_run_options_and_bounds() {
        let (wait, options) = parse(&[
            "u",
            "1s",
            "5s",
            "--min",
            "2s",
            "--max",
            "4s",
            "--seed",
            "7",
            "--dry-run",
            "--skip-exit-code",
            "3",
        ])
        .unwrap()
        .into_parts();
        // --min/--max do not collide with the uniform <MIN>/<MAX>.
        assert_eq!(
            wait.sampler,
            Sampler::Uniform {
                min: Duration::from_secs(1),
                max: Duration::from_secs(5),
            }
        );
        assert_eq!(wait.min, Some(Duration::from_secs(2)));
        assert_eq!(wait.max, Some(Duration::from_secs(4)));
        assert_eq!(
            options,
            RunOptions {
                seed: Some(7),
                dry_run: true,
                skip_exit_code: 3,
            }
        );

        let (_, defaults) = parse(&["d", "1s"]).unwrap().into_parts();
        assert_eq!(
            defaults,
            RunOptions {
                seed: None,
                dry_run: false,
                skip_exit_code: 0,
            }
        );

        // Global options work before the subcommand too.
        let (wait, options) = parse(&["--seed", "1", "--max", "1s", "t", "0s", "2s", "1s"])
            .unwrap()
            .into_parts();
        assert_eq!(wait.max, Some(Duration::from_secs(1)));
        assert_eq!(options.seed, Some(1));

        assert!(parse(&["d", "1s", "--min", "2"]).is_err());
        assert!(parse(&["d", "1s", "--skip-exit-code", "256"]).is_err());
        assert!(parse(&["d", "1s", "--seed", "-1"]).is_err());
    }

    #[test]
    fn test_every_command_maps_to_its_sampler() {
        let cases: [(&[&str], Sampler); 10] = [
            (&["d", "1s"], Sampler::Fixed(Duration::from_secs(1))),
            (&["e", "2"], Sampler::Exponential { lambda: 2.0 }),
            (
                &["n", "1s", "1ms"],
                Sampler::Normal {
                    mean: Duration::from_secs(1),
                    std_dev: Duration::from_millis(1),
                },
            ),
            (
                &["ln", "1s", "1ms"],
                Sampler::LogNormal {
                    mean: Duration::from_secs(1),
                    std_dev: Duration::from_millis(1),
                },
            ),
            (
                &["par", "1s", "2"],
                Sampler::Pareto {
                    scale: Duration::from_secs(1),
                    shape: 2.0,
                },
            ),
            (
                &["u", "1s", "2s"],
                Sampler::Uniform {
                    min: Duration::from_secs(1),
                    max: Duration::from_secs(2),
                },
            ),
            (
                &["t", "1s", "3s", "2s"],
                Sampler::Triangular {
                    min: Duration::from_secs(1),
                    max: Duration::from_secs(3),
                    mode: Duration::from_secs(2),
                },
            ),
            (
                &["g", "2", "1s"],
                Sampler::Gamma {
                    shape: 2.0,
                    scale: Duration::from_secs(1),
                },
            ),
            (&["a", "1m"], Sampler::Align(Duration::from_secs(60))),
            (
                &["at", "22:30"],
                Sampler::At(NaiveTime::from_hms_opt(22, 30, 0).unwrap()),
            ),
        ];
        for (args, sampler) in cases {
            assert_eq!(
                parse(args).unwrap().into_wait().sampler,
                sampler,
                "{args:?}"
            );
        }
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
        // Too-short periods parse; `Wait::validate` rejects them.
        assert_eq!(
            parse(&["d", "1s", "-v", "1ns"]).unwrap().verbose,
            Verbose::Every(Duration::from_nanos(1))
        );
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
