use super::*;

#[test]
fn test_extract_aliases() {
    let zshrc = "alias gs='git status'\nalias ll='ls -la'\nexport PATH=$PATH:/usr/local/bin\n";
    let aliases = extract_aliases(zshrc);
    assert_eq!(aliases.len(), 2);
    assert!(aliases[0].contains("gs="));
    assert!(aliases[1].contains("ll="));
}

#[test]
fn test_export_import_roundtrip() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();

    // Create source files
    std::fs::write(src.path().join(".zshrc"), "alias gs='git status'\n").unwrap();
    std::fs::create_dir_all(src.path().join(".config")).unwrap();
    std::fs::write(
        src.path().join(".config/starship.toml"),
        "[character]\nsymbol = \"➜\"\n",
    )
    .unwrap();

    let config = export_shell_config_from(src.path()).unwrap();
    assert!(config.zshrc.is_some());
    assert!(config.starship_toml.is_some());
    assert_eq!(config.aliases.len(), 1);

    import_shell_config(&config, dst.path()).unwrap();

    let written_zshrc = std::fs::read_to_string(dst.path().join(".zshrc")).unwrap();
    assert_eq!(written_zshrc, "alias gs='git status'\n");

    let written_starship =
        std::fs::read_to_string(dst.path().join(".config/starship.toml")).unwrap();
    assert!(written_starship.contains("character"));
}
