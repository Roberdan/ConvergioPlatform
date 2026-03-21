use std::path::PathBuf;

use convergiomesh_core::{
    env::{export_all, import_all, Selections},
    profiles::list_profiles,
};

use super::helpers::{convergio_dir, json_err, json_ok};

pub fn handle_env_export(output: String) {
    let github_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("GitHub");
    let env_bundle = export_all(&github_dir, &[]);
    let json_bytes = serde_json::to_vec_pretty(&env_bundle)
        .unwrap_or_else(|e| json_err(&format!("serialise env bundle: {e}")));
    std::fs::write(&output, &json_bytes)
        .unwrap_or_else(|e| json_err(&format!("write env bundle: {e}")));
    json_ok(serde_json::json!({
        "exported": output,
        "has_brew": env_bundle.brewfile.is_some(),
        "has_vscode": env_bundle.vscode_extensions.is_some(),
        "has_repos": env_bundle.repos.is_some(),
        "has_shell": env_bundle.shell.is_some()
    }));
}

pub fn handle_env_import(bundle: String, profile: Option<String>) {
    let data = std::fs::read(&bundle)
        .unwrap_or_else(|e| json_err(&format!("read env bundle: {e}")));
    let env_bundle: convergiomesh_core::env::EnvBundle =
        serde_json::from_slice(&data)
        .unwrap_or_else(|e| json_err(&format!("parse env bundle: {e}")));

    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    let selections = if let Some(profile_name) = profile {
        let profiles_dir = convergio_dir().join("profiles");
        let profiles = list_profiles(&profiles_dir);
        let prof = profiles.iter().find(|p| p.name == profile_name)
            .unwrap_or_else(|| json_err(&format!("profile '{}' not found", profile_name)));
        let mods = &prof.modules;
        Selections {
            brew: mods.contains(&"brew".into()),
            vscode: mods.contains(&"vscode".into()),
            repos: mods.contains(&"repos".into()),
            shell: mods.contains(&"shell".into()),
            macos: mods.contains(&"macos".into()),
            runners: mods.contains(&"runners".into()),
        }
    } else {
        Selections::all()
    };

    import_all(&env_bundle, &selections, &home, Some(&home.join("GitHub")))
        .unwrap_or_else(|e| json_err(&format!("import_all failed: {e}")));

    json_ok(serde_json::json!({ "imported": bundle }));
}

pub fn handle_env_list_profiles() {
    let profiles_dir = convergio_dir().join("profiles");
    let profiles = list_profiles(&profiles_dir);
    let list: Vec<_> = profiles.iter().map(|p| serde_json::json!({
        "name": p.name,
        "description": p.description,
        "modules": p.modules
    })).collect();
    json_ok(serde_json::json!({ "profiles": list }));
}
