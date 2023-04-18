use std::{io::Error, process::Command};

use crate::targets::TestTarget;

pub fn start_container(target: &TestTarget) -> Result<String, Error> {
    let memory_arg = format!("{}m", target.ram_mb);
    let cpus_str = target.num_cpus.to_string();
    let name = target.name();
    let docker_target = target.docker_target();

    delete_container_if_present(&name)?;

    println!("Starting container {} with image {}", &name, &docker_target);
    Command::new("docker")
        .args([
            "run",
            "-m",
            &memory_arg,
            "--memory-swap",
            &memory_arg,
            "--cpus",
            &cpus_str,
            "-p",
            "8080:8080",
            "--name",
            &name,
            &docker_target,
        ])
        .spawn()?;

    Ok(name)
}

pub async fn is_healthy() -> bool {
    let hello = reqwest::get("http://localhost:8080/strings/hello").await;
    match hello {
        Ok(r) => r.status().is_success(),
        _ => false,
    }
}

pub async fn await_healthy() {
    println!("Polling until healthy");
    loop {
        if is_healthy().await {
            break;
        }
    }
    println!("Container is ready");
}

pub fn kill_container(name: &str) -> Result<(), Error> {
    println!("Killing container {}", name);
    let status = Command::new("docker")
        .args(["stop", &name])
        .spawn()?
        .wait()?;

    if status.success() {
        return Ok(());
    } else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Exited with status code {}", status),
        ));
    }
}

fn delete_container_if_present(name: &str) -> Result<(), Error> {
    println!("Deleting container {}", name);
    Command::new("docker")
        .args(["rm", "-f", name])
        .spawn()?
        .wait()?;
    Ok(())
}
