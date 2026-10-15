//! Benchmarks for wait condition calculations.
//!
//! Run with: `cargo bench`
//!
//! These benchmarks measure the computational overhead of calculating
//! wait durations, not the actual waiting time.

use std::time::Duration;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use dozr::Verbose;
use dozr::conditions::{
    DurationWait, ExponentialWait, GammaWait, LogNormalWait, NormalWait, ParetoWait,
    TriangularWait, UniformWait, WaitCondition,
};

fn bench_duration_calculation(c: &mut Criterion) {
    let wait = DurationWait {
        duration: Duration::from_secs(1),
        verbose: Verbose::Off,
        jitter: None,
    };

    c.bench_function("duration_calculate", |b| {
        b.iter(|| black_box(wait.calculate_wait_duration()))
    });
}

fn bench_duration_with_jitter(c: &mut Criterion) {
    let wait = DurationWait {
        duration: Duration::from_secs(1),
        verbose: Verbose::Off,
        jitter: Some(Duration::from_millis(100)),
    };

    c.bench_function("duration_with_jitter_calculate", |b| {
        b.iter(|| black_box(wait.calculate_wait_duration()))
    });
}

fn bench_normal_distribution(c: &mut Criterion) {
    let wait = NormalWait {
        mean: Duration::from_secs(1),
        std_dev: Duration::from_secs_f64(0.1),
        verbose: Verbose::Off,
        jitter: None,
    };

    c.bench_function("normal_distribution_calculate", |b| {
        b.iter(|| black_box(wait.calculate_wait_duration()))
    });
}

fn bench_exponential_distribution(c: &mut Criterion) {
    let wait = ExponentialWait {
        lambda: 1.0,
        verbose: Verbose::Off,
        jitter: None,
    };

    c.bench_function("exponential_distribution_calculate", |b| {
        b.iter(|| black_box(wait.calculate_wait_duration()))
    });
}

fn bench_uniform_distribution(c: &mut Criterion) {
    let wait = UniformWait {
        min: Duration::from_millis(500),
        max: Duration::from_secs(2),
        verbose: Verbose::Off,
        jitter: None,
    };

    c.bench_function("uniform_distribution_calculate", |b| {
        b.iter(|| black_box(wait.calculate_wait_duration()))
    });
}

fn bench_lognormal_distribution(c: &mut Criterion) {
    let wait = LogNormalWait {
        mean: Duration::from_secs(1),
        std_dev: Duration::from_secs_f64(0.5),
        verbose: Verbose::Off,
        jitter: None,
    };

    c.bench_function("lognormal_distribution_calculate", |b| {
        b.iter(|| black_box(wait.calculate_wait_duration()))
    });
}

fn bench_pareto_distribution(c: &mut Criterion) {
    let wait = ParetoWait {
        scale: Duration::from_secs_f64(1.0),
        shape: 2.0,
        verbose: Verbose::Off,
        jitter: None,
    };

    c.bench_function("pareto_distribution_calculate", |b| {
        b.iter(|| black_box(wait.calculate_wait_duration()))
    });
}

fn bench_triangular_distribution(c: &mut Criterion) {
    let wait = TriangularWait {
        min: Duration::from_secs_f64(0.5),
        max: Duration::from_secs_f64(2.0),
        mode: Duration::from_secs_f64(1.0),
        verbose: Verbose::Off,
        jitter: None,
    };

    c.bench_function("triangular_distribution_calculate", |b| {
        b.iter(|| black_box(wait.calculate_wait_duration()))
    });
}

fn bench_gamma_distribution(c: &mut Criterion) {
    let wait = GammaWait {
        shape: 2.0,
        scale: Duration::from_secs_f64(0.5),
        verbose: Verbose::Off,
        jitter: None,
    };

    c.bench_function("gamma_distribution_calculate", |b| {
        b.iter(|| black_box(wait.calculate_wait_duration()))
    });
}

criterion_group!(
    benches,
    bench_duration_calculation,
    bench_duration_with_jitter,
    bench_normal_distribution,
    bench_exponential_distribution,
    bench_uniform_distribution,
    bench_lognormal_distribution,
    bench_pareto_distribution,
    bench_triangular_distribution,
    bench_gamma_distribution,
);

criterion_main!(benches);
