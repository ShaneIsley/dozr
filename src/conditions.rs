//! Wait conditions: rules for choosing how long to wait, and the wait itself.
//!
//! Every condition implements [`WaitCondition::sample`], which draws one
//! [`Sample`]. Waiting is a single shared path: draw a sample, then hand it to
//! a [`Sleeper`]. Tests substitute their own sleeper to see exactly what would
//! have been slept without sleeping.

use crate::{Verbose, adaptive_verbose_wait, verbose_wait};
use anyhow::{Result, bail};
use rand::{Rng, RngCore};
use rand_distr::{Distribution, Exp, Gamma, LogNormal, Normal, Pareto, Triangular, Uniform};

use chrono::Local;

use std::time::{Duration, SystemTime};

/// One draw from a [`WaitCondition`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    /// How long to wait.
    pub duration: Duration,
    /// The distribution produced a negative value, which was raised to zero
    /// before any jitter was added.
    pub clamped: bool,
    /// Set when a [`ProbabilityGate`] decided not to wait.
    pub skipped: Option<Skip>,
}

impl Sample {
    /// A plain wait of `duration`.
    pub fn of(duration: Duration) -> Self {
        Sample {
            duration,
            clamped: false,
            skipped: None,
        }
    }
}

/// Details of a skipped probabilistic wait.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Skip {
    pub probability: f64,
    pub roll: f64,
}

/// Carries out a [`Sample`]: reports on it and sleeps for it.
pub trait Sleeper {
    fn perform(&mut self, sample: &Sample, verbose: Verbose);
}

/// The real [`Sleeper`]: writes progress to stderr and blocks the thread.
pub struct ThreadSleeper;

impl Sleeper for ThreadSleeper {
    fn perform(&mut self, sample: &Sample, verbose: Verbose) {
        if let Some(skip) = sample.skipped {
            if verbose.is_on() {
                eprintln!(
                    "Probabilistic wait: Skipping sleep (probability: {}, roll: {:.2})",
                    skip.probability, skip.roll
                );
            }
            return;
        }
        if sample.clamped && verbose.is_on() {
            eprintln!(
                "[{}] [DOZR] Sampled duration was negative and was clamped to zero.",
                Local::now().format("%H:%M:%S")
            );
        }
        perform_wait(sample.duration, verbose);
    }
}

/// Sleeps for `sleep_duration`, printing progress according to `verbose`.
fn perform_wait(sleep_duration: Duration, verbose: Verbose) {
    let report = |remaining: Duration, decimals: usize| {
        let now = Local::now().format("%H:%M:%S");
        if remaining.is_zero() {
            eprintln!("[{now}] Wait complete.");
        } else {
            eprintln!(
                "[{now}] [DOZR] Time remaining: {:.*}s",
                decimals,
                remaining.as_secs_f64()
            );
        }
    };
    match verbose {
        Verbose::Off => std::thread::sleep(sleep_duration),
        Verbose::Adaptive => adaptive_verbose_wait(sleep_duration, |r| report(r, 0)),
        Verbose::Every(period) => {
            let decimals = if period < Duration::from_secs(1) {
                1
            } else {
                0
            };
            verbose_wait(sleep_duration, period, |r| report(r, decimals));
        }
    }
}

/// Source of random jitter, abstracted so tests can supply a fixed value.
pub trait JitterGenerator {
    fn generate(&mut self, max_jitter: Duration) -> Duration;
}

/// [`JitterGenerator`] backed by a random number generator.
pub struct RandomJitterGenerator<T: Rng> {
    rng: T,
}

impl<T: Rng> RandomJitterGenerator<T> {
    pub fn new(rng: T) -> Self {
        Self { rng }
    }
}

impl<T: Rng> JitterGenerator for RandomJitterGenerator<T> {
    fn generate(&mut self, max_jitter: Duration) -> Duration {
        if max_jitter.is_zero() {
            return Duration::ZERO;
        }
        let jitter_nanos = self.rng.random_range(0..=max_jitter.as_nanos() as u64);
        Duration::from_nanos(jitter_nanos)
    }
}

