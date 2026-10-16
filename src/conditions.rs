//! Wait conditions: what to wait for, and the wait itself.
//!
//! A [`Wait`] combines a [`Sampler`], which picks a base duration, with
//! optional jitter, an optional probability gate, and a progress setting.
//! Drawing a wait produces a [`Sample`]; performing it hands that sample to a
//! [`Sleeper`]. Tests substitute their own random number generator, sleeper,
//! and clock so they can check exactly what would happen without waiting.
//!
//! ```
//! use dozr::conditions::{Sampler, Wait};
//! use rand::SeedableRng;
//! use std::time::Duration;
//!
//! let wait = Wait::new(Sampler::Uniform {
//!     min: Duration::from_millis(100),
//!     max: Duration::from_millis(200),
//! })
//! .with_jitter(Duration::from_millis(50));
//!
//! let mut rng = rand::rngs::StdRng::seed_from_u64(7);
//! let sample = wait.sample(&mut rng).unwrap();
//! assert!(sample.duration >= Duration::from_millis(100));
//! assert!(sample.duration <= Duration::from_millis(250));
//! ```

use crate::progress::{
    Clock, MIN_VERBOSE_PERIOD, SystemClock, Verbose, adaptive_verbose_wait_with, verbose_wait_with,
};
use anyhow::{Result, bail};
use chrono::{
    DateTime, Duration as ChronoDuration, Local, LocalResult, NaiveTime, Offset, TimeZone,
};
use rand::{Rng, RngCore};
use rand_distr::{Distribution, Exp, Gamma, LogNormal, Normal, Pareto, Triangular, Uniform};
use std::io::Write;
use std::time::Duration;

/// Picks the base duration of a wait.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sampler {
    /// Always the same duration.
    Fixed(Duration),
    /// Normal distribution. Negative samples become zero.
    Normal { mean: Duration, std_dev: Duration },
    /// Exponential distribution with rate `lambda` per second (mean `1 / lambda` seconds).
    Exponential { lambda: f64 },
    /// Log-normal distribution with the given mean and standard deviation of the waits.
    LogNormal { mean: Duration, std_dev: Duration },
    /// Pareto distribution with minimum wait `scale` and tail index `shape`.
    Pareto { scale: Duration, shape: f64 },
    /// Uniform distribution on `[min, max)`.
    Uniform { min: Duration, max: Duration },
    /// Triangular distribution between `min` and `max`, peaking at `mode`.
    Triangular {
        min: Duration,
        max: Duration,
        mode: Duration,
    },
    /// Gamma distribution; the mean wait is `shape` times `scale`.
    Gamma { shape: f64, scale: Duration },
    /// Until the next multiple of the interval since the Unix epoch.
    Align(Duration),
    /// Until the next occurrence of a local wall-clock time.
    At(NaiveTime),
}

fn fmt(d: Duration) -> humantime::FormattedDuration {
    humantime::format_duration(d)
}

fn require_positive(what: &str, v: f64) -> Result<()> {
    if v.is_finite() && v > 0.0 {
        Ok(())
    } else {
        bail!("{what} must be greater than 0, got {v}")
    }
}

impl Sampler {
    /// Checks the parameters, naming the offending one on failure.
    pub fn validate(&self) -> Result<()> {
        match *self {
            Sampler::Fixed(_) | Sampler::Normal { .. } | Sampler::At(_) => Ok(()),
            Sampler::Exponential { lambda } => require_positive("exponential <LAMBDA>", lambda),
            Sampler::LogNormal { mean, .. } if mean.is_zero() => {
                bail!("log-normal <MEAN> must be greater than 0")
            }
            Sampler::LogNormal { .. } => Ok(()),
            Sampler::Pareto { scale, .. } if scale.is_zero() => {
                bail!("pareto <SCALE> must be greater than 0")
            }
            Sampler::Pareto { shape, .. } => require_positive("pareto <SHAPE>", shape),
            Sampler::Uniform { min, max } if min >= max => bail!(
                "uniform <MIN> must be less than <MAX>, got {} and {}",
                fmt(min),
                fmt(max)
            ),
            Sampler::Uniform { .. } => Ok(()),
            Sampler::Triangular { min, max, .. } if min >= max => bail!(
                "triangular <MIN> must be less than <MAX>, got {} and {}",
                fmt(min),
                fmt(max)
            ),
            Sampler::Triangular { min, max, mode } if mode < min || mode > max => bail!(
                "triangular <MODE> must be between <MIN> and <MAX>, got {} outside {}..{}",
                fmt(mode),
                fmt(min),
                fmt(max)
            ),
            Sampler::Triangular { .. } => Ok(()),
            Sampler::Gamma { shape, .. } if !(shape.is_finite() && shape > 0.0) => {
                require_positive("gamma <SHAPE>", shape)
            }
            Sampler::Gamma { scale, .. } if scale.is_zero() => {
                bail!("gamma <SCALE> must be greater than 0")
            }
            Sampler::Gamma { .. } => Ok(()),
            Sampler::Align(interval) if interval.is_zero() => {
                bail!("align <INTERVAL> must be greater than 0")
            }
            Sampler::Align(_) => Ok(()),
        }
    }

