# dozr Remediation Plan

Prioritised plan to bring `dozr` 0.4.1 in line with what its README promises,
then to tighten its architecture and project hygiene. Every item names the
defect, the fix, the tests that must exist before it is considered done, and
the version it ships in.

Findings were confirmed by running the 0.4.1 release binary; reproduction
commands are included so each fix can be checked the same way.

## Status

| Item | State | Notes |
| :--- | :--- | :--- |
| T-1, T-2 | Done | RNG injected through `WaitCondition::sample_with`. Clock injected through `cli::duration_until` and `conditions::duration_to_next_alignment`. |
| T-3 | Done | Full suite runs in about 6 seconds. Every spawned process has a 15-second timeout so a regression fails instead of hanging. |
| T-4 | Done | Format, Clippy, test, and docs gates, plus a build on Rust 1.85. |
| P0-1 to P0-6 | Done | Deviations from the original wording are noted under each item. |
| P1-1 to P1-6 | Done | P1-5 and P1-6 share one mechanism: `WaitCondition::sample` returns a `Sample` with clamped and skipped details, and every wait goes through a replaceable `Sleeper`. That is the sleep injection from P2-1, done early because both items need it. |
| P2-1 to P2-5 | Done | Notes under each item. P2-2 also fixed a panic on extreme parameters, found while planning it. |
| P3-1 to P3-4 | Done | Notes under each item. |
| Release | 0.5.0 | The planned 0.4.2 was not released separately; its fixes ship in 0.5.0. Published to crates.io, tagged `v0.5.0`, release page created. |
| Post-release review | Done | An independent adversarial review of the shipped code, plus a before-and-after measurement, is recorded under **Post-release review** below. It found one behaviour bug and one misleading error introduced by this work. |
| P4-1 to P4-9 | Not started | The 0.5.1 patch. |

## Priorities

| Priority | Meaning | Target release |
| :--- | :--- | :--- |
| **P0** | Documented behaviour is wrong or the binary can panic. Fix before any other work. | 0.4.2 (patch) |
| **P1** | Behaviour is misleading, undocumented, or inconsistent. Some fixes change the CLI contract. | 0.5.0 (minor) |
| **P2** | Architecture, test infrastructure, and project hygiene. No user-visible change on its own. | 0.5.0 |
| **P3** | New capabilities that scripts need and that the refactor makes cheap. | 0.5.x |
| **P4** | Defects found by the post-release review of 0.5.0. | 0.5.1 (patch) |

Suggested order of work: P2-1 (RNG and clock injection) is a prerequisite
for writing deterministic tests for several P0 items, so start there, then
P0, then P1, then the rest of P2, then P3.

## Test infrastructure changes (do first)

The current suite is slow (the integration run takes about 32 seconds, most of
it in one 21-second wait), depends on the wall clock (the `at` tests fail if
run within a minute of midnight), and cannot assert anything about random
output except "it did not panic". Three changes unlock proper tests for the
items below.

### T-1. Inject the random number generator

Every `calculate_wait_duration` creates its own `ThreadRng`. Change the core
API so sampling takes `&mut impl Rng` (or `&mut dyn RngCore`). The CLI passes
a `ThreadRng` or, once P3-1 lands, a seeded `StdRng`.

Required tests:

- A seeded `StdRng` produces identical durations on two calls with identical
  parameters (determinism).
- Statistical tests become non-flaky: with a fixed seed and 20,000 samples,
  assert the sample mean and standard deviation of each distribution within
  2% of the theoretical value. One test per distribution.

### T-2. Inject the clock

`parse_time_until` in `src/cli.rs` and `TimeAlignWait` in `src/conditions.rs`
read `Local::now()` and `SystemTime::now()` directly. Split each into a pure
function that takes `now` as an argument and a thin wrapper that supplies the
real clock.

Required tests:

- `at` rollover to tomorrow, "same minute later today", and "exactly now"
  tested with a fixed `now`, not the wall clock.
- `align` tested with fixed epoch values; the existing test helper
  `calculate_time_to_next_alignment` in `src/conditions.rs` re-implements the
  production logic instead of calling it. Delete the helper and call the real
  function.

### T-3. Bound wall-clock time in the suite

