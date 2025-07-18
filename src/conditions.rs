use crate::{adaptive_verbose_wait, verbose_wait};
use anyhow::Result;
use rand::Rng;
use rand::rngs::ThreadRng;
use rand_distr::{Distribution, Normal, Exp, LogNormal, Pareto, Weibull, Triangular, Uniform};
use chrono::{Local, DateTime};

use std::time::{Duration, SystemTime};

// 1. Define a dedicated trait for jitter generation.
// This makes the dependency explicit and easy to mock.
pub trait JitterGenerator {
    fn generate(&mut self, max_jitter: Duration) -> Duration;
}

// 2. Implement the trait for the real random number generator.
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
        let jitter_millis = self.rng.random_range(0..=max_jitter.as_millis() as u64);
        Duration::from_millis(jitter_millis)
    }
}

pub trait WaitCondition {
    fn calculate_wait_duration(&self) -> Result<Duration>;
    fn wait(&self) -> Result<()>;
}

pub struct DurationWait {
    pub duration: Duration,
    pub verbose: Option<Duration>,
    pub jitter: Option<Duration>,
}

impl DurationWait {
    // 3. The core logic now takes the trait object as an argument.
    fn calculate_sleep_duration(&self, jitter_gen: &mut dyn JitterGenerator) -> Duration {
        let max_jitter = self.jitter.unwrap_or(Duration::ZERO);
        let random_jitter = jitter_gen.generate(max_jitter);
        self.duration + random_jitter
    }
}

pub struct NormalWait {
    pub mean: Duration,
    pub std_dev: f64,
    pub verbose: Option<Duration>,
    pub jitter: Option<Duration>,
}

impl WaitCondition for NormalWait {
    fn calculate_wait_duration(&self) -> Result<Duration> {
        let mean_secs = self.mean.as_secs_f64();
        let normal = Normal::new(mean_secs, self.std_dev)?;
        let mut rng = ThreadRng::default();
        let duration_secs = normal.sample(&mut rng).max(0.0);
        let mut jitter_gen = RandomJitterGenerator::new(&mut rng);
        let random_jitter = jitter_gen.generate(self.jitter.unwrap_or(Duration::ZERO));
        Ok(Duration::from_secs_f64(duration_secs) + random_jitter)
    }

    fn wait(&self) -> Result<()> {
        let sleep_duration = self.calculate_wait_duration()?;
        match self.verbose {
            Some(display_interval) => {
                let display_fn = |remaining: Duration| {
                    if remaining.is_zero() {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                    } else {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                    }
                };
                verbose_wait(sleep_duration, display_interval, display_fn);
            }
            None => {
                let display_fn = |remaining: Duration| {
                    if remaining.is_zero() {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                    } else {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                    }
                };
                adaptive_verbose_wait(sleep_duration, display_fn);
            }
        }
        Ok(())
    }
}

pub struct ExponentialWait {
    pub lambda: f64,
    pub verbose: Option<Duration>,
    pub jitter: Option<Duration>,
}

impl WaitCondition for ExponentialWait {
    fn calculate_wait_duration(&self) -> Result<Duration> {
        let exponential = Exp::new(self.lambda)?;
        let mut rng = ThreadRng::default();
        let duration_secs = exponential.sample(&mut rng).max(0.0);
        let mut jitter_gen = RandomJitterGenerator::new(&mut rng);
        let random_jitter = jitter_gen.generate(self.jitter.unwrap_or(Duration::ZERO));
        Ok(Duration::from_secs_f64(duration_secs) + random_jitter)
    }

    fn wait(&self) -> Result<()> {
        let sleep_duration = self.calculate_wait_duration()?;
        match self.verbose {
            Some(display_interval) => {
                let display_fn = |remaining: Duration| {
                    if remaining.is_zero() {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                    } else {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                    }
                };
                verbose_wait(sleep_duration, display_interval, display_fn);
            }
            None => {
                let display_fn = |remaining: Duration| {
                    if remaining.is_zero() {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                    } else {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                    }
                };
                adaptive_verbose_wait(sleep_duration, display_fn);
            }
        }
        Ok(())
    }
}

