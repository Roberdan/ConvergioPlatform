mod commands_auth;
mod commands_coordinator;
mod commands_env;
mod helpers;
mod types;

use clap::Parser;
use convergiomesh_core::{
    network::tailscale_status,
    peers::{PeerConfig, PeersRegistry},
    token::{generate_token, revoke_token},
};
use types::{AuthAction, Cli, CoordinatorAction, Commands, EnvAction, NetworkAction, PeersAction};

use helpers::{default_peers_path, json_err, json_ok, load_or_create_secret, open_token_db};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Invite { role, ttl, qr } => {
            let secret = load_or_create_secret()
                .unwrap_or_else(|e| json_err(&format!("secret error: {e}")));

            let my_ip = tailscale_status()
                .map(|s| s.self_ip)
                .unwrap_or_else(|_| "127.0.0.1".into());

            let token = generate_token(
                &secret,
                &role,
                vec!["claude".into()],
                &my_ip,
                ttl as i64,
            )
            .unwrap_or_else(|e| json_err(&format!("generate_token failed: {e}")));

            if qr {
                let qr_str = convergiomesh_core::qr::generate_qr_terminal(&token)
                    .unwrap_or_else(|_| json_err("qr generation failed"));
                eprintln!("{}", qr_str); // QR to stderr, JSON to stdout
            }

            json_ok(serde_json::json!({
                "token": token,
                "role": role,
                "ttl_minutes": ttl,
                "coordinator_ip": my_ip
            }));
        }

        Commands::Join { token, password, yes: _, profiles } => {
            use convergiomesh_core::join::{join, JoinConfig, JoinSelections};
            let secret = load_or_create_secret()
                .unwrap_or_else(|e| json_err(&format!("secret error: {e}")));
            let conn = open_token_db();

            let admin_password = password.unwrap_or_else(|| {
                rpassword::prompt_password("Admin (sudo) password: ")
                    .unwrap_or_else(|e| json_err(&format!("password prompt failed: {e}")))
            });

            let config = JoinConfig {
                token: token.clone(),
                admin_password,
                profiles,
                interactive: true,
                selections: JoinSelections::all(),
            };

            let progress = join(config, &secret, &conn).await
                .unwrap_or_else(|e| json_err(&format!("join failed: {e}")));

            let steps: Vec<_> = progress.iter().map(|p| serde_json::json!({
                "step": p.step,
                "current": p.current,
                "status": format!("{:?}", p.status)
            })).collect();

            json_ok(serde_json::json!({ "steps": steps, "token": token }));
        }

        Commands::Revoke { nonce } => {
            let conn = open_token_db();
            revoke_token(&nonce, &conn)
                .unwrap_or_else(|e| json_err(&format!("revoke failed: {e}")));
            json_ok(serde_json::json!({ "revoked": nonce }));
        }

        Commands::Peers { action } => {
            let peers_path = default_peers_path();
            match action {
                PeersAction::List => {
                    if !peers_path.exists() {
                        json_ok(serde_json::json!({ "peers": [] }));
                        return;
                    }
                    let registry = PeersRegistry::load(&peers_path)
                        .unwrap_or_else(|e| json_err(&format!("load peers failed: {e}")));
                    let active: Vec<_> = registry.list_active().iter().map(|(name, cfg)| {
                        serde_json::json!({
                            "name": name,
                            "role": cfg.role,
                            "os": cfg.os,
                            "tailscale_ip": cfg.tailscale_ip,
                            "status": cfg.status,
                            "capabilities": cfg.capabilities
                        })
                    }).collect();
                    json_ok(serde_json::json!({ "peers": active }));
                }
                PeersAction::Add { name, config_json } => {
                    let peer_cfg: PeerConfig = serde_json::from_str(&config_json)
                        .unwrap_or_else(|e| json_err(&format!("invalid peer config JSON: {e}")));

                    let mut registry = if peers_path.exists() {
                        PeersRegistry::load(&peers_path)
                            .unwrap_or_else(|e| json_err(&format!("load peers failed: {e}")))
                    } else {
                        PeersRegistry {
                            shared_secret: String::new(),
                            peers: std::collections::BTreeMap::new(),
                        }
                    };

                    registry.add_peer(&name, peer_cfg);
                    std::fs::create_dir_all(peers_path.parent().unwrap()).ok();
                    registry.save(&peers_path)
                        .unwrap_or_else(|e| json_err(&format!("save peers failed: {e}")));
                    json_ok(serde_json::json!({ "added": name }));
                }
                PeersAction::Remove { name } => {
                    if !peers_path.exists() {
                        json_err("peers.conf not found");
                    }
                    let mut registry = PeersRegistry::load(&peers_path)
                        .unwrap_or_else(|e| json_err(&format!("load peers failed: {e}")));
                    registry.remove_peer(&name)
                        .unwrap_or_else(|| json_err(&format!("peer '{}' not found", name)));
                    registry.save(&peers_path)
                        .unwrap_or_else(|e| json_err(&format!("save peers failed: {e}")));
                    json_ok(serde_json::json!({ "removed": name }));
                }
            }
        }

        Commands::Network { action } => match action {
            NetworkAction::Status => {
                let status = tailscale_status()
                    .unwrap_or_else(|e| json_err(&format!("tailscale_status failed: {e}")));
                json_ok(serde_json::json!({
                    "self_ip": status.self_ip,
                    "self_name": status.self_name,
                    "peers": status.peers.iter().map(|p| serde_json::json!({
                        "hostname": p.hostname,
                        "ip": p.ip,
                        "online": p.online,
                        "os": p.os
                    })).collect::<Vec<_>>()
                }));
            }
        },

        Commands::Auth { action } => match action {
            AuthAction::Export { output } => commands_auth::handle_auth_export(output),
            AuthAction::Import { bundle } => commands_auth::handle_auth_import(bundle),
        },

        Commands::Env { action } => match action {
            EnvAction::Export { output } => commands_env::handle_env_export(output),
            EnvAction::Import { bundle, profile } => commands_env::handle_env_import(bundle, profile),
            EnvAction::ListProfiles => commands_env::handle_env_list_profiles(),
        },

        Commands::Coordinator { action } => match action {
            CoordinatorAction::Migrate { to } => {
                commands_coordinator::handle_coordinator_migrate(to).await;
            }
            CoordinatorAction::Rollback { snapshot } => {
                commands_coordinator::handle_coordinator_rollback(snapshot).await;
            }
            CoordinatorAction::Status => commands_coordinator::handle_coordinator_status(),
        },

        Commands::Package { usb } => {
            use convergiomesh_core::{auth::export_credentials, env::export_all};
            let env_bundle = export_all(
                &dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp")).join("GitHub"),
                &[],
            );
            let auth_bundle = export_credentials().ok();

            let target = usb.as_deref().unwrap_or("/tmp/convergio-package");
            std::fs::create_dir_all(target)
                .unwrap_or_else(|e| json_err(&format!("create package dir failed: {e}")));

            let env_path = format!("{}/env-bundle.json", target);
            let env_bytes = serde_json::to_vec_pretty(&env_bundle)
                .unwrap_or_else(|e| json_err(&format!("serialise env bundle: {e}")));
            std::fs::write(&env_path, env_bytes)
                .unwrap_or_else(|e| json_err(&format!("write env bundle: {e}")));

            json_ok(serde_json::json!({
                "package_dir": target,
                "env_bundle": env_path,
                "auth_exported": auth_bundle.is_some(),
                "usb": usb
            }));
        }
    }
}
