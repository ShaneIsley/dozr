//! `dozr`: a flexible `sleep` with distributions, jitter, alignment, and
//! probabilistic waits.
//!
//! The command-line interface lives in [`cli`]. Library users build a
//! [`conditions::Wait`] from a [`conditions::Sampler`] and call
//! [`conditions::Wait::wait`].

use anyhow::Result;
use clap::{CommandFactory, Parser};

pub mod cli;
pub mod conditions;
pub mod progress;

pub use progress::{MIN_VERBOSE_PERIOD, Verbose, adaptive_verbose_wait, verbose_wait};

/// The main entry point for the dozr application.
///
/// Parses command-line arguments, validates them, and performs the wait.
/// Invalid arguments print a usage error and exit with status 2.
pub fn run() -> Result<()> {
    let wait = cli::Cli::parse().into_wait();
    if let Err(e) = wait.validate() {
        cli::Cli::command()
            .error(clap::error::ErrorKind::ValueValidation, format!("{e:#}"))
            .exit();
    }
    wait.wait()
}