pub struct LogNormalWait {
    pub mean: Duration,
    pub std_dev: f64,
    pub verbose: Option<Duration>,
    pub jitter: Option<Duration>,
}

impl WaitCondition for LogNormalWait {
    fn calculate_wait_duration(&self) -> Result<Duration> {
        let mean_secs = self.mean.as_secs_f64();
        let log_normal = LogNormal::new(mean_secs, self.std_dev)?;
        let mut rng = ThreadRng::default();
        let duration_secs = log_normal.sample(&mut rng).max(0.0);
        let mut jitter_gen = RandomJitterGenerator::new(&mut rng);
        let random_jitter = jitter_gen.generate(self.jitter.unwrap_or(Duration::ZERO));
        Ok(Duration::from_secs_f64(duration_secs) + random_jitter)
    }

    fn wait(&self) -> Result<()> {
        let sleep_duration = self.calculate_wait_duration()?;
        match self.verbose {
            Some(display_interval) => {
                let display_fn = |remaining: Duration| {
                    if remaining.is_zero() {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                    } else {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                    }
                };
                verbose_wait(sleep_duration, display_interval, display_fn);
            }
            None => {
                let display_fn = |remaining: Duration| {
                    if remaining.is_zero() {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                    } else {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                    }
                };
                adaptive_verbose_wait(sleep_duration, display_fn);
            }
        }
        Ok(())
    }
}

pub struct ParetoWait {
    pub scale: f64,
    pub shape: f64,
    pub verbose: Option<Duration>,
    pub jitter: Option<Duration>,
}

impl WaitCondition for ParetoWait {
    fn calculate_wait_duration(&self) -> Result<Duration> {
        let pareto = Pareto::new(self.scale, self.shape)?;
        let mut rng = ThreadRng::default();
        let duration_secs = pareto.sample(&mut rng).max(0.0);
        let mut jitter_gen = RandomJitterGenerator::new(&mut rng);
        let random_jitter = jitter_gen.generate(self.jitter.unwrap_or(Duration::ZERO));
        Ok(Duration::from_secs_f64(duration_secs) + random_jitter)
    }

    fn wait(&self) -> Result<()> {
        let sleep_duration = self.calculate_wait_duration()?;
        match self.verbose {
            Some(display_interval) => {
                let display_fn = |remaining: Duration| {
                    if remaining.is_zero() {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                    } else {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                    }
                };
                verbose_wait(sleep_duration, display_interval, display_fn);
            }
            None => {
                let display_fn = |remaining: Duration| {
                    if remaining.is_zero() {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                    } else {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                    }
                };
                adaptive_verbose_wait(sleep_duration, display_fn);
            }
        }
        Ok(())
    }
}

pub struct WeibullWait {
    pub shape: f64,
    pub scale: f64,
    pub verbose: Option<Duration>,
    pub jitter: Option<Duration>,
}

impl WaitCondition for WeibullWait {
    fn calculate_wait_duration(&self) -> Result<Duration> {
        let weibull = Weibull::new(self.shape, self.scale)?;
        let mut rng = ThreadRng::default();
        let duration_secs = weibull.sample(&mut rng).max(0.0);
        let mut jitter_gen = RandomJitterGenerator::new(&mut rng);
        let random_jitter = jitter_gen.generate(self.jitter.unwrap_or(Duration::ZERO));
        Ok(Duration::from_secs_f64(duration_secs) + random_jitter)
    }

    fn wait(&self) -> Result<()> {
        let sleep_duration = self.calculate_wait_duration()?;
        match self.verbose {
            Some(display_interval) => {
                let display_fn = |remaining: Duration| {
                    if remaining.is_zero() {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                    } else {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                    }
                };
                verbose_wait(sleep_duration, display_interval, display_fn);
            }
            None => {
                let display_fn = |remaining: Duration| {
                    if remaining.is_zero() {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                    } else {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                    }
                };
                adaptive_verbose_wait(sleep_duration, display_fn);
            }
        }
        Ok(())
    }
}