/// Draws up to `max_jitter` of random jitter from `rng`.
fn sample_jitter(rng: &mut dyn RngCore, max_jitter: Option<Duration>) -> Duration {
    RandomJitterGenerator::new(rng).generate(max_jitter.unwrap_or(Duration::ZERO))
}

/// Converts a sampled number of seconds to a `Duration`. Negative values and
/// NaN become zero, and are reported as clamped.
fn secs_to_duration(secs: f64) -> (Duration, bool) {
    if secs.is_finite() && secs > 0.0 {
        (Duration::from_secs_f64(secs), false)
    } else {
        (Duration::ZERO, secs != 0.0)
    }
}

/// A rule for deciding how long to wait.
pub trait WaitCondition {
    /// Draws one sample using `rng`. Pass a seeded generator for
    /// reproducible results; conditions with no randomness ignore it.
    fn sample(&self, rng: &mut dyn RngCore) -> Result<Sample>;

    /// How this condition reports progress while waiting.
    fn verbose(&self) -> Verbose;

    /// Calculates a wait duration using the supplied random number generator.
    fn sample_with(&self, rng: &mut dyn RngCore) -> Result<Duration> {
        Ok(self.sample(rng)?.duration)
    }

    /// Calculates a wait duration using the thread-local random number generator.
    fn calculate_wait_duration(&self) -> Result<Duration> {
        self.sample_with(&mut rand::rng())
    }

    /// Draws exactly one sample and passes it to `sleeper`.
    fn wait_using(&self, rng: &mut dyn RngCore, sleeper: &mut dyn Sleeper) -> Result<()> {
        let sample = self.sample(rng)?;
        sleeper.perform(&sample, self.verbose());
        Ok(())
    }

    /// Performs the wait.
    fn wait(&self) -> Result<()> {
        self.wait_using(&mut rand::rng(), &mut ThreadSleeper)
    }
}

/// Implements [`WaitCondition`] for a jittered distribution whose `$body`
/// evaluates to a sampled number of seconds.
macro_rules! distribution_wait {
    ($ty:ty, |$self_:ident, $rng:ident| $body:block) => {
        impl WaitCondition for $ty {
            fn sample(&$self_, $rng: &mut dyn RngCore) -> Result<Sample> {
                let secs: f64 = $body;
                let (base, clamped) = secs_to_duration(secs);
                Ok(Sample {
                    duration: base + sample_jitter($rng, $self_.jitter),
                    clamped,
                    skipped: None,
                })
            }

            fn verbose(&self) -> Verbose {
                self.verbose
            }
        }
    };
}

pub struct DurationWait {
    pub duration: Duration,
    pub verbose: Verbose,
    pub jitter: Option<Duration>,
}

impl DurationWait {
    fn calculate_sleep_duration(&self, jitter_gen: &mut dyn JitterGenerator) -> Duration {
        let max_jitter = self.jitter.unwrap_or(Duration::ZERO);
        self.duration + jitter_gen.generate(max_jitter)
    }
}

impl WaitCondition for DurationWait {
    fn sample(&self, rng: &mut dyn RngCore) -> Result<Sample> {
        let mut jitter_gen = RandomJitterGenerator::new(rng);
        Ok(Sample::of(self.calculate_sleep_duration(&mut jitter_gen)))
    }

    fn verbose(&self) -> Verbose {
        self.verbose
    }
}

pub struct NormalWait {
    pub mean: Duration,
    pub std_dev: Duration,
    pub verbose: Verbose,
    pub jitter: Option<Duration>,
}

distribution_wait!(NormalWait, |self, rng| {
    Normal::new(self.mean.as_secs_f64(), self.std_dev.as_secs_f64())?.sample(rng)
});