Replace long sleeps with short ones. No single test may sleep more than 2
seconds; the full `cargo test` run must finish in under 15 seconds. The
21-second adaptive test (`test_verbose_adaptive_long_wait`) is replaced by a
unit test on the adaptive period table plus a 2-second integration run.

### T-4. CI gates

Add to `.github/workflows/rust.yml`:

```yaml
- run: cargo fmt --all -- --check
- run: cargo clippy --all-targets -- -D warnings
- run: cargo test
- run: cargo doc --no-deps
```

Also build on the declared `rust-version` (1.85) so the MSRV claim in
`Cargo.toml` is real.

## P0: wrong or dangerous behaviour

### P0-1. Fixed-period verbose overshoots the wait

Reproduce:

```bash
dozr d 1s -v 10s   # takes 10 s, should take 1 s
```

Cause: `verbose_wait` in `src/lib.rs` sleeps for the full update period
without capping it at the remaining time.

Fix: `sleep_duration = min(update_period, remaining)`. Also remove the dead
`next_update_time - elapsed` arithmetic, which always equals `update_period`.

Tests:

- Unit: `verbose_wait(100ms, 10s, …)` returns in under 200 ms.
- Unit: `verbose_wait(100ms, 10ms, …)` invokes the display callback at least
  once and the final call receives `Duration::ZERO`.
- Integration: `dozr d 200ms -v 10s` exits in under 1 s and prints
  "Wait complete.".

### P0-2. `--probability` is ignored on every subcommand except `duration`

Reproduce:

```bash
dozr n 1s 0.01 -p 0.0   # still waits ~1 s
dozr a 2s -p 0.0        # still waits
```

Cause: `Commands::into_wait_condition` in `src/cli.rs` only wraps
`Duration` in `ProbabilisticWait`.

Fix: probability becomes a wrapper applied to the result of every arm (see
P2-2 for the clean version; a stopgap is to apply the roll in
`run_with_args` before calling `wait`).

Tests:

- Integration, for each of the ten subcommands: `-p 0.0` exits in under
  1 s against a base wait of at least 5 s; with `-v` it prints "Skipping
  sleep". (Implemented with a 1 s threshold rather than 100 ms, because
  debug-binary start-up under parallel test load can approach 100 ms.)
- Integration, for each subcommand: `-p 1.0` produces identical behaviour to
  omitting the flag (same stderr shape, elapsed time within the normal
  range).

### P0-3. `--jitter` is silently dropped when `--probability` is given

Reproduce:

```bash
for i in 1 2 3 4 5; do time dozr d 100ms -j 3s -p 1.0; done   # always 104 ms
```

Cause: `ProbabilisticWait` has no jitter field, and `into_wait_condition`
chooses it instead of `DurationWait` when `-p` is set.

Fix: same composition as P0-2. Jitter is applied to the sampled duration
regardless of whether a probability gate is present.

Tests:

- Unit (with injected RNG): `duration=100ms, jitter=3s, probability=1.0`
  yields durations between 100 ms and 3.1 s over 200 seeded draws, with at
  least one above 1 s. (The original "equal to the un-gated calculation for
  the same seed" is not achievable because the gate's roll consumes the
  first random number.)
- Integration: five runs of `dozr d 10ms -j 500ms -p 1.0` produce at least
  two distinct elapsed times greater than 50 ms apart.

### P0-4. Log-normal `<MEAN>` is not the mean

Reproduce:

```bash
dist_sampler --distribution log_normal --mean 1s --std-dev 0.5 --count 20000
# sample median ≈ 2.72 s, mean ≈ 3.07 s; a 1 s mean was requested
```

Cause: the parsed seconds are passed straight to `LogNormal::new(mu, sigma)`
in `src/conditions.rs`, which takes the parameters of the underlying normal
distribution.

Fix: accept the requested mean `m` and standard deviation `s` and convert.
In 0.4.2 the standard deviation stays a bare number of seconds; P1-3 turns
it into a duration. Implemented with `rand_distr::LogNormal::from_mean_cv`,
which performs this conversion:

```text
sigma² = ln(1 + (s/m)²)
mu     = ln(m) − sigma² / 2
```

