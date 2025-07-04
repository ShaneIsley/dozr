use anyhow::Result;
use clap::Parser;
use dozr::cli::Cli;
use dozr::conditions::WaitCondition;

fn main() -> Result<()> {
    let cli = Cli::parse();

    let condition: Box<dyn WaitCondition> = match (cli.duration, cli.align, cli.probability) {
        (Some(duration), None, Some(probability)) => {
            // Probabilistic wait
            Box::new(dozr::conditions::ProbabilisticWait {
                duration,
                probability,
                verbose: cli.verbose,
            })
        },
        (Some(duration), None, None) => {
            // Standard duration wait (with optional jitter)
            Box::new(dozr::conditions::DurationWait {
                duration,
                jitter: cli.jitter,
                verbose: cli.verbose,
            })
        },
        (None, Some(align_interval), None) => {
            // Time alignment wait
            Box::new(dozr::conditions::TimeAlignWait {
                align_interval,
                verbose: cli.verbose,
            })
        },
        // Clap's `group` and `requires` attributes should handle other invalid combinations,
        // so this `_` case should ideally only catch "no arguments provided" or unexpected states.
        (None, None, None) => {
            return Err(anyhow::anyhow!("Must specify either <DURATION> or --align. Use --help for more information."));
        },
        // This case should be caught by clap's `requires = "duration"` on `probability`
        // if probability is Some but duration is None.
        (None, _, Some(_)) => {
             return Err(anyhow::anyhow!("--probability requires a DURATION. Use --help for more information."));
        },
        // Catch-all for any other unexpected combinations not explicitly handled by clap
        _ => {
            return Err(anyhow::anyhow!("Invalid argument combination. Use --help for more information."));
        }
    };

    condition.wait()
}
