//! Example demonstrating statistical distribution sampling.
//!
//! Run with: `cargo run --example distributions`

use std::time::Duration;

use dozr::conditions::{Sampler, Wait};

fn main() -> anyhow::Result<()> {
    let samplers = [
        (
            "Normal (mean 500ms, standard deviation 100ms)",
            Sampler::Normal {
                mean: Duration::from_millis(500),
                std_dev: Duration::from_millis(100),
            },
        ),
        (
            "Uniform (200ms to 800ms)",
            Sampler::Uniform {
                min: Duration::from_millis(200),
                max: Duration::from_millis(800),
            },
        ),
        (
            "Exponential (rate 2 per second, mean 500ms)",
            Sampler::Exponential { lambda: 2.0 },
        ),
        (
            "Gamma (shape 2, scale 250ms, mean 500ms)",
            Sampler::Gamma {
                shape: 2.0,
                scale: Duration::from_millis(250),
            },
        ),
    ];

    for (label, sampler) in samplers {
        let duration = Wait::new(sampler).calculate()?;
        println!("{label}: sampled {duration:?}");
    }
    Ok(())
}
