# dozr — Probability Distributions as a CLI UX Decision

**dozr** is a Rust CLI tool that replaces the standard `sleep` command. Where `sleep 5` always waits exactly 5 seconds, dozr lets you say *how long you expect to wait* using a statistical distribution, then samples from that distribution each invocation. The result is a wait that is approximately right but naturally variable — which turns out to be exactly what several real workloads need.

## Why Randomised Sleep?

Consider three scenarios where fixed sleep is the wrong answer:

**Load testing with coordinated omission.** When N workers all `sleep 1s` between requests, they fire in lockstep. Every second, all N workers wake simultaneously and hammer the target. You are not testing a server under load; you are testing it under periodic shock. A `dozr normal 1s 0.15` spreads arrivals across a ~0.7–1.3s window, producing something closer to organic traffic.

**Cron-adjacent scheduling.** If 50 machines run the same cron job at :00 and each immediately hits a shared API, the first few seconds become a thundering herd. `dozr uniform 0s 30s` as a preamble staggers them across the minute with zero additional infrastructure.

**Human-like automation.** Scrapers and UI test suites that sleep fixed intervals between actions are trivially fingerprinted. A `dozr normal 800ms 0.2` on each inter-action pause produces a timing signature that is statistically plausible as human behaviour.

In all three cases the *intent* is already there in the user's mental model — "wait about a second" — and a distribution captures that intent better than a constant.

---

## From Command Line to Wait Condition

dozr's CLI is built with [clap](https://docs.rs/clap) using its derive macro. Each distribution is a subcommand that accepts exactly the parameters its distribution needs:

```rust
// src/cli.rs:50-118
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Wait for a fixed duration
    #[command(alias = "d")]
    Duration {
        #[arg(value_parser = humantime::parse_duration)]
        time: Duration,
    },
    /// Wait using a normal distribution
    #[command(alias = "n")]
    Normal {
        #[arg(value_parser = humantime::parse_duration)]
        mean: Duration,
        /// Standard deviation of the Normal distribution (e.g., "0.1").
        std_dev: f64,
    },
    // ... Exponential, LogNormal, Pareto, Uniform, Triangular, Gamma, Align, At
}
```

Three flags are global across all subcommands:

```rust
// src/cli.rs:36-48
/// Add a random duration of jitter (e.g., "100ms").
#[arg(short, long, value_parser = humantime::parse_duration, global = true)]
pub jitter: Option<Duration>,

/// Enable verbose output, with an optional update period (e.g., "250ms").
#[arg(short, long, value_name = "UPDATE_PERIOD", value_parser = humantime::parse_duration,
      num_args = 0..=1, default_missing_value = "1ns", global = true)]
pub verbose: Option<Duration>,

/// Wait only with a certain probability (0.0 to 1.0).
#[arg(short, long, global = true)]
pub probability: Option<f64>,
```

The `humantime::parse_duration` value parser means durations are expressed in human-readable form (`1s`, `500ms`, `2h30m`) rather than raw numbers. This is significant for the normal distribution: the mean is a `Duration`, so you write `dozr n 1s 0.15` — the API makes the intent legible at the call site.

The dispatch from parsed enum to behaviour happens in `into_wait_condition`, which converts each `Commands` variant into a boxed `WaitCondition` trait object:

```rust
// src/cli.rs:136-163
pub fn into_wait_condition(
    self,
    jitter: Option<Duration>,
    verbose: Option<Duration>,
    probability: Option<f64>,
) -> Box<dyn WaitCondition> {
    match self {
        Commands::Duration { time } => {
            if let Some(probability) = probability {
                Box::new(conditions::ProbabilisticWait { duration: time, probability, verbose })
            } else {
                Box::new(conditions::DurationWait { duration: time, jitter, verbose })
            }
        }
        Commands::Normal { mean, std_dev } => Box::new(conditions::NormalWait {
            mean,
            std_dev,
            verbose,
            jitter,
        }),
        // ...
    }
}
```

The top-level `run_with_args` in `lib.rs` then calls `.wait()` on the result:

```rust
// src/lib.rs:18-23
fn run_with_args(args: cli::Cli) -> Result<()> {
    let condition = args
        .command
        .into_wait_condition(args.jitter, args.verbose, args.probability);
    condition.wait()
}
```

