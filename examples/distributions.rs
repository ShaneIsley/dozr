//! Example demonstrating statistical distribution sampling.
//!
//! Run with: `cargo run --example distributions`

use std::time::Duration;

use dozr::conditions::{
    ExponentialWait, GammaWait, NormalWait, UniformWait, WaitCondition,
};

fn main() -> anyhow::Result<()> {
    // Normal distribution: mean of 500ms, std dev of 100ms
    println!("Sampling from Normal distribution (mean=500ms, std_dev=0.1)...");
    let normal = NormalWait {
        mean: Duration::from_millis(500),
        std_dev: 0.1,
        verbose: None,
        jitter: None,
    };
    let duration = normal.calculate_wait_duration()?;
    println!("  Sampled duration: {:?}\n", duration);

    // Uniform distribution: between 200ms and 800ms
    println!("Sampling from Uniform distribution (200ms to 800ms)...");
    let uniform = UniformWait {
        min: Duration::from_millis(200),
        max: Duration::from_millis(800),
        verbose: None,
        jitter: None,
    };
    let duration = uniform.calculate_wait_duration()?;
    println!("  Sampled duration: {:?}\n", duration);

    // Exponential distribution: lambda = 2.0
    println!("Sampling from Exponential distribution (lambda=2.0)...");
    let exponential = ExponentialWait {
        lambda: 2.0,
        verbose: None,
        jitter: None,
    };
    let duration = exponential.calculate_wait_duration()?;
    println!("  Sampled duration: {:?}\n", duration);

    // Gamma distribution: shape = 2.0, scale = 0.5
    println!("Sampling from Gamma distribution (shape=2.0, scale=0.5)...");
    let gamma = GammaWait {
        shape: 2.0,
        scale: 0.5,
        verbose: None,
        jitter: None,
    };
    let duration = gamma.calculate_wait_duration()?;
    println!("  Sampled duration: {:?}\n", duration);

    Ok(())
}
