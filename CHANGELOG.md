# Changelog

This file documents all notable project changes.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Planned as 0.5.1. These defects were found by a post-release review of
0.5.0; see `REMEDIATION_PLAN.md`.

### Fixed
- `at` no longer waits for the second occurrence of a time that happens twice on the night the clocks go back. It picked the later one, so a time a minute away could wait an hour longer.
- Adaptive `-v` on a wait shorter than half a second no longer prints "Wait complete." twice with no countdown. Countdowns now round up in both modes, so a wait never reports `0s` while time remains.
- The hint for a bare number now suggests a value the parser accepts. `0.1` is steered to `100ms`, because `0.1s` was never valid; whole numbers still get `2s or 2ms`.
- A runtime error with a closed or full stderr no longer panics.
- Uniform bounds that differ as durations but are equal once converted to seconds are rejected up front with exit status 2, instead of failing at runtime with exit status 1.
- `align` intervals longer than 584 years no longer wrap.
- `at` with the current second waits zero rather than a day. The comparison is made at whole seconds.

### Changed
- **Behaviour change:** `align` boundaries are measured from local midnight rather than from the Unix epoch in UTC. `dozr a 1h` now lands on the hour in every time zone (it landed at half past in India) and `dozr a 1d` lands at local midnight (it was UTC midnight everywhere). Library: `Sampler::Align` uses the new `duration_to_next_local_alignment`; `duration_to_next_alignment` keeps its epoch semantics.
- The command-line validators that duplicated `Wait::validate` are removed. Every rule is checked once, in the library, with the same exit status 2 and messages naming the argument.
- `scripts/check_examples.sh` also checks backticked commands anywhere in the docs and prompt-style lines in `sh` and `bash` fences, and no longer passes a broken command whose exit status happens to equal its `--skip-exit-code`. CI proves it fails on `scripts/testdata/broken.md`.
- README: the comparison with `sleep` no longer claims GNU `sleep` takes seconds only, and the Precision section describes `-v` accurately.
- The changelog links for 0.1.0 to 0.4.1 point at the commits that set those versions, since they were never tagged.

### Testing
- An end-to-end test runs the binary under a POSIX time-zone rule with a fall-back two minutes ahead. The unit tests with `chrono-tz` could not catch the `at` bug, because that zone type orders a repeated hour differently from `chrono::Local`.
- The two chance-based jitter tests are seeded and no longer sleep.
- Tests on alignment in a half-hour zone and across both daylight-saving changes, and exact-line tests for short countdowns.

## [0.5.0] - 2026-10-16

This release fixes several cases where `dozr` silently did something other than what was asked, and it breaks both the command line and the library API. The fixes planned for a 0.4.2 release ship here instead.

### Fixed
- `-p` / `--probability` applies to every command. It used to be silently ignored for everything except `duration`.
- `-j` / `--jitter` is kept when `-p` is also given, and now applies to `align` and `at` too. It used to be dropped in both cases. `dozr a 1m -j 30s` spreads machines over the first 30 seconds of each minute.
- `-v <PERIOD>` no longer extends the wait when the period is longer than it (`dozr d 1s -v 10s` took 10 seconds).
- `-v <PERIOD>` prints on every period. It used to print only when the whole-second countdown changed, so `-v 250ms` looked the same as `-v 1s`. Periods under a second show tenths, such as `1.5s`.
- **Behaviour change:** `log-normal` treats `<MEAN>` and `<STD_DEV>` as the mean and standard deviation of the waits. They used to be passed as the parameters of the underlying normal distribution, so `dozr ln 1s 500ms` averaged about 3.1 seconds instead of 1.
- `at` no longer panics when the time falls in a daylight-saving gap. A missing time such as 02:30 waits until 03:30; in a repeated hour, the first occurrence still in the future is used.
- Extreme parameters such as `dozr e 1e-30`, `dozr par 1s 0.001`, and `dozr g 1e300 1s` no longer panic. Samples too large to represent wait for the maximum possible duration, and adding jitter near that maximum no longer overflows.
- `-p 0.0` can no longer wait on a zero roll.