Document the previous behaviour in the changelog as a bug fix with a
behaviour change.

Tests:

- Unit (seeded, 20,000 samples): sample mean within 3% of the requested mean
  and sample standard deviation within 5% of the requested value.
- Unit: `m = 0` or `s < 0` returns an error, not a panic or NaN.
- Doc test on the public constructor showing the mean/sd contract.

### P0-5. No validation of `--probability` or distribution parameters

Reproduce:

```bash
dozr d 0s -p 7        # accepted, exit 0
dozr u 5s 1s          # runtime error from rand_distr, exit 1
```

Fix: validate at parse time with clap value parsers so every bad input
produces the standard clap error and exit code 2:

- `-p` in `[0.0, 1.0]`
- `normal`, `log-normal`: standard deviation ≥ 0
- `exponential`: lambda > 0
- `pareto`: scale > 0, shape > 0
- `gamma`: shape > 0, scale > 0
- `uniform`: min < max
- `triangular`: min ≤ mode ≤ max and min < max
- `align`: interval > 0

Tests:

- Integration: one failing case per rule above asserts exit code 2 and that
  stderr contains "error:" and names the offending argument.
- Integration: the boundary values (`-p 0.0`, `-p 1.0`, `sd 0`) are accepted.

### P0-6. `at` can panic on daylight-saving transitions

Cause: `parse_time_until` in `src/cli.rs` sets hour, minute and second on a
`DateTime<Local>` and unwraps. In a timezone with DST, a time inside the
skipped hour (for example 02:30 on the spring-forward day) yields `None`.

Fix: build a `NaiveDateTime` from today's date and the parsed time, then
resolve it with `from_local_datetime(..)`. A time in a gap is shifted forward
by the length of the gap, using the offset in force before it, so 02:30
becomes 03:30. This matches `java.time` and JavaScript Temporal, and replaces
the original "next valid instant" wording, which would have given 03:00.
Ambiguous times (fall back) take the earliest occurrence that is not in the
past, so the second 01:30 is used once the first has gone by.

Tests:

- Add `chrono-tz` as a dev-dependency. Unit tests with the clock injected
  (T-2) and the zone fixed to `America/New_York`:
  - 02:30 requested on the spring-forward date resolves without panicking
    to 03:30 EDT.
  - 01:30 requested on the fall-back date resolves to the earlier instant,
    and to the later one when the earlier has already passed.
- Existing rollover and "in the future" tests reworked to use the fixed
  clock.

## P1: misleading or inconsistent behaviour

### P1-1. README promises a progress bar

The verbose output is timestamped "Time remaining" lines on stderr. Either
implement a bar or change the README and the feature table. Recommended:
fix the docs now; a bar is a P3 candidate.

Tests: none beyond a documentation review. If a bar is implemented later it
needs an integration test that asserts the bar is only drawn when stderr is a
TTY.

### P1-2. Sub-second verbose periods do nothing

Reproduce:

```bash
dozr d 3s -v 250ms 2>&1 | wc -l   # 5 lines, same as -v 1s
```

Cause: `verbose_wait` only calls the display function when the whole-second
rounded ETA changes.

Fix: in fixed-period mode, display on every tick. Format the remaining time
with the precision of the period (`{:.1}s` when the period is under 1 s).

Tests:

- Integration: `dozr d 1s -v 250ms` prints at least 4 "Time remaining" lines.
  (Implemented as `dozr d 1500ms -v 250ms`, six nominal updates, so
  scheduling delay under parallel test load cannot drop it below four.)
- Integration: `dozr d 3s -v 1s` prints at most 4 such lines (no regression
  into spamming). (Implemented as `dozr d 2s -v 1s` printing two or three,
  to keep the test under the 2-second budget.)

### P1-3. Parameter types are inconsistent across subcommands

Today `uniform` takes durations (`1s 5s`), `triangular` takes bare floats
(`0.0 10.0 5.0`), and `normal` takes a duration mean but a bare-float
standard deviation whose unit (seconds) is undocumented.

Fix (breaking, 0.5.0): every parameter that is a length of time is parsed
with `humantime`, so `dozr n 1s 200ms` and `dozr t 1s 5s 2s`. Dimensionless
shape parameters (`exponential` lambda, `pareto` shape, `gamma` shape) stay
as floats. Document the unit of every argument in its help text.

