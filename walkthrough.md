# Dozr: A Code Walkthrough

*2026-03-14T01:18:22Z by Showboat 0.6.1*
<!-- showboat-id: 0ef764a1-9bdb-457d-bc78-f6441d87dab1 -->

## 1. Project Overview

Dozr is a Rust CLI tool (v0.4.1) that extends the standard Unix `sleep` command with:

- **Statistical distributions** — sample a wait duration from Normal, Exponential, LogNormal, Pareto, Uniform, Triangular, or Gamma distributions
- **Time alignment** — wait until the next even interval boundary (e.g., the next whole minute)
- **Absolute time waits** — wait until a specific clock time (e.g., 22:30)
- **Jitter** — add a bounded random offset to any wait
- **Probabilistic execution** — only sleep with a configurable probability
- **Verbose progress** — real-time countdown to stderr, either at a fixed update period or adaptive

The source is organized into four files that correspond directly to the execution pipeline:

```
src/main.rs        →  Binary entry point (6 lines)
src/lib.rs         →  Library root: run(), verbose wait loops
src/cli.rs         →  CLI argument parsing, command dispatch
src/conditions.rs  →  WaitCondition trait + all implementations
```

Let's start with the project manifest to see dependencies.

```bash
cat Cargo.toml
```

```output
[package]
name = "dozr"
version = "0.4.1"
edition = "2024"
description = "A flexible `sleep`-like command-line utility for pausing execution with fun timing features."
license = "MIT"
repository = "https://github.com/ShaneIsley/dozr"
authors = ["Shane Isley <shane.isley@gmail.com>"]
homepage = "https://github.com/ShaneIsley/dozr"
documentation = "https://docs.rs/dozr"
readme = "README.md"
keywords = ["sleep", "cli", "wait", "timing", "utility"]
categories = ["command-line-utilities", "development-tools", "os"]
rust-version = "1.85.0" # Minimum Rust version required

[lib]
name = "dozr"
path = "src/lib.rs"

[[bin]]
name = "dozr"
path = "src/main.rs"

[[bin]]
name = "dist_sampler"
path = "src/bin/dist_sampler.rs"

[dependencies]
anyhow = "1.0.98"
clap = { version = "4.5.40", features = ["derive"] }
humantime = "2.2.0"
rand = "0.9.1"
rand_distr = "0.5"
chrono = "0.4"

[dev-dependencies]
assert_cmd = "2.0.17"
predicates = "3.1.0"
mockall = "0.12.1"
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "wait_conditions"
harness = false
```

Key dependencies: **clap** for argument parsing, **humantime** to parse human-readable durations like `5s` or `1m30s`, **rand** + **rand_distr** for statistical sampling, **chrono** for clock-time arithmetic, and **anyhow** for ergonomic error handling.

## 2. Binary Entry Point — `src/main.rs`

Dozr uses a **library-first** design. The binary (`src/main.rs`) is a six-line shim that imports the library crate and calls its `run()` function. All logic lives in the library, making it independently testable and importable by other Rust crates.

```bash
cat src/main.rs
```

```output
use anyhow::Result;
use dozr::run;

fn main() -> Result<()> {
    run()
}
```

## 3. Library Entry Point — `src/lib.rs`

The library root exposes two public modules (`cli` and `conditions`) and two entry functions.

**`run()`** is the public API: it calls Clap's `parse()` to read `argv` into a `Cli` struct, then hands it to `run_with_args()`.

**`run_with_args(args: Cli)`** is where execution actually starts:
1. It calls `args.command.into_wait_condition(args.jitter, args.verbose, args.probability)` — this converts the parsed subcommand into a `Box<dyn WaitCondition>`.
2. It calls `.wait()` on that trait object, which calculates the sleep duration and then sleeps.

```bash
sed -n '1,23p' src/lib.rs
```

