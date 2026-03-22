// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Project-level audit — fetches aggregated data from daemon API and writes report.

use chrono::Utc;
use serde_json::{json, Value};
use std::fs;

/// Fetch project audit report from daemon API, optionally write to disk.
pub async fn handle(project_id: &str, output: bool, yes: bool, api_url: &str) {
    let url = format!("{api_url}/api/audit/project/{project_id}");
    let report = match reqwest::get(&url).await {
        Ok(resp) => {
            let status = resp.status();
            match resp.json::<Value>().await {
                Ok(val) => {
                    if !status.is_success() {
                        let msg = val["error"].as_str().unwrap_or("unknown error");
                        eprintln!("error: {msg}");
                        std::process::exit(1);
                    }
                    val
                }
                Err(e) => {
                    eprintln!("error parsing response: {e}");
                    std::process::exit(2);
                }
            }
        }
        Err(e) => {
            eprintln!("error connecting to daemon: {e}");
            std::process::exit(2);
        }
    };

    // Pretty-print to stdout
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| report.to_string())
    );

    if output {
        write_report(project_id, &report, yes, api_url).await;
    }
}

/// Write audit report + metadata to project output directory, create deliverable record.
async fn write_report(project_id: &str, report: &Value, yes: bool, api_url: &str) {
    let output_dir = claude_core::platform_paths::project_output_dir(project_id);
    let now = Utc::now();
    let date_prefix = now.format("%Y-%m-%d").to_string();

    // Find next version number by scanning existing audit folders
    let version = next_audit_version(&output_dir, &date_prefix);
    let folder_name = format!("{date_prefix}_audit_v{version}");
    let dest = output_dir.join(&folder_name);

    // Interactive confirmation unless --yes
    if !yes {
        eprintln!("Audit report will be written to:");
        eprintln!("  {}", dest.display());
        eprint!("Continue? [y/N] ");
        let mut answer = String::new();
        if std::io::stdin().read_line(&mut answer).is_err()
            || !matches!(answer.trim().to_lowercase().as_str(), "y" | "yes")
        {
            eprintln!("Aborted.");
            std::process::exit(1);
        }
    }

    // Permission preflight
    if let Err(e) = fs::create_dir_all(&dest) {
        eprintln!("error: cannot create output directory: {e}");
        print_permission_help();
        std::process::exit(2);
    }

    // Write report.json
    let report_path = dest.join("report.json");
    let report_str =
        serde_json::to_string_pretty(report).unwrap_or_else(|_| report.to_string());
    if let Err(e) = fs::write(&report_path, &report_str) {
        eprintln!("error: cannot write report.json: {e}");
        print_permission_help();
        std::process::exit(2);
    }

    // Write metadata.json
    let metadata = json!({
        "project_id": project_id,
        "type": "audit",
        "version": version,
        "created_at": now.to_rfc3339(),
        "output_path": dest.to_string_lossy(),
    });
    let meta_path = dest.join("metadata.json");
    if let Err(e) = fs::write(&meta_path, serde_json::to_string_pretty(&metadata).unwrap()) {
        eprintln!("error: cannot write metadata.json: {e}");
        std::process::exit(2);
    }

    eprintln!("Report written to: {}", dest.display());

    // Create deliverable record in DB via daemon API
    let body = json!({
        "project_id": project_id,
        "name": format!("audit_{date_prefix}"),
        "output_type": "audit",
    });
    let client = reqwest::Client::new();
    match client
        .post(format!("{api_url}/api/deliverables"))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            eprintln!("Deliverable record created.");
        }
        Ok(resp) => {
            let msg = resp.text().await.unwrap_or_default();
            eprintln!("warn: deliverable creation failed: {msg}");
        }
        Err(e) => {
            eprintln!("warn: could not reach daemon for deliverable: {e}");
        }
    }
}

/// Scan output directory for existing audit folders to find next version number.
fn next_audit_version(output_dir: &std::path::Path, date_prefix: &str) -> u32 {
    let pattern = format!("{date_prefix}_audit_v");
    let max = fs::read_dir(output_dir)
        .ok()
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    name.strip_prefix(&pattern)
                        .and_then(|v| v.parse::<u32>().ok())
                })
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0);
    max + 1
}

/// Print OS-specific instructions for permission errors.
fn print_permission_help() {
    if cfg!(target_os = "macos") {
        eprintln!(
            "hint: on macOS, grant Full Disk Access to your terminal app via\n  \
             System Settings > Privacy & Security > Full Disk Access"
        );
    } else if cfg!(target_os = "linux") {
        eprintln!(
            "hint: on Linux, check folder permissions with `ls -la` and fix with\n  \
             chmod -R u+rX <folder>"
        );
    } else {
        eprintln!("hint: ensure the current user has write permissions on the output folder");
    }
}