Tests:

- Integration: each subcommand accepts the new form; the old bare-float form
  for a duration argument fails with exit code 2.
- Unit: the sampled-value bounds tests from T-1 use the new types.

### P1-4. Adaptive verbose is encoded as a `1ns` sentinel

`-v` with no value becomes `Some(1ns)` via `default_missing_value` in
`src/cli.rs`, and `src/conditions.rs` re-derives the meaning by checking
`as_nanos() == 1`. A user typing `-v 1ns` silently gets adaptive mode.

Fix: parse into an enum `Verbose { Off, Adaptive, Every(Duration) }` with a
custom value parser. Reject periods under 1 ms.

Tests:

- Unit: `-v` yields `Adaptive`; `-v 500ms` yields `Every(500ms)`; absence
  yields `Off`. A bare `-v` placed before the subcommand takes the
  subcommand name as its value, as it did in 0.4.1. The README now says to
  put a bare `-v` after the command.
- Integration: `-v 1ns` is rejected with exit code 2.
- Delete `is_adaptive_verbose` and `verbose_period` and their tests.

### P1-5. Normal and other samples are clamped to zero silently

With `dozr n 100ms 500ms` about 43% of waits are zero.

Fix: document the clamp in the help text and README. In verbose mode, print
a one-line note when a sample was clamped. Consider P3-3 (a floor option)
later.

Tests:

- Unit (seeded): a seed that produces a negative sample yields
  `Duration::ZERO` and sets a `clamped` flag on the result.
- Integration: a clamped verbose run prints the note.

### P1-6. `ProbabilisticWait` rolls the dice twice

`calculate_wait_duration` and `wait` each draw their own random number, so
the two trait methods can disagree.

Fix: folded into P2-2. `wait` must use the duration returned by
`calculate_wait_duration`. (Done in P1: `ProbabilisticWait` delegates to
`ProbabilityGate`, and every `wait` draws one `Sample` and passes it to a
`Sleeper`.)

Tests:

- Unit (seeded): `calculate_wait_duration` and the duration actually slept
  (captured through an injected sleep function) are equal.

## P2: architecture and hygiene

### P2-1. Inject RNG, clock, and sleep

Described under T-1 and T-2. Add a third injection point: the sleep function
used by `perform_wait`, so tests can record the requested duration instead of
actually sleeping. This is what makes the P0 and P1 tests fast.

(Done in two layers. A `Sleeper` receives each sample, and `ConsoleSleeper`
takes any writer and any `Clock`. The progress loops take a `Clock` too, so a
`FakeClock` runs a 25-minute adaptive schedule in milliseconds.)

### P2-2. Replace eight copy-pasted structs with one composed `Wait`

`src/conditions.rs` has eight structs whose `calculate_wait_duration` bodies
differ only in the distribution constructor. Replace with:

```rust
pub enum Sampler { Fixed(Duration), Normal{..}, Exponential{..}, LogNormal{..},
                   Pareto{..}, Uniform{..}, Triangular{..}, Gamma{..},
                   Align(Duration), At(Duration) }

pub struct Wait {
    pub sampler: Sampler,
    pub jitter: Option<Duration>,
    pub probability: Option<f64>,
    pub verbose: Verbose,
}
```

`Wait::calculate` samples, applies jitter, then applies the probability gate,
for every variant. This fixes P0-2, P0-3, and P1-6 structurally rather than
case by case. Keep the `WaitCondition` trait only if an external user needs
it; otherwise remove it and the `Box<dyn>`.

(Done. The trait is removed: the crate is pre-1.0 and 0.5.0 already breaks
the library API. `At` holds a `NaiveTime` resolved when sampled, so
`Wait::sample_at` can take the current time as an argument. The probability
gate is rolled before the sampler is drawn, so a skipped wait draws nothing
else. Jitter now applies to `align` and `at` as well, which the plan's "for
every variant" implies and which makes `dozr a 1m -j 30s` useful for
staggering machines. Validation moved into `Wait::validate`, shared by the
CLI and library users. While doing this, a panic on extreme parameters was
found and fixed: samples too large for a `Duration` now saturate.)

