# dozr: Probability Distributions as CLI UX

*2026-03-29T22:23:59Z by Showboat 0.6.1*
<!-- showboat-id: 75433f71-6e8d-4d26-b0ff-855ec2f14861 -->

## Why Randomised Sleep Matters

The Unix `sleep` command is a blunt instrument: `sleep 5` always waits exactly five seconds.
That determinism becomes a liability in three common scenarios:

**Load-test realism.** Real users don't click at metronomic intervals. A test harness that fires requests at perfectly spaced intervals produces optimistic latency numbers; one that draws inter-arrival times from an exponential distribution approximates a Poisson process — the standard model for independent arrivals.

**Thundering-herd avoidance.** When a fleet of cron jobs all `sleep 60` before retrying, they retry in lockstep. Drawing the retry delay from a normal or uniform distribution spreads the load across the window.

**Human-like scheduling.** Automation that should look organic — notification timing, CI polling intervals, chaos-engineering fault injection — benefits from statistical variation rather than fixed cadence.

`dozr` replaces `sleep` with a single binary that accepts seven probability distributions (Normal, Exponential, Log-Normal, Pareto, Uniform, Triangular, Gamma), time-of-day alignment, jitter, and probabilistic execution. It is published on crates.io and written in roughly 800 lines of Rust.

## Architecture at a Glance

The codebase has four source files, each with a clear responsibility:

| File | Role |
|------|------|
| `main.rs` | Six-line entry point; delegates immediately to the library |
| `lib.rs` | Parses CLI args, dispatches to the chosen wait condition, owns verbose-output logic |
| `cli.rs` | Clap-derived argument definitions; converts parsed commands into trait objects |
| `conditions.rs` | Every wait strategy: the `WaitCondition` trait, seven distributions, alignment, jitter, probabilistic execution |

The interesting design thread is how a shell invocation like `dozr normal 2s 0.3 --jitter 100ms` flows through these layers into a single sampled `thread::sleep` call. Let's trace it.

## From CLI Args to a Distribution Sample

### Step 1: Declarative Argument Parsing

The CLI layer uses clap's derive API. Each distribution is a subcommand with typed, validated parameters:

```bash
sed -n '30,48p' /home/user/dozr/src/cli.rs
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

Three global options — `--jitter`, `--verbose`, and `--probability` — compose with any subcommand. The `humantime` crate handles duration parsing, so users write natural strings like `2s`, `500ms`, or `1m30s` instead of raw numbers.

Each distribution maps to a subcommand with domain-specific parameters. Here is the Normal subcommand alongside its neighbours:

```bash
sed -n '50,73p' /home/user/dozr/src/cli.rs
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
```

A design choice worth noting: the mean for Normal is a `Duration` (parsed by humantime), but `std_dev` is a raw `f64` in seconds. This means the user writes `dozr normal 2s 0.3` — the mean in human-readable form, the spread as a dimensionless number. It's a pragmatic UX tradeoff: standard deviation is typically thought of as a multiplier or fraction of the mean, not as an absolute time span.

### Step 2: Command-to-Trait-Object Conversion

The `into_wait_condition()` method on `Commands` is the factory that wires parsed arguments into concrete distribution structs, returning a boxed `WaitCondition` trait object:

```bash
sed -n '135,163p' /home/user/dozr/src/cli.rs
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
```

The `Duration` subcommand has a special case: if `--probability` is set, it returns a `ProbabilisticWait` instead of a `DurationWait`. Every other distribution simply threads the global options (`jitter`, `verbose`) into its struct. The caller — `run_with_args()` in `lib.rs` — sees only a `Box<dyn WaitCondition>` and calls `.wait()`:

```bash
sed -n '17,23p' /home/user/dozr/src/lib.rs
```

```output
/// The main logic of the application, accepting a Cli object.
fn run_with_args(args: cli::Cli) -> Result<()> {
    let condition = args
        .command
        .into_wait_condition(args.jitter, args.verbose, args.probability);
    condition.wait()
}
```

Three lines of application logic. Parse, convert, wait. The complexity lives in the implementations behind the trait.

## Deep Dive: The Normal Distribution Implementation

The `WaitCondition` trait is minimal — two methods, no default implementations:

```bash
sed -n '67,70p' /home/user/dozr/src/conditions.rs
```

```output
pub trait WaitCondition {
    fn calculate_wait_duration(&self) -> Result<Duration>;
    fn wait(&self) -> Result<()>;
}
```

Separating `calculate_wait_duration()` from `wait()` is a deliberate testing seam: tests can assert on the computed duration without actually sleeping. Now the Normal implementation:

```bash
sed -n '87,110p' /home/user/dozr/src/conditions.rs
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