### Changed
- **Breaking:** every argument that is a length of time needs a unit. `dozr n 1s 0.1` becomes `dozr n 1s 100ms`, and the same applies to the `log-normal` standard deviation, the Pareto and gamma scales, and all three `triangular` values. A bare number gets an error suggesting a unit, such as `a unit is required, for example 0.1s or 0.1ms`. The exponential rate and the Pareto and gamma shapes stay bare numbers.
- **Breaking:** invalid parameters are rejected with exit status 2 and an error naming the argument. This covers probabilities outside 0.0 to 1.0, a non-positive exponential rate, log-normal mean, or Pareto and gamma parameter, a uniform `<MIN>` not below `<MAX>`, a triangular `<MODE>` outside its range, and `--min` above `--max`. Negative numbers are read as values, so `dozr e -0.5` reports the bad rate rather than an unknown flag.
- **Breaking:** `-v <PERIOD>` and `align` intervals must be at least `1ms`. `-v 1ns` used to switch to adaptive updates by accident, and `dozr a 1ns` was accepted but meaningless.
- Runtime errors print as a single `error: ...` line with exit status 1, without a Rust backtrace.
- `dist_sampler` is an example rather than an installed binary, so `cargo install dozr` installs only `dozr`. It takes the same command and options as `dozr`, plus `--count`: `cargo run --example dist_sampler -- --count 1000 n 1s 100ms`.
- The README describes verbose output as time-remaining updates rather than a progress bar, and documents units, jitter on every command, and library usage.
- **Breaking (library):** the eight `*Wait` structs, `ProbabilisticWait`, the `WaitCondition` trait, and `Commands::into_wait_condition` are replaced by one `Wait` struct holding a `Sampler` enum, built with `Wait::new(sampler).with_jitter(..).with_probability(..)`, checked with `Wait::validate`, and run with `Wait::wait`. Every parameter rule lives in the library, so library users get the same validation as the command line. `Wait::wait` returns the `Sample` it performed, which records whether it was skipped, clamped, or bounded.
- **Breaking (library):** verbose settings are the `Verbose` enum (`Off`, `Adaptive`, `Every(period)`) instead of `Option<Duration>` with a `1ns` sentinel. Time-valued parameters are `Duration`.
- **Breaking (library):** `run` returns the process exit status. `verbose_wait` calls its display function on every tick with the exact remaining time.

### Added
- `--dry-run` prints the chosen duration, such as `1s 250ms`, instead of waiting. A skipped wait prints `0s`. `dozr d` accepts the output back, so a script can decide first and wait later.
- `--seed <SEED>` makes the random choice repeatable.
- `--skip-exit-code <STATUS>` exits with that status when a `-p` wait is skipped. The default stays 0.
- `--min <TIME>` and `--max <TIME>` keep any wait, after jitter, within bounds.
- With `-v`, `dozr` reports when a sample was negative and clamped to zero, or changed by `--min` or `--max`.
- Help text gives the unit of every argument.
- The README has a Precision section: every wait is a minimum, and each run has about 2 ms of fixed overhead, mostly starting the process.
- Library: `Sample`, `Skip`, `Adjustment`, `Sleeper`, `ConsoleSleeper`, and `DryRunSleeper` in `conditions`; `Clock`, `SystemClock`, `verbose_wait_with`, and `adaptive_verbose_wait_with` in the new `progress` module; and `log_normal_from_mean_sd`, `duration_until`, and `duration_to_next_alignment` for reuse and testing.

### Removed
- Library: `Cli::is_adaptive_verbose` and `Cli::verbose_period`; match on `Verbose` instead. `JitterGenerator` and `RandomJitterGenerator`; pass a seeded generator to `Wait::sample` for repeatable jitter.
- The unused `mockall` dev-dependency.
- `scripts/run_examples.sh`, which used pre-0.4 syntax, and the per-distribution `scripts/sampling/*.sh`. Use `scripts/check_examples.sh` and `scripts/sample_stats.sh` instead.

### Testing
- Statistical tests for every distribution, daylight-saving tests for `at`, and end-to-end tests for every fix. Random numbers, the clock, and output are injectable, so progress output is tested on simulated time. The full suite runs in under 10 seconds instead of about 37.
- CI checks formatting, Clippy with warnings denied, tests, documentation, every command in the README and `examples.md`, that `cargo install` installs only `dozr`, and a build on the minimum supported Rust version, 1.85.

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

[Unreleased]: https://github.com/ShaneIsley/dozr/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/ShaneIsley/dozr/compare/92e11a2...v0.5.0
[0.4.1]: https://github.com/ShaneIsley/dozr/compare/5004a24...92e11a2
[0.4.0]: https://github.com/ShaneIsley/dozr/compare/ccaf43f...5004a24
[0.3.0]: https://github.com/ShaneIsley/dozr/compare/698146f...ccaf43f
[0.2.0]: https://github.com/ShaneIsley/dozr/compare/ff2dc9e...698146f
[0.1.0]: https://github.com/ShaneIsley/dozr/commit/ff2dc9e