Tests:

- Unit: a table-driven test over every `Sampler` variant asserting jitter is
  added and probability gating applies (seeded RNG).
- All existing per-struct tests migrate to the enum.
- `benches/wait_conditions.rs` and `examples/*.rs` updated and compiled in CI
  (`cargo build --all-targets`).

### P2-3. Stop installing `dist_sampler` for end users

`cargo install dozr` currently installs two binaries. Move
`src/bin/dist_sampler.rs` to `examples/dist_sampler.rs`, or keep it as a
binary behind `required-features = ["tools"]`.

(Done as an example. It now parses the same command and options as `dozr`,
so its samples cannot drift from what `dozr` waits. The five
`scripts/sampling/*.sh` scripts called it 1,000 times each with the old
flags. They are replaced by `scripts/sample_stats.sh`.)

Tests:

- CI step: `cargo install --path . --root "$TMP" && test "$(ls "$TMP/bin")" = dozr`.

### P2-4. Dependency and lint hygiene

- Remove `mockall` (unused).
- Fix all 56 clippy warnings (`cargo clippy --fix` handles most).
- Run `cargo fmt`.
- Remove the `anyhow` backtrace noise from user-facing errors by printing
  errors with `{}` rather than `{:?}` in `main`.

Tests: enforced by T-4 CI gates.

### P2-5. Stale and misleading documentation

- `scripts/run_examples.sh` uses the pre-0.4 flag syntax and macOS-only
  `date -v`. Rewrite with subcommand syntax and portable `date -d`/`gdate`
  detection, or delete it in favour of `examples.md`.
- `TEST.md` claims `src/lib.rs` is tested with mock objects. It is not.
  Rewrite after P2-1 so the claim is true.
- The Weibull note at the bottom of `scripts/run_examples.sh` is obsolete.
- `examples.md` says adaptive updates are "500ms for short waits"; the table
  in `src/lib.rs` starts at 1 s. Align the text with the code.
- `CHANGELOG.md`: add the 0.4.2 and 0.5.0 entries as the work lands, calling
  out the log-normal and parameter-type behaviour changes.

Tests: a `scripts/check_examples.sh` CI step that extracts every fenced
`bash` block from `README.md` and `examples.md` and runs it with `--help`
appended, proving each documented command parses.

(Done with `-p 0` instead of `--help`. `--help` stops parsing as soon as it
is seen, so cross-argument rules never run. `-p 0` runs full validation and
then skips the wait. The script also checks inline commands in Markdown
tables. `scripts/run_examples.sh` was deleted rather than rewritten.)

## P3: capabilities scripts need

### P3-1. `--seed <u64>`

Deterministic runs for reproducible load tests and for debugging. Cheap once
P2-1 exists.

Tests: two runs with the same seed and parameters print the same sampled
duration under `--dry-run`.

### P3-2. `--dry-run` (print the sampled duration, do not sleep)

Lets a script read the sample and decide what to do with it, and makes
statistical testing of the CLI possible without sleeping.

Tests: output is a single humantime-formatted duration on stdout; elapsed
time under 50 ms; combined with `--seed` it is stable.

(Done. The elapsed-time check uses a one-hour wait and a 1 s limit rather
than 50 ms, for the same parallel-load reason as P0-2. The printed duration
is also checked to round-trip through `dozr d`. With `-v`, notes such as a
skipped or bounded wait go to stderr; there is no countdown.
`scripts/check_examples.sh` now uses `--dry-run` instead of `-p 0`, so
documented probabilities are checked too.)

### P3-3. Exit status when a probabilistic wait is skipped

Add `--skip-exit-code <n>` (default 0 for compatibility). Scripts can then
branch on whether the wait happened.

Tests: `-p 0.0 --skip-exit-code 3` exits 3; `-p 1.0 --skip-exit-code 3`
exits 0.

(Done. It also applies under `--dry-run`, and a seeded test checks the
status matches whether the wait happened. `Wait::wait` now returns the
performed `Sample`, so library users can see a skip too.)

### P3-4. Optional floor and ceiling for sampled waits