Let's walk through `calculate_wait_duration()` line by line:

1. **`self.mean.as_secs_f64()`** — Converts the user's `Duration` (e.g., `2s` → 2.0) into the `f64` that `rand_distr::Normal` expects. This is the bridge between the CLI's human-readable input and the math layer.

2. **`Normal::new(mean_secs, self.std_dev)?`** — Constructs the distribution. The `?` propagates errors for invalid parameters (e.g., negative standard deviation). The `rand_distr` crate implements the Box-Muller transform under the hood: it converts two uniform random samples into a normally-distributed value via `√(-2 ln U₁) · cos(2π U₂)`.

3. **`.sample(&mut rng).max(0.0)`** — Draws one sample and clamps to zero. This is important: a Normal distribution is symmetric, so `dozr normal 1s 0.8` can produce negative samples. A negative sleep duration would panic, so `.max(0.0)` turns those into instant returns rather than errors. This is a deliberate design choice — truncating rather than folding the distribution preserves simplicity at the cost of slightly shifting the effective mean upward when `std_dev` is large relative to the mean.

4. **Jitter composition** — After sampling, any user-specified jitter (a uniform `[0, max_jitter]` addition) is layered on top. This keeps the distribution's shape intact while adding a secondary source of randomness.

5. **`Duration::from_secs_f64(duration_secs) + random_jitter`** — Converts back from the math domain (`f64` seconds) to Rust's `Duration` type for the actual sleep.

Every other distribution follows the same pattern — the only thing that changes is the `rand_distr` type and its parameters. This consistency means adding a new distribution is a ~25-line copy-and-adapt operation.

## Composable Features: Jitter, Alignment, and Probabilistic Execution

The three cross-cutting features — jitter, time alignment, and probabilistic execution — each solve a distinct operational problem and compose independently.

### Jitter: Trait-Based Injection

Jitter is modelled with a dedicated trait so it can be mocked in tests:

```bash
sed -n '42,65p' /home/user/dozr/src/conditions.rs
```

```output
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
```

The `RandomJitterGenerator` is generic over any `Rng`, and the trait allows tests to substitute a deterministic mock. The implementation draws uniformly from `[0, max_jitter]` at nanosecond precision. The zero-check early return avoids an unnecessary RNG call — a small detail, but it shows attention to the common case where jitter isn't used.

### Time Alignment: Snapping to Clock Boundaries

The `align` command waits until the next even multiple of a given interval — useful for synchronising cron-like tasks to wall-clock boundaries:

```bash
sed -n '261,292p' /home/user/dozr/src/conditions.rs
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

The modular arithmetic here is straightforward: compute the remainder of the current epoch time modulo the interval, then sleep for the difference. If you're exactly on a boundary (`remainder == 0`), it waits a full interval rather than returning instantly — the right default for a polling loop that runs `dozr align 30s` repeatedly.

### Probabilistic Execution: Maybe Don't Sleep at All

The `-p` flag adds a coin flip: the wait only happens with the specified probability.

```bash
sed -n '294,298p' /home/user/dozr/src/conditions.rs && echo '---' && sed -n '317,343p' /home/user/dozr/src/conditions.rs
```

```output
pub struct ProbabilisticWait {
    pub duration: Duration,
    pub probability: f64,
    pub verbose: Option<Duration>,
}
---
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

This is useful for chaos engineering (`dozr d 0s -p 0.1` in front of a fault-injection command means it fires ~10% of the time) or load-shedding simulations. The verbose mode reports when a sleep is skipped, making it observable in pipeline logs.

### The Verbose Output Pipeline

One more composable piece worth examining: the `perform_wait` dispatcher that sits between every `WaitCondition` and the actual `thread::sleep`:

```bash
sed -n '16,38p' /home/user/dozr/src/conditions.rs
```

