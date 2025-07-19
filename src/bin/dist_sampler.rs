use clap::Parser;

use rand::rngs::ThreadRng;
use rand_distr::{Distribution, Normal, Exp, LogNormal, Pareto, Weibull, Uniform, Triangular, Gamma};
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// The type of distribution to sample from.
    #[arg(long)]
    pub distribution: String,

    /// Number of samples to generate.
    #[arg(long, default_value = "1000")]
    pub count: usize,

    /// Mean for Normal and Log-Normal distributions (e.g., "1s").
    #[arg(long, value_parser = humantime::parse_duration)]
    pub mean: Option<Duration>,

    /// Standard deviation for Normal and Log-Normal distributions (e.g., "0.1").
    #[arg(long)]
    pub std_dev: Option<f64>,

    /// Lambda (rate parameter) for Exponential distribution (e.g., "0.5").
    #[arg(long)]
    pub lambda: Option<f64>,

    /// Scale parameter for Pareto and Weibull distributions (e.g., "1.0").
    #[arg(long)]
    pub scale: Option<f64>,

    /// Shape parameter for Pareto and Weibull distributions (e.g., "1.5").
    #[arg(long)]
    pub shape: Option<f64>,

    /// Minimum value for Uniform distribution (e.g., "1s").
    #[arg(long, value_parser = humantime::parse_duration)]
    pub min: Option<Duration>,

    /// Maximum value for Uniform distribution (e.g., "5s").
    #[arg(long, value_parser = humantime::parse_duration)]
    pub max: Option<Duration>,

    /// Minimum value for Triangular distribution (e.g., "0.0").
    #[arg(long)]
    pub triangular_min: Option<f64>,

    /// Maximum value for Triangular distribution (e.g., "1.0").
    #[arg(long)]
    pub triangular_max: Option<f64>,

    /// Mode value for Triangular distribution (e.g., "0.5").
    #[arg(long)]
    pub triangular_mode: Option<f64>,

    /// Shape parameter for Gamma distribution (e.g., "2.0").
    #[arg(long)]
    pub gamma_shape: Option<f64>,

    /// Scale parameter for Gamma distribution (e.g., "1.0").
    #[arg(long)]
    pub gamma_scale: Option<f64>,

    
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let mut rng = ThreadRng::default();

    for _ in 0..args.count {
        let sample = match args.distribution.as_str() {
            "normal" => {
                let mean_secs = args.mean.expect("Mean is required for normal distribution").as_secs_f64();
                let std_dev = args.std_dev.expect("Standard deviation is required for normal distribution");
                Normal::new(mean_secs, std_dev)?.sample(&mut rng)
            }
            "exponential" => {
                let lambda = args.lambda.expect("Lambda is required for exponential distribution");
                Exp::new(lambda)?.sample(&mut rng)
            }
            "log_normal" => {
                let mean_secs = args.mean.expect("Mean is required for log-normal distribution").as_secs_f64();
                let std_dev = args.std_dev.expect("Standard deviation is required for log-normal distribution");
                LogNormal::new(mean_secs, std_dev)?.sample(&mut rng)
            }
            "pareto" => {
                let scale = args.scale.expect("Scale is required for pareto distribution");
                let shape = args.shape.expect("Shape is required for pareto distribution");
                Pareto::new(scale, shape)?.sample(&mut rng)
            }
            "weibull" => {
                let shape = args.shape.expect("Shape is required for weibull distribution");
                let scale = args.scale.expect("Scale is required for weibull distribution");
                Weibull::new(shape, scale)?.sample(&mut rng)
            }
            "uniform" => {
                let min_secs = args.min.expect("Min is required for uniform distribution").as_secs_f64();
                let max_secs = args.max.expect("Max is required for uniform distribution").as_secs_f64();
                Uniform::new(min_secs, max_secs)?.sample(&mut rng)
            }
            "triangular" => {
                let min = args.triangular_min.expect("Min is required for triangular distribution");
                let max = args.triangular_max.expect("Max is required for triangular distribution");
                let mode = args.triangular_mode.expect("Mode is required for triangular distribution");
                Triangular::new(min, max, mode)?.sample(&mut rng)
            }
            "gamma" => {
                let shape = args.gamma_shape.expect("Shape is required for gamma distribution");
                let scale = args.gamma_scale.expect("Scale is required for gamma distribution");
                Gamma::new(shape, scale)?.sample(&mut rng)
            }
            "gamma" => {
                let shape = args.gamma_shape.expect("Shape is required for gamma distribution");
                let scale = args.gamma_scale.expect("Scale is required for gamma distribution");
                Gamma::new(shape, scale)?.sample(&mut rng)
            }
            _ => panic!("Unsupported distribution type"),
        };
        println!("{}", sample.max(0.0));
    }

    Ok(())
}