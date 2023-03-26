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

mod docker;
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
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    let out_dir = prep_out_dir(args.out_dir)?;

    if args.single {
        single::benchmark_all_single(&args.targets, out_dir, args.compress)?
    }

    Ok(())
}

fn prep_out_dir(out_dir: String) -> Result<PathBuf, Box<dyn Error>> {
    let path = Path::new(&out_dir);
    if path.is_dir() {
        return Ok(path.into());
    }
    if path.exists() {
        return Err(format!("out_dir {} already exists, but is not a directory", out_dir).into());
    }
    fs::create_dir_all(&path)?;
    Ok(path.into())
}
