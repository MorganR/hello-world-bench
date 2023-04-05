#[macro_use]
extern crate lazy_static;

#[cfg(test)]
#[macro_use]
extern crate approx;

use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use clap::Parser;
use targets::TestTarget;

mod docker;
mod load;
mod metrics;
mod paths;
mod single;
mod targets;
mod writes;

/// Runs benchmarks for specified hello-world servers.
#[derive(Parser, Debug)]
pub struct Cli {
    /// The list of targets to benchmark, in the form "lang-framework". These must match docker
    /// images with tags "hell-lang-framework". Can be specified multiple times.
    #[arg(short, long)]
    pub targets: Vec<String>,
    ///If specified, tests results with compression enabled.
    #[arg(long)]
    pub compress: bool,
    /// If specified, runs single-request benchmarks.
    #[arg(long)]
    pub single: bool,
    /// If specified, runs load tests.
    #[arg(long)]
    pub load: bool,
    /// Where to write the output data.
    #[arg(short, long)]
    pub out_dir: String,
    /// The number of CPUs to run the image with.
    #[arg(long, default_value="1")]
    pub num_cpus: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    let out_dir = prep_out_dir(&args.out_dir)?;
    let targets = args
        .targets
        .iter()
        .map(|t| TestTarget {
            server_name: t,
            ram_mb: 128,
            num_cpus: args.num_cpus,
            is_compressed: args.compress,
        })
        .collect();

    if args.single {
        let mut single_dir = out_dir.clone();
        single_dir.push("single");
        prep_out_dir(single_dir.to_str().unwrap())?;
        single::benchmark_all(&targets, single_dir)?;
    }

    if args.load {
        let mut load_dir = out_dir.clone();
        load_dir.push("load");
        prep_out_dir(load_dir.to_str().unwrap())?;
        load::benchmark_all(&targets, load_dir).await?;
    }

    Ok(())
}

fn prep_out_dir(out_dir: &str) -> Result<PathBuf, Box<dyn Error>> {
    let path = Path::new(out_dir);
    if path.is_dir() {
        return Ok(path.into());
    }
    if path.exists() {
        return Err(format!("out_dir {} already exists, but is not a directory", out_dir).into());
    }
    fs::create_dir_all(&path)?;
    Ok(path.into())
}
