use std::{error::Error, path::PathBuf, process::Command};

use crate::{
    docker,
    metrics::Metric,
    paths::TestPath,
    targets::{self, TestTarget},
    writes,
};

#[derive(Debug)]
pub struct SingleResult<'a, 'b> {
    pub target: TestTarget<'b>,
    pub path: &'a TestPath,
    pub metrics: Vec<Metric>,
}

impl<'a, 'b> SingleResult<'a, 'b> {
    fn new<'c: 'a>(target: TestTarget<'b>, path: &'c TestPath) -> Self {
        SingleResult {
            target: target,
            path: path,
            metrics: vec![],
        }
    }

    fn push_wrk_results(&mut self, out: Vec<u8>) {
        let out_str = std::str::from_utf8(&out).unwrap();
        let metrics = out_str
            .lines()
            .map(|line| Metric::try_from_wrk_output(line));
        self.metrics.extend(metrics.filter_map(|m| m));
    }
}

fn bench_single<'a: 'c, 'b, 'c>(
    target: TestTarget<'b>,
    path: &'a TestPath,
) -> Result<SingleResult<'a, 'b>, Box<dyn Error>> {
    let full_path = format!("http://localhost:8080{}", &path.path);
    let out = Command::new("wrk")
        .args(["-t", "1", "-c", "1", "-d", "30s", &full_path])
        .output()?;
    if !out.status.success() {
        return Err(format!("Failed to run wrk; code: {:?}", out.status.code()).into());
    }
    let mut result = SingleResult::new(target, path);
    result.push_wrk_results(out.stdout);

    Ok(result)
}

/// Benchmarks each target, writing results to a CSV in out_dir.
pub fn benchmark_all_single(
    targets: &Vec<String>,
    out_dir: PathBuf,
    with_compression: bool,
) -> Result<(), Box<dyn Error>> {
    let mut single_benchmark_path = out_dir;
    single_benchmark_path.push("single-benchmarks.csv");
    let mut single_benchmark_csv = csv::Writer::from_path(&single_benchmark_path)?;

    lazy_static! {
        static ref TEST_PATHS: [TestPath; 8] = [
            TestPath::new("/strings/hello", "hello"),
            TestPath::new("/strings/hello?name=fluffy%20dog", "hello-param"),
            TestPath::new("/strings/hello?name=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "hello-long"),
            TestPath::new("/strings/lines?n=10000", "lines"),
            TestPath::new("/static/scout.webp", "static-img"),
            TestPath::new("/static/basic.html", "static-text"),
            TestPath::new("/math/power-reciprocals-alt?n=10000", "math-powers-light"),
            TestPath::new("/math/power-reciprocals-alt?n=10000000", "math-powers-heavy")
        ];
    }

    for target in targets {
        let tt = targets::TestTarget {
            server_name: target,
            num_cpus: 1,
            ram_mb: 128,
            is_compressed: with_compression,
        };
        let name = docker::start_container(&tt)?;

        for path in TEST_PATHS.iter() {
            let result = bench_single(tt.clone(), path)?;
            writes::write_single_result(&mut single_benchmark_csv, result)?;
        }

        docker::kill_container(&name)?;
    }

    Ok(())
}
