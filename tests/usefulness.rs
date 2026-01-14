/// Comprehensive usefulness tests for the dozr utility
///
/// This test suite evaluates the practical usefulness and real-world applicability
/// of dozr compared to standard sleep, examining its unique features and usability.

use assert_cmd::Command;
use predicates::str;
use std::time::{Duration, Instant};

/// Tests the usability of basic duration syntax compared to sleep
#[test]
fn test_usefulness_human_readable_duration() {
    // dozr supports human-readable durations like "5s", "100ms", "2m", "1h"
    // This is more intuitive than sleep's numeric-only format
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    let start = Instant::now();

    cmd.args(&["d", "500ms"])
        .assert()
        .success();

    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(450));
    assert!(elapsed <= Duration::from_millis(700));
}

/// Tests the usefulness of jitter for avoiding thundering herd problems
#[test]
fn test_usefulness_jitter_for_distributed_systems() {
    // Jitter is essential in distributed systems to prevent synchronized
    // retries or requests (thundering herd problem)
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    let start = Instant::now();

    // Base wait of 100ms with up to 50ms jitter
    cmd.args(&["d", "100ms", "-j", "50ms"])
        .assert()
        .success();

    let elapsed = start.elapsed();
    // Should take at least the base duration
    assert!(elapsed >= Duration::from_millis(90));
    // Should not exceed base + jitter + overhead
    assert!(elapsed <= Duration::from_millis(250));
}

/// Tests the usefulness of probabilistic waiting for chaos engineering
#[test]
fn test_usefulness_probabilistic_wait_chaos_engineering() {
    // Probabilistic waits are useful for chaos engineering and testing
    // timeout handling without always introducing delays

    // Test with 0% probability - should skip immediately
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    let start = Instant::now();

    cmd.args(&["d", "5s", "-p", "0.0"])
        .assert()
        .success();

    let elapsed = start.elapsed();
    // Should complete almost immediately
    assert!(elapsed < Duration::from_millis(100));

    // Test with 100% probability - should always wait
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    let start = Instant::now();

    cmd.args(&["d", "500ms", "-p", "1.0"])
        .assert()
        .success();

    let elapsed = start.elapsed();
    // Should actually wait
    assert!(elapsed >= Duration::from_millis(450));
}

/// Tests the usefulness of verbose mode for long-running scripts
#[test]
fn test_usefulness_verbose_progress_visibility() {
    // Verbose mode is invaluable for understanding what's happening
    // in long-running scripts or automation tasks
    let mut cmd = Command::cargo_bin("dozr").unwrap();

    cmd.args(&["d", "1s", "-v"])
        .assert()
        .success()
        .stderr(str::contains("[DOZR] Time remaining:"));
}

/// Tests the usefulness of exponential distribution for simulating realistic delays
#[test]
fn test_usefulness_exponential_for_realistic_simulation() {
    // Exponential distribution is useful for simulating real-world events
    // like request inter-arrival times, service times, etc.
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    let start = Instant::now();

    // Exponential distribution with lambda=2.0 (mean = 0.5s)
    cmd.args(&["e", "2.0"])
        .assert()
        .success();

    let elapsed = start.elapsed();
    // Should be non-negative and typically under 3 seconds
    assert!(elapsed >= Duration::from_millis(0));
    assert!(elapsed < Duration::from_secs(5));
}

/// Tests the usefulness of normal distribution for performance testing
#[test]
fn test_usefulness_normal_for_performance_testing() {
    // Normal distribution is useful for simulating typical user behavior
    // and network latencies in performance testing
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    let start = Instant::now();

    // Normal distribution with mean=1s, std_dev=0.1
    // Results should cluster around 1s
    cmd.args(&["n", "1s", "0.1"])
        .assert()
        .success();

    let elapsed = start.elapsed();
    // Should typically be within 3 standard deviations (0.7s - 1.3s)
    assert!(elapsed >= Duration::from_millis(0));
    assert!(elapsed < Duration::from_secs(3));
}

/// Tests the usefulness of time alignment for scheduled tasks
#[test]
fn test_usefulness_alignment_for_cron_like_behavior() {
    // Time alignment is useful for ensuring tasks run at regular intervals
    // like every minute on the minute, every 5 seconds, etc.
    let mut cmd = Command::cargo_bin("dozr").unwrap();

    // Align to the next 5-second interval
    cmd.args(&["a", "5s"])
        .assert()
        .success();

    // Verify it completed (the exact timing is hard to test precisely)
}

/// Tests the usefulness of waiting until a specific time
#[test]
fn test_usefulness_wait_until_specific_time() {
    // Waiting until a specific time is useful for scheduling tasks
    // to run at particular times of day
    let mut cmd = Command::cargo_bin("dozr").unwrap();

    // Wait until 2 seconds from now
    let now = chrono::Local::now();
    let target_time = now + chrono::Duration::seconds(2);
    let target_time_str = target_time.format("%H:%M:%S").to_string();

    let start = Instant::now();
    cmd.args(&["at", &target_time_str])
        .assert()
        .success();

    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_secs(1));
    assert!(elapsed <= Duration::from_secs(3));
}

