use anyhow::Result;
use rand::Rng;
use std::thread;
use std::time::{Duration, Instant};

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
    fn wait(&self) -> Result<()>;
}

pub struct DurationWait {
    pub duration: Duration,
    pub jitter: Option<Duration>,
    pub verbose: Option<Duration>,
}

impl DurationWait {
    // 3. The core logic now takes the trait object as an argument.
    fn calculate_sleep_duration(&self, jitter_gen: &mut dyn JitterGenerator) -> Duration {
        let max_jitter = self.jitter.unwrap_or(Duration::ZERO);
        let random_jitter = jitter_gen.generate(max_jitter);
        self.duration + random_jitter
    }
}

// Helper function to format Duration into a human-readable string
fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m {}s", secs / 3600, (secs % 3600) / 60, secs % 60)
    }
}

impl WaitCondition for DurationWait {
    fn wait(&self) -> Result<()> {
        let mut rng = rand::rng();
        let mut jitter_gen = RandomJitterGenerator::new(&mut rng);
        let sleep_duration = self.calculate_sleep_duration(&mut jitter_gen);

        if let Some(display_interval) = self.verbose {
            eprintln!("Waiting for {} (base: {}, jitter: {})", 
                format_duration(sleep_duration),
                format_duration(self.duration),
                format_duration(self.jitter.unwrap_or(Duration::ZERO))
            );

            let start_time = Instant::now();
            let mut next_display_time = start_time + display_interval;

            while start_time.elapsed() < sleep_duration {
                let current_time = Instant::now();

                if current_time >= next_display_time {
                    let remaining_time = sleep_duration.checked_sub(start_time.elapsed()).unwrap_or(Duration::ZERO);
                    eprintln!("ETA: {}", format_duration(remaining_time));
                    next_display_time = current_time + display_interval;
                }
                thread::sleep(Duration::from_millis(100)); // Check every 100ms
            }

            eprintln!("Wait complete.");
        } else {
            // Non-verbose path
            thread::sleep(sleep_duration);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

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
}