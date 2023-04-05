use std::error::Error;
use std::io::Write;
use std::time::Duration;

use crate::metrics::{Metric, MetricData};
use crate::perf::PerfResult;
use crate::warm_up::WarmUpResults;

#[derive(serde::Serialize)]
struct LatencyRow {
    mean_ms: f64,
    std_dev_ms: f64,
    max_ms: f64,
}

impl From<&MetricData<Duration>> for LatencyRow {
    fn from(data: &MetricData<Duration>) -> Self {
        LatencyRow {
            mean_ms: data.mean.as_secs_f64() * 0.001,
            std_dev_ms: data.std_dev.as_secs_f64() * 0.001,
            max_ms: data.max.as_secs_f64() * 0.001,
        }
    }
}

#[derive(serde::Serialize)]
struct PerfResultRow<'a> {
    name: &'a str,
    path: &'a str,
    server_name: &'a str,
    num_cpus: usize,
    ram_mb: usize,
    target: String,
    latency_mean_ms: f64,
    latency_std_dev_ms: f64,
    latency_max_ms: f64,
    qps_mean: f64,
    qps_std_dev: f64,
    qps_max: f64,
}

impl<'a: 'c, 'b: 'c, 'c> From<PerfResult<'a, 'b>> for PerfResultRow<'c> {
    fn from(result: PerfResult<'a, 'b>) -> Self {
        let latency_metric = result
            .metrics
            .iter()
            .find(|m| matches!(m, Metric::Latency(_)))
            .unwrap();
        let latency_row: LatencyRow = match latency_metric {
            Metric::Latency(data) => data.into(),
            _ => panic!("Impossible"),
        };
        let qps_metric = result
            .metrics
            .iter()
            .find(|m| matches!(m, Metric::Qps(_)))
            .unwrap();
        let qps_data: MetricData<f64> = match qps_metric {
            Metric::Qps(data) => data.clone(),
            _ => panic!("Impossible"),
        };
        PerfResultRow {
            name: &result.path.name,
            path: &result.path.path,
            server_name: result.target.server_name,
            num_cpus: result.target.num_cpus,
            ram_mb: result.target.ram_mb,
            target: result.target.name(),
            latency_mean_ms: latency_row.mean_ms,
            latency_std_dev_ms: latency_row.std_dev_ms,
            latency_max_ms: latency_row.max_ms,
            qps_mean: qps_data.mean,
            qps_std_dev: qps_data.std_dev,
            qps_max: qps_data.max,
        }
    }
}

#[derive(serde::Serialize)]
struct WarmUpRequestRow<'a> {
    name: &'a str,
    path: &'a str,
    server_name: &'a str,
    num_cpus: usize,
    ram_mb: usize,
    target: String,
    request_number: usize,
    latency_ms: f64,
}

impl<'a: 'c, 'b: 'c, 'c> From<&WarmUpResults<'a, 'b>> for Vec<WarmUpRequestRow<'c>> {
    fn from(result: &WarmUpResults<'a, 'b>) -> Self {
        result
            .per_path
            .iter()
            .flat_map(|path_result| {
                path_result
                    .latencies
                    .iter()
                    .enumerate()
                    .map(|(i, duration)| WarmUpRequestRow {
                        name: &path_result.path.name,
                        path: &path_result.path.path,
                        server_name: result.target.server_name,
                        num_cpus: result.target.num_cpus,
                        ram_mb: result.target.ram_mb,
                        target: result.target.name(),
                        request_number: i + 1,
                        latency_ms: (duration.as_nanos() as f64) / 1_000_000.0,
                    })
            })
            .collect()
    }
}

#[derive(serde::Serialize)]
struct ServerStartRow<'a> {
    name: &'a str,
    path: &'a str,
    server_name: &'a str,
    num_cpus: usize,
    ram_mb: usize,
    target: String,
    start_up_latency_ms: f64,
}

impl<'a: 'c, 'b: 'c, 'c> From<&WarmUpResults<'a, 'b>> for Vec<ServerStartRow<'c>> {
    fn from(result: &WarmUpResults<'a, 'b>) -> Self {
        result
            .per_path
            .iter()
            .map(|path_result| ServerStartRow {
                name: &path_result.path.name,
                path: &path_result.path.path,
                server_name: result.target.server_name,
                num_cpus: result.target.num_cpus,
                ram_mb: result.target.ram_mb,
                target: result.target.name(),
                start_up_latency_ms: (path_result.startup_time.as_nanos() as f64) / 1_000_000.0,
            })
            .collect()
    }
}

pub fn write_perf_result<W: Write>(
    writer: &mut csv::Writer<W>,
    result: PerfResult,
) -> Result<(), Box<dyn Error>> {
    writer.serialize(PerfResultRow::from(result))?;
    Ok(())
}

pub fn write_warm_up_request_results<W: Write>(
    writer: &mut csv::Writer<W>,
    results: &WarmUpResults,
) -> Result<(), Box<dyn Error>> {
    let rows: Vec<WarmUpRequestRow> = results.into();
    rows.iter()
        .map(|row| writer.serialize(row))
        .collect::<Result<_, csv::Error>>()?;
    Ok(())
}

pub fn write_warm_up_start_time_results<W: Write>(
    writer: &mut csv::Writer<W>,
    results: &WarmUpResults,
) -> Result<(), Box<dyn Error>> {
    let rows: Vec<ServerStartRow> = results.into();
    rows.iter()
        .map(|row| writer.serialize(row))
        .collect::<Result<_, csv::Error>>()?;
    Ok(())
}