```output
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

All output goes to stderr (preserving stdout for pipelines). The 1-nanosecond sentinel value for adaptive mode is a pragmatic encoding trick: clap's `default_missing_value = "1ns"` means `-v` with no argument triggers adaptive updates, while `-v 500ms` sets a fixed period. The adaptive algorithm scales update frequency with remaining time — every second for short waits, every minute for long ones.

## Testing and Benchmarking Strategy

### Unit Tests: Mock-Injected Determinism

The `JitterGenerator` trait pays off at test time:

```bash
sed -n '366,403p' /home/user/dozr/src/conditions.rs
```

```output
    struct MockJitterGenerator {
        jitter: Duration,
    }

    impl JitterGenerator for MockJitterGenerator {
        fn generate(&mut self, _max_jitter: Duration) -> Duration {
            // Return the exact, predictable jitter for the test.
            self.jitter
        }
    }

    #[test]
    fn test_duration_wait_creation() {
        let duration = Duration::from_secs(1);
        let wait_condition = DurationWait {
            duration,
            jitter: None,
            verbose: None,
        };
        assert_eq!(wait_condition.duration, duration);
    }

    #[test]
    fn test_calculate_sleep_duration_with_jitter() {
        let mut mock_gen = MockJitterGenerator {
            jitter: Duration::from_millis(1),
        };
        let wait_condition = DurationWait {
            duration: Duration::from_secs(1),
            jitter: Some(Duration::from_millis(500)),
            verbose: None,
        };

        let calculated_duration = wait_condition.calculate_sleep_duration(&mut mock_gen);

        // Assert that the base duration is correctly added to the mock jitter.
        assert_eq!(calculated_duration, Duration::from_millis(1001));
    }
```

The mock returns a deterministic jitter, making the test assert an exact value (`1000ms + 1ms = 1001ms`). This is the payoff of the trait-based jitter abstraction — production code gets cryptographic-quality randomness, tests get repeatability.

For the stochastic distribution tests, the approach is different — they can't assert exact values, so they verify the contract (non-negative duration, within expected bounds):

```bash
sed -n '500,511p' /home/user/dozr/src/conditions.rs
```

```output
    fn test_normal_wait_calculate_duration() {
        let wait = NormalWait {
            mean: Duration::from_secs(1),
            std_dev: 0.1,
            verbose: None,
            jitter: None,
        };
        // We can't predict the exact value, but we can check if it's reasonable
        // and doesn't panic. A simple check is that it's not negative.
        let duration = wait.calculate_wait_duration().unwrap();
        assert!(duration >= Duration::ZERO);
    }
```

### Integration Tests: End-to-End CLI Validation

The integration test suite in `tests/cli.rs` uses `assert_cmd` to run the actual binary and validate behaviour including argument parsing, timing bounds, error messages, and stderr output. A representative example tests that a normal distribution wait falls within a reasonable time window:

```bash
sed -n '391,401p' /home/user/dozr/tests/cli.rs
```

```output
fn test_normal_distribution_wait_time() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    let start = Instant::now();
    cmd.args(&["n", "1s", "0.1"])
        .assert()
        .success();
    let elapsed = start.elapsed();
    // Assert that the elapsed time is greater than 0 and within a reasonable range (e.g., 0.5s to 2s)
    assert!(elapsed > Duration::from_millis(0));
    assert!(elapsed < Duration::from_secs(2));
}
```

### Criterion Benchmarks: Measuring Computational Overhead

The benchmark suite isolates the computational cost of distribution sampling from the actual sleep time:

```bash
sed -n '40,51p' /home/user/dozr/benches/wait_conditions.rs
```

```output
fn bench_normal_distribution(c: &mut Criterion) {
    let wait = NormalWait {
        mean: Duration::from_secs(1),
        std_dev: 0.1,
        verbose: None,
        jitter: None,
    };

    c.bench_function("normal_distribution_calculate", |b| {
        b.iter(|| black_box(wait.calculate_wait_duration()))
    });
}
```

Every distribution and jitter combination gets its own Criterion benchmark. By calling only `calculate_wait_duration()` (not `wait()`), the benchmarks measure sampling overhead in isolation — typically sub-microsecond — confirming that dozr adds negligible latency on top of the requested sleep. The `black_box()` call prevents the compiler from optimising away the computation.

## What This Demonstrates

This project is a small, self-contained example of several production-Rust practices:

- **Trait-based abstraction with concrete simplicity**: `WaitCondition` is two methods. It doesn't try to be a framework. The trait exists to enable the factory pattern in `into_wait_condition()` and the testing seam in `calculate_wait_duration()`.

- **Dependency injection for testability**: The `JitterGenerator` trait is introduced solely so tests can mock randomness. The production code pays no runtime cost for this abstraction.

- **Error propagation without ceremony**: `anyhow::Result` and the `?` operator thread errors from distribution construction (invalid parameters) through to the caller without boilerplate.

- **Deliberate boundary handling**: Clamping negative Normal samples to zero, guarding against zero-length alignment intervals, rolling past times to tomorrow — each edge case is handled at the point where it occurs.

- **Three-tier test strategy**: Unit tests with mocks for deterministic logic, property-style checks for stochastic behaviour, integration tests for end-to-end CLI correctness, and Criterion benchmarks for performance regression detection.

The central design insight is that probability distributions are a natural CLI UX primitive for timing — not just a library concern. By making each distribution a first-class subcommand with human-readable parameters, dozr turns a statistics concept into a practical shell tool.
