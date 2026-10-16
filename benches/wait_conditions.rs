//! Benchmarks for drawing wait durations.
//!
//! Run with: `cargo bench`
//!
//! These measure the cost of sampling a duration, not the waiting itself.

use std::time::Duration;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use dozr::conditions::{Sampler, Wait};

fn bench_samplers(c: &mut Criterion) {
    let ms = Duration::from_millis;
    let cases = [
        ("fixed", Wait::new(Sampler::Fixed(ms(1_000)))),
        (
            "fixed_with_jitter",
            Wait::new(Sampler::Fixed(ms(1_000))).with_jitter(ms(100)),
        ),
        (
            "fixed_with_probability",
            Wait::new(Sampler::Fixed(ms(1_000))).with_probability(0.5),
        ),
        (
            "normal",
            Wait::new(Sampler::Normal {
                mean: ms(1_000),
                std_dev: ms(100),
            }),
        ),
        (
            "exponential",
            Wait::new(Sampler::Exponential { lambda: 1.0 }),
        ),
        (
            "uniform",
            Wait::new(Sampler::Uniform {
                min: ms(500),
                max: ms(2_000),
            }),
        ),
        (
            "lognormal",
            Wait::new(Sampler::LogNormal {
                mean: ms(1_000),
                std_dev: ms(500),
            }),
        ),
        (
            "pareto",
            Wait::new(Sampler::Pareto {
                scale: ms(1_000),
                shape: 2.0,
            }),
        ),
        (
            "triangular",
            Wait::new(Sampler::Triangular {
                min: ms(500),
                max: ms(2_000),
                mode: ms(1_000),
            }),
        ),
        (
            "gamma",
            Wait::new(Sampler::Gamma {
                shape: 2.0,
                scale: ms(500),
            }),
        ),
    ];

    let mut rng = rand::rng();
    for (name, wait) in cases {
        c.bench_function(name, |b| b.iter(|| black_box(wait.sample(&mut rng))));
    }
}

criterion_group!(benches, bench_samplers);
criterion_main!(benches);
