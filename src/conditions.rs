use crate::{adaptive_verbose_wait, verbose_wait};
use anyhow::{Result, bail};
use rand::{Rng, RngCore};
use rand_distr::{Distribution, Exp, Gamma, LogNormal, Normal, Pareto, Triangular, Uniform};

use chrono::{DateTime, Local};

use std::time::{Duration, SystemTime};

/// Helper function to perform a wait with optional verbose output.
///
/// - If `verbose` is `None`, performs a simple sleep with no output.
/// - If `verbose` is `Some(duration)` where duration is 1ns, uses adaptive verbose output.
/// - Otherwise, uses fixed-interval verbose output with the specified update period.
fn perform_wait(sleep_duration: Duration, verbose: Option<Duration>) {
    match verbose {
        None => {
            std::thread::sleep(sleep_duration);
        }
        Some(update_period) => {
            let is_adaptive = update_period.as_nanos() == 1;
            let display_fn = |remaining: Duration| {
                let now: DateTime<Local> = Local::now();
                if remaining.is_zero() {
                    eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                } else {
                    eprintln!(
                        "[{}] [DOZR] Time remaining: {:.0}s",
                        now.format("%H:%M:%S"),
                        remaining.as_secs_f64()
                    );
                }
            };
            if is_adaptive {
                adaptive_verbose_wait(sleep_duration, display_fn);
            } else {
                verbose_wait(sleep_duration, update_period, display_fn);
            }
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

/// Converts a sampled number of seconds to a `Duration`, clamping negatives
/// and NaN to zero.
fn secs_to_duration(secs: f64) -> Duration {
    if secs.is_finite() && secs > 0.0 {
        Duration::from_secs_f64(secs)
    } else {
        Duration::ZERO
    }
}

/// A rule for deciding how long to wait, and the act of waiting.
pub trait WaitCondition {
    /// Calculates the wait duration using the thread-local random number generator.
    fn calculate_wait_duration(&self) -> Result<Duration>;

    /// Performs the wait.
    fn wait(&self) -> Result<()>;

    /// Calculates the wait duration using the supplied random number generator.
    ///
    /// Pass a seeded generator to get reproducible durations. Conditions with
    /// no randomness ignore `rng`; the default implementation does exactly that.
    fn sample_with(&self, rng: &mut dyn RngCore) -> Result<Duration> {
        let _ = rng;
        self.calculate_wait_duration()
    }
}

/// Implements `calculate_wait_duration` and `wait` for a condition whose
/// randomness lives entirely in `sample_with`.
macro_rules! sampled_wait_condition {
    ($ty:ty, |$self_:ident, $rng:ident| $body:block) => {
        impl WaitCondition for $ty {
            fn calculate_wait_duration(&self) -> Result<Duration> {
                self.sample_with(&mut rand::rng())
            }

            fn wait(&self) -> Result<()> {
                let sleep_duration = self.calculate_wait_duration()?;
                perform_wait(sleep_duration, self.verbose);
                Ok(())
            }

            fn sample_with(&$self_, $rng: &mut dyn RngCore) -> Result<Duration> $body
        }
    };
}

pub struct DurationWait {
    pub duration: Duration,
    pub verbose: Option<Duration>,
    pub jitter: Option<Duration>,
}

impl DurationWait {
    fn calculate_sleep_duration(&self, jitter_gen: &mut dyn JitterGenerator) -> Duration {
        let max_jitter = self.jitter.unwrap_or(Duration::ZERO);
        self.duration + jitter_gen.generate(max_jitter)
    }
}

sampled_wait_condition!(DurationWait, |self, rng| {
    let mut jitter_gen = RandomJitterGenerator::new(rng);
    Ok(self.calculate_sleep_duration(&mut jitter_gen))
});

pub struct NormalWait {
    pub mean: Duration,
    /// Standard deviation, in seconds.
    pub std_dev: f64,
    pub verbose: Option<Duration>,
    pub jitter: Option<Duration>,
}

sampled_wait_condition!(NormalWait, |self, rng| {
    let normal = Normal::new(self.mean.as_secs_f64(), self.std_dev)?;
    let base = secs_to_duration(normal.sample(rng));
    Ok(base + sample_jitter(rng, self.jitter))
});

pub struct ExponentialWait {
    /// Rate parameter, in events per second. The mean wait is `1 / lambda` seconds.
    pub lambda: f64,
    pub verbose: Option<Duration>,
    pub jitter: Option<Duration>,
}

sampled_wait_condition!(ExponentialWait, |self, rng| {
    let exponential = Exp::new(self.lambda)?;
    let base = secs_to_duration(exponential.sample(rng));
    Ok(base + sample_jitter(rng, self.jitter))
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
    /// Mean of the sampled waits.
    pub mean: Duration,
    /// Standard deviation of the sampled waits, in seconds.
    pub std_dev: f64,
    pub verbose: Option<Duration>,
    pub jitter: Option<Duration>,
}

sampled_wait_condition!(LogNormalWait, |self, rng| {
    let log_normal = log_normal_from_mean_sd(self.mean.as_secs_f64(), self.std_dev)?;
    let base = secs_to_duration(log_normal.sample(rng));
    Ok(base + sample_jitter(rng, self.jitter))
});

pub struct ParetoWait {
    pub scale: f64,
    pub shape: f64,
    pub verbose: Option<Duration>,
    pub jitter: Option<Duration>,
}

sampled_wait_condition!(ParetoWait, |self, rng| {
    let pareto = Pareto::new(self.scale, self.shape)?;
    let base = secs_to_duration(pareto.sample(rng));
    Ok(base + sample_jitter(rng, self.jitter))
});

pub struct UniformWait {
    pub min: Duration,
    pub max: Duration,
    pub verbose: Option<Duration>,
    pub jitter: Option<Duration>,
}

sampled_wait_condition!(UniformWait, |self, rng| {
    let uniform = Uniform::new(self.min.as_secs_f64(), self.max.as_secs_f64())?;
    let base = secs_to_duration(uniform.sample(rng));
    Ok(base + sample_jitter(rng, self.jitter))
});

pub struct TriangularWait {
    pub min: f64,
    pub max: f64,
    pub mode: f64,
    pub verbose: Option<Duration>,
    pub jitter: Option<Duration>,
}

sampled_wait_condition!(TriangularWait, |self, rng| {
    let triangular = Triangular::new(self.min, self.max, self.mode)?;
    let base = secs_to_duration(triangular.sample(rng));
    Ok(base + sample_jitter(rng, self.jitter))
});

pub struct GammaWait {
    pub shape: f64,
    pub scale: f64,
    pub verbose: Option<Duration>,
    pub jitter: Option<Duration>,
}

sampled_wait_condition!(GammaWait, |self, rng| {
    let gamma = Gamma::new(self.shape, self.scale)?;
    let base = secs_to_duration(gamma.sample(rng));
    Ok(base + sample_jitter(rng, self.jitter))
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
    pub verbose: Option<Duration>,
}

impl WaitCondition for TimeAlignWait {
    fn calculate_wait_duration(&self) -> Result<Duration> {
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        Ok(duration_to_next_alignment(now, self.align_interval))
    }

    fn wait(&self) -> Result<()> {
        let sleep_duration = self.calculate_wait_duration()?;
        perform_wait(sleep_duration, self.verbose);
        Ok(())
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
    pub verbose: Option<Duration>,
}

#[allow(deprecated)]
impl WaitCondition for ProbabilisticWait {
    fn calculate_wait_duration(&self) -> Result<Duration> {
        ProbabilityGate {
            inner: Box::new(DurationWait {
                duration: self.duration,
                verbose: None,
                jitter: None,
            }),
            probability: self.probability,
            verbose: self.verbose,
        }
        .calculate_wait_duration()
    }

    fn wait(&self) -> Result<()> {
        ProbabilityGate {
            inner: Box::new(DurationWait {
                duration: self.duration,
                verbose: None,
                jitter: None,
            }),
            probability: self.probability,
            verbose: self.verbose,
        }
        .wait()
    }
}

/// Wraps any [`WaitCondition`] so that it only waits with the given probability.
///
/// The inner condition's own duration, including any jitter, is used
/// unchanged when the wait happens.
pub struct ProbabilityGate {
    pub inner: Box<dyn WaitCondition>,
    /// Chance of waiting, from 0.0 (never) to 1.0 (always).
    pub probability: f64,
    pub verbose: Option<Duration>,
}

/// Outcome of a [`ProbabilityGate`] roll.
enum GateOutcome {
    Wait(Duration),
    Skip { roll: f64 },
}

impl ProbabilityGate {
    fn decide(&self, rng: &mut dyn RngCore) -> Result<GateOutcome> {
        let roll: f64 = rng.random_range(0.0..1.0);
        // `roll` is in [0, 1), so probability 0.0 never waits and 1.0 always does.
        if roll < self.probability {
            Ok(GateOutcome::Wait(self.inner.sample_with(rng)?))
        } else {
            Ok(GateOutcome::Skip { roll })
        }
    }
}

impl WaitCondition for ProbabilityGate {
    fn calculate_wait_duration(&self) -> Result<Duration> {
        self.sample_with(&mut rand::rng())
    }

    fn wait(&self) -> Result<()> {
        match self.decide(&mut rand::rng())? {
            GateOutcome::Wait(sleep_duration) => perform_wait(sleep_duration, self.verbose),
            GateOutcome::Skip { roll } => {
                if self.verbose.is_some() {
                    eprintln!(
                        "Probabilistic wait: Skipping sleep (probability: {}, roll: {:.2})",
                        self.probability, roll
                    );
                }
            }
        }
        Ok(())
    }

    fn sample_with(&self, rng: &mut dyn RngCore) -> Result<Duration> {
        match self.decide(rng)? {
            GateOutcome::Wait(d) => Ok(d),
            GateOutcome::Skip { .. } => Ok(Duration::ZERO),
        }
    }
}

pub struct UntilTimeWait {
    pub sleep_duration: Duration,
    pub verbose: Option<Duration>,
}

impl WaitCondition for UntilTimeWait {
    fn calculate_wait_duration(&self) -> Result<Duration> {
        Ok(self.sleep_duration)
    }

    fn wait(&self) -> Result<()> {
        let sleep_duration = self.calculate_wait_duration()?;
        perform_wait(sleep_duration, self.verbose);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::{StdRng, ThreadRng};
    use std::time::Instant;

    const SAMPLES: usize = 20_000;

    struct MockJitterGenerator {
        jitter: Duration,
    }

    impl JitterGenerator for MockJitterGenerator {
        fn generate(&mut self, _max_jitter: Duration) -> Duration {
            self.jitter
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

    #[test]
    fn test_calculate_sleep_duration_with_jitter() {
        let mut mock_gen = MockJitterGenerator {
            jitter: Duration::from_millis(1),
        };
        let wait_condition = DurationWait {
            duration: Duration::from_secs(1),
            jitter: Some(Duration::from_millis(500)),
            verbose: None,
        };
        let calculated = wait_condition.calculate_sleep_duration(&mut mock_gen);
        assert_eq!(calculated, Duration::from_millis(1001));
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
        assert_eq!(
            duration_to_next_alignment(Duration::from_millis(1_250), Duration::from_millis(500)),
            Duration::from_millis(250)
        );
    }

    #[test]
    fn test_jitter_generator_non_zero_max_jitter() {
        let mut jitter_gen = RandomJitterGenerator::new(ThreadRng::default());
        let max_jitter = Duration::from_millis(100);
        assert!(jitter_gen.generate(max_jitter) <= max_jitter);
    }

    #[test]
    fn test_jitter_generator_zero_max_jitter() {
        let mut jitter_gen = RandomJitterGenerator::new(ThreadRng::default());
        assert_eq!(jitter_gen.generate(Duration::ZERO), Duration::ZERO);
    }

    #[test]
    fn test_seeded_sampling_is_deterministic() {
        let conditions: Vec<Box<dyn WaitCondition>> = vec![
            Box::new(DurationWait {
                duration: Duration::from_secs(1),
                verbose: None,
                jitter: Some(Duration::from_secs(1)),
            }),
            Box::new(NormalWait {
                mean: Duration::from_secs(1),
                std_dev: 0.1,
                verbose: None,
                jitter: Some(Duration::from_millis(50)),
            }),
            Box::new(ExponentialWait {
                lambda: 1.0,
                verbose: None,
                jitter: None,
            }),
            Box::new(LogNormalWait {
                mean: Duration::from_secs(1),
                std_dev: 0.5,
                verbose: None,
                jitter: None,
            }),
            Box::new(ParetoWait {
                scale: 1.0,
                shape: 3.0,
                verbose: None,
                jitter: None,
            }),
            Box::new(UniformWait {
                min: Duration::from_secs(1),
                max: Duration::from_secs(2),
                verbose: None,
                jitter: None,
            }),
            Box::new(TriangularWait {
                min: 0.0,
                max: 2.0,
                mode: 1.0,
                verbose: None,
                jitter: None,
            }),
            Box::new(GammaWait {
                shape: 2.0,
                scale: 1.0,
                verbose: None,
                jitter: None,
            }),
        ];
        for c in &conditions {
            let a = c.sample_with(&mut StdRng::seed_from_u64(42)).unwrap();
            let b = c.sample_with(&mut StdRng::seed_from_u64(42)).unwrap();
            assert_eq!(a, b);
        }
    }

    #[test]
    fn test_normal_statistics() {
        let c = NormalWait {
            mean: Duration::from_secs(1),
            std_dev: 0.1,
            verbose: None,
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
            verbose: None,
            jitter: None,
        };
        let (mean, sd) = sample_stats(&c, 2);
        assert_close("exponential mean", mean, 0.5, 0.02);
        assert_close("exponential sd", sd, 0.5, 0.02);
    }

    #[test]
    fn test_log_normal_statistics_use_real_mean_and_sd() {
        let c = LogNormalWait {
            mean: Duration::from_secs(1),
            std_dev: 0.5,
            verbose: None,
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
            std_dev: 0.5,
            verbose: None,
            jitter: None,
        };
        assert!(zero_mean.calculate_wait_duration().is_err());

        let negative_sd = LogNormalWait {
            mean: Duration::from_secs(1),
            std_dev: -0.1,
            verbose: None,
            jitter: None,
        };
        assert!(negative_sd.calculate_wait_duration().is_err());

        let zero_sd = LogNormalWait {
            mean: Duration::from_secs(1),
            std_dev: 0.0,
            verbose: None,
            jitter: None,
        };
        let d = zero_sd.calculate_wait_duration().unwrap();
        assert!((d.as_secs_f64() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_pareto_statistics() {
        // Shape 6 keeps the variance finite and its estimate stable.
        let (scale, shape) = (1.0, 6.0);
        let c = ParetoWait {
            scale,
            shape,
            verbose: None,
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
            min: Duration::from_secs(1),
            max: Duration::from_secs(3),
            verbose: None,
            jitter: None,
        };
        let (mean, sd) = sample_stats(&c, 5);
        assert_close("uniform mean", mean, 2.0, 0.02);
        assert_close("uniform sd", sd, 2.0 / 12f64.sqrt(), 0.02);
    }

    #[test]
    fn test_triangular_statistics() {
        let (a, b, c_) = (1.0, 4.0, 2.0);
        let c = TriangularWait {
            min: a,
            max: b,
            mode: c_,
            verbose: None,
            jitter: None,
        };
        let (mean, sd) = sample_stats(&c, 6);
        let expected_var = (a * a + b * b + c_ * c_ - a * b - a * c_ - b * c_) / 18.0;
        assert_close("triangular mean", mean, (a + b + c_) / 3.0, 0.02);
        assert_close("triangular sd", sd, expected_var.sqrt(), 0.02);
    }

    #[test]
    fn test_gamma_statistics() {
        let (shape, scale) = (2.0, 1.5);
        let c = GammaWait {
            shape,
            scale,
            verbose: None,
            jitter: None,
        };
        let (mean, sd) = sample_stats(&c, 7);
        assert_close("gamma mean", mean, shape * scale, 0.02);
        assert_close("gamma sd", sd, shape.sqrt() * scale, 0.02);
    }

    #[test]
    fn test_uniform_samples_stay_in_range() {
        let c = UniformWait {
            min: Duration::from_secs(1),
            max: Duration::from_secs(2),
            verbose: None,
            jitter: None,
        };
        let mut rng = StdRng::seed_from_u64(8);
        for _ in 0..1_000 {
            let d = c.sample_with(&mut rng).unwrap();
            assert!(d >= Duration::from_secs(1) && d <= Duration::from_secs(2));
        }
    }

    #[test]
    fn test_triangular_samples_stay_in_range() {
        let c = TriangularWait {
            min: 1.0,
            max: 3.0,
            mode: 2.0,
            verbose: None,
            jitter: None,
        };
        let mut rng = StdRng::seed_from_u64(9);
        for _ in 0..1_000 {
            let d = c.sample_with(&mut rng).unwrap();
            assert!(d >= Duration::from_secs(1) && d <= Duration::from_secs(3));
        }
    }

    #[test]
    fn test_negative_samples_clamp_to_zero() {
        // Mean 0 means roughly half the raw samples are negative.
        let c = NormalWait {
            mean: Duration::ZERO,
            std_dev: 1.0,
            verbose: None,
            jitter: None,
        };
        let mut rng = StdRng::seed_from_u64(10);
        let zeros = (0..1_000)
            .filter(|_| c.sample_with(&mut rng).unwrap().is_zero())
            .count();
        assert!(zeros > 300, "expected many clamped samples, got {zeros}");
    }

    fn gate(probability: f64, jitter: Option<Duration>) -> ProbabilityGate {
        ProbabilityGate {
            inner: Box::new(DurationWait {
                duration: Duration::from_millis(100),
                verbose: None,
                jitter,
            }),
            probability,
            verbose: None,
        }
    }

    #[test]
    fn test_probability_gate_keeps_inner_jitter() {
        let g = gate(1.0, Some(Duration::from_secs(3)));
        let mut rng = StdRng::seed_from_u64(11);
        let samples: Vec<Duration> = (0..200).map(|_| g.sample_with(&mut rng).unwrap()).collect();
        assert!(
            samples
                .iter()
                .all(|d| *d >= Duration::from_millis(100) && *d <= Duration::from_millis(3_100))
        );
        assert!(
            samples.iter().any(|d| *d > Duration::from_secs(1)),
            "jitter was not applied through the gate"
        );
    }

    #[test]
    fn test_probability_gate_extremes() {
        let mut rng = StdRng::seed_from_u64(12);
        let never = gate(0.0, None);
        let always = gate(1.0, None);
        for _ in 0..1_000 {
            assert_eq!(never.sample_with(&mut rng).unwrap(), Duration::ZERO);
            assert_eq!(
                always.sample_with(&mut rng).unwrap(),
                Duration::from_millis(100)
            );
        }
    }

    #[test]
    fn test_probability_gate_rate() {
        let g = gate(0.3, None);
        let mut rng = StdRng::seed_from_u64(13);
        let waited = (0..SAMPLES)
            .filter(|_| !g.sample_with(&mut rng).unwrap().is_zero())
            .count();
        assert_close("gate rate", waited as f64 / SAMPLES as f64, 0.3, 0.03);
    }

    #[test]
    fn test_probability_gate_wraps_any_condition() {
        let g = ProbabilityGate {
            inner: Box::new(NormalWait {
                mean: Duration::from_secs(5),
                std_dev: 0.0,
                verbose: None,
                jitter: None,
            }),
            probability: 0.0,
            verbose: None,
        };
        let start = Instant::now();
        g.wait().unwrap();
        assert!(start.elapsed() < Duration::from_millis(500));
    }

    #[test]
    #[allow(deprecated)]
    fn test_deprecated_probabilistic_wait_still_works() {
        let always = ProbabilisticWait {
            duration: Duration::from_millis(50),
            probability: 1.0,
            verbose: None,
        };
        let start = Instant::now();
        always.wait().unwrap();
        assert!(start.elapsed() >= Duration::from_millis(50));

        let never = ProbabilisticWait {
            duration: Duration::from_secs(5),
            probability: 0.0,
            verbose: None,
        };
        let start = Instant::now();
        never.wait().unwrap();
        assert!(start.elapsed() < Duration::from_millis(500));
    }
}
