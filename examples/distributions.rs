//! Example demonstrating statistical distribution sampling.
//!
//! Run with: `cargo run --example distributions`

use std::time::Duration;

use dozr::Verbose;
use dozr::conditions::{ExponentialWait, GammaWait, NormalWait, UniformWait, WaitCondition};

fn main() -> anyhow::Result<()> {
    // Normal distribution: mean of 500ms, standard deviation of 100ms
    println!("Sampling from Normal distribution (mean=500ms, std_dev=100ms)...");
    let normal = NormalWait {
        mean: Duration::from_millis(500),
        std_dev: Duration::from_secs_f64(0.1),
        verbose: Verbose::Off,
        jitter: None,
    };
    let duration = normal.calculate_wait_duration()?;
    println!("  Sampled duration: {:?}\n", duration);

    // Uniform distribution: between 200ms and 800ms
    println!("Sampling from Uniform distribution (200ms to 800ms)...");
    let uniform = UniformWait {
        min: Duration::from_millis(200),
        max: Duration::from_millis(800),
        verbose: Verbose::Off,
        jitter: None,
    };
    let duration = uniform.calculate_wait_duration()?;
    println!("  Sampled duration: {:?}\n", duration);

    // Exponential distribution: lambda = 2.0
    println!("Sampling from Exponential distribution (lambda=2.0)...");
    let exponential = ExponentialWait {
        lambda: 2.0,
        verbose: Verbose::Off,
        jitter: None,
    };
    let duration = exponential.calculate_wait_duration()?;
    println!("  Sampled duration: {:?}\n", duration);

    // Gamma distribution: shape = 2.0, scale = 0.5
    println!("Sampling from Gamma distribution (shape=2.0, scale=500ms)...");
    let gamma = GammaWait {
        shape: 2.0,
        scale: Duration::from_secs_f64(0.5),
        verbose: Verbose::Off,
        jitter: None,
    };
    let duration = gamma.calculate_wait_duration()?;
    println!("  Sampled duration: {:?}\n", duration);

    Ok(())
}
