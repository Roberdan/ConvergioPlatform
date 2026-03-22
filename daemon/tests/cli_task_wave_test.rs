// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Integration tests: task/wave CLI arg parsing + cvg argv[0] detection.
// Local mirror types used — binary cli_task/cli_wave are not in lib crate.
// See cli_plan_test.rs for plan subcommand tests.

use clap::{Parser, Subcommand};

// -- Mirror: TaskCommands --

#[derive(Debug, Parser)]
#[command(name = "cvg-task-test")]
struct TaskCli {
    #[command(subcommand)]
    command: TaskCommands,
}

#[derive(Debug, Subcommand, PartialEq)]
enum TaskCommands {
    Update {
        task_id: i64,
        status: String,
        #[arg(long)]
        summary: Option<String>,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    Validate {
        task_id: i64,
        plan_id: i64,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    KbSearch {
        query: String,
        #[arg(long, default_value_t = 5)]
        limit: u32,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
}

// -- Mirror: WaveCommands --

#[derive(Debug, Parser)]
#[command(name = "cvg-wave-test")]
struct WaveCli {
    #[command(subcommand)]
    command: WaveCommands,
}

#[derive(Debug, Subcommand, PartialEq)]
enum WaveCommands {
    Update {
        wave_id: i64,
        status: String,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    Context {
        plan_id: i64,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    Validate {
        wave_id: i64,
        plan_id: i64,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
}

// -- Task parsing tests --

#[test]
fn cli_plan_task_update_parses_id_and_status() {
    let cli = TaskCli::try_parse_from(["cvg-task-test", "update", "8795", "done"])
        .expect("parse");
    if let TaskCommands::Update { task_id, status, summary, human, .. } = cli.command {
        assert_eq!(task_id, 8795);
        assert_eq!(status, "done");
        assert!(summary.is_none());
        assert!(!human);
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn cli_plan_task_update_parses_optional_summary() {
    let cli = TaskCli::try_parse_from([
        "cvg-task-test", "update", "10", "in_progress",
        "--summary", "still running",
    ])
    .expect("parse");
    if let TaskCommands::Update { task_id, status, summary, .. } = cli.command {
        assert_eq!(task_id, 10);
        assert_eq!(status, "in_progress");
        assert_eq!(summary.as_deref(), Some("still running"));
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn cli_plan_task_update_missing_args_fails() {
    assert!(TaskCli::try_parse_from(["cvg-task-test", "update"]).is_err());
    assert!(TaskCli::try_parse_from(["cvg-task-test", "update", "1"]).is_err());
}

#[test]
fn cli_plan_task_validate_parses_task_and_plan_id() {
    let cli = TaskCli::try_parse_from(["cvg-task-test", "validate", "8795", "685"])
        .expect("parse");
    if let TaskCommands::Validate { task_id, plan_id, .. } = cli.command {
        assert_eq!(task_id, 8795);
        assert_eq!(plan_id, 685);
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn cli_plan_task_kb_search_parses_query_default_limit() {
    let cli = TaskCli::try_parse_from(["cvg-task-test", "kb-search", "rust clap"])
        .expect("parse");
    if let TaskCommands::KbSearch { query, limit, .. } = cli.command {
        assert_eq!(query, "rust clap");
        assert_eq!(limit, 5);
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn cli_plan_task_kb_search_parses_custom_limit() {
    let cli = TaskCli::try_parse_from([
        "cvg-task-test", "kb-search", "integration", "--limit", "10",
    ])
    .expect("parse");
    if let TaskCommands::KbSearch { limit, .. } = cli.command {
        assert_eq!(limit, 10);
    } else {
        panic!("wrong variant");
    }
}

// -- Wave parsing tests --

#[test]
fn cli_plan_wave_update_parses_id_and_status() {
    let cli = WaveCli::try_parse_from(["cvg-wave-test", "update", "3", "done"])
        .expect("parse");
    if let WaveCommands::Update { wave_id, status, .. } = cli.command {
        assert_eq!(wave_id, 3);
        assert_eq!(status, "done");
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn cli_plan_wave_update_missing_status_fails() {
    let result = WaveCli::try_parse_from(["cvg-wave-test", "update", "3"]);
    assert!(result.is_err(), "status arg is required");
}

#[test]
fn cli_plan_wave_context_parses_plan_id() {
    let cli = WaveCli::try_parse_from(["cvg-wave-test", "context", "685"]).expect("parse");
    if let WaveCommands::Context { plan_id, .. } = cli.command {
        assert_eq!(plan_id, 685);
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn cli_plan_wave_validate_parses_wave_and_plan_id() {
    let cli = WaveCli::try_parse_from(["cvg-wave-test", "validate", "7", "685"])
        .expect("parse");
    if let WaveCommands::Validate { wave_id, plan_id, .. } = cli.command {
        assert_eq!(wave_id, 7);
        assert_eq!(plan_id, 685);
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn cli_plan_wave_unknown_subcommand_fails() {
    let result = WaveCli::try_parse_from(["cvg-wave-test", "merge"]);
    assert!(result.is_err(), "unknown subcommand should fail");
}

// -- cvg argv[0] detection: mirrors main.rs logic, no process spawn needed --

fn is_cvg_invocation(argv0: &str) -> bool {
    std::path::Path::new(argv0)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        == "cvg"
}

fn is_agent_ipc_invocation(argv0: &str) -> bool {
    argv0.contains("agent-ipc")
}

#[test]
fn cli_plan_argv0_plain_cvg_detected() {
    assert!(is_cvg_invocation("cvg"), "bare 'cvg' must be detected");
}

#[test]
fn cli_plan_argv0_absolute_path_cvg_detected() {
    assert!(is_cvg_invocation("/usr/local/bin/cvg"));
}

#[test]
fn cli_plan_argv0_relative_path_cvg_detected() {
    assert!(is_cvg_invocation("./bin/cvg"));
}

#[test]
fn cli_plan_argv0_daemon_binary_not_detected_as_cvg() {
    assert!(!is_cvg_invocation("convergio-platform-daemon"));
}

#[test]
fn cli_plan_argv0_agent_ipc_symlink_detected() {
    assert!(is_agent_ipc_invocation("agent-ipc"));
    assert!(is_agent_ipc_invocation("/home/user/.claude/scripts/agent-ipc"));
}

#[test]
fn cli_plan_argv0_cvg_not_detected_as_agent_ipc() {
    assert!(!is_agent_ipc_invocation("cvg"));
}
