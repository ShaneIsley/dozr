# Changelog

This file documents all notable project changes.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Planned as 0.5.0. This release breaks the command line and the library API.

### Changed
- **Breaking:** every argument that is a length of time now needs a unit. `dozr n 1s 0.1` becomes `dozr n 1s 100ms`, and the same applies to the `log-normal` standard deviation, the Pareto and gamma scales, and all three `triangular` values. A bare number such as `0.1` is rejected with exit status 2. The exponential rate and the Pareto and gamma shapes stay bare numbers.
- **Breaking:** `-v <PERIOD>` must be at least `1ms`. `-v 1ns` used to switch to adaptive updates by accident and is now rejected.
- `-v <PERIOD>` prints an update on every period. Previously it only printed when the whole-second countdown changed, so `-v 250ms` looked the same as `-v 1s`. Periods under a second show tenths, such as `1.5s`.
- The README describes verbose output as time-remaining updates. It previously promised a progress bar that never existed.
- **Breaking (library):** the `WaitCondition` trait now has two required methods, `sample` and `verbose`. `sample` returns a `Sample` that records the duration and whether it was clamped or skipped. `wait` draws exactly one sample and hands it to a `Sleeper`. Use `wait_using` to substitute a sleeper in tests.
- **Breaking (library):** `verbose` fields are now the `Verbose` enum (`Off`, `Adaptive`, `Every(period)`) instead of `Option<Duration>` with a `1ns` sentinel. Time-valued fields such as `NormalWait::std_dev`, `ParetoWait::scale`, `GammaWait::scale`, and the `TriangularWait` bounds are now `Duration`.
- `verbose_wait` calls its display function on every tick with the exact remaining time.
- `-j` / `--jitter` now applies to `align` and `at` too, ending the wait up to the jitter after the boundary or clock time. It used to be silently ignored for those commands. `dozr a 1m -j 30s` spreads machines over the first 30 seconds of each minute.
- **Breaking (library):** the eight `*Wait` structs, `ProbabilisticWait`, `ProbabilityGate`, and the `WaitCondition` trait are replaced by one `Wait` struct holding a `Sampler` enum, plus `jitter`, `probability`, and `verbose`. Build one with `Wait::new(sampler).with_jitter(..).with_probability(..)`, check it with `Wait::validate`, and run it with `Wait::wait`. Every parameter rule now lives in the library, so library users get the same validation as the command line.
- **Breaking (library):** `ThreadSleeper` is now `ConsoleSleeper`, which writes to any writer and sleeps on any `Clock`. `verbose_wait_with` and `adaptive_verbose_wait_with` take a `Clock` so progress loops can run on simulated time.
- Runtime errors print as a single `error: ...` line with exit status 1, without a Rust backtrace.
- `dist_sampler` is now an example rather than an installed binary, so `cargo install dozr` installs only `dozr`. It takes the same command and options as `dozr`, plus `--count` and `--seed`: `cargo run --example dist_sampler -- --count 1000 n 1s 100ms`.

### Fixed
- Extreme parameters such as `dozr e 1e-30`, `dozr par 1s 0.001`, or `dozr g 1e300 1s` no longer panic. Samples too large to represent now wait for the maximum possible duration.
- Adding jitter to a duration near the maximum no longer overflows.

### Added
- `--dry-run` prints the chosen duration, such as `1s 250ms`, to stdout instead of waiting. A skipped wait prints `0s`. The output is accepted back by `dozr d`, so a script can decide first and wait later.
- `--seed <SEED>` makes the random choice repeatable: the same arguments and seed always pick the same duration.
- `--skip-exit-code <STATUS>` exits with that status when a `-p` wait is skipped, so scripts can branch on it. The default stays 0.
- `--min <TIME>` and `--max <TIME>` keep any wait, after jitter, within bounds. With `-v`, `dozr` says when a bound changed the wait. `--min` above `--max` is rejected with exit status 2.
- Library: `Wait::with_min`, `Wait::with_max`, `DryRunSleeper`, and `Sample::adjusted`. `Wait::wait` and `Wait::wait_using` return the `Sample` they performed, and `run` returns the process exit status.
- With `-v`, `dozr` reports when a sampled duration was negative and was clamped to zero.
- Help text gives the unit of every argument and notes that negative `normal` samples wait for zero.
- A bare number where a length of time is expected, including `-j` and `-v`, gets an error suggesting a unit, such as `a unit is required, for example 0.1s or 0.1ms`.

- **Breaking:** `align` intervals must be at least `1ms`. Starting a process takes longer than that, so `dozr a 1ns` was accepted but meaningless.
- The README has a Precision section: every wait is a minimum, and each run has about 2 ms of fixed overhead, mostly starting the process.

### Removed
- `Cli::is_adaptive_verbose` and `Cli::verbose_period`. Match on `Verbose` instead.
- `JitterGenerator` and `RandomJitterGenerator`. Pass a seeded random number generator to `Wait::sample` for repeatable jitter.
- The unused `mockall` dev-dependency.
- `scripts/run_examples.sh`, which used pre-0.4 syntax, and the per-distribution `scripts/sampling/*.sh`. Use `scripts/check_examples.sh` and `scripts/sample_stats.sh` instead.

