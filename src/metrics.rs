use std::fmt::Debug;
use std::time::Duration;

use regex::Regex;

#[derive(Debug, serde::Serialize)]
pub enum Metric {
    Latency(MetricData<Duration>),
    Qps(MetricData<f64>),
}

impl Metric {
    pub fn try_from_wrk_output(line: &str) -> Option<Self> {
        lazy_static! {
            // Example output:
            // Thread Stats   Avg      Stdev     Max   +/- Stdev
            //  Latency   635.91us    0.89ms  12.92ms   93.69%
            //  Req/Sec     57.2k       4k      100k    93.69%
            static ref LATENCY: Regex = Regex::new(r"[[:space:]]+Latency").unwrap();
            static ref QPS: Regex = Regex::new(r"[[:space:]]+Req/Sec").unwrap();
        }
        if LATENCY.is_match(line) {
            return MetricData::try_from_wrk_latency(line).map(Metric::Latency);
        }
        if QPS.is_match(line) {
            return MetricData::try_from_wrk_qps(line).map(Metric::Qps);
        }
        None
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricData<T: Debug> {
    pub mean: T,
    pub std_dev: T,
    pub max: T,
}

impl MetricData<Duration> {
    pub fn try_from_wrk_latency(latency_line: &str) -> Option<Self> {
        /// Matches (and captures) a time value that is in either us, ns, ms, or seconds.
        const TIME_REGEX: &str = r"([[:digit:]]+(?:\.[[:digit:]]+)?[mun]?s)";
        lazy_static! {
            // Example output:
            // Thread Stats   Avg      Stdev     Max   +/- Stdev
            //  Latency   635.91us    0.89ms  12.92ms   93.69%
            static ref ALL_TIMES: Regex = Regex::new(
                &format!(r"[[:space:]]*Latency[[:space:]]+{}[[:space:]]+{}[[:space:]]+{}", TIME_REGEX, TIME_REGEX, TIME_REGEX)).unwrap();
        }

        ALL_TIMES.captures(latency_line).map(|captures| Self {
            mean: time_str_into_duration(captures.get(1).unwrap().as_str()),
            std_dev: time_str_into_duration(captures.get(2).unwrap().as_str()),
            max: time_str_into_duration(captures.get(3).unwrap().as_str()),
        })
    }
}

impl MetricData<f64> {
    pub fn try_from_wrk_qps(qps_line: &str) -> Option<Self> {
        /// Matches (and captures) a time value that is in either us, ns, ms, or seconds.
        const COUNT_REGEX: &str = r"([[:digit:]]+(?:\.[[:digit:]]+)?[mkMG]?)";
        lazy_static! {
            // Example output:
            // Thread Stats   Avg      Stdev     Max   +/- Stdev
            //  Req/Sec     57.2k       4k      100k    93.69%
            static ref ALL_COUNTS: Regex = Regex::new(
                &format!(r"[[:space:]]*Req/Sec[[:space:]]+{}[[:space:]]+{}[[:space:]]+{}", COUNT_REGEX, COUNT_REGEX, COUNT_REGEX)).unwrap();
        }

        ALL_COUNTS.captures(qps_line).map(|captures| Self {
            mean: str_into_count(captures.get(1).unwrap().as_str()),
            std_dev: str_into_count(captures.get(2).unwrap().as_str()),
            max: str_into_count(captures.get(3).unwrap().as_str()),
        })
    }
}

fn time_str_into_duration(time_str: &str) -> Duration {
    lazy_static! {
        static ref NS: Regex = Regex::new(r"([[:digit:]]+(?:\.[[:digit:]]+)?)ns").unwrap();
        static ref US: Regex = Regex::new(r"([[:digit:]]+(?:\.[[:digit:]]+)?)us").unwrap();
        static ref MS: Regex = Regex::new(r"([[:digit:]]+(?:\.[[:digit:]]+)?)ms").unwrap();
        static ref S: Regex = Regex::new(r"([[:digit:]]+(?:\.[[:digit:]]+)?)s").unwrap();
    }

    NS.captures(time_str)
        .map(|c| c.get(1).unwrap().as_str().parse::<f64>().unwrap())
        .or_else(|| {
            US.captures(time_str)
                .map(|c| c.get(1).unwrap().as_str().parse::<f64>().unwrap() * 1000.0)
        })
        .or_else(|| {
            MS.captures(time_str)
                .map(|c| c.get(1).unwrap().as_str().parse::<f64>().unwrap() * 1000_000.0)
        })
        .or_else(|| {
            S.captures(time_str)
                .map(|c| c.get(1).unwrap().as_str().parse::<f64>().unwrap() * 1000_000_000.0)
        })
        .map(|ns| Duration::from_nanos(ns.round() as u64))
        .expect(&format!("Could not parse time {}", time_str))
}

fn str_into_count(count_str: &str) -> f64 {
    lazy_static! {
        static ref MILLI: Regex = Regex::new(r"([[:digit:]]+(?:\.[[:digit:]]+)?)m").unwrap();
        static ref PLAIN: Regex = Regex::new(r"([[:digit:]]+(?:\.[[:digit:]]+)?)$").unwrap();
        static ref KILO: Regex = Regex::new(r"([[:digit:]]+(?:\.[[:digit:]]+)?)k").unwrap();
        static ref MEGA: Regex = Regex::new(r"([[:digit:]]+(?:\.[[:digit:]]+)?)M").unwrap();
        static ref GIGA: Regex = Regex::new(r"([[:digit:]]+(?:\.[[:digit:]]+)?)G").unwrap();
    }

    MILLI
        .captures(count_str)
        .map(|c| c.get(1).unwrap().as_str().parse::<f64>().unwrap() * 0.001)
        .or_else(|| {
            PLAIN
                .captures(count_str)
                .map(|c| c.get(1).unwrap().as_str().parse::<f64>().unwrap())
        })
        .or_else(|| {
            KILO.captures(count_str)
                .map(|c| c.get(1).unwrap().as_str().parse::<f64>().unwrap() * 1000.0)
        })
        .or_else(|| {
            MEGA.captures(count_str)
                .map(|c| c.get(1).unwrap().as_str().parse::<f64>().unwrap() * 1000_000.0)
        })
        .or_else(|| {
            GIGA.captures(count_str)
                .map(|c| c.get(1).unwrap().as_str().parse::<f64>().unwrap() * 1000_000_000.0)
        })
        .expect(&format!("Could not parse count {}", count_str))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_time_str_into_duration() {
        [
            ("123.01ms", Duration::from_micros(123_010)),
            ("123ms", Duration::from_millis(123)),
            ("123.4ns", Duration::from_nanos(123)),
            ("123.4us", Duration::from_nanos(123_400)),
            ("123.4s", Duration::from_millis(123_400)),
        ]
        .map(|(time_str, expected)| assert_eq!(time_str_into_duration(time_str), expected));
    }

    #[test]
    fn test_str_into_count() {
        [
            ("123.01m", 0.12301),
            ("123m", 0.123),
            ("123.4", 123.4),
            ("123.4k", 123_400.0),
            ("123.4M", 123_400_000.0),
            ("123.4G", 123_400_000_000.0),
        ]
        .map(|(count_str, expected)| assert_relative_eq!(str_into_count(count_str), expected));
    }

    #[test]
    fn test_try_from_wrk_latency() {
        let m =
            MetricData::try_from_wrk_latency("    Latency   441.23ms   58.18us   2.63s   91.22%");
        assert_eq!(m.is_some(), true);
        let m = m.unwrap();
        assert_eq!(
            m.mean,
            Duration::from_nanos((441.23f64 * 1_000_000.0).round() as u64)
        );
        assert_eq!(
            m.std_dev,
            Duration::from_nanos((58.18f64 * 1_000.0).round() as u64)
        );
        assert_eq!(
            m.max,
            Duration::from_nanos((2.63f64 * 1_000_000_000.0).round() as u64)
        );
    }

    #[test]
    fn test_try_from_wrk_qps() {
        let m = MetricData::try_from_wrk_qps("    Req/Sec   441.23m   58.18   2.63k   91.22%");
        assert_eq!(m.is_some(), true);
        let m = m.unwrap();
        assert_relative_eq!(m.mean, 441.23f64 * 0.001);
        assert_relative_eq!(m.std_dev, 58.18f64);
        assert_relative_eq!(m.max, 2.63f64 * 1000.0);
    }
}