pub struct ExponentialWait {
    /// Rate parameter, in events per second. The mean wait is `1 / lambda` seconds.
    pub lambda: f64,
    pub verbose: Verbose,
    pub jitter: Option<Duration>,
}

distribution_wait!(ExponentialWait, |self, rng| {
    Exp::new(self.lambda)?.sample(rng)
});

/// Builds a log-normal distribution from its real (linear-space) mean and
/// standard deviation, both in seconds.
///
/// ```
/// use dozr::conditions::log_normal_from_mean_sd;
/// use rand::SeedableRng;
/// use rand_distr::Distribution;
///
/// let dist = log_normal_from_mean_sd(2.0, 0.5).unwrap();
/// let mut rng = rand::rngs::StdRng::seed_from_u64(1);
/// let n = 20_000;
/// let mean = (0..n).map(|_| dist.sample(&mut rng)).sum::<f64>() / n as f64;
/// assert!((mean - 2.0).abs() < 0.05);
///
/// assert!(log_normal_from_mean_sd(0.0, 0.5).is_err());
/// assert!(log_normal_from_mean_sd(1.0, -0.1).is_err());
/// ```
pub fn log_normal_from_mean_sd(mean_secs: f64, std_dev_secs: f64) -> Result<LogNormal<f64>> {
    if !(mean_secs.is_finite() && mean_secs > 0.0) {
        bail!("log-normal mean must be greater than zero, got {mean_secs}");
    }
    if !(std_dev_secs.is_finite() && std_dev_secs >= 0.0) {
        bail!("log-normal standard deviation must be zero or greater, got {std_dev_secs}");
    }
    Ok(LogNormal::from_mean_cv(
        mean_secs,
        std_dev_secs / mean_secs,
    )?)
}

pub struct LogNormalWait {
    /// Mean of the sampled waits. Must be greater than zero.
    pub mean: Duration,
    /// Standard deviation of the sampled waits.
    pub std_dev: Duration,
    pub verbose: Verbose,
    pub jitter: Option<Duration>,
}

distribution_wait!(LogNormalWait, |self, rng| {
    log_normal_from_mean_sd(self.mean.as_secs_f64(), self.std_dev.as_secs_f64())?.sample(rng)
});

pub struct ParetoWait {
    /// Minimum wait. Must be greater than zero.
    pub scale: Duration,
    /// Tail index; smaller values give a heavier tail. Must be greater than zero.
    pub shape: f64,
    pub verbose: Verbose,
    pub jitter: Option<Duration>,
}

distribution_wait!(ParetoWait, |self, rng| {
    Pareto::new(self.scale.as_secs_f64(), self.shape)?.sample(rng)
});

pub struct UniformWait {
    pub min: Duration,
    pub max: Duration,
    pub verbose: Verbose,
    pub jitter: Option<Duration>,
}

distribution_wait!(UniformWait, |self, rng| {
    Uniform::new(self.min.as_secs_f64(), self.max.as_secs_f64())?.sample(rng)
});

pub struct TriangularWait {
    pub min: Duration,
    pub max: Duration,
    /// Most likely wait. Must be between `min` and `max`.
    pub mode: Duration,
    pub verbose: Verbose,
    pub jitter: Option<Duration>,
}

distribution_wait!(TriangularWait, |self, rng| {
    Triangular::new(
        self.min.as_secs_f64(),
        self.max.as_secs_f64(),
        self.mode.as_secs_f64(),
    )?
    .sample(rng)
});

pub struct GammaWait {
    /// Shape parameter. Must be greater than zero.
    pub shape: f64,
    /// Scale parameter. Must be greater than zero.
    pub scale: Duration,
    pub verbose: Verbose,
    pub jitter: Option<Duration>,
}

distribution_wait!(GammaWait, |self, rng| {
    Gamma::new(self.shape, self.scale.as_secs_f64())?.sample(rng)
});

