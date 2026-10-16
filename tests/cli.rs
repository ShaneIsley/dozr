//! End-to-end tests that run the `dozr` binary.
//!
//! Keep every test's total sleeping under 2 seconds so the suite stays fast.

use assert_cmd::Command;
use predicates::prelude::{Predicate, PredicateBooleanExt};
use predicates::str;
use std::process::Output;
use std::time::{Duration, Instant};

/// Upper bound on any single dozr run, so a regression fails instead of hanging.
const RUN_TIMEOUT: Duration = Duration::from_secs(15);

fn dozr() -> Command {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.timeout(RUN_TIMEOUT);
    cmd
}

/// Runs dozr with `args`, returning how long it took and its output.
fn timed(args: &[&str]) -> (Duration, Output) {
    let start = Instant::now();
    let output = dozr().args(args).output().unwrap();
    (start.elapsed(), output)
}

/// Wall-clock time `offset` from now, formatted for the `at` subcommand.
fn clock_in(offset: chrono::Duration, format: &str) -> String {
    (chrono::Local::now() + offset).format(format).to_string()
}

/// Arguments for each subcommand that would wait for several seconds or more.
fn long_wait_commands() -> Vec<Vec<String>> {
    let at = clock_in(chrono::Duration::hours(1), "%H:%M");
    [
        vec!["d", "5s"],
        vec!["n", "5s", "100ms"],
        vec!["e", "0.01"],
        vec!["ln", "5s", "100ms"],
        vec!["par", "5s", "2"],
        vec!["u", "5s", "6s"],
        vec!["t", "5s", "6s", "5500ms"],
        vec!["g", "50", "200ms"],
        vec!["a", "1h"],
        vec!["at", at.as_str()],
    ]
    .into_iter()
    .map(|v| v.into_iter().map(String::from).collect())
    .collect()
}

/// Arguments for each subcommand that wait for at most about two seconds.
///
/// The `at` target is computed lazily, when the iterator reaches it, so the
/// commands run before it in a loop cannot push it into the past.
fn short_wait_commands() -> impl Iterator<Item = Vec<String>> {
    [
        vec!["d", "10ms"],
        vec!["n", "10ms", "1ms"],
        vec!["e", "100"],
        vec!["ln", "10ms", "1ms"],
        vec!["par", "10ms", "5"],
        vec!["u", "1ms", "10ms"],
        vec!["t", "0s", "20ms", "10ms"],
        vec!["g", "2", "5ms"],
        vec!["a", "100ms"],
    ]
    .into_iter()
    .map(|v| v.into_iter().map(String::from).collect())
    .chain(std::iter::once_with(|| {
        vec![
            "at".to_string(),
            clock_in(chrono::Duration::seconds(2), "%H:%M:%S"),
        ]
    }))
}

fn with_args(base: &[String], extra: &[&str]) -> Vec<String> {
    base.iter()
        .cloned()
        .chain(extra.iter().map(|s| s.to_string()))
        .collect()
}

fn as_strs(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
}

// ---------------------------------------------------------------------------
// Basic behaviour
// ---------------------------------------------------------------------------

#[test]
fn test_duration_is_valid() {
    dozr().args(["d", "10ms"]).assert().success();
}

#[test]
fn test_jitter_flag_accepts_argument() {
    dozr()
        .args(["d", "0s", "--jitter", "0s"])
        .assert()
        .success();
}

#[test]
fn test_jitter_adds_time() {
    let (elapsed, output) = timed(&["d", "100ms", "--jitter", "200ms"]);
    assert!(output.status.success());
    assert!(elapsed >= Duration::from_millis(100));
    assert!(elapsed <= Duration::from_millis(1_500));
}

#[test]
fn test_verbose_output_includes_eta() {
    dozr()
        .args(["d", "1500ms", "-v"])
        .assert()
        .success()
        .stderr(str::contains("[DOZR] Time remaining:").and(str::contains("Wait complete.")));
}

#[test]
fn test_verbose_custom_update_period() {
    dozr()
        .args(["d", "1s500ms", "-v", "500ms"])
        .assert()
        .success()
        .stderr(str::contains("[DOZR] Time remaining:"));
}