`--min <TIME>` and `--max <TIME>` clamp any sampled duration. Replaces the
silent zero clamp of P1-5 with something the user controls, and tames
Pareto's heavy tail (a scale-1 shape-2 run produced a 127 s sample).

Tests: seeded samples never fall outside the bounds; `--min` greater than
`--max` is rejected at parse time.

(Done. Bounds apply to the total after jitter, and a skipped wait stays
skipped. The sample records an `Adjustment` with the original value, which
`-v` reports. The flags use internal ids distinct from the `uniform` and
`triangular` `<MIN>`/`<MAX>` arguments, and a test runs both together.)

## Definition of done per release

**0.4.2** ships when: all P0 items are merged with their tests, T-3 and T-4
are in place, and `cargo clippy -- -D warnings` passes.

**0.5.0** ships when: all P1 and P2 items are merged, the README and
`examples.md` reflect the new argument types, `CHANGELOG.md` lists every
breaking change, and the documented-command CI check (P2-5) passes.

**0.5.x** adds P3 items individually as they are ready.

**0.5.1** ships when: P4-1 to P4-7 are merged with their tests, the
end-to-end time-zone test runs in CI, and `scripts/check_examples.sh` fails
on the planted file described under P4-5. P4-8 and P4-9 may follow in 0.5.2.

## Post-release review

Carried out after 0.5.0 was published. Two inputs: a side-by-side run of the
original critique's reproductions against a rebuilt 0.4.1 and the released
0.5.0, and an independent adversarial review of the shipped code that was
given the code, not the plan, so it was not anchored on the author's reading.

### Outcomes against the original critique

Every defect in the original critique is fixed and the fix is measurable:

| Check | 0.4.1 | 0.5.0 |
| :--- | :--- | :--- |
| `d 1s -v 10s` wall time | 10.0 s | 1.0 s |
| `n 1s … -p 0.0` wall time | 1.0 s | 4 ms |
| `d 100ms -j 3s -p 1.0`, longest of 5 | 105 ms | 2,987 ms |
| `a 100ms -j 500ms`, longest of 5 | 100 ms | 538 ms |
| `d 2s -v 250ms` update lines | 2 | 8 |
| `-p 7` exit status | 0 | 2 |
| `a 1ns` | accepted | rejected |
| `e 1e-30` | panic | waits |
| log-normal "1s" sampled mean | 3.08 s | 1.00 s |

| Metric | 0.4.1 | 0.5.0 |
| :--- | :--- | :--- |
| Clippy warnings | 56 | 0 |
| Tests | 60 | 114 |
| Suite wall time | 36 s | 8 s |
| Runtime dependencies | 38 | 38 |

CI passed on `main` for both merges. The crate installs from crates.io with
current dependency versions. The docs.rs build was not verified.

### Code size, measured correctly

A first measurement reported 43% growth in production code. It was wrong: it
counted doc comments as code, compared unformatted 0.4.1 against formatted
0.5.0, and counted the old `dist_sampler` binary but not the new example.
With both versions formatted and comments and tests excluded:

| Scope | 0.4.1 | 0.5.0 | Change |
| :--- | :--- | :--- | :--- |
| Everything shipped | 712 | 837 | +18% |
| Library and binary, sampler excluded on both sides | 600 | 837 | +40% |
| Comment lines | 77 | 231 | +200% |

Where the 237 library lines came from, by function:

| Category | Lines |
| :--- | :--- |
| Parameter validation | about +140 |
| New options and their reporting (`--dry-run`, `--seed`, `--skip-exit-code`, `--min`, `--max`, clamp and bound notes) | about +130 |
| Clock and sleeper injection | about +50 |
| Daylight-saving handling for `at` | about +28 |
| Core wait model (one `Wait` and `Sampler` replacing eight structs) | about -105 |
| Progress schedule | about -26 |

The refactor itself shrank the code; the growth is features and validation.
About 45 of the validation lines are duplicated between CLI value parsers
and `Wait::validate` (see P4-9).

### Findings

Ranked by severity. Each was reproduced on the released binary. "Introduced"
means the defect did not exist in 0.4.1.

1. **`at` picks the second occurrence during a repeated hour.** Introduced.
   See P4-1.
