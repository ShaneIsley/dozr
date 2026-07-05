# dozr — Repository Review (v0.4.1)

Scope: architecture, technical correctness, code quality, usability, tests, and tooling.
All behavioral findings below were reproduced against a debug build of the current `main`.

---

## Summary

dozr is a well-scoped, cleanly organized small utility. The library/binary split, the
`WaitCondition` trait, subcommand-based CLI, CHANGELOG discipline, benchmarks, and examples
are all above average for a crate this size. The main problems are:

1. **Two confirmed runtime bugs** in the fixed-period verbose path (overshoot + wrong
   "Wait complete" output).
2. **Global flags that silently do nothing** (`--probability` outside `duration`,
   `--jitter` with `--probability` or with `align`/`at`) — the CLI accepts them and
   quietly ignores them.
3. **Heavy structural duplication** in `src/conditions.rs` (7 near-identical distribution
   impls) and between the two verbose-wait loops in `src/lib.rs`.
4. A leaky `1ns` sentinel encoding for "adaptive verbose" that crosses module boundaries.

---

## Confirmed bugs

### B1. Fixed-period verbose wait overshoots the total wait — `src/lib.rs:50-54`

```rust
let next_update_time = elapsed + update_period;
let sleep_duration = next_update_time.saturating_sub(elapsed);
```

`sleep_duration` always equals `update_period` (the two lines cancel out), and it is never
capped to the remaining time. The final sleep therefore runs a full update period past the
deadline.

Repro: `dozr d 200ms -v 2s` sleeps **2.0s** instead of ~0.2s (measured). A
`dozr d 61s -v 1m` would take ~2 minutes.

Fix: `let sleep_duration = update_period.min(remaining);` (and the dead
`next_update_time` arithmetic can be deleted, along with the unreachable
`else` branches at `src/lib.rs:55-62`).

### B2. Premature and duplicated "Wait complete" for sub-second remainders — `src/lib.rs:37-47`, `src/conditions.rs:23-30`

The ETA is rounded to whole seconds (`eta.round() as u64`) before being handed to the
display closure, and the closure prints "Wait complete." whenever the value is zero. With
<500ms remaining the rounded ETA is 0, so the tool prints "Wait complete." while it is
still waiting — and then prints it a second time when the wait actually finishes.

Repro (same command as B1):

```
[09:07:36] Wait complete.
[09:07:38] Wait complete.
```

Fix: signal completion explicitly (e.g. pass an enum or a separate `on_complete`
callback) instead of overloading `Duration::ZERO`, or display sub-second remainders as
such instead of rounding.

### B3. `--probability` is silently ignored for every command except `duration` — `src/cli.rs:142-208`

`-p` is declared `global = true`, but `into_wait_condition` only consults it in the
`Commands::Duration` arm. Confirmed: `dozr n 2s 0.001 -p 0.0` still sleeps ~2s. A user
who writes `dozr e 0.5 -p 0.25` in a cron job gets a 100% wait probability with no
warning.

Fix: either apply probability uniformly (wrap any condition in a probabilistic
decorator — the trait design already supports composition) or reject the flag for
unsupported commands via clap `conflicts_with`.

### B4. `--jitter` is silently dropped in two situations

* `ProbabilisticWait` has no jitter field, so `dozr d 10s -p 0.9 -j 2s` silently loses
  the jitter (`src/cli.rs:144-149`). Confirmed.
