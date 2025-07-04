use anyhow::Result;
use clap::Parser;
use dozr::cli::Cli;
use dozr::conditions::{DurationWait, WaitCondition};

fn main() -> Result<()> {
    let cli = Cli::parse();

    let condition = DurationWait {
        duration: cli.duration,
        jitter: cli.jitter,
        verbose: cli.verbose,
    };

    condition.wait()
}