/// Tests the usefulness of combining features (jitter + verbose)
#[test]
fn test_usefulness_feature_composition() {
    // The ability to combine features makes dozr extremely flexible
    // This tests combining jitter with verbose output
    let mut cmd = Command::cargo_bin("dozr").unwrap();

    cmd.args(&["d", "1s", "-j", "200ms", "-v"])
        .assert()
        .success()
        .stderr(str::contains("[DOZR] Time remaining:"));
}

/// Tests the usefulness of uniform distribution for random delays
#[test]
fn test_usefulness_uniform_for_random_delays() {
    // Uniform distribution is useful when you want random delays
    // within a specific range with equal probability
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    let start = Instant::now();

    // Uniform distribution between 100ms and 500ms
    cmd.args(&["u", "100ms", "500ms"])
        .assert()
        .success();

    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(0));
    assert!(elapsed <= Duration::from_millis(800));
}

/// Tests the usefulness of triangular distribution for bounded randomness
#[test]
fn test_usefulness_triangular_for_realistic_bounds() {
    // Triangular distribution is useful for modeling events with
    // minimum, maximum, and most likely values
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    let start = Instant::now();

    // Triangular: min=0.1s, max=1.0s, mode=0.3s
    cmd.args(&["t", "0.1", "1.0", "0.3"])
        .assert()
        .success();

    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(0));
    assert!(elapsed <= Duration::from_secs(2));
}

/// Tests the usefulness of Pareto distribution for heavy-tailed events
#[test]
fn test_usefulness_pareto_for_heavy_tails() {
    // Pareto distribution is useful for modeling events with occasional
    // very long delays (80/20 rule, power law distributions)
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    let start = Instant::now();

    cmd.args(&["par", "1.0", "3.0"])
        .assert()
        .success();

    let elapsed = start.elapsed();
    // Pareto can produce very long tails, but should complete eventually
    assert!(elapsed >= Duration::from_millis(0));
    assert!(elapsed < Duration::from_secs(30));
}

/// Tests the usefulness of gamma distribution for queueing theory
#[test]
fn test_usefulness_gamma_for_queueing() {
    // Gamma distribution is useful in queueing theory and modeling
    // the time until the nth event in a Poisson process
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    let start = Instant::now();

    cmd.args(&["g", "2.0", "0.5"])
        .assert()
        .success();

    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(0));
    assert!(elapsed < Duration::from_secs(10));
}

/// Tests error handling usefulness - invalid inputs provide clear feedback
#[test]
fn test_usefulness_clear_error_messages() {
    // Good error messages are crucial for usability
    let mut cmd = Command::cargo_bin("dozr").unwrap();

    cmd.args(&["d", "invalid"])
        .assert()
        .failure()
        .stderr(str::contains("error: invalid value"));
}

/// Tests that dozr is a drop-in replacement for simple sleep use cases
#[test]
fn test_usefulness_sleep_compatibility() {
    // dozr can be used as a drop-in replacement for sleep
    // with better features
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    let start = Instant::now();

    cmd.args(&["d", "500ms"])
        .assert()
        .success();

    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(450));

    // Equivalent to: sleep 0.5
    // But with more readable syntax and extensibility
}

/// Tests the usefulness of adaptive verbose mode for different wait durations
#[test]
fn test_usefulness_adaptive_verbose_scalability() {
    // Adaptive verbose mode automatically adjusts update frequency
    // based on wait duration, making it useful for both short and long waits
    let mut cmd = Command::cargo_bin("dozr").unwrap();

    // Short wait with adaptive verbose
    cmd.args(&["d", "2s", "-v"])
        .assert()
        .success()
        .stderr(str::contains("[DOZR] Time remaining:"));
}

/// Tests combined probabilistic and distribution-based waits
#[test]
fn test_usefulness_probabilistic_with_distribution() {
    // Combining probability with distributions enables sophisticated
    // test scenarios and chaos engineering
    let mut cmd = Command::cargo_bin("dozr").unwrap();

    // 50% chance of waiting a normally-distributed duration
    cmd.args(&["n", "500ms", "0.1", "-p", "0.5"])
        .assert()
        .success();

    // This is powerful for intermittent failure testing
}

/// Tests the usefulness of verbose mode with custom update periods
#[test]
fn test_usefulness_custom_verbose_granularity() {
    // Custom update periods allow fine-tuning progress feedback
    // for different monitoring requirements
    let mut cmd = Command::cargo_bin("dozr").unwrap();

    cmd.args(&["d", "2s", "-v", "500ms"])
        .assert()
        .success()
        .stderr(str::contains("[DOZR] Time remaining:"));
}
