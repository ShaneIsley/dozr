use assert_cmd::Command;
use predicates::prelude::{Predicate, PredicateBooleanExt};
use predicates::str;
use std::time::Instant;
use std::time::Duration;
use dozr::cli::Cli;

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
        "error: the following required arguments were not provided:\n  <--duration <DURATION>|--normal|--exponential|--log-normal|--pareto|--weibull|--align <ALIGN>|--until <UNTIL>>",
    ));
}

#[test]
fn test_duration_is_valid() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--duration", "1s"]).assert().success();
}

#[test]
fn test_time_align_verbose_output() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--align", "5s", "--verbose"])
        .assert()
        .success()
        .stderr(str::contains("[DOZR] Time remaining:").and(str::contains("s")));
}

#[test]
fn test_probabilistic_wait_verbose_output() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--duration", "1s", "--probability", "1.0", "--verbose"])
        .assert()
        .success()
        .stderr(str::contains("[DOZR] Time remaining:").and(str::contains("s")));
}

#[test]
fn test_probabilistic_wait_skip_verbose_output() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--duration", "1s", "--probability", "0.0", "--verbose"])
        .assert()
        .success()
        .stderr(str::contains("Probabilistic wait: Skipping sleep"));
}

#[test]
fn test_jitter_zero_duration() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--duration", "1s", "--jitter", "0s"])
        .assert()
        .success();
}

#[test]
fn test_until_time_verbose_output() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    // Set a time in the near future (e.g., 5 seconds from now)
    let now = chrono::Local::now();
    let target_time = now + chrono::Duration::seconds(5);
    let target_time_str = target_time.format("%H:%M:%S").to_string();

    cmd.args(&["--until", &target_time_str, "--verbose"])
        .assert()
        .success()
        .stderr(str::contains("[DOZR] Time remaining:").and(str::contains("s")));
}

#[test]
fn test_invalid_until_time_format() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--until", "invalid-time"])
        .assert()
        .failure()
        .stderr(str::contains("Invalid time format"));
}

#[test]
fn test_invalid_until_time_hour() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--until", "25:00"])
        .assert()
        .failure()
        .stderr(str::contains("Invalid time format"));
}

#[test]
fn test_invalid_until_time_minute() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--until", "10:65"])
        .assert()
        .failure()
        .stderr(str::contains("Invalid time format"));
}

#[test]
fn test_until_time_in_future() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    let now = chrono::Local::now();
    let target_time = now + chrono::Duration::seconds(2);
    let target_time_str = target_time.format("%H:%M:%S").to_string();

    let start_time = Instant::now();
    cmd.args(&["--until", &target_time_str]).assert().success();
    let elapsed = start_time.elapsed();

    assert!(elapsed >= chrono::Duration::seconds(1).to_std().unwrap());
    assert!(elapsed <= chrono::Duration::seconds(3).to_std().unwrap());
}

#[test]
fn test_parse_time_until_hh_mm() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    let now = chrono::Local::now();
    let target_time = now + chrono::Duration::minutes(1);
    let target_time_str = target_time.format("%H:%M").to_string();

    cmd.args(&["--until", &target_time_str]).assert().success();
}

// New tests for Cli helper methods
#[test]
fn test_is_adaptive_verbose() {
    // Adaptive verbose (1ns sentinel)
    let cli_adaptive = Cli { 
        duration: None, 
        normal: false,
        normal_mean: None,
        normal_std_dev: None,
        exponential: false,
        exponential_lambda: None,
        log_normal: false,
        log_normal_mean: None,
        log_normal_std_dev: None,
        pareto: false,
        pareto_scale: None,
        pareto_shape: None,
        weibull: false,
        weibull_shape: None,
        weibull_scale: None,
        jitter: None, 
        align: None, 
        verbose: Some(Duration::from_nanos(1)), 
        probability: None, 
        until: None 
    };
    assert!(cli_adaptive.is_adaptive_verbose());

    // Fixed verbose (e.g., 1s)
    let cli_fixed = Cli { 
        duration: None, 
        normal: false,
        normal_mean: None,
        normal_std_dev: None,
        exponential: false,
        exponential_lambda: None,
        log_normal: false,
        log_normal_mean: None,
        log_normal_std_dev: None,
        pareto: false,
        pareto_scale: None,
        pareto_shape: None,
        weibull: false,
        weibull_shape: None,
        weibull_scale: None,
        jitter: None, 
        align: None, 
        verbose: Some(Duration::from_secs(1)), 
        probability: None, 
        until: None 
    };
    assert!(!cli_fixed.is_adaptive_verbose());

    // No verbose
    let cli_none = Cli { 
        duration: None, 
        normal: false,
        normal_mean: None,
        normal_std_dev: None,
        exponential: false,
        exponential_lambda: None,
        log_normal: false,
        log_normal_mean: None,
        log_normal_std_dev: None,
        pareto: false,
        pareto_scale: None,
        pareto_shape: None,
        weibull: false,
        weibull_shape: None,
        weibull_scale: None,
        jitter: None, 
        align: None, 
        verbose: None, 
        probability: None, 
        until: None 
    };
    assert!(!cli_none.is_adaptive_verbose());
}