### Testing
- Progress loops run on a simulated clock in unit tests, so the full adaptive schedule of a 25-minute wait is checked in milliseconds.
- `scripts/check_examples.sh` runs every command in the README and `examples.md` with `--dry-run`, so each is parsed and fully validated without waiting, and CI runs it.
- CI checks that `cargo install` installs only the `dozr` binary.

## [0.4.2] - Not yet released

### Fixed
- `-v <PERIOD>` no longer extends the wait when the period is longer than the remaining time (`dozr d 1s -v 10s` now takes 1 second, not 10).
- `-p` / `--probability` now applies to every command. Previously it was silently ignored for everything except `duration`.
- `-j` / `--jitter` is no longer dropped when `-p` is also given.
- `log-normal` now treats `<MEAN>` and `<STD_DEV>` as the mean and standard deviation of the waits. Previously they were passed as the parameters of the underlying normal distribution, so `dozr ln 1s 0.5` averaged about 3.1 seconds instead of 1. **Behaviour change:** scripts that relied on the old meaning will now wait for different durations. `dist_sampler` uses the same meaning.
- `at` no longer panics when the requested time falls inside a daylight-saving gap.
- `-p 0.0` can no longer wait on a zero roll.

### Changed
- Invalid parameters are rejected at parse time with exit status 2 and an error naming the argument: probability outside 0.0-1.0, negative standard deviations, non-positive exponential rate, Pareto and gamma parameters, log-normal mean, and `align` interval, a uniform `<MIN>` not below `<MAX>`, and a triangular `<MODE>` outside `<MIN>`-`<MAX>`. Negative numbers are now read as values, so `dozr e -0.5` reports the bad rate rather than an unknown flag.
- `ProbabilisticWait` is deprecated in favour of `ProbabilityGate`, which wraps any `WaitCondition`.

### Added
- `WaitCondition::sample_with` takes a caller-supplied random number generator, so library users can get reproducible durations from a seeded generator.
- `conditions::log_normal_from_mean_sd`, `conditions::duration_to_next_alignment`, and `cli::duration_until` expose the underlying calculations for testing and reuse.
- CI now checks formatting, Clippy with warnings denied, documentation, and a build on the minimum supported Rust version, 1.85.
- Statistical tests for every distribution, daylight-saving tests for `at`, and end-to-end tests for each fix above. The full suite runs in under 10 seconds.

## [0.4.1] - 2025-12-18

### Added
- GitHub Actions CI workflow for automated build and test
- MIT LICENSE file in repository root
- Runnable examples in `examples/` directory (`cargo run --example <name>`)
- Criterion benchmarks in `benches/` directory (`cargo bench`)

### Changed
- Reorganized repository: moved helper scripts to `scripts/` directory
- Made main `dozr` binary explicit in Cargo.toml for clarity
- Updated `examples.md` to use current subcommand syntax
- Updated `TEST.md` to document benchmarks and runnable examples

## [0.4.0] - 2025-01-15

### Added
- Uniform distribution support (`uniform` / `u` command)
- Triangular distribution support (`triangular` / `t` command)
- Subcommand-based CLI architecture with short aliases
- Testing documentation (`TEST.md`)

### Changed
- Refactored CLI to use subcommands instead of flags
- Encapsulated WaitCondition creation in CLI module
- Improved code organization and reduced code smells

### Removed
- Weibull distribution (replaced by more commonly used distributions)

## [0.3.0] - 2025-01-10

### Added
- Normal distribution support (`normal` / `n` command)
- Exponential distribution support (`exponential` / `e` command)
- Log-normal distribution support (`log-normal` / `ln` command)
- Pareto distribution support (`pareto` / `par` command)
- Adaptive verbose mode that adjusts update frequency based on remaining time
- Comprehensive test suite with unit and integration tests

### Fixed
- Verbose output timing accuracy improvements
- Adaptive verbose output alignment to time markers

## [0.2.0] - 2025-01-05

### Added
- `--until` / `at` command to wait until a specific time of day
- `--probability` flag for probabilistic execution
- Time alignment with `align` command (snap to intervals)
- Customizable verbose update period
- Jitter support with `--jitter` flag

### Changed
- Refactored to library-first architecture
- Decoupled verbose output from wait logic
- Improved CLI argument validation using clap groups

## [0.1.0] - 2025-01-01

### Added
- Initial release
- Fixed duration waits with human-readable time parsing
- `--verbose` flag with ETA countdown
- Adaptive ETA display with human-readable formatting
- Support for duration units: seconds, minutes, hours, days, milliseconds

[Unreleased]: https://github.com/ShaneIsley/dozr/compare/v0.4.2...HEAD
[0.4.2]: https://github.com/ShaneIsley/dozr/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/ShaneIsley/dozr/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/ShaneIsley/dozr/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/ShaneIsley/dozr/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/ShaneIsley/dozr/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ShaneIsley/dozr/releases/tag/v0.1.0
