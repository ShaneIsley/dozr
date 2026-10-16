//! Prints durations sampled exactly as `dozr` would, one per line, in seconds.
//!
//! Takes the same command and options as `dozr`, plus a sample count and an
//! optional seed. Nothing is slept.
//!
//! ```text
//! cargo run --release --example dist_sampler -- --count 5000 n 1s 100ms
//! cargo run --release --example dist_sampler -- --seed 1 ln 1s 500ms -j 50ms
//! ```

use clap::{CommandFactory, Parser};
use dozr::cli::Cli;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::io::{BufWriter, Write};

#[derive(Parser, Debug)]
#[command(
    name = "dist_sampler",
    about = "Print durations sampled exactly as dozr would, one per line, in seconds"
)]
struct Args {
    /// Number of samples to print.
    #[arg(long, default_value_t = 1000)]
    count: usize,

    /// Seed for reproducible output.
    #[arg(long)]
    seed: Option<u64>,

    #[command(flatten)]
    dozr: Cli,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let wait = args.dozr.into_wait();
    if let Err(e) = wait.validate() {
        Args::command()
            .error(clap::error::ErrorKind::ValueValidation, format!("{e:#}"))
            .exit();
    }

    let mut rng = match args.seed {
        Some(seed) => StdRng::seed_from_u64(seed),
        None => StdRng::from_rng(&mut rand::rng()),
    };
    let mut out = BufWriter::new(std::io::stdout().lock());
    for _ in 0..args.count {
        writeln!(out, "{}", wait.sample(&mut rng)?.duration.as_secs_f64())?;
    }
    out.flush()?;
    Ok(())
}