2. **Adaptive `-v` on waits under 500 ms prints "Wait complete." twice** and
   no countdown. Pre-existing; two tests pass through this path and assert
   too little. See P4-2.
3. **The unit hint recommends `0.1s`, which the parser rejects.** Introduced.
   See P4-3.
4. **`align` boundaries are measured from the Unix epoch in UTC,** but the
   README says "on the hour". Pre-existing. See P4-4.
5. **`scripts/check_examples.sh` passes broken commands** in some forms.
   Introduced. See P4-5.
6. **A runtime error with a closed stderr pipe panics.** See P4-6.
7. **Two chance-based tests** have about a 0.03% chance each of a false
   failure per run. See P4-7.
8. **Edge cases on absurd input:** uniform bounds that differ as `Duration`
   but collide as `f64` fail at runtime with exit 1; `a 1000y` wraps; `-v 1s`
   shows "0s" half a second before the end; `at` for the current second
   waits a day. See P4-8.
9. **Documentation nits:** the README says `sleep` takes "seconds only" (GNU
   `sleep` accepts `0.05s` and `0.001m`); "one sleep call for the whole
   duration" is only true without `-v`; builder methods and some struct
   fields lack doc comments; the changelog links for 0.1.0 to 0.4.1 point at
   tags that do not exist. See P4-4 and P4-8.

### Process lessons

- **A test double that differs from production gives false confidence.**
  The DST unit tests used the `chrono-tz` zone type, which orders an
  ambiguous local time by instant. The real `Local` type orders it by UTC
  offset, the reverse for a fall-back. The tests passed while the binary was
  wrong. The plan's own test requirement for P0-6 was written in terms of
  the double; it should have demanded an end-to-end run of the binary under
  a controlled `TZ`.
- **Tests that assert only a completion line pass for the wrong reason.**
  Assert the exact lines where output is the behaviour under test.
- **Rebuild before measuring.** A stale release binary produced a false
  regression during the review; `cargo build --examples` does not rebuild
  the main binary.
- **Measure like for like.** The 43% code-growth figure compared different
  things; see above.

## P4: defects found by the post-release review

### P4-1. `at` waits for the second occurrence in a repeated hour

Reproduce, with a POSIX time-zone rule whose fall-back happens two minutes
from now (the three values are local DST times):

```bash
now=$(date -u +%s)
fold=$(date -u -d "@$((now+3600+120))" +%H:%M:%S)
target=$(date -u -d "@$((now+3600+60))" +%H:%M:%S)
TZ="STD0DST,J259/02:00,J$(date -u +%j)/$fold" dozr at "$target" --dry-run
# prints about 1h 1m; the first occurrence is 1m away
```

Cause: the `LocalResult::Ambiguous` branch of `duration_until` in
`src/conditions.rs` assumes the pair is ordered `(earliest, latest)` by
instant. `chrono::Local` on Unix orders it by UTC offset, so for a fall-back
the first element is the later instant.

Fix: sort the two candidates by instant before choosing the first that is
not in the past.

Tests:

- Integration: run the binary under the rule above and assert the dry-run
  output is under five minutes. This is the test the unit tests with
  `chrono-tz` could not provide.
- Unit: the existing fall-back tests, plus the same cases run through
  `Local` with `TZ=America/New_York` set for the test process.

### P4-2. Adaptive `-v` on a short wait prints "Wait complete." twice

Reproduce: `dozr d 300ms -v` prints the completion line twice and no
countdown.

Cause: `adaptive_verbose_wait_with` in `src/progress.rs` rounds the
remaining time to whole seconds and passes zero to the display function
while time remains; the display function treats zero as completion.

Fix: never pass zero while time remains; show `1s` for anything under 1.5 s,
or tenths as the fixed-period mode does.

Tests:

- Unit, on the fake clock: the exact display sequence for a 300 ms adaptive
  wait is one countdown line and one completion line.
- Tighten `test_time_align_verbose_output` and
  `test_clamped_sample_is_reported_when_verbose` in `tests/cli.rs` to assert
  exactly one "Wait complete." line.

### P4-3. The unit hint recommends a value the parser rejects

Reproduce: `dozr d 0.1` says "a unit is required, for example 0.1s or
0.1ms", and `dozr d 0.1s` is then rejected with "invalid character at 1",
because `humantime` only accepts whole numbers per unit.

