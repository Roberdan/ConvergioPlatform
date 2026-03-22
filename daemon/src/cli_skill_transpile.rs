// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Transpile subcommand implementation — converts skill.yaml + SKILL.md to provider formats.

use std::path::PathBuf;

pub fn handle_transpile(skill_dir: &PathBuf, output_dir: &PathBuf, provider: &str, human: bool) {
    let yaml_path = skill_dir.join("skill.yaml");
    let md_path = skill_dir.join("SKILL.md");

    for p in [&yaml_path, &md_path] {
        if !p.is_file() {
            eprintln!("missing: {}", p.display());
            std::process::exit(2);
        }
    }

    let yaml_content = match std::fs::read_to_string(&yaml_path) {
        Ok(c) => c,
        Err(e) => { eprintln!("read error: {e}"); std::process::exit(2); }
    };
    let md_content = match std::fs::read_to_string(&md_path) {
        Ok(c) => c,
        Err(e) => { eprintln!("read error: {e}"); std::process::exit(2); }
    };

    let output = match provider {
        "claude-code" => transpile_claude(&yaml_content, &md_content, output_dir),
        "copilot-cli" => transpile_copilot(&yaml_content, &md_content, output_dir),
        "generic-llm" => transpile_generic(&yaml_content, &md_content, output_dir),
        other => {
            eprintln!("unknown provider '{other}'; use: claude-code, copilot-cli, generic-llm");
            std::process::exit(2);
        }
    };

    match output {
        Ok(path) => {
            if human {
                println!("Written: {}", path.display());
            } else {
                println!("{}", serde_json::json!({"ok": true, "path": path.display().to_string()}));
            }
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    }
}

pub fn transpile_claude(yaml: &str, md: &str, output_dir: &PathBuf) -> Result<PathBuf, String> {
    let name = crate::cli_skill::yaml_get(yaml, "name").ok_or("skill.yaml missing 'name'")?;
    let version = crate::cli_skill::yaml_get(yaml, "version").ok_or("skill.yaml missing 'version'")?;
    let description = crate::cli_skill::yaml_get(yaml, "description").unwrap_or_default();
    let arguments = crate::cli_skill::yaml_get(yaml, "arguments").unwrap_or_default();

    let md_body = crate::cli_skill::strip_h1(md);
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("name: {name}\n"));
    out.push_str(&format!("version: \"{version}\"\n"));
    out.push_str("---\n\n");
    out.push_str(&format!("<!-- v{version} -->\n\n"));
    out.push_str(&format!("# {name}\n\n"));
    if !description.is_empty() {
        out.push_str(&format!("{description}\n\n"));
    }
    if !arguments.is_empty() && arguments != "none" {
        out.push_str(&format!("ARGUMENTS: {arguments}\n\n"));
    }
    out.push_str(&md_body);

    let out_path = output_dir.join(format!("{name}.md"));
    std::fs::write(&out_path, &out).map_err(|e| format!("write error: {e}"))?;
    Ok(out_path)
}

pub fn transpile_copilot(yaml: &str, md: &str, output_dir: &PathBuf) -> Result<PathBuf, String> {
    let name = crate::cli_skill::yaml_get(yaml, "name").ok_or("skill.yaml missing 'name'")?;
    let description = crate::cli_skill::yaml_get(yaml, "description").unwrap_or_default();
    let model = crate::cli_skill::yaml_get(yaml, "model").unwrap_or_default();

    let md_body = crate::cli_skill::strip_h1(md);
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("name: {name}\n"));
    if !description.is_empty() {
        out.push_str(&format!("description: {description}\n"));
    }
    if !model.is_empty() {
        out.push_str(&format!("model: {model}\n"));
    }
    out.push_str("---\n\n");
    let cap_name = crate::cli_skill::capitalise(&name);
    out.push_str(&format!("# {cap_name}\n\n"));
    out.push_str(&md_body);

    let out_path = output_dir.join(format!("{name}.agent.md"));
    std::fs::write(&out_path, &out).map_err(|e| format!("write error: {e}"))?;
    Ok(out_path)
}

pub fn transpile_generic(yaml: &str, md: &str, output_dir: &PathBuf) -> Result<PathBuf, String> {
    let name = crate::cli_skill::yaml_get(yaml, "name").ok_or("skill.yaml missing 'name'")?;
    let description = crate::cli_skill::yaml_get(yaml, "description").unwrap_or_default();
    let domain = crate::cli_skill::yaml_get(yaml, "domain").unwrap_or_else(|| "general".into());
    let const_ver = crate::cli_skill::yaml_get(yaml, "constitution-version").unwrap_or_else(|| "2.0.0".into());
    let license = crate::cli_skill::yaml_get(yaml, "license").unwrap_or_else(|| "MPL-2.0".into());

    let mut out = String::new();
    out.push_str(&format!("You are {name}, a {description}.\n\n"));
    out.push_str(&format!("Domain: {domain}\n"));
    out.push_str(&format!("Constitution: v{const_ver}\n"));
    out.push_str(&format!("License: {license}\n\n"));
    out.push_str("## Instructions\n\n");
    out.push_str(md);
    out.push_str("\n\n## Constraints\n");
    out.push_str(&format!("- Follow the Convergio Constitution v{const_ver}\n"));
    out.push_str(&format!("- {license} licensed\n"));

    let safe_name: String = name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    let out_path = output_dir.join(format!("{safe_name}.system-prompt.txt"));
    std::fs::write(&out_path, &out).map_err(|e| format!("write error: {e}"))?;
    Ok(out_path)
}