#[test]
fn test_verbose_adaptive_short_wait() {
    let (elapsed, output) = timed(&["d", "2s", "-v"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success());
    assert!(
        stderr.matches("[DOZR] Time remaining:").count() >= 2,
        "{stderr}"
    );
    assert!(elapsed < Duration::from_millis(2_500));
}

#[test]
fn test_time_align_verbose_output() {
    dozr()
        .args(["a", "1s", "-v"])
        .assert()
        .success()
        .stderr(str::contains("Wait complete."));
}

#[test]
fn test_until_time_verbose_output() {
    let target = clock_in(chrono::Duration::seconds(2), "%H:%M:%S");
    dozr()
        .args(["at", &target, "-v"])
        .assert()
        .success()
        .stderr(str::contains("[DOZR] Time remaining:").and(str::contains("Wait complete.")));
}

#[test]
fn test_until_time_in_future() {
    let target = clock_in(chrono::Duration::seconds(2), "%H:%M:%S");
    let (elapsed, output) = timed(&["at", &target]);
    assert!(output.status.success());
    // HH:MM:SS truncates sub-second precision, so the wait is 1-2 seconds.
    assert!(elapsed >= Duration::from_millis(900), "{elapsed:?}");
    assert!(elapsed <= Duration::from_secs(3), "{elapsed:?}");
}

#[test]
fn test_parse_time_until_hh_mm() {
    // An HH:MM target would make this test wait minutes, so skip the wait
    // with -p 0 and only check that the format is accepted.
    let target = clock_in(chrono::Duration::hours(1), "%H:%M");
    dozr().args(["at", &target, "-p", "0"]).assert().success();
}

#[test]
fn test_distribution_waits_complete_quickly() {
    for args in short_wait_commands() {
        let (elapsed, output) = timed(&as_strs(&args));
        assert!(output.status.success(), "{args:?} failed: {output:?}");
        assert!(
            elapsed < Duration::from_millis(2_500),
            "{args:?} took {elapsed:?}"
        );
    }
}

#[test]
fn test_triangular_distribution_wait_time() {
    let (elapsed, output) = timed(&["t", "100ms", "500ms", "200ms"]);
    assert!(output.status.success());
    assert!(elapsed >= Duration::from_millis(100));
    assert!(elapsed < Duration::from_secs(1));
}

#[test]
fn test_normal_distribution_wait_time() {
    let (elapsed, output) = timed(&["n", "200ms", "10ms"]);
    assert!(output.status.success());
    assert!(elapsed >= Duration::from_millis(150));
    assert!(elapsed < Duration::from_secs(1));
}

// ---------------------------------------------------------------------------
// Argument errors
// ---------------------------------------------------------------------------

#[test]
fn test_invalid_duration_arg() {
    dozr()
        .args(["d", "invalid-duration"])
        .assert()
        .failure()
        .stderr(str::contains("error: invalid value"));
}

#[test]
fn test_invalid_jitter_arg() {
    dozr()
        .args(["d", "1s", "-j", "invalid-jitter"])
        .assert()
        .failure()
        .stderr(str::contains("error: invalid value"));
}

#[test]
fn test_invalid_verbose_period_arg() {
    dozr()
        .args(["d", "1s", "-v", "invalid-period"])
        .assert()
        .failure()
        .stderr(str::contains("error: invalid value"));
}

#[test]
fn test_duration_and_align_are_mutually_exclusive() {
    dozr()
        .args(["d", "1s", "a", "5s"])
        .assert()
        .failure()
        .stderr(str::contains("error: unexpected argument 'a' found"));
}

#[test]
fn test_mutually_exclusive_distribution_args() {
    dozr()
        .args(["d", "1s", "n", "1s", "100ms"])
        .assert()
        .failure()
        .stderr(str::contains("error: unexpected argument 'n' found"));
}

#[test]
fn test_duration_or_align_is_required() {
    dozr()
        .assert()
        .failure()
        .stderr(str::contains("Usage: dozr [OPTIONS] <COMMAND>"));
}

#[test]
fn test_normal_distribution_missing_param() {
    dozr()
        .args(["n", "1s"])
        .assert()
        .failure()
        .stderr(str::contains("required arguments were not provided"));
}

#[test]
fn test_normal_distribution_missing_all_params() {
    dozr()
        .args(["n"])
        .assert()
        .failure()
        .stderr(str::contains("required arguments were not provided"));
}

#[test]
fn test_invalid_until_time_format() {
    dozr()
        .args(["at", "invalid-time"])
        .assert()
        .failure()
        .stderr(str::contains("Invalid time format"));
}

#[test]
fn test_invalid_until_time_hour() {
    dozr()
        .args(["at", "25:00"])
        .assert()
        .failure()
        .stderr(str::contains("Invalid time format"));
}

#[test]
fn test_invalid_until_time_minute() {
    dozr()
        .args(["at", "10:65"])
        .assert()
        .failure()
        .stderr(str::contains("Invalid time format"));
}

// ---------------------------------------------------------------------------
// P0-1: a fixed verbose period longer than the wait must not extend it
// ---------------------------------------------------------------------------

#[test]
fn test_verbose_period_longer_than_wait_does_not_overshoot() {
    let (elapsed, output) = timed(&["d", "200ms", "-v", "10s"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Wait complete."));
    assert!(elapsed < Duration::from_secs(1), "took {elapsed:?}");
}

// ---------------------------------------------------------------------------
// P0-2: --probability applies to every subcommand
// ---------------------------------------------------------------------------

#[test]
fn test_probability_zero_skips_every_subcommand() {
    for args in long_wait_commands() {
        let args = with_args(&args, &["-p", "0.0", "-v"]);
        let (elapsed, output) = timed(&as_strs(&args));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{args:?} failed: {stderr}");
        assert!(
            elapsed < Duration::from_secs(1),
            "{args:?} took {elapsed:?}; probability was ignored"
        );
        assert!(stderr.contains("Skipping sleep"), "{args:?}: {stderr}");
    }
}

#[test]
fn test_probability_one_waits_for_every_subcommand() {
    for args in short_wait_commands() {
        let args = with_args(&args, &["-p", "1.0", "-v"]);
        let output = dozr().args(&args).output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{args:?} failed: {stderr}");
        assert!(stderr.contains("Wait complete."), "{args:?}: {stderr}");
        assert!(!stderr.contains("Skipping sleep"), "{args:?}: {stderr}");
    }
}

#[test]
fn test_probabilistic_wait_skip_verbose_output() {
    dozr()
        .args(["d", "1s", "-p", "0.0", "-v"])
        .assert()
        .success()
        .stderr(str::contains("Skipping sleep"));
}

// ---------------------------------------------------------------------------
// P0-3: --jitter still applies when --probability is given
// ---------------------------------------------------------------------------

#[test]
fn test_jitter_applies_with_probability() {
    // Each run has a 1-in-5 chance of drawing under 100ms of jitter, so the
    // chance that all five do is 0.2^5, about 0.03%.
    let longest = (0..5)
        .map(|_| {
            let (elapsed, output) = timed(&["d", "10ms", "-j", "500ms", "-p", "1.0"]);
            assert!(output.status.success());
            elapsed
        })
        .max()
        .unwrap();
    assert!(
        longest > Duration::from_millis(110),
        "longest run was {longest:?}; jitter appears to be dropped"
    );
}

// ---------------------------------------------------------------------------
// P0-5: invalid parameters are rejected at parse time with exit code 2
// ---------------------------------------------------------------------------

#[test]
fn test_invalid_parameters_exit_2_and_name_the_argument() {
    let cases: &[(&[&str], &str)] = &[
        (&["d", "0s", "-p", "7"], "--probability"),
        (&["d", "0s", "-p", "-0.1"], "--probability"),
        (&["ln", "0s", "100ms"], "<MEAN>"),
        (&["e", "0"], "<LAMBDA>"),
        (&["e", "-0.5"], "<LAMBDA>"),
        (&["par", "0s", "1"], "<SCALE>"),
        (&["par", "1s", "0"], "<SHAPE>"),
        (&["g", "0", "1s"], "<SHAPE>"),
        (&["g", "1", "0s"], "<SCALE>"),
        (&["u", "5s", "1s"], "<MIN>"),
        (&["u", "1s", "1s"], "<MIN>"),
        (&["t", "0s", "1s", "2s"], "<MODE>"),
        (&["t", "1s", "1s", "1s"], "<MIN>"),
        (&["a", "0s"], "<INTERVAL>"),
        (&["d", "1s", "-v", "1ns"], "--verbose"),
        (&["d", "1s", "-v", "0s"], "--verbose"),
    ];
    for (args, names) in cases {
        let output = dozr().args(*args).output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "{args:?}: {stderr}");
        assert!(stderr.contains("error:"), "{args:?}: {stderr}");
        assert!(
            stderr.contains(names),
            "{args:?} should name {names}: {stderr}"
        );
    }
}

#[test]
fn test_boundary_parameters_are_accepted() {
    let cases: &[&[&str]] = &[
        &["d", "10ms", "-p", "0.0"],
        &["d", "10ms", "-p", "1.0"],
        &["n", "10ms", "0s"],
        &["ln", "10ms", "0s"],
        &["t", "0s", "10ms", "0s"],
        &["t", "0s", "10ms", "10ms"],
        &["d", "10ms", "-v", "1ms"],
    ];
    for args in cases {
        dozr().args(*args).assert().success();
    }
}

#[test]
fn test_skip_message_is_only_shown_when_verbose() {
    let output = dozr().args(["d", "1s", "-p", "0"]).output().unwrap();
    assert!(output.status.success());
    assert!(!str::contains("Skipping sleep").eval(&String::from_utf8_lossy(&output.stderr)));
}

// ---------------------------------------------------------------------------
// P1-2: sub-second verbose periods produce sub-second updates
// ---------------------------------------------------------------------------

fn count_updates(stderr: &[u8]) -> usize {
    String::from_utf8_lossy(stderr)
        .matches("[DOZR] Time remaining:")
        .count()
}

#[test]
fn test_sub_second_verbose_period_updates_every_tick() {
    // 1.5s at 250ms is six ticks; allow for scheduling delay under load.
    let output = dozr()
        .args(["d", "1500ms", "-v", "250ms"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let updates = count_updates(&output.stderr);
    assert!(updates >= 4, "only {updates} updates");
    // Sub-second periods show tenths of a second.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Time remaining: 1.5s"), "{stderr}");
}

#[test]
fn test_whole_second_verbose_period_does_not_spam() {
    let output = dozr().args(["d", "2s", "-v", "1s"]).output().unwrap();
    assert!(output.status.success());
    let updates = count_updates(&output.stderr);
    assert!((2..=3).contains(&updates), "{updates} updates");
}

// ---------------------------------------------------------------------------
// P1-3: lengths of time need units
// ---------------------------------------------------------------------------

#[test]
fn test_bare_numbers_for_time_arguments_exit_2() {
    let cases: &[(&[&str], &str)] = &[
        (&["n", "1s", "0.1"], "<STD_DEV>"),
        (&["ln", "1s", "0.1"], "<STD_DEV>"),
        (&["par", "1", "2"], "<SCALE>"),
        (&["u", "1", "2s"], "<MIN>"),
        (&["t", "1", "3s", "2s"], "<MIN>"),
        (&["g", "2", "1.5"], "<SCALE>"),
    ];
    for (args, names) in cases {
        let output = dozr().args(*args).output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "{args:?}: {stderr}");
        assert!(
            stderr.contains(names),
            "{args:?} should name {names}: {stderr}"
        );
        assert!(stderr.contains("a unit is required"), "{args:?}: {stderr}");
    }
}

#[test]
fn test_help_documents_units() {
    for (sub, expected) in [
        ("normal", "Standard deviation (e.g., \"100ms\")"),
        ("exponential", "events per second"),
        ("pareto", "Minimum wait"),
        ("triangular", "Most likely wait"),
        ("gamma", "Scale (e.g., \"1s\")"),
    ] {
        dozr()
            .args([sub, "--help"])
            .assert()
            .success()
            .stdout(str::contains(expected));
    }
}

// ---------------------------------------------------------------------------
// P1-4: --verbose forms
// ---------------------------------------------------------------------------

#[test]
fn test_verbose_flag_before_subcommand() {
    dozr()
        .args(["-v", "250ms", "d", "300ms"])
        .assert()
        .success()
        .stderr(str::contains("Time remaining: 0.3s"));
}

// ---------------------------------------------------------------------------
// P1-5: clamped samples are reported in verbose mode
// ---------------------------------------------------------------------------

#[test]
fn test_clamped_sample_is_reported_when_verbose() {
    // A normal with mean 0 is negative half the time, so twenty runs all
    // missing it has a 1-in-a-million chance.
    let note = "clamped to zero";
    let mut saw_note = false;
    for _ in 0..20 {
        let output = dozr().args(["n", "0s", "10ms", "-v"]).output().unwrap();
        assert!(output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains(note) {
            assert!(stderr.contains("Wait complete."), "{stderr}");
            saw_note = true;
            break;
        }
    }
    assert!(saw_note, "no run reported a clamped sample");
}

#[test]
fn test_clamp_note_is_silent_without_verbose() {
    for _ in 0..10 {
        let output = dozr().args(["n", "0s", "10ms"]).output().unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty(), "{output:?}");
    }
}

// ---------------------------------------------------------------------------
// P2-2: extreme samples saturate instead of panicking; jitter applies to
// every command, including align and at
// ---------------------------------------------------------------------------

#[test]
fn test_extreme_parameters_do_not_crash() {
    // These used to panic within milliseconds converting an enormous sample
    // to a Duration. Now they wait (effectively forever), so check the
    // process is still alive after a moment, then stop it.
    for args in [
        &["e", "1e-30"][..],
        &["par", "1s", "0.001"],
        &["g", "1e300", "1s"],
    ] {
        let mut child = std::process::Command::new(assert_cmd::cargo::cargo_bin("dozr"))
            .args(args)
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_millis(300));
        let status = child.try_wait().unwrap();
        child.kill().ok();
        child.wait().ok();
        assert!(status.is_none(), "{args:?} exited early with {status:?}");
    }
}

#[test]
fn test_jitter_applies_to_align() {
    // `a 10ms` alone never waits more than 10ms. With up to 300ms of
    // jitter, the chance that seven runs all stay under 110ms is (1/3)^7,
    // about 0.05%.
    let longest = (0..7)
        .map(|_| {
            let (elapsed, output) = timed(&["a", "10ms", "-j", "300ms"]);
            assert!(output.status.success());
            elapsed
        })
        .max()
        .unwrap();
    assert!(
        longest > Duration::from_millis(110),
        "longest run was {longest:?}; jitter appears to be ignored for align"
    );
}

// ---------------------------------------------------------------------------
// P3: --dry-run, --seed, --skip-exit-code, --min and --max
// ---------------------------------------------------------------------------

/// Runs dozr with `args` plus `--dry-run`, returning exit status and stdout.
fn dry_run(args: &[&str]) -> (Option<i32>, String) {
    let output = dozr().args(args).arg("--dry-run").output().unwrap();
    (
        output.status.code(),
        String::from_utf8(output.stdout).unwrap(),
    )
}

#[test]
fn test_dry_run_prints_duration_without_waiting() {
    let (elapsed, output) = timed(&["d", "1h", "--dry-run"]);
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "1h\n");
    assert!(output.stderr.is_empty());
    assert!(elapsed < Duration::from_secs(1), "took {elapsed:?}");
}

#[test]
fn test_dry_run_works_for_every_subcommand() {
    for args in long_wait_commands() {
        let args = with_args(&args, &["--dry-run"]);
        let (elapsed, output) = timed(&as_strs(&args));
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success(), "{args:?}: {output:?}");
        assert!(
            elapsed < Duration::from_secs(1),
            "{args:?} took {elapsed:?}"
        );
        assert_eq!(stdout.lines().count(), 1, "{args:?}: {stdout:?}");
        let d = humantime::parse_duration(stdout.trim())
            .unwrap_or_else(|e| panic!("{args:?}: {stdout:?}: {e}"));

        // The printed duration is accepted back by `dozr d` unchanged.
        let (code, again) = dry_run(&["d", stdout.trim()]);
        assert_eq!(code, Some(0));
        assert_eq!(humantime::parse_duration(again.trim()).unwrap(), d);
    }
}