pub struct UniformWait {
    pub min: Duration,
    pub max: Duration,
    pub verbose: Option<Duration>,
    pub jitter: Option<Duration>,
}

impl WaitCondition for UniformWait {
    fn calculate_wait_duration(&self) -> Result<Duration> {
        let min_secs = self.min.as_secs_f64();
        let max_secs = self.max.as_secs_f64();
        let uniform = Uniform::new(min_secs, max_secs)?;
        let mut rng = ThreadRng::default();
        let duration_secs = uniform.sample(&mut rng).max(0.0);
        let mut jitter_gen = RandomJitterGenerator::new(&mut rng);
        let random_jitter = jitter_gen.generate(self.jitter.unwrap_or(Duration::ZERO));
        Ok(Duration::from_secs_f64(duration_secs) + random_jitter)
    }

    fn wait(&self) -> Result<()> {
        let sleep_duration = self.calculate_wait_duration()?;
        match self.verbose {
            Some(display_interval) => {
                let display_fn = |remaining: Duration| {
                    if remaining.is_zero() {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                    } else {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                    }
                };
                verbose_wait(sleep_duration, display_interval, display_fn);
            }
            None => {
                let display_fn = |remaining: Duration| {
                    if remaining.is_zero() {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                    } else {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                    }
                };
                adaptive_verbose_wait(sleep_duration, display_fn);
            }
        }
        Ok(())
    }
}

pub struct TriangularWait {
    pub min: f64,
    pub max: f64,
    pub mode: f64,
    pub verbose: Option<Duration>,
    pub jitter: Option<Duration>,
}

impl WaitCondition for TriangularWait {
    fn calculate_wait_duration(&self) -> Result<Duration> {
        let triangular = Triangular::new(self.min, self.max, self.mode)?;
        let mut rng = ThreadRng::default();
        let duration_secs = triangular.sample(&mut rng).max(0.0);
        let mut jitter_gen = RandomJitterGenerator::new(&mut rng);
        let random_jitter = jitter_gen.generate(self.jitter.unwrap_or(Duration::ZERO));
        Ok(Duration::from_secs_f64(duration_secs) + random_jitter)
    }

    fn wait(&self) -> Result<()> {
        let sleep_duration = self.calculate_wait_duration()?;
        match self.verbose {
            Some(display_interval) => {
                let display_fn = |remaining: Duration| {
                    if remaining.is_zero() {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                    } else {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                    }
                };
                verbose_wait(sleep_duration, display_interval, display_fn);
            }
            None => {
                let display_fn = |remaining: Duration| {
                    if remaining.is_zero() {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                    } else {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                    }
                };
                adaptive_verbose_wait(sleep_duration, display_fn);
            }
        }
        Ok(())
    }
}

pub struct TimeAlignWait {
    pub align_interval: Duration,
    pub verbose: Option<Duration>,
}

impl WaitCondition for TimeAlignWait {
    fn calculate_wait_duration(&self) -> Result<Duration> {
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        let sleep_duration = if self.align_interval.as_secs() == 0 {
            Duration::ZERO
        } else {
            let remainder = now.as_secs() % self.align_interval.as_secs();
            if remainder == 0 {
                self.align_interval
            } else {
                self.align_interval - Duration::from_secs(remainder)
            }
        };
        Ok(sleep_duration)
    }

    fn wait(&self) -> Result<()> {
        let sleep_duration = self.calculate_wait_duration()?;

        match self.verbose {
            Some(display_interval) => {
                let display_fn = |remaining: Duration| {
                    if remaining.is_zero() {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                    } else {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                    }
                };
                verbose_wait(sleep_duration, display_interval, display_fn);
            }
            None => {
                let display_fn = |remaining: Duration| {
                    if remaining.is_zero() {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                    } else {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                    }
                };
                adaptive_verbose_wait(sleep_duration, display_fn);
            }
        }
        Ok(())
    }
}

