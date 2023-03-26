use std::error::Error;
use std::io::Write;
use std::time::Duration;

use crate::metrics::{Metric, MetricData};
use crate::single::SingleResult;

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
struct SingleResultRow<'a> {
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

impl<'a: 'c, 'b: 'c, 'c> From<SingleResult<'a, 'b>> for SingleResultRow<'c> {
    fn from(result: SingleResult<'a, 'b>) -> Self {
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
        SingleResultRow {
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

pub fn write_single_result<W: Write>(
    writer: &mut csv::Writer<W>,
    result: SingleResult,
) -> Result<(), Box<dyn Error>> {
    writer.serialize(SingleResultRow::from(result))?;
    Ok(())
}
