//! Basic example demonstrating simple duration waits.
//!
//! Run with: `cargo run --example basic_wait`

use std::time::Duration;

use dozr::conditions::{DurationWait, WaitCondition};

fn main() -> anyhow::Result<()> {
    println!("Starting a 2-second wait...");

    let wait = DurationWait {
        duration: Duration::from_secs(2),
        verbose: None,
        jitter: None,
    };

    wait.wait()?;

    println!("Wait complete!");

    // Example with jitter: wait 1 second plus up to 500ms random jitter
    println!("\nStarting a 1-second wait with up to 500ms jitter...");

    let wait_with_jitter = DurationWait {
        duration: Duration::from_secs(1),
        verbose: None,
        jitter: Some(Duration::from_millis(500)),
    };

    wait_with_jitter.wait()?;

    println!("Wait with jitter complete!");

    Ok(())
}