Three lines. The trait object erases the type; the entry point doesn't need to know which distribution was selected.

---

## The WaitCondition Trait

Every distribution implements a two-method trait:

```rust
// src/conditions.rs:67-70
pub trait WaitCondition {
    fn calculate_wait_duration(&self) -> Result<Duration>;
    fn wait(&self) -> Result<()>;
}
```

Separating calculation from execution is the key design decision here. It means:

1. Benchmarks can call `calculate_wait_duration` in a tight loop without actually sleeping.
2. Unit tests can assert on the returned `Duration` without wall-clock timing.
3. The verbose wait path can calculate the duration once, then hand it to a display loop.

---

## Normal Distribution in Depth

The normal (Gaussian) distribution is the natural choice when you have a process whose duration clusters around a mean with symmetric variance. It is fully specified by two parameters: µ (mean) and σ (standard deviation).

**The mathematics.** If X ~ N(µ, σ²), then roughly:
- 68% of samples fall within µ ± σ
- 95% within µ ± 2σ
- 99.7% within µ ± 3σ

For `dozr n 1s 0.15`: mean is 1 second, std_dev is 0.15. About 68% of waits will be between 0.85s and 1.15s; essentially none will be negative (the tail beyond -6σ is computationally irrelevant).

**Parameter mapping.** The mean is a `Duration` expressed in human-readable form. The std_dev is a dimensionless `f64`. Inside the implementation, the mean is converted to fractional seconds before being handed to `rand_distr`:

```rust
// src/conditions.rs:94-103
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
```

Three things worth noting:

**`.max(0.0)` before the conversion.** The normal distribution has support on all of ℝ. For very small means with large σ, a sample can be negative. `Duration::from_secs_f64` panics on negative input, so the clamp to zero is a correctness requirement, not a convenience. This is the kind of detail that separates a working implementation from a production one.

**`Normal::new` returns `Result`.** `rand_distr` validates parameters (σ must be ≥ 0). The `?` propagates this as an `anyhow::Error` up the call stack. Parameter validation happens in the library, not in dozr's CLI layer.

**The same `rng` is reused for jitter.** After sampling the distribution, the same `ThreadRng` is threaded into `RandomJitterGenerator`. This avoids a second RNG initialisation.

The struct itself is minimal:

```rust
// src/conditions.rs:87-92
pub struct NormalWait {
    pub mean: Duration,
    pub std_dev: f64,
    pub verbose: Option<Duration>,
    pub jitter: Option<Duration>,
}
```

All fields are `pub` — this is a library crate and exposing the fields allows tests and examples to construct `NormalWait` directly without builder boilerplate.

---

## Composing on Top: Jitter, Alignment, Probabilistic Execution

### Jitter

Jitter adds a uniform random offset on top of whatever the base distribution computed. It is useful when you want to keep the base distribution semantics but break any residual synchronisation. The implementation is extracted into a dedicated trait to make it testable without real randomness:

```rust
// src/conditions.rs:42-65
pub trait JitterGenerator {
    fn generate(&mut self, max_jitter: Duration) -> Duration;
}

pub struct RandomJitterGenerator<T: Rng> {
    rng: T,
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

The unit tests use a `MockJitterGenerator` that returns a fixed value, making `DurationWait::calculate_sleep_duration` fully deterministic:

```rust
// src/conditions.rs:366-375
struct MockJitterGenerator {
    jitter: Duration,
}

impl JitterGenerator for MockJitterGenerator {
    fn generate(&mut self, _max_jitter: Duration) -> Duration {
        self.jitter
    }
}
```

### Time Alignment

`dozr align 5m` waits until the next 5-minute boundary of wall-clock time. This is useful for scripts that should run at predictable intervals relative to the clock rather than relative to when they start:

```rust
// src/conditions.rs:266-284
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
}
```

The alignment is computed entirely in nanoseconds using integer modular arithmetic — no floating point, no chrono, no chance of rounding error. If the current time is exactly on a boundary (`remainder == 0`), it sleeps a full interval rather than returning zero; a zero sleep at exactly :00:00 would be surprising behaviour for a cron-preamble use case.

### Probabilistic Execution

`--probability 0.3` means "30% of the time, actually sleep; 70% of the time, return immediately." The `ProbabilisticWait` wrapper is only constructed when both a `Duration` command and a `--probability` flag are present:

```rust
// src/conditions.rs:317-343
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

