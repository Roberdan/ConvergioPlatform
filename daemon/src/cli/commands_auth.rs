use std::path::PathBuf;

use convergiomesh_core::auth::{
    decrypt_bundle, encrypt_bundle, export_credentials, import_credentials, load_bundle,
    save_bundle,
};

use super::helpers::{json_err, json_ok};

pub fn handle_auth_export(output: String) {
    let bundle = export_credentials()
        .unwrap_or_else(|e| json_err(&format!("export_credentials failed: {e}")));

    let token_str = std::env::var("CONVERGIO_MESH_TOKEN")
        .unwrap_or_else(|_| "mesh-transfer".into());
    let password = rpassword::prompt_password("Encryption password: ")
        .unwrap_or_else(|e| json_err(&format!("password prompt failed: {e}")));

    let encrypted = encrypt_bundle(&bundle, &token_str, &password)
        .unwrap_or_else(|e| json_err(&format!("encrypt_bundle failed: {e}")));

    let out_path = PathBuf::from(&output);
    save_bundle(&encrypted, &out_path)
        .unwrap_or_else(|e| json_err(&format!("save_bundle failed: {e}")));

    json_ok(serde_json::json!({
        "exported": output,
        "has_claude": bundle.claude_creds.is_some(),
        "has_gh": bundle.gh_token.is_some(),
        "has_azure": bundle.az_tokens.is_some()
    }));
}

pub fn handle_auth_import(bundle: String) {
    let bundle_path = PathBuf::from(&bundle);
    let encrypted = load_bundle(&bundle_path)
        .unwrap_or_else(|e| json_err(&format!("load_bundle failed: {e}")));

    let token_str = std::env::var("CONVERGIO_MESH_TOKEN")
        .unwrap_or_else(|_| "mesh-transfer".into());
    let password = rpassword::prompt_password("Decryption password: ")
        .unwrap_or_else(|e| json_err(&format!("decrypt_bundle failed: {e}")));

    let auth_bundle = decrypt_bundle(&encrypted, &token_str, &password)
        .unwrap_or_else(|e| json_err(&format!("decrypt_bundle failed: {e}")));

    import_credentials(&auth_bundle)
        .unwrap_or_else(|e| json_err(&format!("import_credentials failed: {e}")));

    json_ok(serde_json::json!({ "imported": bundle }));
}
