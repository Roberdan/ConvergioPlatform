// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Shared HTTP helpers for CLI subcommands that delegate to the daemon HTTP API.

pub async fn fetch_and_print(url: &str, human: bool) {
    match reqwest::get(url).await {
        Ok(resp) => {
            let status = resp.status();
            match resp.json::<serde_json::Value>().await {
                Ok(val) => print_value(&val, human),
                Err(e) => { eprintln!("error parsing response: {e}"); std::process::exit(2); }
            }
            if !status.is_success() { std::process::exit(1); }
        }
        Err(e) => { eprintln!("error connecting to daemon: {e}"); std::process::exit(2); }
    }
}

pub async fn post_and_print(url: &str, body: &serde_json::Value, human: bool) {
    let client = reqwest::Client::new();
    match client.post(url).json(body).send().await {
        Ok(resp) => {
            let status = resp.status();
            match resp.json::<serde_json::Value>().await {
                Ok(val) => print_value(&val, human),
                Err(e) => { eprintln!("error parsing response: {e}"); std::process::exit(2); }
            }
            if !status.is_success() { std::process::exit(1); }
        }
        Err(e) => { eprintln!("error connecting to daemon: {e}"); std::process::exit(2); }
    }
}

pub fn print_value(val: &serde_json::Value, human: bool) {
    if human {
        println!("{}", serde_json::to_string_pretty(val).unwrap_or_else(|_| val.to_string()));
    } else {
        println!("{val}");
    }
}
