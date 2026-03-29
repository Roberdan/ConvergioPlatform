use super::*;

#[test]
fn test_selections_all() {
    let sel = Selections::all();
    assert!(sel.brew && sel.vscode && sel.repos && sel.shell && sel.macos && sel.runners);
}

#[test]
fn test_env_bundle_default() {
    let bundle = EnvBundle::default();
    assert!(bundle.brewfile.is_none());
    assert!(bundle.repos.is_none());
}

#[test]
fn test_export_all_nonexistent_github_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let github_dir = tmp.path().join("nonexistent");
    let bundle = export_all(&github_dir, &[]);
    assert!(bundle.repos.is_none());
}

#[test]
fn test_export_all_empty_github_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let bundle = export_all(tmp.path(), &[]);
    assert!(bundle.repos.is_some());
    assert_eq!(bundle.repos.unwrap().len(), 0);
}

// ── Additional tests ─────────────────────────────────────────────────────────

#[test]
fn test_selections_default_all_false() {
    let sel = Selections::default();
    assert!(!sel.brew);
    assert!(!sel.vscode);
    assert!(!sel.repos);
    assert!(!sel.shell);
    assert!(!sel.macos);
    assert!(!sel.runners);
}

#[test]
fn test_env_bundle_serialization_roundtrip() {
    let bundle = EnvBundle {
        brewfile: None,
        vscode_extensions: Some(vec!["ext1".into(), "ext2".into()]),
        vscode_settings: Some("{\"editor.fontSize\": 14}".into()),
        repos: None,
        shell: None,
        runners: Some(vec![runners::RunnerConfig {
            name: "runner-1".into(),
            path: std::path::PathBuf::from("/tmp/runner"),
            labels: vec!["self-hosted".into()],
            repository: Some("owner/repo".into()),
            service_name: None,
        }]),
    };
    let json = serde_json::to_string(&bundle).unwrap();
    let back: EnvBundle = serde_json::from_str(&json).unwrap();
    assert_eq!(back.vscode_extensions.as_ref().unwrap().len(), 2);
    assert!(back.runners.is_some());
    assert_eq!(back.runners.unwrap()[0].name, "runner-1");
}

#[test]
fn test_import_all_with_no_selections() {
    let bundle = EnvBundle::default();
    let sel = Selections::default(); // all false
    let tmp = tempfile::tempdir().unwrap();
    // Should not error — no operations performed
    import_all(&bundle, &sel, tmp.path(), None).unwrap();
}

#[test]
fn test_import_all_with_shell_selection() {
    let bundle = EnvBundle {
        shell: Some(shell::ShellConfig {
            zshrc: Some("export PATH=/usr/local/bin:$PATH\n".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let sel = Selections {
        shell: true,
        ..Default::default()
    };
    let tmp = tempfile::tempdir().unwrap();
    import_all(&bundle, &sel, tmp.path(), None).unwrap();
    let written = std::fs::read_to_string(tmp.path().join(".zshrc")).unwrap();
    assert!(written.contains("/usr/local/bin"));
}

#[test]
fn test_export_all_with_runner_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let runner_dir = tmp.path().join("runner");
    std::fs::create_dir(&runner_dir).unwrap();
    std::fs::write(
        runner_dir.join(".runner"),
        r#"{"name":"test-runner","labels":["ci"]}"#,
    )
    .unwrap();
    let bundle = export_all(
        &tmp.path().join("nonexistent"),
        &[runner_dir.to_string_lossy().to_string()],
    );
    assert!(bundle.runners.is_some());
    assert_eq!(bundle.runners.unwrap()[0].name, "test-runner");
}