    /// Draws a base duration at time `now`. The flag is true when a negative
    /// value was raised to zero.
    fn draw(&self, rng: &mut dyn RngCore, now: &DateTime<Local>) -> Result<(Duration, bool)> {
        let secs = match *self {
            Sampler::Fixed(d) => return Ok((d, false)),
            Sampler::Align(interval) => {
                let since_epoch = now
                    .signed_duration_since(DateTime::UNIX_EPOCH)
                    .to_std()
                    .unwrap_or_default();
                return Ok((duration_to_next_alignment(since_epoch, interval), false));
            }
            Sampler::At(time) => {
                return duration_until(now, time)
                    .map(|d| (d, false))
                    .map_err(anyhow::Error::msg);
            }
            Sampler::Normal { mean, std_dev } => {
                Normal::new(mean.as_secs_f64(), std_dev.as_secs_f64())?.sample(rng)
            }
            Sampler::Exponential { lambda } => Exp::new(lambda)?.sample(rng),
            Sampler::LogNormal { mean, std_dev } => {
                log_normal_from_mean_sd(mean.as_secs_f64(), std_dev.as_secs_f64())?.sample(rng)
            }
            Sampler::Pareto { scale, shape } => {
                Pareto::new(scale.as_secs_f64(), shape)?.sample(rng)
            }
            Sampler::Uniform { min, max } => {
                Uniform::new(min.as_secs_f64(), max.as_secs_f64())?.sample(rng)
            }
            Sampler::Triangular { min, max, mode } => {
                Triangular::new(min.as_secs_f64(), max.as_secs_f64(), mode.as_secs_f64())?
                    .sample(rng)
            }
            Sampler::Gamma { shape, scale } => Gamma::new(shape, scale.as_secs_f64())?.sample(rng),
        };
        Ok(secs_to_duration(secs))
    }
}

/// Converts a sampled number of seconds to a `Duration`.
///
/// Negative values and NaN become zero and are reported as clamped. Values
/// too large for a `Duration`, including infinity, saturate to
/// `Duration::MAX` rather than panicking.
fn secs_to_duration(secs: f64) -> (Duration, bool) {
    if secs.is_nan() || secs < 0.0 {
        (Duration::ZERO, true)
    } else {
        (
            Duration::try_from_secs_f64(secs).unwrap_or(Duration::MAX),
            false,
        )
    }
}

/// Draws up to `max_jitter` of uniform random jitter from `rng`.
fn sample_jitter(rng: &mut dyn RngCore, max_jitter: Duration) -> Duration {
    if max_jitter.is_zero() {
        return Duration::ZERO;
    }
    let max_nanos = u64::try_from(max_jitter.as_nanos()).unwrap_or(u64::MAX);
    Duration::from_nanos(rng.random_range(0..=max_nanos))
}

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

/// Time from `now` until the next occurrence of the wall-clock time `target`
/// in `now`'s time zone.
///
/// - A target equal to `now` returns zero; an earlier target rolls to tomorrow.
/// - A target inside a daylight-saving gap (a local time that does not exist)
///   is moved forward by the length of the gap, so 02:30 on a spring-forward
///   night becomes 03:30.
/// - A target that occurs twice (fall back) uses the earliest occurrence that
///   is not in the past.
pub fn duration_until<Tz: TimeZone>(
    now: &DateTime<Tz>,
    target: NaiveTime,
) -> std::result::Result<Duration, String> {
    let tz = now.timezone();
    let mut date = now.date_naive();

    // Two days is always enough: today, or failing that tomorrow. The third
    // iteration only guards against a zone that skips an entire day.
    for _ in 0..3 {
        let naive = date.and_time(target);
        let candidate = match tz.from_local_datetime(&naive) {
            LocalResult::Single(dt) => (dt >= *now).then_some(dt),
            LocalResult::Ambiguous(earliest, latest) => {
                if earliest >= *now {
                    Some(earliest)
                } else if latest >= *now {
                    Some(latest)
                } else {
                    None
                }
            }
            LocalResult::None => {
                // Interpret the wall time using the offset in force before the
                // gap, which lands just after it.
                let before = tz.offset_from_utc_datetime(&(naive - ChronoDuration::days(1)));
                let utc =
                    naive - ChronoDuration::seconds(i64::from(before.fix().local_minus_utc()));
                let dt = tz.from_utc_datetime(&utc);
                (dt >= *now).then_some(dt)
            }
        };

        if let Some(target_dt) = candidate {
            return target_dt
                .signed_duration_since(now.clone())
                .to_std()
                .map_err(|e| e.to_string());
        }
        date = date
            .succ_opt()
            .ok_or_else(|| "date out of range".to_string())?;
    }
    Err(format!("could not resolve {target} to a future time"))
}

