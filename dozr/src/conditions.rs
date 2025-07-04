use anyhow::Result;
use rand::Rng;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

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
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m {}s", secs / 3600, (secs % 3600) / 60, secs % 60)
    }
}

pub struct TimeAlignWait {
    pub align_interval: Duration,
    pub verbose: Option<Duration>,
}

impl WaitCondition for TimeAlignWait {
    fn wait(&self) -> Result<()> {
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

        if let Some(display_interval) = self.verbose {
            eprintln!(
                "Aligning to next {} interval. Waiting for {}",
                format_duration(self.align_interval),
                format_duration(sleep_duration)
            );

            let start_time = Instant::now();
            let mut next_display_time = start_time + display_interval;

            while start_time.elapsed() < sleep_duration {
                let current_time = Instant::now();

                if current_time >= next_display_time {
                    let remaining_time = sleep_duration
                        .checked_sub(start_time.elapsed())
                        .unwrap_or(Duration::ZERO);
                    eprintln!("ETA: {}", format_duration(remaining_time));
                    next_display_time = current_time + display_interval;
                }
                thread::sleep(Duration::from_millis(10)); // Check every 10ms
            }

            eprintln!("Alignment complete.");
        } else {
            thread::sleep(sleep_duration);
        }
        Ok(())
    }
}

impl WaitCondition for DurationWait {
    fn wait(&self) -> Result<()> {
        let mut rng = rand::rng();
        let mut jitter_gen = RandomJitterGenerator::new(&mut rng);
        let sleep_duration = self.calculate_sleep_duration(&mut jitter_gen);

        if let Some(display_interval) = self.verbose {
            eprintln!(
                "Waiting for {} (base: {}, jitter: {})",
                format_duration(sleep_duration),
                format_duration(self.duration),
                format_duration(self.jitter.unwrap_or(Duration::ZERO))
            );

            let start_time = Instant::now();
            let mut next_display_time = start_time + display_interval;

            while start_time.elapsed() < sleep_duration {
                let current_time = Instant::now();

                if current_time >= next_display_time {
                    let remaining_time = sleep_duration
                        .checked_sub(start_time.elapsed())
                        .unwrap_or(Duration::ZERO);
                    eprintln!("ETA: {}", format_duration(remaining_time));
                    next_display_time = current_time + display_interval;
                }
                thread::sleep(Duration::from_millis(10)); // Check every 10ms
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
}