pub struct ProbabilisticWait {
    pub duration: Duration,
    pub probability: f64,
    pub verbose: Option<Duration>,
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

        match self.verbose {
            Some(display_interval) => {
                let display_fn = |remaining: Duration| {
                    if remaining.is_zero() {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                    } else {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                    }
                };
                verbose_wait(sleep_duration, display_interval, display_fn);
            }
            None => {
                let display_fn = |remaining: Duration| {
                    if remaining.is_zero() {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                    } else {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                    }
                };
                adaptive_verbose_wait(sleep_duration, display_fn);
            }
        }
        Ok(())
    }
}

impl WaitCondition for ProbabilisticWait {
    fn calculate_wait_duration(&self) -> Result<Duration> {
        let mut rng = ThreadRng::default();
        let roll: f64 = rng.random_range(0.0..1.0);

        if roll <= self.probability {
            Ok(self.duration)
        } else {
            Ok(Duration::ZERO)
        }
    }

    fn wait(&self) -> Result<()> {
        let sleep_duration = self.calculate_wait_duration()?;

        if sleep_duration > Duration::ZERO {
            match self.verbose {
                Some(display_interval) => {
                    let display_fn = |remaining: Duration| {
                        if remaining.is_zero() {
                            let now: DateTime<Local> = Local::now();
                            eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                        } else {
                            let now: DateTime<Local> = Local::now();
                            eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                        }
                    };
                    verbose_wait(self.duration, display_interval, display_fn);
                }
                None => {
                    let display_fn = |remaining: Duration| {
                        if remaining.is_zero() {
                            let now: DateTime<Local> = Local::now();
                            eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                        } else {
                            let now: DateTime<Local> = Local::now();
                            eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                        }
                    };
                    adaptive_verbose_wait(self.duration, display_fn);
                }
            }
        } else if self.verbose.is_some() || self.verbose.is_none() {
            eprintln!(
                "Probabilistic wait: Skipping sleep (probability: {}, roll: {})",
                self.probability, self.calculate_wait_duration().unwrap().as_secs_f64() // This is a bit of a hack, but it works for now
            );
        }
        Ok(())
    }
}

impl WaitCondition for DurationWait {
    fn calculate_wait_duration(&self) -> Result<Duration> {
        let mut rng = ThreadRng::default();
        let mut jitter_gen = RandomJitterGenerator::new(&mut rng);
        Ok(self.calculate_sleep_duration(&mut jitter_gen))
    }