/// Time from `since_epoch` until the next multiple of `interval`.
///
/// When `since_epoch` is already on a boundary, a full `interval` is returned
/// so the wait always lands on the *next* boundary. A zero interval returns zero.
pub fn duration_to_next_alignment(since_epoch: Duration, interval: Duration) -> Duration {
    let interval_nanos = interval.as_nanos();
    if interval_nanos == 0 {
        return Duration::ZERO;
    }
    let remainder = since_epoch.as_nanos() % interval_nanos;
    if remainder == 0 {
        interval
    } else {
        Duration::from_nanos((interval_nanos - remainder) as u64)
    }
}

pub struct TimeAlignWait {
    pub align_interval: Duration,
    pub verbose: Verbose,
}

impl WaitCondition for TimeAlignWait {
    fn sample(&self, _rng: &mut dyn RngCore) -> Result<Sample> {
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        Ok(Sample::of(duration_to_next_alignment(
            now,
            self.align_interval,
        )))
    }

    fn verbose(&self) -> Verbose {
        self.verbose
    }
}

pub struct UntilTimeWait {
    pub sleep_duration: Duration,
    pub verbose: Verbose,
}

impl WaitCondition for UntilTimeWait {
    fn sample(&self, _rng: &mut dyn RngCore) -> Result<Sample> {
        Ok(Sample::of(self.sleep_duration))
    }

    fn verbose(&self) -> Verbose {
        self.verbose
    }
}

/// Wraps any [`WaitCondition`] so that it only waits with the given probability.
///
/// The inner condition's own sample, including any jitter, is used unchanged
/// when the wait happens.
pub struct ProbabilityGate {
    pub inner: Box<dyn WaitCondition>,
    /// Chance of waiting, from 0.0 (never) to 1.0 (always).
    pub probability: f64,
    pub verbose: Verbose,
}

impl WaitCondition for ProbabilityGate {
    fn sample(&self, rng: &mut dyn RngCore) -> Result<Sample> {
        let roll: f64 = rng.random_range(0.0..1.0);
        // `roll` is in [0, 1), so probability 0.0 never waits and 1.0 always does.
        if roll < self.probability {
            self.inner.sample(rng)
        } else {
            Ok(Sample {
                duration: Duration::ZERO,
                clamped: false,
                skipped: Some(Skip {
                    probability: self.probability,
                    roll,
                }),
            })
        }
    }

    fn verbose(&self) -> Verbose {
        self.verbose
    }
}

/// Fixed-duration wait gated by a probability.
///
/// Superseded by [`ProbabilityGate`], which works with any condition and
/// keeps its jitter.
#[deprecated(
    since = "0.4.2",
    note = "use ProbabilityGate, which wraps any WaitCondition and keeps its jitter"
)]
pub struct ProbabilisticWait {
    pub duration: Duration,
    pub probability: f64,
    pub verbose: Verbose,
}

#[allow(deprecated)]
impl WaitCondition for ProbabilisticWait {
    fn sample(&self, rng: &mut dyn RngCore) -> Result<Sample> {
        ProbabilityGate {
            inner: Box::new(DurationWait {
                duration: self.duration,
                verbose: Verbose::Off,
                jitter: None,
            }),
            probability: self.probability,
            verbose: self.verbose,
        }
        .sample(rng)
    }

