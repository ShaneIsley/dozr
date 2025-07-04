use anyhow::Result;
use clap::Parser;
use dozr::cli::Cli;
use dozr::conditions::WaitCondition;

fn main() -> Result<()> {
    let cli = Cli::parse();

    let condition: Box<dyn WaitCondition> = if let Some(align_interval) = cli.align {
        if cli.duration.as_secs() > 0 {
            return Err(anyhow::anyhow!("Cannot specify both --duration and --align"));
        }
        Box::new(dozr::conditions::TimeAlignWait {
            align_interval,
            verbose: cli.verbose,
        })
    } else {
        Box::new(dozr::conditions::DurationWait {
            duration: cli.duration,
            jitter: cli.jitter,
            verbose: cli.verbose,
        })
    };

    condition.wait()
}