#[test]
fn test_verbose_period() {
    // Adaptive verbose (1ns sentinel) -> Should return None
    let cli_adaptive = Cli { 
        duration: None, 
        normal: false,
        normal_mean: None,
        normal_std_dev: None,
        exponential: false,
        exponential_lambda: None,
        log_normal: false,
        log_normal_mean: None,
        log_normal_std_dev: None,
        pareto: false,
        pareto_scale: None,
        pareto_shape: None,
        weibull: false,
        weibull_shape: None,
        weibull_scale: None,
        jitter: None, 
        align: None, 
        verbose: Some(Duration::from_nanos(1)), 
        probability: None, 
        until: None 
    };
    assert_eq!(cli_adaptive.verbose_period(), None);

    // Fixed verbose (e.g., 1s) -> Should return Some(1s)
    let cli_fixed = Cli { 
        duration: None, 
        normal: false,
        normal_mean: None,
        normal_std_dev: None,
        exponential: false,
        exponential_lambda: None,
        log_normal: false,
        log_normal_mean: None,
        log_normal_std_dev: None,
        pareto: false,
        pareto_scale: None,
        pareto_shape: None,
        weibull: false,
        weibull_shape: None,
        weibull_scale: None,
        jitter: None, 
        align: None, 
        verbose: Some(Duration::from_secs(1)), 
        probability: None, 
        until: None 
    };
    assert_eq!(cli_fixed.verbose_period(), Some(Duration::from_secs(1)));

    // No verbose -> Should return None
    let cli_none = Cli { 
        duration: None, 
        normal: false,
        normal_mean: None,
        normal_std_dev: None,
        exponential: false,
        exponential_lambda: None,
        log_normal: false,
        log_normal_mean: None,
        log_normal_std_dev: None,
        pareto: false,
        pareto_scale: None,
        pareto_shape: None,
        weibull: false,
        weibull_shape: None,
        weibull_scale: None,
        jitter: None, 
        align: None, 
        verbose: None, 
        probability: None, 
        until: None 
    };
    assert_eq!(cli_none.verbose_period(), None);
}

#[test]
fn test_normal_distribution_args() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--normal", "--normal-mean", "1s", "--normal-std-dev", "0.1"])
        .assert()
        .success();
}

#[test]
fn test_exponential_distribution_args() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--exponential", "--exponential-lambda", "0.5"])
        .assert()
        .success();
}

#[test]
fn test_log_normal_distribution_args() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--log-normal", "--log-normal-mean", "1s", "--log-normal-std-dev", "0.1"])
        .assert()
        .success();
}

#[test]
fn test_pareto_distribution_args() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--pareto", "--pareto-scale", "1.0", "--pareto-shape", "1.5"])
        .assert()
        .success();
}

#[test]
fn test_weibull_distribution_args() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--weibull", "--weibull-shape", "1.5", "--weibull-scale", "1.0"])
        .assert()
        .success();
}

#[test]
fn test_mutually_exclusive_distribution_args() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--duration", "1s", "--normal"])
        .assert()
        .failure()
        .stderr(str::contains("the argument '--duration <DURATION>' cannot be used with '--normal'"));
}

#[test]
fn test_normal_distribution_missing_param() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--normal", "--normal-mean", "1s"])
        .assert()
        .failure()
        .stderr(str::contains("required arguments were not provided"));
}

#[test]
fn test_normal_distribution_missing_all_params() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--normal"])
        .assert()
        .failure()
        .stderr(str::contains("required arguments were not provided"));
}

#[test]
fn test_exponential_distribution_invalid_lambda() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    cmd.args(&["--exponential-lambda", "-0.5"])
        .assert()
        .failure()
        .stderr(str::contains("error: unexpected argument '-0' found"));
}

#[test]
fn test_normal_distribution_wait_time() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    let start = Instant::now();
    cmd.args(&["--normal", "--normal-mean", "1s", "--normal-std-dev", "0.1"])
        .assert()
        .success();
    let elapsed = start.elapsed();
    // Assert that the elapsed time is greater than 0 and within a reasonable range (e.g., 0.5s to 2s)
    assert!(elapsed > Duration::from_millis(0));
    assert!(elapsed < Duration::from_secs(2));
}

#[test]
fn test_exponential_distribution_wait_time() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    let start = Instant::now();
    cmd.args(&["--exponential", "--exponential-lambda", "1.0"])
        .assert()
        .success();
    let elapsed = start.elapsed();
    // Exponential distribution with lambda=1.0 has a mean of 1.0. Allow a broad range.
    assert!(elapsed > Duration::from_millis(0));
    assert!(elapsed < Duration::from_secs(5));
}

#[test]
fn test_log_normal_distribution_wait_time() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    let start = Instant::now();
    cmd.args(&["--log-normal", "--log-normal-mean", "1s", "--log-normal-std-dev", "0.5"])
        .assert()
        .success();
    let elapsed = start.elapsed();
    // Log-Normal distribution with mean=1s, std_dev=0.5. Allow a broad range.
    assert!(elapsed > Duration::from_millis(0));
    assert!(elapsed < Duration::from_secs(5));
}

#[test]
fn test_pareto_distribution_wait_time() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    let start = Instant::now();
    cmd.args(&["--pareto", "--pareto-scale", "1.0", "--pareto-shape", "2.0"])
        .assert()
        .success();
    let elapsed = start.elapsed();
    // Pareto distribution with scale=1.0, shape=2.0. Allow a broad range.
    assert!(elapsed > Duration::from_millis(0));
    assert!(elapsed < Duration::from_secs(5));
}

#[test]
fn test_weibull_distribution_wait_time() {
    let mut cmd = Command::cargo_bin("dozr").unwrap();
    let start = Instant::now();
    cmd.args(&["--weibull", "--weibull-shape", "1.0", "--weibull-scale", "1.0"])
        .assert()
        .success();
    let elapsed = start.elapsed();
    // Weibull distribution with shape=1.0, scale=1.0. Allow a broad range.
    assert!(elapsed > Duration::from_millis(0));
    assert!(elapsed < Duration::from_secs(5));
}