Note that `wait()` rolls its own RNG independently of `calculate_wait_duration()`. The two methods are not composed; `wait()` re-implements the decision to avoid sampling twice when it can short-circuit. This is a deliberate trade-off: `calculate_wait_duration()` is primarily used for testing and benchmarking, while `wait()` is the hot path.

---

## Testing and Benchmarking Strategy

### Unit tests: pure calculation

The unit tests in `conditions.rs` call `calculate_wait_duration()` rather than `wait()`. This means they complete instantly and assert mathematical properties:

```rust
// src/conditions.rs:500-511
#[test]
fn test_normal_wait_calculate_duration() {
    let wait = NormalWait {
        mean: Duration::from_secs(1),
        std_dev: 0.1,
        verbose: None,
        jitter: None,
    };
    let duration = wait.calculate_wait_duration().unwrap();
    assert!(duration >= Duration::ZERO);
}
```

For `UniformWait`, the test can be tighter because the bounds are known:

```rust
// src/conditions.rs:549-558
fn test_uniform_wait_calculate_duration() {
    let wait = UniformWait {
        min: Duration::from_secs(1),
        max: Duration::from_secs(2),
        verbose: None,
        jitter: None,
    };
    let duration = wait.calculate_wait_duration().unwrap();
    assert!(duration >= Duration::from_secs(1) && duration <= Duration::from_secs(2));
}
```

For stochastic distributions (Normal, Exponential, Gamma), the only universally assertable property is non-negativity — and that's the `.max(0.0)` clamp being verified, not just the happy path.

### Integration tests: CLI surface

The tests in `tests/cli.rs` use `assert_cmd` to invoke the built binary and verify exit codes, argument validation, and stderr output. Notably, the integration tests that exercise actual waiting keep durations short and use generous bounds:

```rust
// tests/cli.rs:392-401
fn test_normal_distribution_wait_time() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    let start = Instant::now();
    cmd.args(&["n", "1s", "0.1"]).assert().success();
    let elapsed = start.elapsed();
    assert!(elapsed > Duration::from_millis(0));
    assert!(elapsed < Duration::from_secs(2));
}
```

This is honest: the test is verifying that the command completes within a plausible window for Normal(1s, 0.1), not that it hits exactly 1s. The bounds are wide enough to avoid flakiness while still catching a broken implementation that sleeps 0ms or 60s.

### Benchmarks: overhead isolation

The Criterion benchmarks in `benches/wait_conditions.rs` explicitly benchmark `calculate_wait_duration`, not `wait`. This isolates the cost of sampling the distribution from the cost of actually sleeping:

```rust
// benches/wait_conditions.rs:40-51
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

`black_box` prevents the compiler from optimising away the calculation. All nine distributions are benchmarked, letting you compare the overhead of, say, Gamma (which uses a rejection sampling algorithm internally) versus Uniform (a single RNG call). This matters when dozr is used in a tight loop — though in practice, sampling overhead is microseconds against sleep durations of hundreds of milliseconds, so the benchmarks are more about documentation and regression detection than performance optimisation.

---

## What the Architecture Demonstrates

The design reflects a specific set of priorities:

- **The trait separation (`calculate_wait_duration` vs `wait`)** makes stochastic code testable and benchmarkable without wall-clock timing. This is a pattern worth stealing.
- **`JitterGenerator` as a trait with a mock** shows understanding that randomness is a dependency that should be injectable, not a global.
- **Global CLI flags routed through the dispatch function** means every distribution gets jitter, verbose, and probabilistic execution "for free" without each implementation needing to handle it.
- **Distribution parameters that match user mental models** — a `Duration` for mean rather than a raw float, human-readable time strings via `humantime` — is API design that treats the CLI contract as seriously as a library API.

dozr is a small tool, but it earns its complexity: every added mechanism (distributions, jitter, alignment, probabilistic skip) exists because fixed `sleep` is insufficient for a real class of problems, and each is composed cleanly on top of a minimal core abstraction.
