use std::{error::Error, path::PathBuf, time::Duration};

use futures::future::join_all;
use goose::{config::GooseConfiguration, logger::GooseLogFormat, prelude::*};

use crate::{docker, targets::TestTarget};

const REQUEST_LOG_FORMAT: GooseLogFormat = GooseLogFormat::Csv;
static APP_USER_AGENT: &str = "http-load-tester/0.0.1";

async fn configure_user_without_compression(user: &mut GooseUser) -> TransactionResult {
    let builder = reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .no_brotli()
        .no_gzip()
        .timeout(Duration::from_secs(10));
    user.set_client_builder(builder).await?;
    Ok(())
}

async fn configure_user_with_compression(user: &mut GooseUser) -> TransactionResult {
    let builder = reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .gzip(true)
        .timeout(Duration::from_secs(10));
    user.set_client_builder(builder).await?;
    Ok(())
}

async fn loadtest_strings(user: &mut GooseUser) -> TransactionResult {
    let _goose_metrics = user.get_named("/strings/hello", "hello").await?;
    let _goose_metrics = user
        .get_named("/strings/hello?name=cool%20gal", "hello-param")
        .await?;
    let _goose_metrics = user.get_named("/strings/hello?name=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "hello-long").await?;
    let _goose_metrics = user
        .get_named("/strings/async-hello", "async-hello")
        .await?;
    let _goose_metrics = user.get_named("/strings/lines?n=10000", "lines").await?;

    Ok(())
}

async fn loadtest_static(user: &mut GooseUser) -> TransactionResult {
    let _goose_metrics = user.get_named("/static/basic.html", "basic-html").await?;
    let _goose_metrics = user.get_named("/static/scout.webp", "scout-img").await?;

    Ok(())
}

async fn loadtest_math(user: &mut GooseUser) -> TransactionResult {
    let _goose_metrics = user
        .get_named("/math/power-reciprocals-alt?n=1000", "power-sum-easy")
        .await?;
    let _goose_metrics = user
        .get_named("/math/power-reciprocals-alt?n=10000000", "power-sum-hard")
        .await?;

    Ok(())
}

fn report_log_path(mut out_dir: PathBuf, iteration: usize) -> String {
    out_dir.push(format!("report-{}.html", iteration));
    out_dir.to_str().unwrap().to_string()
}

fn request_log_path(mut out_dir: PathBuf, iteration: usize) -> String {
    out_dir.push(format!("requests-{}.csv", iteration));
    out_dir.to_str().unwrap().to_string()
}

async fn bench_target(
    tt: &TestTarget<'_>,
    out_dir: PathBuf,
    iteration: usize,
) -> Result<(), Box<dyn Error>> {
    let mut configuration = GooseConfiguration::default();
    configuration.host = "http://localhost:8080".to_string();
    configuration.users = Some(6);
    configuration.startup_time = "60s".to_string();
    configuration.run_time = "10s".to_string();
    configuration.report_file = report_log_path(out_dir.clone(), iteration);
    configuration.request_log = request_log_path(out_dir, iteration);
    configuration.request_format = Some(REQUEST_LOG_FORMAT);

    let configure_user_fn = match tt.is_compressed {
        true => transaction!(configure_user_with_compression),
        false => transaction!(configure_user_without_compression),
    };

    println!("Starting load test against target {}", tt.name());

    GooseAttack::initialize_with_config(configuration)?
        .register_scenario(
            scenario!("LoadTest")
                .register_transaction(configure_user_fn.set_on_start())
                .register_transaction(transaction!(loadtest_strings).set_name("strings"))
                .register_transaction(transaction!(loadtest_static).set_name("static"))
                .register_transaction(transaction!(loadtest_math).set_name("math")),
        )
        .execute()
        .await?;

    println!("Finished load test against target {}", tt.name());

    Ok(())
}

/// Benchmarks each target with a load test, producing an HTML report and requests CSV for each iteration.
pub async fn benchmark_all(
    targets: &Vec<TestTarget<'_>>,
    out_dir: PathBuf,
) -> Result<(), Box<dyn Error>> {
    for target in targets {
        let name = docker::start_container(target)?;
        docker::await_healthy().await;

        let mut target_dir = out_dir.clone();
        target_dir.push(target.name());
        if !target_dir.exists() {
            tokio::fs::create_dir_all(&target_dir).await?;
        }

        for i in 1..=2 {
            bench_target(target, target_dir.clone(), i).await?;
            tokio::time::sleep(Duration::from_secs(5)).await;
        }

        docker::kill_container(&name)?;
    }

    Ok(())
}
