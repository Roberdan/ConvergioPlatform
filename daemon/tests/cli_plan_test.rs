// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Integration tests for plan CLI arg parsing (cvg plan <subcommand>).
// Uses a local mirror PlanCommands so tests are self-contained —
// binary cli_plan module is not exposed via the library crate.
//
// Why local mirror: cli_plan lives in main.rs scope (binary-only).
// See cli_task_wave_test.rs for task/wave/argv[0] tests.

use clap::{Parser, Subcommand};

// ---------------------------------------------------------------------------
// Mirror: PlanCommands
// ---------------------------------------------------------------------------

#[derive(Debug, Parser)]
#[command(name = "cvg-plan-test")]
struct PlanCli {
    #[command(subcommand)]
    command: PlanCommands,
}

#[derive(Debug, Subcommand, PartialEq)]
enum PlanCommands {
    List {
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    Tree {
        plan_id: i64,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    Show {
        plan_id: i64,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    Drift {
        plan_id: i64,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
    Validate {
        plan_id: i64,
        #[arg(long)]
        human: bool,
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
    },
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn cli_plan_list_parses_defaults() {
    let cli = PlanCli::try_parse_from(["cvg-plan-test", "list"]).expect("parse");
    assert!(matches!(cli.command, PlanCommands::List { human: false, .. }));
}

#[test]
fn cli_plan_list_parses_human_flag() {
    let cli = PlanCli::try_parse_from(["cvg-plan-test", "list", "--human"]).expect("parse");
    assert!(matches!(cli.command, PlanCommands::List { human: true, .. }));
}

#[test]
fn cli_plan_list_parses_custom_api_url() {
    let cli = PlanCli::try_parse_from([
        "cvg-plan-test", "list", "--api-url", "http://localhost:9000",
    ])
    .expect("parse");
    if let PlanCommands::List { api_url, .. } = cli.command {
        assert_eq!(api_url, "http://localhost:9000");
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn cli_plan_tree_parses_plan_id() {
    let cli = PlanCli::try_parse_from(["cvg-plan-test", "tree", "685"]).expect("parse");
    if let PlanCommands::Tree { plan_id, human, .. } = cli.command {
        assert_eq!(plan_id, 685);
        assert!(!human);
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn cli_plan_show_parses_plan_id_with_human() {
    let cli =
        PlanCli::try_parse_from(["cvg-plan-test", "show", "42", "--human"]).expect("parse");
    if let PlanCommands::Show { plan_id, human, .. } = cli.command {
        assert_eq!(plan_id, 42);
        assert!(human);
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn cli_plan_drift_parses_plan_id() {
    let cli = PlanCli::try_parse_from(["cvg-plan-test", "drift", "100"]).expect("parse");
    assert!(matches!(
        cli.command,
        PlanCommands::Drift { plan_id: 100, .. }
    ));
}

#[test]
fn cli_plan_validate_parses_plan_id() {
    let cli = PlanCli::try_parse_from(["cvg-plan-test", "validate", "200"]).expect("parse");
    assert!(matches!(
        cli.command,
        PlanCommands::Validate { plan_id: 200, .. }
    ));
}

#[test]
fn cli_plan_unknown_subcommand_fails() {
    let result = PlanCli::try_parse_from(["cvg-plan-test", "bogus"]);
    assert!(result.is_err(), "unknown subcommand should fail");
}

#[test]
fn cli_plan_tree_missing_plan_id_fails() {
    let result = PlanCli::try_parse_from(["cvg-plan-test", "tree"]);
    assert!(result.is_err(), "missing required plan_id should fail");
}
