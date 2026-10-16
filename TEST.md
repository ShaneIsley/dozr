# Testing dozr

Run everything with:

```bash
cargo test
```

The full suite runs in under 10 seconds. No test waits longer than about 2 seconds, and most unit tests do not wait at all.

## What makes the tests fast and repeatable

The library takes its three sources of nondeterminism as arguments, so tests can replace each one:

| Source | Real implementation | Test replacement |
| :--- | :--- | :--- |
| Random numbers | `rand::rng()` | A seeded `StdRng`, passed to `Wait::sample` or `Wait::wait_using` |
| Sleeping and the passage of time | `SystemClock` | `FakeClock` in `src/progress.rs`, which advances only when slept on |
| Where a wait goes | `ConsoleSleeper::stderr()` | `RecordingSleeper`, which records each sample, or a `ConsoleSleeper` writing to a `Vec<u8>` with a `FakeClock` |

The current time is also an argument: `Wait::sample_at` takes the local time, and `duration_until` and `duration_to_next_alignment` are pure functions.

## Unit tests

Unit tests live beside the code in `#[cfg(test)]` modules.

*   **`src/conditions.rs`**
    *   Statistical checks: 20,000 seeded samples per distribution, with the mean and standard deviation compared to theory.
    *   Table-driven checks over every `Sampler` variant: jitter is added, the probability gate applies, and seeded draws repeat exactly.
    *   The duration passed to the sleeper is exactly the one sampled.
    *   Negative samples clamp to zero and are flagged. Oversized samples saturate at `Duration::MAX` instead of panicking.
    *   Every validation rule, and the exact progress, clamp, and skip messages, using a fake clock.
    *   `at` across midnight, at a fixed UTC offset, and through daylight-saving changes in `America/New_York` (using the `chrono-tz` dev-dependency).
*   **`src/progress.rs`**: the fixed-period and adaptive update schedules, including a full 25-minute adaptive run, all on a fake clock.
*   **`src/cli.rs`**: argument parsing, units, `--verbose` forms, and the mapping from each subcommand to its `Sampler`.

## Integration tests

`tests/cli.rs` runs the real `dozr` binary for every subcommand: exit codes, error messages, stderr output, and elapsed time. Every spawned process has a 15-second timeout so a regression fails instead of hanging. Tests that depend on chance give their failure probability in a comment; all are below 0.1%.

## Doc tests

Examples in the API documentation are compiled and run by `cargo test`.

## Documented commands

`scripts/check_examples.sh` runs every `dozr` command shown in `README.md` and `examples.md` with `-p 0`, so each is parsed and validated without waiting. CI runs it on every push.

## Inspecting distributions by hand

The `dist_sampler` example prints sampled durations without waiting, and `scripts/sample_stats.sh` summarises them:

```bash
scripts/sample_stats.sh --count 20000 --seed 1 ln 1s 500ms
```

To plot a histogram, install `scripts/requirements.txt` and pipe the sampler into `scripts/analyze_distributions.py`.

## Benchmarks

`benches/wait_conditions.rs` uses [Criterion](https://github.com/bheisler/criterion.rs) to measure the cost of drawing a sample for each distribution:

```bash
cargo bench
```

Reports are written to `target/criterion/`.

## Continuous integration

`.github/workflows/rust.yml` checks formatting, runs Clippy with warnings denied, runs the tests, builds the documentation with warnings denied, checks the documented commands, confirms that `cargo install` installs only the `dozr` binary, and builds on the minimum supported Rust version, 1.85.