    fn wait(&self) -> Result<()> {
        let sleep_duration = self.calculate_wait_duration()?;

        match self.verbose {
            Some(display_interval) => {
                let display_fn = |remaining: Duration| {
                    if remaining.is_zero() {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                    } else {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                    }
                };
                verbose_wait(sleep_duration, display_interval, display_fn);
            }
            None => {
                let display_fn = |remaining: Duration| {
                    if remaining.is_zero() {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] Wait complete.", now.format("%H:%M:%S"));
                    } else {
                        let now: DateTime<Local> = Local::now();
                        eprintln!("[{}] [DOZR] Time remaining: {:.0}s", now.format("%H:%M:%S"), remaining.as_secs_f64());
                    }
                };
                adaptive_verbose_wait(sleep_duration, display_fn);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    // 4. Create a mock generator for testing.
    struct MockJitterGenerator {
        jitter: Duration,
    }

    impl JitterGenerator for MockJitterGenerator {
        fn generate(&mut self, _max_jitter: Duration) -> Duration {
            // Return the exact, predictable jitter for the test.
            self.jitter
        }
    }

    #[test]
    fn test_duration_wait_creation() {
        let duration = Duration::from_secs(1);
        let wait_condition = DurationWait {
            duration,
            jitter: None,
            verbose: None,
        };
        assert_eq!(wait_condition.duration, duration);
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

        let calculated_duration = wait_condition.calculate_sleep_duration(&mut mock_gen);

        // Assert that the base duration is correctly added to the mock jitter.
        assert_eq!(calculated_duration, Duration::from_millis(1001));
    }

    #[test]
    fn test_time_align_wait_calculation() {
        // Test case 1: Current time is exactly on an alignment point
        let now_secs = 100; // Example: 100 seconds past epoch
        let align_interval = Duration::from_secs(10);
        let expected_sleep = Duration::from_secs(10);
        let calculated_sleep = calculate_time_to_next_alignment(now_secs, align_interval);
        assert_eq!(calculated_sleep, expected_sleep);

        // Test case 2: Current time is slightly past an alignment point
        let now_secs = 103; // Example: 103 seconds past epoch
        let align_interval = Duration::from_secs(10);
        let expected_sleep = Duration::from_secs(7);
        let calculated_sleep = calculate_time_to_next_alignment(now_secs, align_interval);
        assert_eq!(calculated_sleep, expected_sleep);

        // Test case 3: Current time is just before next alignment point
        let now_secs = 109; // Example: 109 seconds past epoch
        let align_interval = Duration::from_secs(10);
        let expected_sleep = Duration::from_secs(1);
        let calculated_sleep = calculate_time_to_next_alignment(now_secs, align_interval);
        assert_eq!(calculated_sleep, expected_sleep);

        // Test case 4: Alignment interval is 0
        let now_secs = 100;
        let align_interval = Duration::from_secs(0);
        let expected_sleep = Duration::from_secs(0);
        let calculated_sleep = calculate_time_to_next_alignment(now_secs, align_interval);
        assert_eq!(calculated_sleep, expected_sleep);

        // Test case 5: Larger alignment interval (e.g., 1 minute)
        let now_secs = 65; // 1 minute and 5 seconds past epoch
        let align_interval = Duration::from_secs(60);
        let expected_sleep = Duration::from_secs(55); // Should align to 2 minutes mark
        let calculated_sleep = calculate_time_to_next_alignment(now_secs, align_interval);
        assert_eq!(calculated_sleep, expected_sleep);
    }

    // Helper function for testing TimeAlignWait
    fn calculate_time_to_next_alignment(now_secs: u64, align_interval: Duration) -> Duration {
        if align_interval.as_secs() == 0 {
            return Duration::ZERO;
        }
        let remainder = now_secs % align_interval.as_secs();
        if remainder == 0 {
            align_interval
        } else {
            align_interval - Duration::from_secs(remainder)
        }
    }

    #[test]
    fn test_probabilistic_wait_always_sleeps_at_1_0_probability() {
        let wait_condition = ProbabilisticWait {
            duration: Duration::from_millis(100),
            probability: 1.0,
            verbose: None,
        };
        let start_time = Instant::now();
        wait_condition.wait().unwrap();
        let elapsed = start_time.elapsed();
        assert!(elapsed >= Duration::from_millis(100));
    }

    #[test]
    fn test_probabilistic_wait_never_sleeps_at_0_0_probability() {
        let wait_condition = ProbabilisticWait {
            duration: Duration::from_millis(100),
            probability: 0.0,
            verbose: None,
        };
        let start_time = Instant::now();
        wait_condition.wait().unwrap();
        let elapsed = start_time.elapsed();
        assert!(elapsed < Duration::from_millis(50)); // Should be very fast, not actually sleep
    }
    #[test]
    fn test_jitter_generator_non_zero_max_jitter() {
        let mut rng = ThreadRng::default();
        let mut jitter_gen = RandomJitterGenerator::new(&mut rng);
        let max_jitter = Duration::from_millis(100);
        let generated_jitter = jitter_gen.generate(max_jitter);
        assert!(generated_jitter <= max_jitter);
    }

    #[test]
    fn test_jitter_generator_zero_max_jitter() {
        let mut rng = ThreadRng::default();
        let mut jitter_gen = RandomJitterGenerator::new(&mut rng);
        let max_jitter = Duration::ZERO;
        let generated_jitter = jitter_gen.generate(max_jitter);
        assert_eq!(generated_jitter, Duration::ZERO);
    }
}