/// One draw from a [`Wait`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    /// How long to wait.
    pub duration: Duration,
    /// The sampler produced a negative value, which was raised to zero
    /// before any jitter was added.
    pub clamped: bool,
    /// Set when the probability gate decided not to wait.
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

/// A complete wait: a sampler plus jitter, probability, and progress settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wait {
    pub sampler: Sampler,
    /// Up to this much uniform random time is added to every sample.
    pub jitter: Option<Duration>,
    /// Chance of waiting at all, from 0.0 (never) to 1.0 (always).
    pub probability: Option<f64>,
    pub verbose: Verbose,
}

impl Wait {
    /// A wait with no jitter, no probability gate, and no progress output.
    pub fn new(sampler: Sampler) -> Self {
        Wait {
            sampler,
            jitter: None,
            probability: None,
            verbose: Verbose::Off,
        }
    }

    pub fn with_jitter(mut self, max_jitter: Duration) -> Self {
        self.jitter = Some(max_jitter);
        self
    }

    pub fn with_probability(mut self, probability: f64) -> Self {
        self.probability = Some(probability);
        self
    }

    pub fn with_verbose(mut self, verbose: Verbose) -> Self {
        self.verbose = verbose;
        self
    }

    /// Checks every parameter, naming the offending one on failure.
    pub fn validate(&self) -> Result<()> {
        self.sampler.validate()?;
        // Written without let-chains to keep the 1.85 minimum Rust version.
        match self.probability {
            Some(p) if !(0.0..=1.0).contains(&p) => {
                bail!("--probability must be between 0.0 and 1.0, got {p}")
            }
            _ => {}
        }
        match self.verbose {
            Verbose::Every(period) if period < MIN_VERBOSE_PERIOD => bail!(
                "--verbose update period must be at least {}",
                fmt(MIN_VERBOSE_PERIOD)
            ),
            _ => Ok(()),
        }
    }

    /// Draws one sample at the current local time.
    pub fn sample(&self, rng: &mut dyn RngCore) -> Result<Sample> {
        self.sample_at(rng, &Local::now())
    }

    /// Draws one sample as if the local time were `now`.
    ///
    /// The probability gate is rolled first; a skipped wait draws nothing
    /// else. Otherwise the sampler is drawn and jitter added.
    pub fn sample_at(&self, rng: &mut dyn RngCore, now: &DateTime<Local>) -> Result<Sample> {
        self.validate()?;
        if let Some(probability) = self.probability {
            let roll: f64 = rng.random_range(0.0..1.0);
            // `roll` is in [0, 1), so 0.0 never waits and 1.0 always does.
            if roll >= probability {
                return Ok(Sample {
                    duration: Duration::ZERO,
                    clamped: false,
                    skipped: Some(Skip { probability, roll }),
                });
            }
        }
        let (base, clamped) = self.sampler.draw(rng, now)?;
        let jitter = sample_jitter(rng, self.jitter.unwrap_or_default());
        Ok(Sample {
            duration: base.saturating_add(jitter),
            clamped,
            skipped: None,
        })
    }

    /// Draws a duration using the thread-local random number generator.
    pub fn calculate(&self) -> Result<Duration> {
        Ok(self.sample(&mut rand::rng())?.duration)
    }

    /// Draws exactly one sample and passes it to `sleeper`.
    pub fn wait_using(&self, rng: &mut dyn RngCore, sleeper: &mut dyn Sleeper) -> Result<()> {
        let sample = self.sample(rng)?;
        sleeper.perform(&sample, self.verbose);
        Ok(())
    }

    /// Performs the wait, reporting progress on stderr.
    pub fn wait(&self) -> Result<()> {
        self.wait_using(&mut rand::rng(), &mut ConsoleSleeper::stderr())
    }
}

/// Carries out a [`Sample`]: reports on it and sleeps for it.
pub trait Sleeper {
    fn perform(&mut self, sample: &Sample, verbose: Verbose);
}

/// A [`Sleeper`] that writes timestamped progress lines to `out` and sleeps
/// on `clock`.
pub struct ConsoleSleeper<W: Write, C: Clock> {
    pub out: W,
    pub clock: C,
}

impl ConsoleSleeper<std::io::Stderr, SystemClock> {
    /// Progress on stderr, real sleeping.
    pub fn stderr() -> Self {
        ConsoleSleeper {
            out: std::io::stderr(),
            clock: SystemClock::new(),
        }
    }
}