#[test]
fn test_dry_run_with_verbose_has_notes_but_no_countdown() {
    let output = dozr()
        .args(["d", "5s", "-p", "0", "-v", "--dry-run"])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&output.stdout), "0s\n");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Skipping sleep"), "{stderr}");

    let output = dozr()
        .args(["d", "5s", "-v", "--dry-run"])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&output.stdout), "5s\n");
    assert!(output.stderr.is_empty(), "{output:?}");
}

#[test]
fn test_same_seed_gives_same_duration() {
    let random_commands: &[&[&str]] = &[
        &["n", "1s", "100ms"],
        &["e", "2"],
        &["ln", "1s", "500ms"],
        &["par", "1s", "2"],
        &["u", "1s", "2s"],
        &["t", "1s", "3s", "2s"],
        &["g", "2", "1s"],
        &["d", "1s", "-j", "1s"],
        &["d", "1s", "-p", "0.5"],
    ];
    for args in random_commands {
        let seeded = |seed: &str| dry_run(&[*args, &["--seed", seed]].concat()).1;
        assert_eq!(seeded("42"), seeded("42"), "{args:?}");
        let distinct: std::collections::HashSet<String> =
            (0..10).map(|s| seeded(&s.to_string())).collect();
        assert!(distinct.len() > 1, "{args:?}: every seed gave {distinct:?}");
    }
}