    fn verbose(&self) -> Verbose {
        self.verbose
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::{StdRng, ThreadRng};
    use std::time::Instant;

    const SAMPLES: usize = 20_000;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn secs(x: f64) -> Duration {
        Duration::from_secs_f64(x)
    }

    struct MockJitterGenerator {
        jitter: Duration,
    }

    impl JitterGenerator for MockJitterGenerator {
        fn generate(&mut self, _max_jitter: Duration) -> Duration {
            self.jitter
        }
    }

    /// Records every sample it is asked to perform instead of sleeping.
    #[derive(Default)]
    struct RecordingSleeper {
        performed: Vec<(Sample, Verbose)>,
    }

    impl Sleeper for RecordingSleeper {
        fn perform(&mut self, sample: &Sample, verbose: Verbose) {
            self.performed.push((*sample, verbose));
        }
    }

    /// Draws `SAMPLES` durations from `condition` with a fixed seed and
    /// returns their mean and standard deviation in seconds.
    fn sample_stats(condition: &dyn WaitCondition, seed: u64) -> (f64, f64) {
        let mut rng = StdRng::seed_from_u64(seed);
        let xs: Vec<f64> = (0..SAMPLES)
            .map(|_| condition.sample_with(&mut rng).unwrap().as_secs_f64())
            .collect();
        let mean = xs.iter().sum::<f64>() / xs.len() as f64;
        let var = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (xs.len() - 1) as f64;
        (mean, var.sqrt())
    }

    fn assert_close(label: &str, actual: f64, expected: f64, rel_tol: f64) {
        let err = (actual - expected).abs() / expected;
        assert!(
            err <= rel_tol,
            "{label}: got {actual:.4}, expected {expected:.4} (relative error {:.2}% > {:.2}%)",
            err * 100.0,
            rel_tol * 100.0
        );
    }

    /// One of every condition, with jitter where it applies.
    fn all_conditions() -> Vec<Box<dyn WaitCondition>> {
        let jitter = Some(ms(50));
        let verbose = Verbose::Off;
        vec![
            Box::new(DurationWait {
                duration: secs(1.0),
                verbose,
                jitter: Some(secs(1.0)),
            }),
            Box::new(NormalWait {
                mean: secs(1.0),
                std_dev: ms(100),
                verbose,
                jitter,
            }),
            Box::new(ExponentialWait {
                lambda: 1.0,
                verbose,
                jitter,
            }),
            Box::new(LogNormalWait {
                mean: secs(1.0),
                std_dev: ms(500),
                verbose,
                jitter,
            }),
            Box::new(ParetoWait {
                scale: secs(1.0),
                shape: 3.0,
                verbose,
                jitter,
            }),
            Box::new(UniformWait {
                min: secs(1.0),
                max: secs(2.0),
                verbose,
                jitter,
            }),
            Box::new(TriangularWait {
                min: secs(0.0),
                max: secs(2.0),
                mode: secs(1.0),
                verbose,
                jitter,
            }),
            Box::new(GammaWait {
                shape: 2.0,
                scale: secs(1.0),
                verbose,
                jitter,
            }),
            Box::new(ProbabilityGate {
                inner: Box::new(UniformWait {
                    min: secs(1.0),
                    max: secs(2.0),
                    verbose,
                    jitter,
                }),
                probability: 0.5,
                verbose,
            }),
        ]
    }

    #[test]
    fn test_calculate_sleep_duration_with_jitter() {
        let mut mock_gen = MockJitterGenerator { jitter: ms(1) };
        let wait_condition = DurationWait {
            duration: secs(1.0),
            jitter: Some(ms(500)),
            verbose: Verbose::Off,
        };
        let calculated = wait_condition.calculate_sleep_duration(&mut mock_gen);
        assert_eq!(calculated, ms(1001));
    }

    #[test]
    fn test_duration_to_next_alignment() {
        let s = Duration::from_secs;
        // Exactly on a boundary waits a full interval.
        assert_eq!(duration_to_next_alignment(s(100), s(10)), s(10));
        // Just past a boundary.
        assert_eq!(duration_to_next_alignment(s(103), s(10)), s(7));
        // Just before the next boundary.
        assert_eq!(duration_to_next_alignment(s(109), s(10)), s(1));
        // Zero interval.
        assert_eq!(duration_to_next_alignment(s(100), s(0)), s(0));
        // Larger interval.
        assert_eq!(duration_to_next_alignment(s(65), s(60)), s(55));
        // Sub-second precision is preserved.
        assert_eq!(duration_to_next_alignment(ms(1_250), ms(500)), ms(250));
    }

    #[test]
    fn test_jitter_generator_non_zero_max_jitter() {
        let mut jitter_gen = RandomJitterGenerator::new(ThreadRng::default());
        assert!(jitter_gen.generate(ms(100)) <= ms(100));
    }

    #[test]
    fn test_jitter_generator_zero_max_jitter() {
        let mut jitter_gen = RandomJitterGenerator::new(ThreadRng::default());
        assert_eq!(jitter_gen.generate(Duration::ZERO), Duration::ZERO);
    }

    #[test]
    fn test_seeded_sampling_is_deterministic() {
        for c in all_conditions() {
            let a = c.sample(&mut StdRng::seed_from_u64(42)).unwrap();
            let b = c.sample(&mut StdRng::seed_from_u64(42)).unwrap();
            assert_eq!(a, b);
        }
    }

    // P1-6: the duration handed to the sleeper is the one the condition
    // samples, drawn once.
    #[test]
    fn test_wait_sleeps_exactly_the_sampled_duration() {
        for (i, c) in all_conditions().iter().enumerate() {
            for seed in 0..20 {
                let expected = c.sample(&mut StdRng::seed_from_u64(seed)).unwrap();
                let mut sleeper = RecordingSleeper::default();
                c.wait_using(&mut StdRng::seed_from_u64(seed), &mut sleeper)
                    .unwrap();
                assert_eq!(
                    sleeper.performed,
                    vec![(expected, Verbose::Off)],
                    "condition {i}, seed {seed}"
                );
            }
        }
    }

    #[test]
    fn test_wait_passes_the_condition_verbose_setting() {
        let c = DurationWait {
            duration: ms(1),
            verbose: Verbose::Every(ms(250)),
            jitter: None,
        };
        let mut sleeper = RecordingSleeper::default();
        c.wait_using(&mut StdRng::seed_from_u64(0), &mut sleeper)
            .unwrap();
        assert_eq!(sleeper.performed[0].1, Verbose::Every(ms(250)));
    }

    #[test]
    fn test_normal_statistics() {
        let c = NormalWait {
            mean: secs(1.0),
            std_dev: ms(100),
            verbose: Verbose::Off,
            jitter: None,
        };
        let (mean, sd) = sample_stats(&c, 1);
        assert_close("normal mean", mean, 1.0, 0.02);
        assert_close("normal sd", sd, 0.1, 0.02);
    }

    #[test]
    fn test_exponential_statistics() {
        let c = ExponentialWait {
            lambda: 2.0,
            verbose: Verbose::Off,
            jitter: None,
        };
        let (mean, sd) = sample_stats(&c, 2);
        assert_close("exponential mean", mean, 0.5, 0.02);
        assert_close("exponential sd", sd, 0.5, 0.02);
    }

    #[test]
    fn test_log_normal_statistics_use_real_mean_and_sd() {
        let c = LogNormalWait {
            mean: secs(1.0),
            std_dev: ms(500),
            verbose: Verbose::Off,
            jitter: None,
        };
        let (mean, sd) = sample_stats(&c, 3);
        assert_close("log-normal mean", mean, 1.0, 0.03);
        assert_close("log-normal sd", sd, 0.5, 0.05);
    }

    #[test]
    fn test_log_normal_rejects_invalid_parameters() {
        let zero_mean = LogNormalWait {
            mean: Duration::ZERO,
            std_dev: ms(500),
            verbose: Verbose::Off,
            jitter: None,
        };
        assert!(zero_mean.calculate_wait_duration().is_err());

        let zero_sd = LogNormalWait {
            mean: secs(1.0),
            std_dev: Duration::ZERO,
            verbose: Verbose::Off,
            jitter: None,
        };
        let d = zero_sd.calculate_wait_duration().unwrap();
        assert!((d.as_secs_f64() - 1.0).abs() < 1e-9);

        assert!(log_normal_from_mean_sd(1.0, -0.1).is_err());
    }

    #[test]
    fn test_pareto_statistics() {
        // Shape 6 keeps the variance finite and its estimate stable.
        let (scale, shape) = (1.0, 6.0);
        let c = ParetoWait {
            scale: secs(scale),
            shape,
            verbose: Verbose::Off,
            jitter: None,
        };
        let (mean, sd) = sample_stats(&c, 4);
        let expected_mean = shape * scale / (shape - 1.0);
        let expected_sd = scale / (shape - 1.0) * (shape / (shape - 2.0)).sqrt();
        assert_close("pareto mean", mean, expected_mean, 0.02);
        assert_close("pareto sd", sd, expected_sd, 0.05);
    }

    #[test]
    fn test_uniform_statistics() {
        let c = UniformWait {
            min: secs(1.0),
            max: secs(3.0),
            verbose: Verbose::Off,
            jitter: None,
        };
        let (mean, sd) = sample_stats(&c, 5);
        assert_close("uniform mean", mean, 2.0, 0.02);
        assert_close("uniform sd", sd, 2.0 / 12f64.sqrt(), 0.02);
    }

    #[test]
    fn test_triangular_statistics() {
        let (a, b, m) = (1.0, 4.0, 2.0);
        let c = TriangularWait {
            min: secs(a),
            max: secs(b),
            mode: secs(m),
            verbose: Verbose::Off,
            jitter: None,
        };
        let (mean, sd) = sample_stats(&c, 6);
        let expected_var = (a * a + b * b + m * m - a * b - a * m - b * m) / 18.0;
        assert_close("triangular mean", mean, (a + b + m) / 3.0, 0.02);
        assert_close("triangular sd", sd, expected_var.sqrt(), 0.02);
    }

    #[test]
    fn test_gamma_statistics() {
        let (shape, scale) = (2.0, 1.5);
        let c = GammaWait {
            shape,
            scale: secs(scale),
            verbose: Verbose::Off,
            jitter: None,
        };
        let (mean, sd) = sample_stats(&c, 7);
        assert_close("gamma mean", mean, shape * scale, 0.02);
        assert_close("gamma sd", sd, shape.sqrt() * scale, 0.02);
    }

    #[test]
    fn test_uniform_samples_stay_in_range() {
        let c = UniformWait {
            min: secs(1.0),
            max: secs(2.0),
            verbose: Verbose::Off,
            jitter: None,
        };
        let mut rng = StdRng::seed_from_u64(8);
        for _ in 0..1_000 {
            let d = c.sample_with(&mut rng).unwrap();
            assert!(d >= secs(1.0) && d <= secs(2.0));
        }
    }

    #[test]
    fn test_triangular_samples_stay_in_range() {
        let c = TriangularWait {
            min: secs(1.0),
            max: secs(3.0),
            mode: secs(2.0),
            verbose: Verbose::Off,
            jitter: None,
        };
        let mut rng = StdRng::seed_from_u64(9);
        for _ in 0..1_000 {
            let d = c.sample_with(&mut rng).unwrap();
            assert!(d >= secs(1.0) && d <= secs(3.0));
        }
    }

    // P1-5: negative samples become zero and are flagged as clamped.
    #[test]
    fn test_negative_samples_clamp_to_zero_and_are_flagged() {
        let c = NormalWait {
            mean: Duration::ZERO,
            std_dev: secs(1.0),
            verbose: Verbose::Off,
            jitter: None,
        };
        let mut rng = StdRng::seed_from_u64(10);
        let samples: Vec<Sample> = (0..1_000).map(|_| c.sample(&mut rng).unwrap()).collect();
        let clamped = samples.iter().filter(|s| s.clamped).count();
        assert!(
            clamped > 300,
            "expected many clamped samples, got {clamped}"
        );
        for s in &samples {
            assert_eq!(s.clamped, s.duration.is_zero(), "{s:?}");
        }
    }

    #[test]
    fn test_clamped_sample_still_gets_jitter() {
        let c = NormalWait {
            mean: Duration::ZERO,
            std_dev: secs(1.0),
            verbose: Verbose::Off,
            jitter: Some(secs(1.0)),
        };
        let mut rng = StdRng::seed_from_u64(11);
        let clamped: Vec<Sample> = (0..1_000)
            .map(|_| c.sample(&mut rng).unwrap())
            .filter(|s| s.clamped)
            .collect();
        assert!(!clamped.is_empty());
        assert!(clamped.iter().any(|s| !s.duration.is_zero()));
        assert!(clamped.iter().all(|s| s.duration <= secs(1.0)));
    }

    #[test]
    fn test_positive_samples_are_not_flagged() {
        assert_eq!(secs_to_duration(1.5), (secs(1.5), false));
        assert_eq!(secs_to_duration(0.0), (Duration::ZERO, false));
        assert_eq!(secs_to_duration(-0.1), (Duration::ZERO, true));
        assert_eq!(secs_to_duration(f64::NAN), (Duration::ZERO, true));
    }

    fn gate(probability: f64, jitter: Option<Duration>) -> ProbabilityGate {
        ProbabilityGate {
            inner: Box::new(DurationWait {
                duration: ms(100),
                verbose: Verbose::Off,
                jitter,
            }),
            probability,
            verbose: Verbose::Off,
        }
    }

    #[test]
    fn test_probability_gate_keeps_inner_jitter() {
        let g = gate(1.0, Some(secs(3.0)));
        let mut rng = StdRng::seed_from_u64(12);
        let samples: Vec<Duration> = (0..200).map(|_| g.sample_with(&mut rng).unwrap()).collect();
        assert!(samples.iter().all(|d| *d >= ms(100) && *d <= ms(3_100)));
        assert!(
            samples.iter().any(|d| *d > secs(1.0)),
            "jitter was not applied through the gate"
        );
    }

    #[test]
    fn test_probability_gate_extremes() {
        let mut rng = StdRng::seed_from_u64(13);
        let never = gate(0.0, None);
        let always = gate(1.0, None);
        for _ in 0..1_000 {
            let skipped = never.sample(&mut rng).unwrap();
            assert_eq!(skipped.duration, Duration::ZERO);
            assert!(skipped.skipped.is_some());

            let waited = always.sample(&mut rng).unwrap();
            assert_eq!(waited, Sample::of(ms(100)));
        }
    }

    #[test]
    fn test_probability_gate_rate() {
        let g = gate(0.3, None);
        let mut rng = StdRng::seed_from_u64(14);
        let waited = (0..SAMPLES)
            .filter(|_| g.sample(&mut rng).unwrap().skipped.is_none())
            .count();
        assert_close("gate rate", waited as f64 / SAMPLES as f64, 0.3, 0.03);
    }

    #[test]
    fn test_probability_gate_wraps_any_condition() {
        let g = ProbabilityGate {
            inner: Box::new(NormalWait {
                mean: secs(5.0),
                std_dev: Duration::ZERO,
                verbose: Verbose::Off,
                jitter: None,
            }),
            probability: 0.0,
            verbose: Verbose::Off,
        };
        let start = Instant::now();
        g.wait().unwrap();
        assert!(start.elapsed() < ms(500));
    }

    #[test]
    #[allow(deprecated)]
    fn test_deprecated_probabilistic_wait_still_works() {
        let always = ProbabilisticWait {
            duration: ms(50),
            probability: 1.0,
            verbose: Verbose::Off,
        };
        let start = Instant::now();
        always.wait().unwrap();
        assert!(start.elapsed() >= ms(50));

        let never = ProbabilisticWait {
            duration: secs(5.0),
            probability: 0.0,
            verbose: Verbose::Off,
        };
        let start = Instant::now();
        never.wait().unwrap();
        assert!(start.elapsed() < ms(500));
    }
}
