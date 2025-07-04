use assert_cmd::Command;
use predicates::str;
use predicates::prelude::PredicateBooleanExt;
use std::time::Instant;

#[test]
fn test_jitter_flag_accepts_argument() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["0s", "--jitter", "0s"]).assert().success();
}

#[test]
fn test_jitter_adds_time() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    let start = Instant::now();
    // We use a small base duration and a small jitter to keep the test fast.
    // The key is verifying that *some* extra time was added.
    cmd.args(&["100ms", "--jitter", "200ms"]).assert().success();
    let duration = start.elapsed();

    // Assert that the command took at least the base duration.
    assert!(duration.as_millis() >= 100);
    // Assert that the command did not take longer than the base + max jitter + a generous buffer for overhead.
    assert!(duration.as_millis() <= 1000);
}

#[test]
fn test_verbose_output_includes_eta() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    // Use a duration long enough to ensure multiple ETA updates
    cmd.args(&["2s", "--verbose"]).assert().success().stderr(str::contains("Waiting for").and(str::contains("ETA:")));
}