fn as_owned(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| s.to_string()).collect()
}

#[test]
fn test_skip_exit_code() {
    for extra in [&[][..], &["--dry-run"]] {
        let skipped = with_args(
            &as_owned(&["d", "5s", "-p", "0.0", "--skip-exit-code", "3"]),
            extra,
        );
        let output = dozr().args(&skipped).output().unwrap();
        assert_eq!(output.status.code(), Some(3), "{skipped:?}");

        let waited = with_args(
            &as_owned(&["d", "10ms", "-p", "1.0", "--skip-exit-code", "3"]),
            extra,
        );
        let output = dozr().args(&waited).output().unwrap();
        assert_eq!(output.status.code(), Some(0), "{waited:?}");
    }
    // Without the option a skip still exits 0, as before.
    let output = dozr().args(["d", "5s", "-p", "0"]).output().unwrap();
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn test_skip_exit_code_matches_what_happened() {
    let mut seen = std::collections::HashSet::new();
    for seed in 0..20 {
        let seed = seed.to_string();
        let (code, out) = dry_run(&[
            "d",
            "1s",
            "-p",
            "0.5",
            "--skip-exit-code",
            "4",
            "--seed",
            &seed,
        ]);
        match out.as_str() {
            "0s\n" => assert_eq!(code, Some(4), "seed {seed}"),
            "1s\n" => assert_eq!(code, Some(0), "seed {seed}"),
            other => panic!("seed {seed}: unexpected output {other:?}"),
        }
        seen.insert(code);
    }
    assert_eq!(seen.len(), 2, "20 seeds never both waited and skipped");
}

#[test]
fn test_min_and_max_bound_every_sample() {
    for seed in 0..30 {
        let seed = seed.to_string();
        let (code, out) = dry_run(&[
            "par", "1s", "0.5", "--min", "2s", "--max", "30s", "--seed", &seed,
        ]);
        assert_eq!(code, Some(0));
        let d = humantime::parse_duration(out.trim()).unwrap();
        assert!(
            d >= Duration::from_secs(2) && d <= Duration::from_secs(30),
            "seed {seed}: {d:?}"
        );
    }
}

#[test]
fn test_min_and_max_do_not_clash_with_uniform_bounds() {
    for seed in 0..10 {
        let seed = seed.to_string();
        let (code, out) = dry_run(&[
            "u", "1s", "5s", "--min", "2s", "--max", "4s", "--seed", &seed,
        ]);
        assert_eq!(code, Some(0));
        let d = humantime::parse_duration(out.trim()).unwrap();
        assert!(
            d >= Duration::from_secs(2) && d <= Duration::from_secs(4),
            "{d:?}"
        );
    }
}

#[test]
fn test_min_greater_than_max_is_rejected() {
    let output = dozr()
        .args(["d", "1s", "--min", "5s", "--max", "1s"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(
        stderr.contains("--min must not be greater than --max"),
        "{stderr}"
    );
}

#[test]
fn test_bound_adjustment_is_reported_when_verbose() {
    // A Pareto with shape 0.5 exceeds 1.5s about 82% of the time.
    let mut saw_note = false;
    for seed in 0..10 {
        let seed = seed.to_string();
        let output = dozr()
            .args([
                "par",
                "1s",
                "0.5",
                "--max",
                "1500ms",
                "-v",
                "--dry-run",
                "--seed",
                &seed,
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("is above --max; waiting 1.500s instead.") {
            assert_eq!(String::from_utf8_lossy(&output.stdout), "1s 500ms\n");
            saw_note = true;
        }
    }
    assert!(saw_note, "no seed reported lowering to --max");
}
