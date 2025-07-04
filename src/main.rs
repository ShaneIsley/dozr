use anyhow::Result;
use clap::Parser;
use dozr::cli::Cli;
use dozr::conditions::WaitCondition;

fn main() -> Result<()> {
    let cli = Cli::parse();

    let condition: Box<dyn WaitCondition> = match (cli.duration, cli.align, cli.probability, cli.until) {
        (Some(duration), None, Some(probability), None) => {
            // Probabilistic wait
            Box::new(dozr::conditions::ProbabilisticWait {
                duration,
                probability,
                verbose: cli.verbose,
            })
        },
        (Some(duration), None, None, None) => {
            // Standard duration wait (with optional jitter)
            Box::new(dozr::conditions::DurationWait {
                duration,
                jitter: cli.jitter,
                verbose: cli.verbose,
            })
        },
        (None, Some(align_interval), None, None) => {
            // Time alignment wait
            Box::new(dozr::conditions::TimeAlignWait {
                align_interval,
                verbose: cli.verbose,
            })
        },
        (None, None, None, Some(until_duration)) => {
            // Wait until specific time
            Box::new(dozr::conditions::UntilTimeWait {
                sleep_duration: until_duration,
                verbose: cli.verbose,
            })
        },
        // Clap's `group` and `requires` attributes should handle other invalid combinations,
        // so this `_` case should ideally only catch "no arguments provided" or unexpected states.
        (None, None, None, None) => {
            return Err(anyhow::anyhow!("Must specify either <DURATION>, --align, or --until. Use --help for more information."));
        },
        // This case should be caught by clap's `requires = "duration"` on `probability`
        // if probability is Some but duration is None.
        (None, _, Some(_), _) => {
             return Err(anyhow::anyhow!("--probability requires a DURATION. Use --help for more information."));
        },
        // Catch-all for any other unexpected combinations not explicitly handled by clap
        _ => {
            return Err(anyhow::anyhow!("Invalid argument combination. Use --help for more information."));
        }
    };

    condition.wait()
}