```output
use anyhow::Result;
use clap::Parser;


pub mod cli;
pub mod conditions;

/// The main entry point for the dozr application.
///
/// This function parses command-line arguments, determines the appropriate
/// wait condition, and then executes the wait.
pub fn run() -> Result<()> {
    let args = cli::Cli::parse();
    run_with_args(args)
}

/// The main logic of the application, accepting a Cli object.
fn run_with_args(args: cli::Cli) -> Result<()> {
    let condition = args
        .command
        .into_wait_condition(args.jitter, args.verbose, args.probability);
    condition.wait()
}
```

## 4. CLI Parsing — `src/cli.rs`

### 4a. The `Cli` Struct — Global Options

`Cli` is the top-level Clap parser. It has one subcommand field plus three **global** options that apply to any subcommand:

- `--jitter <TIME>` — maximum random duration to add on top of the sampled wait
- `--verbose [UPDATE_PERIOD]` — enable stderr countdown; if omitted, uses a sentinel value of `1ns` to signal *adaptive* mode
- `--probability <FLOAT>` — roll a die on each invocation; only sleep if the roll is within the probability

The `1ns` sentinel for adaptive verbose is a clever trick: Clap's `default_missing_value` fills in `"1ns"` when `-v` is given with no argument, and later code checks `duration.as_nanos() == 1` to distinguish adaptive from a genuine 1-nanosecond update period.

```bash
sed -n '30,48p' src/cli.rs
```

```output
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
```

### 4b. The `Commands` Enum — All Subcommands

`Commands` is a Clap `Subcommand` enum with ten variants. Each variant maps directly to one `WaitCondition` implementation. Short aliases are provided for the most-used ones:

| Variant | Alias | Parameters |
|---|---|---|
| `Duration` | `d` | `time` (humantime duration) |
| `Normal` | `n` | `mean` (duration), `std_dev` (f64) |
| `Exponential` | `e` | `lambda` (f64, rate parameter) |
| `LogNormal` | `ln` | `mean` (duration), `std_dev` (f64) |
| `Pareto` | `par` | `scale` (f64), `shape` (f64) |
| `Uniform` | `u` | `min`, `max` (durations) |
| `Triangular` | `t` | `min`, `max`, `mode` (f64s, seconds) |
| `Gamma` | `g` | `shape`, `scale` (f64s) |
| `Align` | `a` / `ali` | `interval` (humantime duration) |
| `At` | — | `time` (HH:MM or HH:MM:SS) |

Duration arguments use `humantime::parse_duration` as their `value_parser`, so users write `5s`, `1m30s`, etc. instead of raw integers.

```bash
sed -n '50,133p' src/cli.rs
```

```output
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
```

### 4c. `parse_time_until()` — Converting a Clock Time to a Duration

The `At` subcommand's `time` argument doesn't store a clock time — it stores a **`Duration` representing how long to wait** until that time. The conversion happens at parse time inside `parse_time_until()`:

1. Parse the input string as `HH:MM` or `HH:MM:SS` using `chrono::NaiveTime`.
2. Build a `DateTime<Local>` for that time today.
3. If that datetime is already in the past (the time has passed today), add one day to schedule it for tomorrow.
4. Compute the signed duration between now and the target; convert from `chrono::Duration` to `std::time::Duration`.

By the time the `At` variant reaches `into_wait_condition()`, it is just a pre-baked duration — no clock arithmetic needed at sleep time.

```bash
sed -n '1,28p' src/cli.rs
```

```output
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
```

### 4d. `into_wait_condition()` — Command Dispatch

`Commands::into_wait_condition()` is the bridge between the parsed CLI and the execution engine. It pattern-matches on each variant and constructs the matching struct from `conditions.rs`, then boxes it as `Box<dyn WaitCondition>`.

One special case: if `Commands::Duration` is combined with `--probability`, it creates a `ProbabilisticWait` instead of a `DurationWait`. Probability is not supported for distribution-based commands — it only makes sense to skip an exact-duration wait; skipping a sampled wait would be confusing.

