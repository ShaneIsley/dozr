//! `dozr`: a flexible `sleep` with distributions, jitter, alignment, and
//! probabilistic waits.
//!
//! The command-line interface lives in [`cli`]. Library users build a
//! [`conditions::Wait`] from a [`conditions::Sampler`] and call
//! [`conditions::Wait::wait`].

use anyhow::Result;
use clap::{CommandFactory, Parser};
use conditions::{ConsoleSleeper, DryRunSleeper, Sleeper};
use rand::{RngCore, SeedableRng};
use std::process::ExitCode;

pub mod cli;
pub mod conditions;
pub mod progress;

pub use progress::{MIN_VERBOSE_PERIOD, Verbose, adaptive_verbose_wait, verbose_wait};

/// The main entry point for the dozr application.
///
/// Parses command-line arguments, validates them, and performs the wait.
/// Invalid arguments print a usage error and exit with status 2. Returns the
/// `--skip-exit-code` status when a probabilistic wait was skipped, and
/// success otherwise.
pub fn run() -> Result<ExitCode> {
    let (wait, options) = cli::Cli::parse().into_parts();
    if let Err(e) = wait.validate() {
        cli::Cli::command()
            .error(clap::error::ErrorKind::ValueValidation, format!("{e:#}"))
            .exit();
    }

    let mut rng: Box<dyn RngCore> = match options.seed {
        Some(seed) => Box::new(rand::rngs::StdRng::seed_from_u64(seed)),
        None => Box::new(rand::rng()),
    };
    let mut sleeper: Box<dyn Sleeper> = if options.dry_run {
        Box::new(DryRunSleeper::stdio())
    } else {
        Box::new(ConsoleSleeper::stderr())
    };

    let sample = wait.wait_using(&mut *rng, &mut *sleeper)?;
    Ok(if sample.skipped.is_some() {
        ExitCode::from(options.skip_exit_code)
    } else {
        ExitCode::SUCCESS
    })
}
