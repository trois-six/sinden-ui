//! A command-line client of the driver's control API.
//!
//! ```text
//! sinden-ctl version | status | get [KEY...] | set KEY=VALUE... | subscribe
//! ```
//!
//! The endpoint is `$SINDEN_DRIVER_SOCKET` or the system's default.

use std::collections::BTreeMap;
use std::process::ExitCode;

use sinden_control::{Client, Endpoint, Result};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let endpoint = Endpoint::from_env_or_default();
    match run(&endpoint, &args) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => {
            eprintln!(
                "usage: sinden-ctl version | status | get [KEY...] | set KEY=VALUE... | subscribe"
            );
            ExitCode::from(2)
        }
        Err(err) => {
            eprintln!("sinden-ctl: {endpoint}: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(endpoint: &Endpoint, args: &[String]) -> Result<bool> {
    let Some((command, rest)) = args.split_first() else {
        return Ok(false);
    };
    match command.as_str() {
        "version" => print(&Client::connect(endpoint)?.version()?),
        "status" => print(&Client::connect(endpoint)?.status()?),
        "get" => {
            let snapshot = Client::connect(endpoint)?.get_config()?;
            println!("# {}", snapshot.path);
            for (key, value) in &snapshot.values {
                if rest.is_empty() || rest.contains(key) {
                    println!("{key}={value}");
                }
            }
        }
        "set" => {
            let mut values = BTreeMap::new();
            for pair in rest {
                let Some((key, value)) = pair.split_once('=') else {
                    return Ok(false);
                };
                values.insert(key.to_owned(), value.to_owned());
            }
            if values.is_empty() {
                return Ok(false);
            }
            Client::connect(endpoint)?.set_config(&values)?;
        }
        "subscribe" => {
            for event in Client::connect_with_timeout(endpoint, None)?.subscribe()? {
                print(&event?);
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}

fn print(value: &impl serde::Serialize) {
    println!(
        "{}",
        serde_json::to_string(value).expect("values serialise")
    );
}