```bash
sed -n '135,210p' src/cli.rs
```

```output
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
```

## 5. The Core Abstractions — `src/conditions.rs`

### 5a. `WaitCondition` Trait and `JitterGenerator`

The execution engine is built around two traits:

**`WaitCondition`** — implemented by every condition type:
- `calculate_wait_duration(&self) -> Result<Duration>` — compute (and sample, if stochastic) the duration to sleep, without actually sleeping
- `wait(&self) -> Result<()>` — call `calculate_wait_duration`, then sleep (with optional verbose output)

Separating calculation from execution enables unit testing of the math without real sleeps, and allows callers to inspect the chosen duration before committing.

**`JitterGenerator`** trait + `RandomJitterGenerator` — a small strategy abstraction for jitter:
- The trait defines a single `generate(max_jitter: Duration) -> Duration` method.
- The production implementation (`RandomJitterGenerator<T: Rng>`) wraps any `rand::Rng` and returns a uniformly random duration in `[0, max_jitter]`.
- Tests can inject a `MockJitterGenerator` that returns a fixed, predictable jitter, making `DurationWait` tests deterministic.

```bash
sed -n '40,70p' src/conditions.rs
```

```output
// 1. Define a dedicated trait for jitter generation.
// This makes the dependency explicit and easy to mock.
pub trait JitterGenerator {
    fn generate(&mut self, max_jitter: Duration) -> Duration;
}

// 2. Implement the trait for the real random number generator.
pub struct RandomJitterGenerator<T: Rng> {
    rng: T,
}

impl<T: Rng> RandomJitterGenerator<T> {
    pub fn new(rng: T) -> Self {
        Self { rng }
    }
}

impl<T: Rng> JitterGenerator for RandomJitterGenerator<T> {
    fn generate(&mut self, max_jitter: Duration) -> Duration {
        if max_jitter.is_zero() {
            return Duration::ZERO;
        }
        let jitter_nanos = self.rng.random_range(0..=max_jitter.as_nanos() as u64);
        Duration::from_nanos(jitter_nanos)
    }
}

pub trait WaitCondition {
    fn calculate_wait_duration(&self) -> Result<Duration>;
    fn wait(&self) -> Result<()>;
}
```

### 5b. `perform_wait()` — The Unified Sleep Dispatcher

Every `wait()` implementation ends with a call to `perform_wait(sleep_duration, verbose)`. This private helper is the single point where actual sleeping happens. It branches on the `verbose` option:

- `None` → `std::thread::sleep(sleep_duration)` — silent, simple
- `Some(1ns)` → `adaptive_verbose_wait()` (the 1ns sentinel signals adaptive mode)
- `Some(period)` → `verbose_wait()` with that fixed update period

The `display_fn` closure passed into the verbose loops formats the stderr output: `[HH:MM:SS] [DOZR] Time remaining: Xs` while waiting, and `[HH:MM:SS] Wait complete.` when done.

```bash
sed -n '12,38p' src/conditions.rs
```

```output
///
/// - If `verbose` is `None`, performs a simple sleep with no output.
/// - If `verbose` is `Some(duration)` where duration is 1ns, uses adaptive verbose output.
/// - Otherwise, uses fixed-interval verbose output with the specified update period.
fn perform_wait(sleep_duration: Duration, verbose: Option<Duration>) {
    match verbose {
        None => {
            std::thread::sleep(sleep_duration);
        }
        Some(update_period) => {
            let is_adaptive = update_period.as_nanos() == 1;
            let display_fn = |remaining: Duration| {
                let now: DateTime<Local> = Local::now();
                if remaining.is_zero() {
                    eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                } else {
                    eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                }
            };
            if is_adaptive {
                adaptive_verbose_wait(sleep_duration, display_fn);
            } else {
                verbose_wait(sleep_duration, update_period, display_fn);
            }
        }
    }
}
```

