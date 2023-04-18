use std::{error::Error, path::PathBuf, process::Command, io};

use crate::{docker, metrics::Metric, paths::TestPath, targets::TestTarget, writes};

#[derive(Debug)]
pub struct PerfResult<'a, 'b> {
    pub target: TestTarget<'b>,
    pub path: &'a TestPath,
    pub metrics: Vec<Metric>,
}

impl<'a, 'b> PerfResult<'a, 'b> {
    fn new<'c: 'a>(target: TestTarget<'b>, path: &'c TestPath) -> Self {
        PerfResult {
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

fn warm_up(path: &str) -> io::Result<()>{
    Command::new("wrk")
        .args(["-t", "1", "-c", "1", "-d", "1s", &path])
        .output()?;
    Ok(())
}

fn bench_path<'a: 'c, 'b, 'c>(
    target: TestTarget<'b>,
    path: &'a TestPath,
) -> Result<PerfResult<'a, 'b>, Box<dyn Error>> {
    let full_path = format!("http://localhost:8080{}", &path.path);

    warm_up(&full_path)?;

    let out = Command::new("wrk")
        .args(["-t", "1", "-c", "1", "-d", "10s", &full_path])
        .output()?;
    if !out.status.success() {
        return Err(format!("Failed to run wrk; code: {:?}", out.status.code()).into());
    }
    let mut result = PerfResult::new(target, path);
    result.push_wrk_results(out.stdout);
    println!("\tLatency: {:?}", result.metrics.iter().find(|m| matches!(m, Metric::Latency(_))).unwrap());

    Ok(result)
}

/// Benchmarks each target, writing results to a CSV in out_dir.
pub async fn benchmark_all<'a>(
    targets: &Vec<TestTarget<'a>>,
    out_dir: PathBuf,
) -> Result<(), Box<dyn Error>> {
    let mut perf_benchmark_path = out_dir;
    perf_benchmark_path.push("benchmarks.csv");
    let mut benchmark_csv = csv::Writer::from_path(&perf_benchmark_path)?;

    lazy_static! {
        static ref TEST_PATHS: [TestPath; 9] = [
            TestPath::new("/strings/hello", "hello"),
            TestPath::new("/strings/hello?name=fluffy%20dog", "hello-param"),
            TestPath::new("/strings/hello?name=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "hello-long"),
            TestPath::new("/strings/async-hello", "async-hello"),
            TestPath::new("/strings/lines?n=10000", "lines"),
            TestPath::new("/static/scout.webp", "static-img"),
            TestPath::new("/static/basic.html", "static-text"),
            TestPath::new("/math/power-reciprocals-alt?n=10000", "math-powers-light"),
            TestPath::new("/math/power-reciprocals-alt?n=10000000", "math-powers-heavy")
        ];
    }

    for target in targets {
        let name = docker::start_container(&target)?;
        docker::await_healthy().await;

        println!("Starting performance benchmark on target {}", target.name());
        for path in TEST_PATHS.iter() {
            println!("Benchmarking path {:?}", path);
            let result = bench_path(target.clone(), path)?;
            writes::write_perf_result(&mut benchmark_csv, result)?;
        }
        println!("Finished performance benchmark on target {}", target.name());

        docker::kill_container(&name)?;
    }

    Ok(())
}
