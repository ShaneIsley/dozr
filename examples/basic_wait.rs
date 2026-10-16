//! Basic example demonstrating simple duration waits.
//!
//! Run with: `cargo run --example basic_wait`

use std::time::Duration;

use dozr::conditions::{Sampler, Wait};

fn main() -> anyhow::Result<()> {
    println!("Starting a 2-second wait...");
    Wait::new(Sampler::Fixed(Duration::from_secs(2))).wait()?;
    println!("Wait complete!");

    // Wait 1 second plus up to 500ms of random jitter.
    println!("\nStarting a 1-second wait with up to 500ms jitter...");
    Wait::new(Sampler::Fixed(Duration::from_secs(1)))
        .with_jitter(Duration::from_millis(500))
        .wait()?;
    println!("Wait with jitter complete!");

    Ok(())
}