## 6. The Simplest Condition — `DurationWait`

`DurationWait` is the baseline: wait for a fixed duration, optionally adding jitter.

Its `calculate_sleep_duration()` method takes a `&mut dyn JitterGenerator` — rather than reaching for a global RNG directly — so that tests can inject a `MockJitterGenerator` and get deterministic results. Production code passes in a `RandomJitterGenerator` wrapping `ThreadRng`.

The `WaitCondition` impl for `DurationWait` is the canonical pattern followed by all conditions: call `calculate_*`, then `perform_wait`.

```bash
sed -n '72,85p' src/conditions.rs
```

```output
pub struct DurationWait {
    pub duration: Duration,
    pub verbose: Option<Duration>,
    pub jitter: Option<Duration>,
}

impl DurationWait {
    // 3. The core logic now takes the trait object as an argument.
    fn calculate_sleep_duration(&self, jitter_gen: &mut dyn JitterGenerator) -> Duration {
        let max_jitter = self.jitter.unwrap_or(Duration::ZERO);
        let random_jitter = jitter_gen.generate(max_jitter);
        self.duration + random_jitter
    }
}
```

```bash
sed -n '346,358p' src/conditions.rs
```

```output
impl WaitCondition for DurationWait {
    fn calculate_wait_duration(&self) -> Result<Duration> {
        let mut rng = ThreadRng::default();
        let mut jitter_gen = RandomJitterGenerator::new(&mut rng);
        Ok(self.calculate_sleep_duration(&mut jitter_gen))
    }

    fn wait(&self) -> Result<()> {
        let sleep_duration = self.calculate_wait_duration()?;
        perform_wait(sleep_duration, self.verbose);
        Ok(())
    }
}
```

## 7. Distribution-Based Conditions

Seven conditions sample a wait duration from a statistical distribution. They all follow the same four-step pattern:

1. **Create distribution** — pass parameters to the `rand_distr` constructor (e.g., `Normal::new(mean_secs, std_dev)`)
2. **Sample** — call `distribution.sample(&mut rng)`, obtaining an f64 in seconds
3. **Clamp** — `.max(0.0)` ensures negative samples (possible in Normal, LogNormal, Triangular) don't produce negative durations
4. **Add jitter** — call `RandomJitterGenerator::generate()` and add the result

`NormalWait` is shown as the representative example. All seven follow this identical shape — only the distribution type and parameters differ.

```bash
sed -n '87,110p' src/conditions.rs
```

```output
pub struct NormalWait {
    pub mean: Duration,
    pub std_dev: f64,
    pub verbose: Option<Duration>,
    pub jitter: Option<Duration>,
}

impl WaitCondition for NormalWait {
    fn calculate_wait_duration(&self) -> Result<Duration> {
        let mean_secs = self.mean.as_secs_f64();
        let normal = Normal::new(mean_secs, self.std_dev)?;
        let mut rng = ThreadRng::default();
        let duration_secs = normal.sample(&mut rng).max(0.0);
        let mut jitter_gen = RandomJitterGenerator::new(&mut rng);
        let random_jitter = jitter_gen.generate(self.jitter.unwrap_or(Duration::ZERO));
        Ok(Duration::from_secs_f64(duration_secs) + random_jitter)
    }

    fn wait(&self) -> Result<()> {
        let sleep_duration = self.calculate_wait_duration()?;
        perform_wait(sleep_duration, self.verbose);
        Ok(())
    }
}
```

The remaining six distribution structs are listed below. Each uses the same four-step pattern; what varies is the distribution type, its parameters, and what those parameters mean statistically:

