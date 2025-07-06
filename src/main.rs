use anyhow::Result;
use clap::Parser;

mod cli;
mod conditions;

fn main() -> Result<()> {
    let args = cli::Cli::parse();

    let condition: Box<dyn conditions::WaitCondition> = match (args.duration, args.align) {
        (Some(duration), None) => {
            if let Some(probability) = args.probability {
                Box::new(conditions::ProbabilisticWait {
                    duration,
                    probability,
                    verbose: args.verbose,
                })
            } else {
                Box::new(conditions::DurationWait {
                    duration,
                    jitter: args.jitter,
                    verbose: args.verbose,
                })
            }
        }
        (None, Some(align_to)) => Box::new(conditions::TimeAlignWait {
            align_interval: align_to,
            verbose: args.verbose,
        }),
        // This case is now unreachable because of the clap group validation
        _ => unreachable!(),
    };

    condition.wait()
}