Fix: for an integer, suggest `5s or 5ms`; for a decimal, convert to whole
milliseconds and suggest that (`0.1` becomes `100ms`); for anything else,
say that a whole number of units is required. Correct the changelog example.

Tests:

- Unit: the hint for `0.1`, `2`, `1e3`, and `inf`.
- Integration: following the hint for `0.1` is accepted.

### P4-4. `align` boundaries are not local wall-clock boundaries

Reproduce: `TZ=Asia/Kolkata dozr a 1h --dry-run` lands at half past the
hour, and `dozr a 1d` lands at UTC midnight in every zone.

Decide between two fixes: measure boundaries from local midnight, which
makes `a 1h` land on the hour everywhere and `a 1d` land at local midnight;
or keep the epoch and say so in the README, help text, and `examples.md`.
Measuring from local midnight is recommended, since every example in the
docs assumes it.

Tests:

- Unit: alignment to `1h` and `1d` at a fixed instant in a half-hour zone
  (`Asia/Kolkata`) and across a DST change, with the clock injected.
- Integration: `TZ=Asia/Kolkata dozr a 1h --dry-run` plus the current local
  time lands on a whole hour.

Also in this item: the README's "seconds only" claim about `sleep`, and the
"one sleep call" sentence, which holds only without `-v`.

### P4-5. `scripts/check_examples.sh` can pass a broken command

Reproduce: a file containing `$ dozr d 5x`, a ```` ```sh ```` fence, an
inline `` `dozr d 5x` `` in prose, and `dozr d 5x -p 0.2 --skip-exit-code 2`
reports "0 failed", because those forms are not extracted and an allowed
skip status of 1 or 2 masks real errors.

Fix: extract every backticked `dozr …` from any line and every `dozr …` or
`$ dozr …` line in `bash` and `sh` fences; accept the documented skip status
only when the dry-run output is exactly `0s`.

Tests:

- Keep the planted file under `scripts/testdata/` and add a CI step that
  asserts the script fails on it. The real docs must still pass.

### P4-6. A runtime error with a closed stderr panics

Reproduce: `dozr u 1000000000s 1000000000s1ns --dry-run 2>&1 >/dev/null |
head -c 0` exits 101, because `eprintln!` in `src/main.rs` panics on a
broken pipe.

Fix: write the error with `let _ = writeln!(std::io::stderr(), …)`.

Tests: integration, the command above exits 1.

### P4-7. Two chance-based tests

`test_jitter_applies_with_probability` and `test_jitter_applies_to_align`
in `tests/cli.rs` fail by chance about 0.03% of runs each.

Fix: drive both with `--seed` and `--dry-run`, as the P3 tests already do,
so they are deterministic and no longer sleep.

### P4-8. Absurd-input edges

- Uniform bounds that differ by a nanosecond at a billion seconds pass
  `Duration` validation but collide as `f64` and fail at runtime with exit 1.
  Validate on the converted `f64` values.
- `a 1000y` wraps in the `u128` to `u64` cast in `duration_to_next_alignment`.
  Saturate, as `secs_to_duration` does.
- `-v 1s` shows "0s" half a second before the end. Round up for display.
- `at` for the current second waits a day because `now` has sub-second
  precision. Compare at second granularity, or document.
- Add doc comments to the builder methods, `Skip` fields, sleeper fields,
  `RunOptions`, and `SystemClock::new`. Tag 0.1.0 to 0.4.1 at the commits
  that set those versions, or drop their changelog links.

Tests: one unit test per bullet where behaviour changes.

### P4-9. Duplicated validation

Five CLI value parsers in `src/cli.rs` (`parse_finite`, `parse_positive`,
`parse_probability`, `parse_positive_duration`, `parse_align_interval`)
check rules that `Wait::validate` checks again, about 45 lines. `run`
already exits with status 2 and an error naming the argument when
`validate` fails. Remove the five parsers and keep `parse_time` and
`parse_verbose`, which exist for typing rather than validation.

Tests: the existing `test_invalid_parameters_exit_2_and_name_the_argument`
must still pass unchanged; it is the contract.