- **`ExponentialWait`** — `lambda` (rate, λ); mean wait = 1/λ seconds. Models memoryless inter-arrival times.
- **`LogNormalWait`** — `mean` + `std_dev`; the underlying normal is in log-space, so the sample is always positive and right-skewed.
- **`ParetoWait`** — `scale` (minimum value) + `shape` (α); heavy-tailed — rare very long waits are possible.
- **`UniformWait`** — `min`/`max` as durations; equal probability across the range.
- **`TriangularWait`** — `min`, `max`, `mode` as raw f64 seconds; the most likely value is at `mode`.
- **`GammaWait`** — `shape` (k) + `scale` (θ); mean = k·θ, generalizes Exponential (shape=1).

```bash
grep -n 'pub struct.*Wait' src/conditions.rs
```

```output
72:pub struct DurationWait {
87:pub struct NormalWait {
112:pub struct ExponentialWait {
135:pub struct LogNormalWait {
160:pub struct ParetoWait {
186:pub struct UniformWait {
212:pub struct TriangularWait {
237:pub struct GammaWait {
261:pub struct TimeAlignWait {
294:pub struct ProbabilisticWait {
300:pub struct UntilTimeWait {
```

## 8. Time-Based Conditions

### 8a. `TimeAlignWait` — Aligning to an Interval Boundary

Rather than sleeping for a fixed duration, `TimeAlignWait` sleeps until the next multiple of a given interval. For example, `dozr align 1m` waits until the next whole minute on the system clock.

The math:
- Get current UNIX time in nanoseconds (`now_nanos`)
- Compute `remainder = now_nanos % interval_nanos`
- If `remainder == 0`, we are exactly on a boundary — sleep a full interval (to advance to the *next* boundary)
- Otherwise, sleep `interval - remainder` nanoseconds

This is nanosecond-precise because UNIX epoch arithmetic is exact. No chrono needed — just `SystemTime::now()`.

```bash
sed -n '261,292p' src/conditions.rs
```

```output
pub struct TimeAlignWait {
    pub align_interval: Duration,
    pub verbose: Option<Duration>,
}

impl WaitCondition for TimeAlignWait {
    fn calculate_wait_duration(&self) -> Result<Duration> {
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        let align_interval_nanos = self.align_interval.as_nanos();

        if align_interval_nanos == 0 {
            return Ok(Duration::ZERO);
        }

        let now_nanos = now.as_nanos();
        let remainder = now_nanos % align_interval_nanos;

        let sleep_duration = if remainder == 0 {
            self.align_interval
        } else {
            Duration::from_nanos((align_interval_nanos - remainder) as u64)
        };

        Ok(sleep_duration)
    }

    fn wait(&self) -> Result<()> {
        let sleep_duration = self.calculate_wait_duration()?;
        perform_wait(sleep_duration, self.verbose);
        Ok(())
    }
}
```

### 8b. `UntilTimeWait` — Waiting Until a Clock Time

`UntilTimeWait` is intentionally trivial: its `sleep_duration` field was already computed by `parse_time_until()` at argument-parse time. The `wait()` implementation simply calls `perform_wait` with the stored duration. All the interesting logic (next-day rollover, chrono parsing) already happened in the CLI layer.

```bash
sed -n '300,315p' src/conditions.rs
```

```output
pub struct UntilTimeWait {
    pub sleep_duration: Duration,
    pub verbose: Option<Duration>,
}

impl WaitCondition for UntilTimeWait {
    fn calculate_wait_duration(&self) -> Result<Duration> {
        Ok(self.sleep_duration)
    }

    fn wait(&self) -> Result<()> {
        let sleep_duration = self.calculate_wait_duration()?;
        perform_wait(sleep_duration, self.verbose);
        Ok(())
    }
}
```

## 9. Probabilistic Execution — `ProbabilisticWait`

`ProbabilisticWait` wraps a fixed duration with a coin-flip gate. On each call to `wait()`:

1. Sample a float uniformly in `[0.0, 1.0)` using `ThreadRng`.
2. If `roll <= probability`, call `perform_wait` to sleep for the configured duration.
3. If the roll fails and verbose mode is on, print a skip message to stderr so the caller knows the sleep was intentionally skipped.