* `align` and `at` accept `-j` (it's global) and ignore it. The README documents this
  ("Not applicable to align or at") but the tool should say so at runtime — silently
  accepting an option is how misconfigured scripts go unnoticed.

Fix: add jitter to `ProbabilisticWait`, and declare conflicts in clap so `align`/`at`
reject `-j` loudly.

### B5. No range validation on `--probability` — `src/cli.rs:45-47`

`dozr d 0s -p 5.0` is accepted (confirmed); negative values parse too (clap's `-p=-1`
form). Anything >1 behaves as "always", anything <0 as "never", without feedback.

Fix: `#[arg(value_parser = clap::value_parser!(f64), ...)]` plus a custom parser or
`Probability` newtype rejecting values outside `[0.0, 1.0]`.

### B6. Possible panic in `parse_time_until` on DST transitions — `src/cli.rs:12-17`

`now.with_hour(...)` on a `DateTime<Local>` returns `None` when the resulting local time
does not exist (spring-forward gap) or is ambiguous, and the code `unwrap()`s with a
comment claiming safety. `dozr at 02:30` run on the morning of a spring-forward day in a
DST timezone panics. Related: `target_datetime + ChronoDuration::days(1)` adds exactly
24h of absolute time, so a roll-to-tomorrow that crosses a DST boundary lands an hour off
the requested wall-clock time.

Fix: build the target with `Local.with_ymd_and_hms(...)` / `NaiveDate::and_time` +
`and_local_timezone`, handling the `LocalResult::None/Ambiguous` cases, and roll dates
with `checked_add_days` on the naive date before resolving the timezone.

### B7. Log-normal parameters are not what the docs say — `src/conditions.rs:143-151`

`LogNormal::new(mu, sigma)` takes the mean/std-dev of the **underlying normal in
log-space**, but the CLI feeds it `mean.as_secs_f64()` from a humantime Duration and the
help text calls it "Mean of the Log-Normal distribution (e.g., 1s)". `dozr ln 1s 0.5`
actually produces a distribution with median e¹ ≈ 2.72s and mean ≈ 3.08s — not 1s.

Fix: either document the parameters as μ/σ of the log, or (better UX) accept the desired
distribution mean/std-dev and derive μ = ln(m²/√(m²+v)), σ² = ln(1+v/m²).

---

## Architecture

**What's good**

* Library-first design (`src/lib.rs` + thin `src/main.rs`) makes the logic reusable and
  testable; examples and benches exercise the library API directly.
* `WaitCondition` trait gives a clean seam; `JitterGenerator` is a textbook
  dependency-injection seam with a mock in tests.
* Subcommand CLI with aliases is the right structure for this feature set.

**Issues**

* **A1 — Seven near-identical distribution impls** (`src/conditions.rs:87-259`). Every
  distribution struct repeats the same body: build distribution → sample → clamp →
  jitter → wrap. This is ~170 lines that could be one generic
  `DistributionWait<D: Distribution<f64>>` (or a small macro), with `wait()` provided
  once as a default trait method:

  ```rust
  fn wait(&self) -> Result<()> {
      perform_wait(self.calculate_wait_duration()?, self.verbose());
      Ok(())
  }
  ```

  Today every new distribution costs ~35 copy-pasted lines and re-fixes of any bug in
  the pattern.

* **A2 — The `1ns` sentinel for adaptive verbose leaks across three modules.**
  `cli.rs` sets `default_missing_value = "1ns"`, `conditions.rs:22` re-detects it with
  `update_period.as_nanos() == 1`, and a user who legitimately passes `-v 1ns` silently
  gets adaptive mode. Meanwhile `Cli::is_adaptive_verbose()`/`verbose_period()`
  (`src/cli.rs:212-228`) encode the same knowledge but are **dead code** — nothing in the
  runtime path calls them; only tests do. Model it explicitly:

  ```rust
  enum Verbosity { Off, Adaptive, Fixed(Duration) }
  ```

* **A3 — `verbose_wait` and `adaptive_verbose_wait` are ~90% identical**
  (`src/lib.rs:26-128`). One loop parameterized by a `next_period(remaining)` function
  covers both; the fixed variant is just a constant function. This would also have
  prevented bug B1, which was fixed in the adaptive loop's logic but not the fixed one.

* **A4 — `ProbabilisticWait` rolls the dice twice** (`src/conditions.rs:317-344`):
  `calculate_wait_duration()` and `wait()` each draw independently, so the two methods
  can disagree about whether a wait happens. `wait()` should use the result of
  `calculate_wait_duration()` like every other impl.

* **A5 — `dist_sampler` ships to end users.** It's a `[[bin]]` in Cargo.toml, so
  `cargo install dozr` installs a second `dist_sampler` executable into the user's PATH.
  It looks like an internal analysis tool for `scripts/analyze_distributions.py`. Move it
  to `examples/` or gate it (`required-features = ["dev-tools"]`). Internally it also
  uses a stringly-typed `--distribution normal` + 13 optional flags with `.expect()`
  panics for missing combinations — subcommands would validate this for free.

* **A6 — `at` resolves the wall-clock time at argument-parse time** (`src/cli.rs:130`
  via `value_parser = parse_time_until`). Parsing and semantics are entangled: the
  duration is baked in before the app "runs", `--help` behavior aside, and any future
  re-parse (e.g. config files, retries) would silently recompute. Parse to a `NaiveTime`
  and compute the duration in `into_wait_condition`.

---

## Code quality

* **Q1 — 56 clippy warnings** on `--all-targets`: 35 `needless_borrows_for_generic_args`
  (`cmd.args(&[...])` → `cmd.args([...])`), 19 `redundant_field_names`
  (`verbose: verbose`), 2 `map_or` simplifications. All mechanical:
  `cargo clippy --fix` clears nearly everything. Clippy is not in CI, which is how these
  accumulated.
* **Q2 — Unused dev-dependency**: `mockall = "0.12.1"` appears nowhere in the code (the
  mock in `conditions.rs` tests is hand-rolled). Drop it. Relatedly `TEST.md` claims
  `src/lib.rs` is "tested with mock objects to verify the correct WaitCondition is
  created" — no such tests exist; the lib tests actually sleep.
* **Q3 — The test suite really sleeps.** `tests/cli.rs` contains a 21-second test
  (`test_verbose_adaptive_long_wait`), a 5s, several 1–2s, and the lib tests sleep ~6s
  more. Full `cargo test` costs ~45–60s of pure sleeping, and the timing assertions
  (`elapsed < 2s` for a Normal(1s, 0.1) sample) are probabilistic — a 10σ event is
  "never", but CI-runner clock skew is not. Consider a `Sleeper` trait (injected like
  `JitterGenerator`) so unit tests assert on the *computed* duration instead of wall
  time, keeping only a couple of smoke tests that actually sleep.
* **Q4 — `test_jitter_adds_time` doesn't test jitter** (`tests/cli.rs:28-42`): it
  asserts `elapsed >= 100ms`, which the base duration alone guarantees; the jitter could
  be entirely broken (always zero) and this test would pass.
* **Q5 — Duplicated jitter boilerplate**: every distribution's
  `calculate_wait_duration` re-creates a `ThreadRng` and a `RandomJitterGenerator`;
  collapses naturally with A1. (Also, rand 0.9 idiom is `rand::rng()`.)

---

## Usability

* **U1 — The most common case is wordier than `sleep`.** `dozr d 5s` vs `sleep 5`. For a
  "sleep replacement", consider making a bare duration work: `dozr 5s`. Clap supports
  this pattern (optional positional that conflicts with subcommands), keeping all
  subcommands intact.
* **U2 — Silent no-op flags are the biggest UX trap** (B3/B4 above): the tool accepts
  `-p`/`-j` combinations it does not honor. In a tool designed for scripts and cron,
  errors beat silence.
* **U3 — README oversells "progress bar".** Features list says "Display a progress bar
  with the time remaining" — output is a timestamped countdown line, not a bar. Small,
  but it's the first thing a user checks.
* **U4 — Inconsistent parameter types across commands.** `normal`/`log-normal`/`uniform`
  take humantime Durations (`1s`, `500ms`); `triangular`, `pareto`, `gamma` take raw
  float seconds; `normal`'s std-dev is a bare float (seconds) next to a Duration mean.
  `dozr t 0 10 5` and `dozr u 1s 10s` shouldn't require the user to remember which style
  each command wants. Recommend accepting Durations everywhere a time quantity is meant.
* **U5 — `exponential` takes λ, not a mean.** `dozr e 0.5` = mean 2s requires the user
  to do the reciprocal in their head. `dozr e 2s` (mean wait) would match the tool's
  Duration-first ergonomics; keep `--lambda` for those who think in rates.
* **U6 — Sub-second verbose display rounds to whole seconds** ("Time remaining: 1s" for
  500ms). Cosmetic; falls out of fixing B2.
* **U7 — Negative floats fail with a confusing error**: `dozr e -0.5` reports
  "unexpected argument '-0' found" (clap tries to parse it as a flag) instead of "lambda
  must be positive". `allow_negative_numbers = true` + explicit validation gives a real
  message. (The existing test at `tests/cli.rs:382` locks in the confusing behavior.)

---

## CI / tooling

* **C1 — CI only builds and tests** (`.github/workflows/rust.yml`). Recommended
  additions, in priority order: `cargo clippy --all-targets -- -D warnings`,
  `cargo fmt --check`, a job pinned to the declared MSRV (1.85.0 — currently asserted in
  Cargo.toml but never verified), `Swatinem/rust-cache` (build is repeated from scratch),
  and `cargo test --locked`.
* **C2 — No release automation** despite crates.io publishing; a tag-triggered
  `cargo publish` workflow (or `release-plz`) would keep CHANGELOG/tags/crates.io in
  sync.
* **C3 — Nice touches already present**: Keep-a-Changelog discipline, MSRV declared,
  keywords/categories set, benches and examples wired up. Add a CI status badge to the
  README next to the crates.io badges.

---

## Prioritized recommendations

| # | Action | Findings addressed |
|---|--------|--------------------|
| 1 | Fix fixed-period verbose loop (cap sleep to remaining; fix completion signal) | B1, B2, U6 |
| 2 | Make `-p`/`-j` either work or error on every command; validate `-p` range | B3, B4, B5, U2 |
| 3 | Replace the `1ns` sentinel with a `Verbosity` enum; delete dead helpers | A2 |
| 4 | Collapse the 7 distribution impls into one generic + default `wait()` | A1, Q5 |
| 5 | Merge the two verbose loops | A3, prevents B1-class regressions |
| 6 | Fix DST handling in `parse_time_until`; move time resolution out of the parser | B6, A6 |
| 7 | Correct log-normal parameter semantics (or docs) | B7 |
| 8 | Add clippy + fmt + MSRV to CI; run `cargo clippy --fix`; drop `mockall` | Q1, Q2, C1 |
| 9 | Stop installing `dist_sampler` with the crate | A5 |
| 10 | Inject a `Sleeper` to de-flake and speed up the test suite | Q3, Q4 |
| 11 | Ergonomics: bare `dozr 5s`, Durations everywhere, mean-based exponential | U1, U4, U5 |
