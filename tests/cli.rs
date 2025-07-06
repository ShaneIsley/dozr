use assert_cmd::Command;
use predicates::prelude::{Predicate, PredicateBooleanExt};
use predicates::str;
use std::time::Instant;

#[test]
fn test_jitter_flag_accepts_argument() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--duration", "0s", "--jitter", "0s"])
        .assert()
        .success();
}

#[test]
fn test_jitter_adds_time() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    let start = Instant::now();
    // We use a small base duration and a small jitter to keep the test fast.
    // The key is verifying that *some* extra time was added.
    cmd.args(&["--duration", "100ms", "--jitter", "200ms"])
        .assert()
        .success();
    let duration = start.elapsed();

    // Assert that the command took at least the base duration.
    assert!(duration.as_millis() >= 100);
    // Assert that the command did not take longer than the base + max jitter + a generous buffer for overhead.
    assert!(duration.as_millis() <= 1500);
}

#[test]
fn test_verbose_output_includes_eta() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    // Use a duration long enough to ensure multiple ETA updates
    cmd.args(&["--duration", "2s", "--verbose"])
        .assert()
        .success()
        .stderr(str::contains("[DOZR] Time remaining:").and(str::contains("s")));
}

#[test]
fn test_verbose_custom_update_period() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    // Test with a 1.5s wait and 500ms update period.
    let assert = cmd
        .args(&["--duration", "1s500ms", "--verbose", "500ms"])
        .assert()
        .success();
    let output = assert.get_output();

    let stderr_str = String::from_utf8_lossy(&output.stderr);

    assert!(str::contains("[DOZR] Time remaining:").eval(&stderr_str));
}

#[test]
fn test_verbose_adaptive_short_wait() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    // Test with a 1.5s wait (adaptive 500ms update).
    let assert = cmd
        .args(&["--duration", "1s500ms", "--verbose"])
        .assert()
        .success();
    let output = assert.get_output();

    let stderr_str = String::from_utf8_lossy(&output.stderr);

    assert!(str::contains("[DOZR] Time remaining:").eval(&stderr_str));
}

#[test]
fn test_verbose_adaptive_long_wait() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    // Test with a 5s wait (adaptive 1s update).
    let assert = cmd
        .args(&["--duration", "5s", "--verbose"])
        .assert()
        .success();
    let output = assert.get_output();

    let stderr_str = String::from_utf8_lossy(&output.stderr);

    assert!(str::contains("[DOZR] Time remaining:").eval(&stderr_str));
}

#[test]
fn test_invalid_duration_arg() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--duration", "invalid-duration"])
        .assert()
        .failure()
        .stderr(str::contains("error: invalid value"));
}

#[test]
fn test_invalid_jitter_arg() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--duration", "1s", "--jitter", "invalid-jitter"])
        .assert()
        .failure()
        .stderr(str::contains("error: invalid value"));
}

#[test]
fn test_invalid_verbose_period_arg() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--duration", "1s", "--verbose", "invalid-period"])
        .assert()
        .failure()
        .stderr(str::contains("error: invalid value"));
}

#[test]
fn test_duration_and_align_are_mutually_exclusive() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--duration", "1s", "--align", "5s"])
        .assert()
        .failure()
        .stderr(str::contains(
            "the argument '--duration <DURATION>' cannot be used with '--align <ALIGN>'",
        ));
}

#[test]
fn test_duration_or_align_is_required() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.assert().failure().stderr(str::contains(
        "error: the following required arguments were not provided:
  <--duration <DURATION>|--align <ALIGN>>",
    ));
}

#[test]
fn test_duration_is_valid() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--duration", "1s"]).assert().success();
}