A `probability` of `1.0` always sleeps; `0.0` always skips (the `<=` comparison ensures the `0.0` case correctly skips, since no f64 roll in `[0.0, 1.0)` is `<= 0.0` — actually the test confirms it: `probability: 0.0` never sleeps because `roll > 0.0` always).

```bash
sed -n '294,344p' src/conditions.rs
```

```output
pub struct ProbabilisticWait {
    pub duration: Duration,
    pub probability: f64,
    pub verbose: Option<Duration>,
}

pub struct UntilTimeWait {
    pub sleep_duration: Duration,
    pub verbose: Option<Duration>,
}

impl WaitCondition for UntilTimeWait {
    fn calculate_wait_duration(&self) -> Result<Duration> {
        Ok(self.sleep_duration)
    }

    fn wait(&self) -> Result<()> {
        let sleep_duration = self.calculate_wait_duration()?;
        perform_wait(sleep_duration, self.verbose);
        Ok(())
    }
}

impl WaitCondition for ProbabilisticWait {
    fn calculate_wait_duration(&self) -> Result<Duration> {
        let mut rng = ThreadRng::default();
        let roll: f64 = rng.random_range(0.0..1.0);

        if roll <= self.probability {
            Ok(self.duration)
        } else {
            Ok(Duration::ZERO)
        }
    }

    fn wait(&self) -> Result<()> {
        let mut rng = ThreadRng::default();
        let roll: f64 = rng.random_range(0.0..1.0);
        let should_sleep = roll <= self.probability;

        if should_sleep {
            perform_wait(self.duration, self.verbose);
        } else if self.verbose.is_some() {
            eprintln!(
                "Probabilistic wait: Skipping sleep (probability: {}, roll: {:.2})",
                self.probability, roll
            );
        }
        Ok(())
    }
}
```

## 10. Verbose Progress Output — `src/lib.rs`

### 10a. `verbose_wait()` — Fixed-Interval Updates

`verbose_wait` drives the display loop for a known total wait duration with a fixed update period:

1. Record `start = Instant::now()`.
2. On each iteration, compute `elapsed` and `remaining = total_wait - elapsed`.
3. Round `remaining` to the nearest second and compare to the previously displayed value — **only call `display_fn` when the rounded ETA changes** to avoid spamming the same line.
4. Sleep for `update_period` before the next iteration.
5. If `remaining` reaches zero, call `display_fn(Duration::ZERO)` for the completion message and break.

Edge case: if `sleep_duration` computes to zero (e.g., update period is tiny), it yields the thread instead of busy-waiting.

```bash
sed -n '26,64p' src/lib.rs
```

```output
pub fn verbose_wait<F>(total_wait: std::time::Duration, update_period: std::time::Duration, mut display_fn: F)
where
    F: FnMut(std::time::Duration),
{
    let start = std::time::Instant::now();
    let mut last_displayed_eta: Option<u64> = None;

    loop {
        let elapsed = start.elapsed();
        let remaining = total_wait.saturating_sub(elapsed);
        let eta = remaining.as_secs_f64();
        let rounded_eta = eta.round() as u64;

        if remaining == std::time::Duration::ZERO {
            display_fn(std::time::Duration::ZERO);
            break;
        }

        // Only display if ETA has changed or it's the very first display
        if last_displayed_eta.map_or(true, |last_eta| last_eta != rounded_eta) {
            display_fn(std::time::Duration::from_secs(rounded_eta));
            last_displayed_eta = Some(rounded_eta);
        }

        let next_update_time = elapsed + update_period;
        let sleep_duration = next_update_time.saturating_sub(elapsed);

        if sleep_duration > std::time::Duration::ZERO {
            std::thread::sleep(sleep_duration);
        } else if remaining > std::time::Duration::ZERO {
            // If sleep_duration is zero or negative, but there's still time remaining,
            // yield to ensure other threads can run and prevent busy-waiting.
            std::thread::yield_now();
        } else {
            // If no time remaining, break the loop
            break;
        }
    }
}
```