impl<W: Write, C: Clock> Sleeper for ConsoleSleeper<W, C> {
    fn perform(&mut self, sample: &Sample, verbose: Verbose) {
        // Progress output is best effort: a closed stderr must not stop the wait.
        let out = &mut self.out;
        let stamp = || Local::now().format("%H:%M:%S");

        if let Some(skip) = sample.skipped {
            if verbose.is_on() {
                let _ = writeln!(
                    out,
                    "Probabilistic wait: Skipping sleep (probability: {}, roll: {:.2})",
                    skip.probability, skip.roll
                );
            }
            return;
        }
        if sample.clamped && verbose.is_on() {
            let _ = writeln!(
                out,
                "[{}] [DOZR] Sampled duration was negative and was clamped to zero.",
                stamp()
            );
        }

        let mut report = |remaining: Duration, decimals: usize| {
            let _ = if remaining.is_zero() {
                writeln!(out, "[{}] Wait complete.", stamp())
            } else {
                writeln!(
                    out,
                    "[{}] [DOZR] Time remaining: {:.*}s",
                    stamp(),
                    decimals,
                    remaining.as_secs_f64()
                )
            };
        };
        match verbose {
            Verbose::Off => self.clock.sleep(sample.duration),
            Verbose::Adaptive => {
                adaptive_verbose_wait_with(&mut self.clock, sample.duration, |r| report(r, 0))
            }
            Verbose::Every(period) => {
                let decimals = if period < Duration::from_secs(1) {
                    1
                } else {
                    0
                };
                verbose_wait_with(&mut self.clock, sample.duration, period, |r| {
                    report(r, decimals)
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::tests::FakeClock;
    use chrono::{FixedOffset, Utc};
    use chrono_tz::America::New_York;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    const SAMPLES: usize = 20_000;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn secs(x: f64) -> Duration {
        Duration::from_secs_f64(x)
    }

    fn rng(seed: u64) -> StdRng {
        StdRng::seed_from_u64(seed)
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

    /// Runs `sample` through a console sleeper with a fake clock and
    /// returns what it printed and how long it slept in total.
    fn console(sample: Sample, verbose: Verbose) -> (String, Duration) {
        let mut sleeper = ConsoleSleeper {
            out: Vec::new(),
            clock: FakeClock::default(),
        };
        sleeper.perform(&sample, verbose);
        (String::from_utf8(sleeper.out).unwrap(), sleeper.clock.now)
    }

    /// One of every random sampler, with its expected mean and standard
    /// deviation in seconds and the relative tolerance for the sd check.
    fn random_samplers() -> Vec<(&'static str, Sampler, f64, f64, f64)> {
        let tri = (1.0, 4.0, 2.0);
        let tri_var = (tri.0 * tri.0 + tri.1 * tri.1 + tri.2 * tri.2
            - tri.0 * tri.1
            - tri.0 * tri.2
            - tri.1 * tri.2)
            / 18.0;
        // Pareto shape 6 keeps the variance finite and its estimate stable.
        let (p_scale, p_shape) = (1.0, 6.0);
        vec![
            (
                "normal",
                Sampler::Normal {
                    mean: secs(1.0),
                    std_dev: ms(100),
                },
                1.0,
                0.1,
                0.02,
            ),
            (
                "exponential",
                Sampler::Exponential { lambda: 2.0 },
                0.5,
                0.5,
                0.02,
            ),
            (
                "log-normal",
                Sampler::LogNormal {
                    mean: secs(1.0),
                    std_dev: ms(500),
                },
                1.0,
                0.5,
                0.05,
            ),
            (
                "pareto",
                Sampler::Pareto {
                    scale: secs(p_scale),
                    shape: p_shape,
                },
                p_shape * p_scale / (p_shape - 1.0),
                p_scale / (p_shape - 1.0) * (p_shape / (p_shape - 2.0)).sqrt(),
                0.05,
            ),
            (
                "uniform",
                Sampler::Uniform {
                    min: secs(1.0),
                    max: secs(3.0),
                },
                2.0,
                2.0 / 12f64.sqrt(),
                0.02,
            ),
            (
                "triangular",
                Sampler::Triangular {
                    min: secs(tri.0),
                    max: secs(tri.1),
                    mode: secs(tri.2),
                },
                (tri.0 + tri.1 + tri.2) / 3.0,
                tri_var.sqrt(),
                0.02,
            ),
            (
                "gamma",
                Sampler::Gamma {
                    shape: 2.0,
                    scale: secs(1.5),
                },
                3.0,
                2f64.sqrt() * 1.5,
                0.02,
            ),
        ]
    }

    /// Every sampler variant, including the deterministic ones.
    fn all_samplers() -> Vec<Sampler> {
        let mut all: Vec<Sampler> = random_samplers().into_iter().map(|t| t.1).collect();
        all.push(Sampler::Fixed(secs(1.0)));
        all.push(Sampler::Align(secs(60.0)));
        all.push(Sampler::At(NaiveTime::from_hms_opt(12, 0, 0).unwrap()));
        all
    }

    fn noon() -> DateTime<Local> {
        Local.with_ymd_and_hms(2025, 6, 1, 11, 59, 30).unwrap()
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
    fn test_sampler_statistics() {
        for (seed, (name, sampler, mean, sd, sd_tol)) in random_samplers().into_iter().enumerate() {
            let wait = Wait::new(sampler);
            let mut r = rng(seed as u64);
            let xs: Vec<f64> = (0..SAMPLES)
                .map(|_| wait.sample(&mut r).unwrap().duration.as_secs_f64())
                .collect();
            let m = xs.iter().sum::<f64>() / xs.len() as f64;
            let v = xs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (xs.len() - 1) as f64;
            let mean_tol = if name == "log-normal" { 0.03 } else { 0.02 };
            assert_close(&format!("{name} mean"), m, mean, mean_tol);
            assert_close(&format!("{name} sd"), v.sqrt(), sd, sd_tol);
        }
    }

    #[test]
    fn test_seeded_sampling_is_deterministic() {
        for sampler in all_samplers() {
            let wait = Wait::new(sampler).with_jitter(ms(50)).with_probability(0.5);
            let now = noon();
            for seed in 0..20 {
                let a = wait.sample_at(&mut rng(seed), &now).unwrap();
                let b = wait.sample_at(&mut rng(seed), &now).unwrap();
                assert_eq!(a, b, "{sampler:?}");
            }
        }
    }

    // P2-2: jitter and probability apply to every sampler.
    #[test]
    fn test_jitter_applies_to_every_sampler() {
        let now = noon();
        for sampler in all_samplers() {
            let plain = Wait::new(sampler);
            let jittered = plain.with_jitter(secs(1.0));
            let mut saw_large_jitter = false;
            for seed in 0..50 {
                let base = plain.sample_at(&mut rng(seed), &now).unwrap().duration;
                let total = jittered.sample_at(&mut rng(seed), &now).unwrap().duration;
                // Jitter is drawn after the base, so the same seed gives the
                // same base and the difference is the jitter alone.
                let jitter = total - base;
                assert!(jitter <= secs(1.0), "{sampler:?}: {jitter:?}");
                saw_large_jitter |= jitter > ms(500);
            }
            assert!(saw_large_jitter, "{sampler:?}: jitter never exceeded 500ms");
        }
    }

    #[test]
    fn test_probability_gates_every_sampler() {
        let now = noon();
        for sampler in all_samplers() {
            let never = Wait::new(sampler).with_probability(0.0);
            let always = Wait::new(sampler).with_probability(1.0);
            let ungated = Wait::new(sampler);
            for seed in 0..50 {
                let skipped = never.sample_at(&mut rng(seed), &now).unwrap();
                assert_eq!(skipped.duration, Duration::ZERO, "{sampler:?}");
                assert!(skipped.skipped.is_some(), "{sampler:?}");

                let waited = always.sample_at(&mut rng(seed), &now).unwrap();
                assert!(waited.skipped.is_none(), "{sampler:?}");
                if matches!(
                    sampler,
                    Sampler::Fixed(_) | Sampler::Align(_) | Sampler::At(_)
                ) {
                    let expected = ungated.sample_at(&mut rng(seed), &now).unwrap();
                    assert_eq!(waited.duration, expected.duration, "{sampler:?}");
                }
            }
        }
    }

    #[test]
    fn test_probability_rate() {
        let wait = Wait::new(Sampler::Fixed(ms(100))).with_probability(0.3);
        let mut r = rng(14);
        let waited = (0..SAMPLES)
            .filter(|_| wait.sample(&mut r).unwrap().skipped.is_none())
            .count();
        assert_close("gate rate", waited as f64 / SAMPLES as f64, 0.3, 0.03);
    }

    #[test]
    fn test_deterministic_samplers() {
        let now = noon();
        let draw = |s: Sampler| Wait::new(s).sample_at(&mut rng(0), &now).unwrap();
        assert_eq!(draw(Sampler::Fixed(ms(250))), Sample::of(ms(250)));
        assert_eq!(
            draw(Sampler::At(NaiveTime::from_hms_opt(12, 0, 0).unwrap())),
            Sample::of(secs(30.0))
        );
        let aligned = draw(Sampler::Align(secs(60.0))).duration;
        assert!(aligned > Duration::ZERO && aligned <= secs(60.0));
    }

    #[test]
    fn test_range_samplers_stay_in_range() {
        let cases = [
            (
                Sampler::Uniform {
                    min: secs(1.0),
                    max: secs(2.0),
                },
                secs(1.0),
                secs(2.0),
            ),
            (
                Sampler::Triangular {
                    min: secs(1.0),
                    max: secs(3.0),
                    mode: secs(2.0),
                },
                secs(1.0),
                secs(3.0),
            ),
            (
                Sampler::Pareto {
                    scale: secs(1.0),
                    shape: 3.0,
                },
                secs(1.0),
                Duration::MAX,
            ),
        ];
        for (sampler, lo, hi) in cases {
            let wait = Wait::new(sampler);
            let mut r = rng(8);
            for _ in 0..1_000 {
                let d = wait.sample(&mut r).unwrap().duration;
                assert!(d >= lo && d <= hi, "{sampler:?}: {d:?}");
            }
        }
    }

    // P1-6: the duration handed to the sleeper is the one sampled, drawn once.
    #[test]
    fn test_wait_sleeps_exactly_the_sampled_duration() {
        for sampler in all_samplers() {
            if matches!(sampler, Sampler::Align(_) | Sampler::At(_)) {
                continue; // depend on the real clock between the two calls
            }
            let wait = Wait::new(sampler)
                .with_jitter(ms(50))
                .with_probability(0.5)
                .with_verbose(Verbose::Every(ms(250)));
            for seed in 0..20 {
                let expected = wait.sample(&mut rng(seed)).unwrap();
                let mut sleeper = RecordingSleeper::default();
                wait.wait_using(&mut rng(seed), &mut sleeper).unwrap();
                assert_eq!(
                    sleeper.performed,
                    vec![(expected, Verbose::Every(ms(250)))],
                    "{sampler:?}, seed {seed}"
                );
            }
        }
    }

    // P1-5: negative samples become zero and are flagged as clamped.
    #[test]
    fn test_negative_samples_clamp_to_zero_and_are_flagged() {
        let wait = Wait::new(Sampler::Normal {
            mean: Duration::ZERO,
            std_dev: secs(1.0),
        });
        let mut r = rng(10);
        let samples: Vec<Sample> = (0..1_000).map(|_| wait.sample(&mut r).unwrap()).collect();
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
        let wait = Wait::new(Sampler::Normal {
            mean: Duration::ZERO,
            std_dev: secs(1.0),
        })
        .with_jitter(secs(1.0));
        let mut r = rng(11);
        let clamped: Vec<Sample> = (0..1_000)
            .map(|_| wait.sample(&mut r).unwrap())
            .filter(|s| s.clamped)
            .collect();
        assert!(!clamped.is_empty());
        assert!(clamped.iter().any(|s| !s.duration.is_zero()));
        assert!(clamped.iter().all(|s| s.duration <= secs(1.0)));
    }

    #[test]
    fn test_secs_to_duration() {
        assert_eq!(secs_to_duration(1.5), (secs(1.5), false));
        assert_eq!(secs_to_duration(0.0), (Duration::ZERO, false));
        assert_eq!(secs_to_duration(-0.1), (Duration::ZERO, true));
        assert_eq!(secs_to_duration(f64::NAN), (Duration::ZERO, true));
        assert_eq!(secs_to_duration(1e30), (Duration::MAX, false));
        assert_eq!(secs_to_duration(f64::INFINITY), (Duration::MAX, false));
    }

    // Extreme parameters used to panic converting the sample to a Duration.
    #[test]
    fn test_extreme_samples_saturate_instead_of_panicking() {
        let samplers = [
            Sampler::Exponential { lambda: 1e-30 },
            Sampler::Pareto {
                scale: secs(1.0),
                shape: 0.001,
            },
            Sampler::Gamma {
                shape: 1e300,
                scale: secs(1.0),
            },
        ];
        for sampler in samplers {
            let wait = Wait::new(sampler).with_jitter(secs(1.0));
            let mut r = rng(15);
            let draws: Vec<Duration> = (0..100)
                .map(|_| wait.sample(&mut r).unwrap().duration)
                .collect();
            assert!(draws.iter().all(|d| *d >= secs(1.0)), "{sampler:?}");
            assert!(
                draws.contains(&Duration::MAX),
                "{sampler:?}: no draw reached the cap"
            );
        }
        let huge = Wait::new(Sampler::Fixed(Duration::MAX)).with_jitter(secs(1.0));
        assert_eq!(huge.sample(&mut rng(0)).unwrap().duration, Duration::MAX);
    }

    #[test]
    fn test_validate_rejects_bad_parameters() {
        let bad = [
            (Sampler::Exponential { lambda: 0.0 }, "<LAMBDA>"),
            (Sampler::Exponential { lambda: f64::NAN }, "<LAMBDA>"),
            (
                Sampler::LogNormal {
                    mean: Duration::ZERO,
                    std_dev: secs(1.0),
                },
                "<MEAN>",
            ),
            (
                Sampler::Pareto {
                    scale: Duration::ZERO,
                    shape: 1.0,
                },
                "<SCALE>",
            ),
            (
                Sampler::Pareto {
                    scale: secs(1.0),
                    shape: -1.0,
                },
                "<SHAPE>",
            ),
            (
                Sampler::Uniform {
                    min: secs(2.0),
                    max: secs(2.0),
                },
                "<MIN>",
            ),
            (
                Sampler::Triangular {
                    min: secs(1.0),
                    max: secs(1.0),
                    mode: secs(1.0),
                },
                "<MIN>",
            ),
            (
                Sampler::Triangular {
                    min: secs(1.0),
                    max: secs(2.0),
                    mode: secs(3.0),
                },
                "<MODE>",
            ),
            (
                Sampler::Gamma {
                    shape: 0.0,
                    scale: secs(1.0),
                },
                "<SHAPE>",
            ),
            (
                Sampler::Gamma {
                    shape: 1.0,
                    scale: Duration::ZERO,
                },
                "<SCALE>",
            ),
            (Sampler::Align(Duration::ZERO), "<INTERVAL>"),
        ];
        for (sampler, names) in bad {
            let err = Wait::new(sampler).validate().unwrap_err().to_string();
            assert!(err.contains(names), "{sampler:?}: {err}");
            // Sampling refuses invalid parameters too.
            assert!(Wait::new(sampler).sample(&mut rng(0)).is_err());
        }

        let err = Wait::new(Sampler::Fixed(secs(1.0)))
            .with_probability(1.5)
            .validate()
            .unwrap_err();
        assert!(err.to_string().contains("--probability"));
        let err = Wait::new(Sampler::Fixed(secs(1.0)))
            .with_verbose(Verbose::Every(Duration::from_micros(10)))
            .validate()
            .unwrap_err();
        assert!(err.to_string().contains("--verbose"));

        for sampler in all_samplers() {
            assert!(Wait::new(sampler).validate().is_ok(), "{sampler:?}");
        }
    }

    #[test]
    fn test_log_normal_from_mean_sd_rejects_invalid_parameters() {
        assert!(log_normal_from_mean_sd(0.0, 0.5).is_err());
        assert!(log_normal_from_mean_sd(1.0, -0.1).is_err());
        let zero_sd = Wait::new(Sampler::LogNormal {
            mean: secs(1.0),
            std_dev: Duration::ZERO,
        });
        let d = zero_sd.calculate().unwrap();
        assert!((d.as_secs_f64() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_duration_to_next_alignment() {
        let s = Duration::from_secs;
        assert_eq!(duration_to_next_alignment(s(100), s(10)), s(10));
        assert_eq!(duration_to_next_alignment(s(103), s(10)), s(7));
        assert_eq!(duration_to_next_alignment(s(109), s(10)), s(1));
        assert_eq!(duration_to_next_alignment(s(100), s(0)), s(0));
        assert_eq!(duration_to_next_alignment(s(65), s(60)), s(55));
        assert_eq!(duration_to_next_alignment(ms(1_250), ms(500)), ms(250));
    }

    #[test]
    fn test_jitter_bounds() {
        let mut r = rng(16);
        assert_eq!(sample_jitter(&mut r, Duration::ZERO), Duration::ZERO);
        for _ in 0..1_000 {
            assert!(sample_jitter(&mut r, ms(100)) <= ms(100));
        }
        // Larger than u64 nanoseconds must not panic.
        sample_jitter(&mut r, Duration::MAX);
    }

    fn hms(h: u32, m: u32, s: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, s).unwrap()
    }

    fn whole(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    #[test]
    fn test_duration_until_later_today() {
        let now = Utc.with_ymd_and_hms(2025, 6, 1, 10, 0, 0).unwrap();
        assert_eq!(duration_until(&now, hms(10, 1, 0)), Ok(whole(60)));
    }

    #[test]
    fn test_duration_until_exactly_now_is_zero() {
        let now = Utc.with_ymd_and_hms(2025, 6, 1, 10, 0, 0).unwrap();
        assert_eq!(duration_until(&now, hms(10, 0, 0)), Ok(Duration::ZERO));
    }

    #[test]
    fn test_duration_until_past_rolls_to_tomorrow() {
        let now = Utc.with_ymd_and_hms(2025, 6, 1, 10, 0, 0).unwrap();
        assert_eq!(
            duration_until(&now, hms(9, 59, 0)),
            Ok(whole(24 * 3600 - 60))
        );
    }

    #[test]
    fn test_duration_until_across_midnight() {
        let now = Utc.with_ymd_and_hms(2025, 6, 1, 23, 59, 58).unwrap();
        assert_eq!(duration_until(&now, hms(0, 0, 3)), Ok(whole(5)));
    }

    #[test]
    fn test_duration_until_respects_fixed_offset() {
        let tz = FixedOffset::east_opt(5 * 3600 + 1800).unwrap();
        let now = tz.with_ymd_and_hms(2025, 6, 1, 8, 0, 0).unwrap();
        assert_eq!(duration_until(&now, hms(9, 30, 0)), Ok(whole(5400)));
    }

    #[test]
    fn test_duration_until_spring_forward_gap_does_not_panic() {
        // 2025-03-09: New York clocks jump from 02:00 EST to 03:00 EDT.
        let now = New_York.with_ymd_and_hms(2025, 3, 9, 0, 0, 0).unwrap();
        // 02:30 does not exist; it resolves to 03:30 EDT (07:30 UTC), which
        // is 2h30m after 00:00 EST (05:00 UTC).
        assert_eq!(duration_until(&now, hms(2, 30, 0)), Ok(whole(9000)));
    }

    #[test]
    fn test_duration_until_spring_forward_gap_already_past() {
        // At 04:00 EDT the gap has passed, so 02:30 means tomorrow at 02:30 EDT.
        let now = New_York.with_ymd_and_hms(2025, 3, 9, 4, 0, 0).unwrap();
        assert_eq!(
            duration_until(&now, hms(2, 30, 0)),
            Ok(whole(22 * 3600 + 1800))
        );
    }

    #[test]
    fn test_duration_until_fall_back_uses_earliest() {
        // 2025-11-02: New York clocks fall back from 02:00 EDT to 01:00 EST.
        let now = New_York.with_ymd_and_hms(2025, 11, 2, 0, 0, 0).unwrap();
        // 01:30 happens twice; the first is 01:30 EDT, 1h30m after 00:00 EDT.
        assert_eq!(duration_until(&now, hms(1, 30, 0)), Ok(whole(5400)));
    }

    #[test]
    fn test_duration_until_fall_back_uses_second_occurrence_when_first_passed() {
        // 01:45 EDT (05:45 UTC): the first 01:30 has passed, the second
        // (01:30 EST, 06:30 UTC) is 45 minutes away.
        let now = New_York.from_utc_datetime(
            &Utc.with_ymd_and_hms(2025, 11, 2, 5, 45, 0)
                .unwrap()
                .naive_utc(),
        );
        assert_eq!(duration_until(&now, hms(1, 30, 0)), Ok(whole(2700)));
    }

    // ConsoleSleeper output, driven by a fake clock.

    #[test]
    fn test_console_silent_wait_sleeps_once() {
        let (out, slept) = console(Sample::of(secs(2.0)), Verbose::Off);
        assert_eq!(out, "");
        assert_eq!(slept, secs(2.0));
    }

    #[test]
    fn test_console_fixed_period_output() {
        let (out, slept) = console(Sample::of(secs(1.0)), Verbose::Every(ms(250)));
        assert_eq!(slept, secs(1.0));
        let lines: Vec<&str> = out.lines().map(|l| &l[11..]).collect();
        assert_eq!(
            lines,
            [
                "[DOZR] Time remaining: 1.0s",
                "[DOZR] Time remaining: 0.8s",
                "[DOZR] Time remaining: 0.5s",
                "[DOZR] Time remaining: 0.2s",
                "Wait complete.",
            ]
        );
    }

    #[test]
    fn test_console_whole_second_period_has_no_decimals() {
        let (out, _) = console(Sample::of(secs(2.0)), Verbose::Every(secs(1.0)));
        assert!(out.contains("Time remaining: 2s"), "{out}");
        assert!(out.contains("Time remaining: 1s"), "{out}");
    }

    #[test]
    fn test_console_adaptive_output() {
        let (out, slept) = console(Sample::of(secs(3.0)), Verbose::Adaptive);
        assert_eq!(slept, secs(3.0));
        assert_eq!(out.matches("Time remaining").count(), 3, "{out}");
        assert!(out.ends_with("Wait complete.\n"), "{out}");
    }

    #[test]
    fn test_console_reports_clamp_only_when_verbose() {
        let clamped = Sample {
            duration: Duration::ZERO,
            clamped: true,
            skipped: None,
        };
        let (out, _) = console(clamped, Verbose::Adaptive);
        assert!(out.contains("clamped to zero"), "{out}");
        let (out, _) = console(clamped, Verbose::Off);
        assert_eq!(out, "");
    }

    #[test]
    fn test_console_reports_skip_only_when_verbose() {
        let skipped = Sample {
            duration: Duration::ZERO,
            clamped: false,
            skipped: Some(Skip {
                probability: 0.25,
                roll: 0.5,
            }),
        };
        let (out, slept) = console(skipped, Verbose::Every(secs(1.0)));
        assert_eq!(
            out,
            "Probabilistic wait: Skipping sleep (probability: 0.25, roll: 0.50)\n"
        );
        assert_eq!(slept, Duration::ZERO);
        let (out, _) = console(skipped, Verbose::Off);
        assert_eq!(out, "");
    }
}
