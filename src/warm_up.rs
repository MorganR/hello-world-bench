use std::{
    error::Error,
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::{docker, paths::TestPath, targets::TestTarget, writes};

#[derive(Debug)]
pub struct WarmUpResults<'a, 'b> {
    pub target: TestTarget<'a>,
    pub per_path: Vec<WarmUpResult<'b>>,
}

impl<'a, 'b> WarmUpResults<'a, 'b> {
    fn new(target: TestTarget<'a>) -> Self {
        Self {
            target,
            per_path: vec![],
        }
    }
}

#[derive(Debug)]
pub struct WarmUpResult<'a> {
    pub path: &'a TestPath,
    pub startup_time: Duration,
    pub latencies: Vec<Duration>,
}

impl<'a> WarmUpResult<'a> {
    fn new(path: &'a TestPath, startup_time: Duration) -> Self {
        Self {
            path,
            startup_time,
            latencies: vec![],
        }
    }
}

async fn bench_path<'a: 'c, 'b, 'c>(
    path: &'a TestPath,
    start_time: Instant,
) -> Result<WarmUpResult<'a>, Box<dyn Error>> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_millis(10))
        .timeout(Duration::from_secs(1))
        .build()?;
    let full_path = format!("http://localhost:8080{}", &path.path);
    let mut result = wait_for_first_response(path, &full_path, start_time, &client).await;

    for _ in 0..5 {
        let req_time = Instant::now();
        let _ = client.get(&full_path).send().await;
        let duration = req_time.elapsed();
        result.latencies.push(duration);
    }

    Ok(result)
}

async fn wait_for_first_response<'a>(
    path: &'a TestPath,
    full_path: &str,
    start_time: Instant,
    client: &reqwest::Client,
) -> WarmUpResult<'a> {
    loop {
        let req_time = Instant::now();
        let resp = client.get(full_path).send().await;
        if resp.is_ok() && resp.unwrap().status().is_success() {
            let duration = req_time.elapsed();
            let mut result = WarmUpResult::new(path, req_time.duration_since(start_time));
            result.latencies.push(duration);
            return result;
        }
    }
}

/// Benchmarks each target, writing results to a CSV in out_dir.
pub async fn benchmark_all<'a>(
    targets: &Vec<TestTarget<'a>>,
    out_dir: PathBuf,
) -> Result<(), Box<dyn Error>> {
    let mut requests_csv_path = out_dir.clone();
    requests_csv_path.push("request-benchmarks.csv");
    let mut requests_csv = csv::Writer::from_path(&requests_csv_path)?;
    let mut start_times_csv_path = out_dir;
    start_times_csv_path.push("start-time-benchmarks.csv");
    let mut start_times_csv = csv::Writer::from_path(&start_times_csv_path)?;

    lazy_static! {
        static ref TEST_PATHS: [TestPath; 4] = [
            TestPath::new("/strings/hello", "hello"),
            TestPath::new("/strings/lines?n=50000", "lines"),
            TestPath::new("/static/basic.html", "static-text"),
            TestPath::new("/math/power-reciprocals-alt?n=1000000", "powers-sum")
        ];
    }

    for target in targets {
        let mut results = WarmUpResults::new(target.clone());
        for path in TEST_PATHS.iter() {
            for i in 0..3 {
                let name = docker::start_container(&target)?;

                let result = bench_path(path, Instant::now()).await?;
                results.per_path.push(result);

                docker::kill_container(&name)?;
                let last_result = results.per_path.last().unwrap();
                println!(
                    "Benchmarked warm-up {} on path {:?}.\n\tStartup: {:?}\n\tLatencies: {:?}",
                    i, last_result.path, last_result.startup_time, last_result.latencies
                );
            }
        }
        writes::write_warm_up_request_results(&mut requests_csv, &results)?;
        writes::write_warm_up_start_time_results(&mut start_times_csv, &results)?;
    }

    Ok(())
}
