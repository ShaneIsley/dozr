//! Example demonstrating verbose progress output.
//!
//! Run with: `cargo run --example verbose_progress`

use std::time::Duration;

use dozr::{adaptive_verbose_wait, verbose_wait};

fn main() {
    // Fixed update period verbose wait
    println!("Verbose wait with 500ms update period (3 seconds total):");
    println!("---");

    verbose_wait(
        Duration::from_secs(3),
        Duration::from_millis(500),
        |remaining| {
            if remaining.is_zero() {
                println!("  Complete!");
            } else {
                println!("  Time remaining: {:.1}s", remaining.as_secs_f64());
            }
        },
    );

    println!();

    // Adaptive verbose wait (adjusts update frequency based on remaining time)
    println!("Adaptive verbose wait (5 seconds total):");
    println!("---");

    adaptive_verbose_wait(Duration::from_secs(5), |remaining| {
        if remaining.is_zero() {
            println!("  Complete!");
        } else {
            println!("  Time remaining: {}s", remaining.as_secs());
        }
    });
}
