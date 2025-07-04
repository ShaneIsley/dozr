use anyhow::Result;
use clap::Parser;
use dozr::cli::Cli;
use dozr::conditions::WaitCondition;

fn main() -> Result<()> {
    let cli = Cli::parse();

    let condition: Box<dyn WaitCondition> = match (cli.duration, cli.align) {
        (Some(duration), None) => {
            Box::new(dozr::conditions::DurationWait {
                duration,
                jitter: cli.jitter,
                verbose: cli.verbose,
            })
        },
        (None, Some(align_interval)) => {
            Box::new(dozr::conditions::TimeAlignWait {
                align_interval,
                verbose: cli.verbose,
            })
        },
        (Some(_), Some(_)) => {
            return Err(anyhow::anyhow!("Cannot specify both --duration and --align"));
        },
        (None, None) => {
            return Err(anyhow::anyhow!("Must specify either <DURATION> or --align"));
        },
    };

    condition.wait()
}
