use super::*;

// -- Wave parsing tests --

#[test]
fn cli_plan_wave_update_parses_id_and_status() {
    let cli = WaveCli::try_parse_from(["cvg-wave-test", "update", "3", "done"]).expect("parse");
    if let WaveCommands::Update {
        wave_id, status, ..
    } = cli.command
    {
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
    let cli = WaveCli::try_parse_from(["cvg-wave-test", "validate", "7", "685"]).expect("parse");
    if let WaveCommands::Validate {
        wave_id, plan_id, ..
    } = cli.command
    {
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