### 10b. `adaptive_verbose_wait()` and `get_adaptive_update_period()`

`adaptive_verbose_wait` uses the same loop structure as `verbose_wait` but dynamically adjusts the sleep between updates based on how much time remains:

| Remaining time | Update period |
|---|---|
| 0 – 20 s | 1 s |
| 21 – 60 s | 5 s |
| 1 – 5 min | 10 s |
| 6 – 10 min | 15 s |
| > 10 min | 60 s |

This avoids flooding the terminal when there are hours left while still giving second-by-second updates in the final 20 seconds.

Two additional refinements prevent clock drift and busy-waiting:

- **Boundary alignment**: the loop computes `time_to_next_marker` (time until the current period's boundary) and `time_to_next_threshold` (time until the tier changes). It sleeps the *minimum* of all three values so it wakes up right when something interesting happens — a new display tick *or* a tier boundary.
- **1ms floor**: `sleep_duration.max(1ms)` ensures we always sleep at least a millisecond, preventing the loop from becoming a busy-spin.

```bash
sed -n '66,146p' src/lib.rs
```

```output
/// Performs the wait with adaptive verbose progress updates.
pub fn adaptive_verbose_wait<F>(total_wait: std::time::Duration, mut display_fn: F)
where
    F: FnMut(std::time::Duration),
{
    let start = std::time::Instant::now();
    let mut last_displayed_eta: Option<u64> = None;

    loop {
        let elapsed = start.elapsed();
        let remaining = total_wait.saturating_sub(elapsed);
        let eta = remaining.as_secs_f64();
        let rounded_eta = eta.round() as u64;

        if remaining == std::time::Duration::ZERO {
            display_fn(std::time::Duration::ZERO);
            break;
        }

        // Only display if ETA has changed or it's the very first display
        if last_displayed_eta.map_or(true, |last_eta| last_eta != rounded_eta) {
            display_fn(std::time::Duration::from_secs(rounded_eta));
            last_displayed_eta = Some(rounded_eta);
        }

        let current_update_period = get_adaptive_update_period(remaining);

        let remaining_secs = remaining.as_secs();

        let time_to_next_marker = if current_update_period.as_secs() == 0 {
            remaining
        } else {
            let target_marker_secs = (remaining_secs / current_update_period.as_secs()) * current_update_period.as_secs();
            remaining.saturating_sub(std::time::Duration::from_secs(target_marker_secs))
        };

        let time_to_next_threshold = if remaining_secs > 600 {
            remaining.saturating_sub(std::time::Duration::from_secs(600))
        } else if remaining_secs > 300 {
            remaining.saturating_sub(std::time::Duration::from_secs(300))
        } else if remaining_secs > 60 {
            remaining.saturating_sub(std::time::Duration::from_secs(60))
        } else if remaining_secs > 20 {
            remaining.saturating_sub(std::time::Duration::from_secs(20))
        } else {
            remaining
        };

        let sleep_duration = std::cmp::min(current_update_period, std::cmp::min(time_to_next_threshold, time_to_next_marker));
        let sleep_duration = sleep_duration.max(std::time::Duration::from_millis(1)); // Ensure at least 1ms sleep to avoid busy-waiting

        if sleep_duration > std::time::Duration::ZERO {
            std::thread::sleep(sleep_duration);
        } else if remaining > std::time::Duration::ZERO {
            // If sleep_duration is zero or negative, but there's still time remaining,
            // yield to ensure other threads can run and prevent busy-waiting.
            std::thread::yield_now();
        } else {
            // If no time remaining, break the loop
            break;
        }
    }
}

fn get_adaptive_update_period(remaining: std::time::Duration) -> std::time::Duration {
    let remaining_secs = remaining.as_secs();

    if remaining_secs <= 20 {
        std::time::Duration::from_secs(1) // 0-20s: 1s
    } else if remaining_secs <= 60 {
        std::time::Duration::from_secs(5) // 21-60s: 5s
    } else if remaining_secs <= 300 {
        // 5 minutes
        std::time::Duration::from_secs(10) // 1-5m: 10s
    } else if remaining_secs <= 600 {
        // 10 minutes
        std::time::Duration::from_secs(15) // 6-10m: 15s
    } else {
        std::time::Duration::from_secs(60) // 10m+: 1m
    }
}
```

## 11. Putting It All Together

Here is a complete end-to-end trace for the command `dozr d 10s -j 1s -v`:

```
dozr d 10s -j 1s -v
```

**Step 1 — `main()` in `src/main.rs`**  
Calls `dozr::run()`.

**Step 2 — `run()` in `src/lib.rs`**  
Clap parses argv:
- `d` → `Commands::Duration { time: 10s }`
- `-j 1s` → `jitter: Some(1s)`
- `-v` → `verbose: Some(1ns)` (adaptive sentinel)

Calls `run_with_args(args)`.

**Step 3 — `run_with_args()` in `src/lib.rs`**  
Calls `Commands::Duration { time: 10s }.into_wait_condition(Some(1s), Some(1ns), None)`.

**Step 4 — `into_wait_condition()` in `src/cli.rs`**  
No `probability` → creates `DurationWait { duration: 10s, jitter: Some(1s), verbose: Some(1ns) }`.  
Returns it boxed as `Box<dyn WaitCondition>`.

**Step 5 — `DurationWait::wait()` in `src/conditions.rs`**  
Calls `calculate_wait_duration()`:
- Creates `ThreadRng` and `RandomJitterGenerator`.
- `calculate_sleep_duration()`: base = 10s + random jitter in [0, 1s] = **10.0s – 11.0s** total.

Calls `perform_wait(total, Some(1ns))`.

**Step 6 — `perform_wait()` in `src/conditions.rs`**  
`update_period.as_nanos() == 1` → adaptive mode.  
Calls `adaptive_verbose_wait(total, display_fn)` from `lib.rs`.

**Step 7 — `adaptive_verbose_wait()` in `src/lib.rs`**  
Since total ≤ 20s, update period = 1s.  
Loops, printing to stderr each second:

```
[HH:MM:SS] [DOZR] Time remaining: 11s
[HH:MM:SS] [DOZR] Time remaining: 10s
...
[HH:MM:SS] [DOZR] Time remaining: 1s
[HH:MM:SS] Wait complete.
```

Then returns. `wait()` returns `Ok(())`. `run()` returns `Ok(())`. `main()` exits 0.

```bash
./target/release/dozr --help
```

```output
A flexible `sleep`-like command-line utility for pausing execution with fun timing features.

Usage: dozr [OPTIONS] <COMMAND>

Commands:
  duration     Wait for a fixed duration
  normal       Wait using a normal distribution
  exponential  Wait using an exponential distribution
  log-normal   Wait using a log-normal distribution
  pareto       Wait using a Pareto distribution
  uniform      Wait using a uniform distribution
  triangular   Wait using a triangular distribution
  gamma        Wait using a gamma distribution
  align        Align the wait to the next even interval
  at           Wait until a specific time of day
  help         Print this message or the help of the given subcommand(s)

Options:
  -j, --jitter <JITTER>            Add a random duration of jitter (e.g., "100ms")
  -v, --verbose [<UPDATE_PERIOD>]  Enable verbose output, with an optional update period (e.g., "250ms"). If no update period is specified, adaptive verbose output is used
  -p, --probability <PROBABILITY>  Wait only with a certain probability (0.0 to 1.0)
  -h, --help                       Print help
  -V, --version                    Print version
```
