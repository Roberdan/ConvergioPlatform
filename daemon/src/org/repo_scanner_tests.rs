//! Tests for the repo scanner engine.

use std::fs;
use std::path::Path;

use super::repo_scanner::scan_repo;

/// Scan the ConvergioPlatform repo itself — it lives at the worktree root.
#[test]
fn scan_convergio_platform() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("repo root");
    let profile = scan_repo(root).expect("scan should succeed");

    // Must detect Rust as a primary language (the daemon is Rust).
    let rust_entry = profile.languages.iter().find(|(l, _)| l == "Rust");
    assert!(rust_entry.is_some(), "should detect Rust");
    let (_, rust_count) = rust_entry.unwrap();
    assert!(*rust_count > 50, "should have many Rust files, got {rust_count}");

    // Languages are sorted descending by count.
    for window in profile.languages.windows(2) {
        assert!(window[0].1 >= window[1].1, "languages not sorted desc");
    }

    // Framework detection — daemon uses axum.
    assert!(
        profile.frameworks.iter().any(|f| f == "Axum"),
        "should detect Axum framework, got: {:?}",
        profile.frameworks
    );

    // Structure checks.
    assert!(profile.structure.has_src, "should detect src dir");
    assert!(profile.structure.has_tests, "should detect tests dir");
    assert!(!profile.structure.manifest_files.is_empty(), "should find manifests");

    // CI detection.
    assert!(profile.ci.is_some(), "should detect CI");
    let ci = profile.ci.as_ref().unwrap();
    assert_eq!(ci.provider, "github-actions");
    assert!(!ci.workflows.is_empty(), "should have workflow files");

    // README present.
    assert!(!profile.readme_summary.is_empty(), "should have README content");
    assert!(profile.readme_summary.len() <= 600, "readme should be truncated");

    // Totals.
    assert!(profile.total_files > 100, "should have many files");
    assert!(profile.total_lines > 1000, "should have many lines");
}

/// Scanning an empty temp directory should produce an empty profile.
#[test]
fn scan_empty_directory() {
    let tmp = tempfile::tempdir().expect("create temp dir");
    let profile = scan_repo(tmp.path()).expect("scan should succeed");

    assert!(profile.languages.is_empty(), "no languages in empty dir");
    assert!(profile.frameworks.is_empty(), "no frameworks in empty dir");
    assert_eq!(profile.total_files, 0);
    assert_eq!(profile.total_lines, 0);
    assert!(profile.ci.is_none(), "no CI in empty dir");
    assert!(profile.readme_summary.is_empty(), "no README in empty dir");
    assert!(profile.dependencies.is_empty(), "no deps in empty dir");
}

/// Scanning a non-existent path should return an error.
#[test]
fn scan_nonexistent_path() {
    let result = scan_repo(Path::new("/tmp/convergio-does-not-exist-9999"));
    assert!(result.is_err(), "should error on non-existent path");
}

/// Language detection mapping: verify known extensions produce correct labels.
#[test]
fn language_detection_from_files() {
    let tmp = tempfile::tempdir().expect("create temp dir");
    let root = tmp.path();

    // Create files with various extensions.
    fs::write(root.join("main.rs"), "fn main() {}").expect("write");
    fs::write(root.join("app.ts"), "export {}").expect("write");
    fs::write(root.join("index.tsx"), "export {}").expect("write");
    fs::write(root.join("util.js"), "module.exports = {}").expect("write");
    fs::write(root.join("script.py"), "pass").expect("write");
    fs::write(root.join("data.csv"), "a,b,c").expect("write"); // not a language

    let profile = scan_repo(root).expect("scan");

    let lang_names: Vec<&str> = profile.languages.iter().map(|(l, _)| l.as_str()).collect();
    assert!(lang_names.contains(&"Rust"), "should detect Rust");
    assert!(lang_names.contains(&"TypeScript"), "should detect TypeScript");
    assert!(lang_names.contains(&"JavaScript"), "should detect JavaScript");
    assert!(lang_names.contains(&"Python"), "should detect Python");
    assert!(!lang_names.contains(&"CSV"), "should not detect CSV as language");

    // TypeScript should count 2 files (.ts + .tsx).
    let ts = profile.languages.iter().find(|(l, _)| l == "TypeScript").unwrap();
    assert_eq!(ts.1, 2, "TypeScript should count .ts and .tsx");
}

/// Framework detection from a mock package.json.
#[test]
fn framework_detection_from_package_json() {
    let tmp = tempfile::tempdir().expect("create temp dir");
    let root = tmp.path();

    let pkg = r#"{
  "name": "test-project",
  "dependencies": {
    "next": "14.0.0",
    "react": "18.0.0"
  }
}"#;
    fs::write(root.join("package.json"), pkg).expect("write");

    let profile = scan_repo(root).expect("scan");

    // "next" present means Next.js detected; React is suppressed when Next.js exists.
    assert!(
        profile.frameworks.contains(&"Next.js".to_string()),
        "should detect Next.js, got: {:?}",
        profile.frameworks
    );
    assert!(
        !profile.frameworks.contains(&"React".to_string()),
        "React should be suppressed when Next.js detected"
    );
}

/// Framework detection from a mock Cargo.toml.
#[test]
fn framework_detection_from_cargo_toml() {
    let tmp = tempfile::tempdir().expect("create temp dir");
    let root = tmp.path();

    let cargo = r#"[package]
name = "test-daemon"
version = "0.1.0"

[dependencies]
axum = "0.7"
tokio = "1"
"#;
    fs::write(root.join("Cargo.toml"), cargo).expect("write");

    let profile = scan_repo(root).expect("scan");

    assert!(
        profile.frameworks.contains(&"Axum".to_string()),
        "should detect Axum from Cargo.toml, got: {:?}",
        profile.frameworks
    );
}

/// Dependencies parsing extracts crate names from Cargo.toml.
#[test]
fn dependencies_from_cargo_toml() {
    let tmp = tempfile::tempdir().expect("create temp dir");
    let root = tmp.path();

    let cargo = r#"[package]
name = "dep-test"

[dependencies]
serde = "1"
tokio = { version = "1", features = ["full"] }

[dev-dependencies]
tempfile = "3"
"#;
    fs::write(root.join("Cargo.toml"), cargo).expect("write");

    let profile = scan_repo(root).expect("scan");

    assert!(profile.dependencies.contains(&"serde".to_string()), "should find serde");
    assert!(profile.dependencies.contains(&"tokio".to_string()), "should find tokio");
    // dev-dependencies are under a different section header, not parsed
}

/// Tauri detection from src-tauri directory.
#[test]
fn tauri_detection() {
    let tmp = tempfile::tempdir().expect("create temp dir");
    let root = tmp.path();
    fs::create_dir(root.join("src-tauri")).expect("mkdir");

    let profile = scan_repo(root).expect("scan");
    assert!(
        profile.frameworks.contains(&"Tauri".to_string()),
        "should detect Tauri from src-tauri dir"
    );
}
