use clap::{Parser, Subcommand};
use serde_json::json;
use std::path::PathBuf;

use convergiomesh_core::{
    auth::{decrypt_bundle, export_credentials, import_credentials, load_bundle, save_bundle,
           encrypt_bundle},
    coordinator::{load_migration_state, migrate_coordinator, rollback},
    env::{export_all, import_all, Selections},
    join::{join, JoinConfig, JoinSelections},
    network::tailscale_status,
    peers::{PeerConfig, PeersRegistry},
    profiles::list_profiles,
    token::{generate_token, init_token_db, revoke_token},
};

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "convergiomesh-cli", version, about = "Mesh node onboarding and environment migration tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate an invite token
    Invite {
        #[arg(long, default_value = "worker")]
        role: String,
        #[arg(long, default_value_t = 30)]
        ttl: u64,
        #[arg(long)]
        qr: bool,
    },
    /// Join mesh with invite token
    Join {
        token: String,
        #[arg(long)]
        password: Option<String>,
        #[arg(long)]
        yes: bool,
        #[arg(long, value_delimiter = ',')]
        profiles: Vec<String>,
    },
    /// Revoke an invite token by nonce
    Revoke {
        nonce: String,
    },
    /// Peer management
    Peers {
        #[command(subcommand)]
        action: PeersAction,
    },
    /// Network status
    Network {
        #[command(subcommand)]
        action: NetworkAction,
    },
    /// Auth credential management
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
    /// Environment management
    Env {
        #[command(subcommand)]
        action: EnvAction,
    },
    /// Coordinator management
    Coordinator {
        #[command(subcommand)]
        action: CoordinatorAction,
    },
    /// Package for USB/distribution
    Package {
        #[arg(long)]
        usb: Option<String>,
    },
}

#[derive(Subcommand)]
enum PeersAction {
    /// List active peers
    List,
    /// Add a peer (config_json must be a JSON PeerConfig)
    Add { name: String, config_json: String },
    /// Remove a peer by name
    Remove { name: String },
}

#[derive(Subcommand)]
enum NetworkAction {
    /// Show Tailscale status
    Status,
}

#[derive(Subcommand)]
enum AuthAction {
    /// Export credentials to encrypted bundle
    Export {
        #[arg(long)]
        output: String,
    },
    /// Import credentials from encrypted bundle
    Import {
        #[arg(long)]
        bundle: String,
    },
}

#[derive(Subcommand)]
enum EnvAction {
    /// Export environment bundle to file
    Export {
        #[arg(long)]
        output: String,
    },
    /// Import environment bundle from file
    Import {
        #[arg(long)]
        bundle: String,
        #[arg(long)]
        profile: Option<String>,
    },
    /// List available profiles
    ListProfiles,
}

#[derive(Subcommand)]
enum CoordinatorAction {
    /// Migrate coordinator role to a new node
    Migrate {
        #[arg(long)]
        to: String,
    },
    /// Rollback coordinator migration from a snapshot file
    Rollback {
        #[arg(long)]
        snapshot: String,
    },
    /// Show coordinator migration status
    Status,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn convergio_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".convergio")
}

fn load_or_create_secret() -> Result<Vec<u8>, String> {
    let path = convergio_dir().join("mesh-secret");
    if path.exists() {
        let s = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        return Ok(s.trim().as_bytes().to_vec());
    }
    // Generate a random secret and persist it
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&path, &hex).map_err(|e| e.to_string())?;
    Ok(hex.as_bytes().to_vec())
}

fn default_peers_path() -> PathBuf {
    let candidate = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".claude/config/peers.conf");
    if candidate.exists() {
        return candidate;
    }
    convergio_dir().join("peers.conf")
}

fn json_ok(data: serde_json::Value) {
    println!("{}", serde_json::to_string_pretty(&json!({"success": true, "data": data}))
        .unwrap_or_else(|_| r#"{"success":true}"#.into()));
}

fn json_err(msg: &str) -> ! {
    eprintln!("{}", serde_json::to_string_pretty(&json!({"success": false, "error": msg}))
        .unwrap_or_else(|_| format!(r#"{{"success":false,"error":{:?}}}"#, msg)));
    std::process::exit(1);
}

fn open_token_db() -> rusqlite::Connection {
    let db_path = convergio_dir().join("tokens.db");
    std::fs::create_dir_all(db_path.parent().unwrap())
        .unwrap_or_else(|e| json_err(&format!("cannot create ~/.convergio: {e}")));
    let conn = rusqlite::Connection::open(&db_path)
        .unwrap_or_else(|e| json_err(&format!("cannot open tokens.db: {e}")));
    init_token_db(&conn)
        .unwrap_or_else(|e| json_err(&format!("cannot init token db: {e}")));
    conn
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        // ── Invite ────────────────────────────────────────────────────────────
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

            json_ok(json!({
                "token": token,
                "role": role,
                "ttl_minutes": ttl,
                "coordinator_ip": my_ip
            }));
        }

        // ── Join ──────────────────────────────────────────────────────────────
        Commands::Join { token, password, yes: _, profiles } => {
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

            let steps: Vec<_> = progress.iter().map(|p| json!({
                "step": p.step,
                "current": p.current,
                "status": format!("{:?}", p.status)
            })).collect();

            json_ok(json!({ "steps": steps, "token": token }));
        }

        // ── Revoke ────────────────────────────────────────────────────────────
        Commands::Revoke { nonce } => {
            let conn = open_token_db();
            revoke_token(&nonce, &conn)
                .unwrap_or_else(|e| json_err(&format!("revoke failed: {e}")));
            json_ok(json!({ "revoked": nonce }));
        }

        // ── Peers ─────────────────────────────────────────────────────────────
        Commands::Peers { action } => {
            let peers_path = default_peers_path();
            match action {
                PeersAction::List => {
                    if !peers_path.exists() {
                        json_ok(json!({ "peers": [] }));
                        return;
                    }
                    let registry = PeersRegistry::load(&peers_path)
                        .unwrap_or_else(|e| json_err(&format!("load peers failed: {e}")));
                    let active: Vec<_> = registry.list_active().iter().map(|(name, cfg)| {
                        json!({
                            "name": name,
                            "role": cfg.role,
                            "os": cfg.os,
                            "tailscale_ip": cfg.tailscale_ip,
                            "status": cfg.status,
                            "capabilities": cfg.capabilities
                        })
                    }).collect();
                    json_ok(json!({ "peers": active }));
                }
                PeersAction::Add { name, config_json } => {
                    let peer_cfg: PeerConfig = serde_json::from_str(&config_json)
                        .unwrap_or_else(|e| json_err(&format!("invalid peer config JSON: {e}")));

                    // Load or create registry
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
                    json_ok(json!({ "added": name }));
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
                    json_ok(json!({ "removed": name }));
                }
            }
        }

        // ── Network ───────────────────────────────────────────────────────────
        Commands::Network { action } => match action {
            NetworkAction::Status => {
                let status = tailscale_status()
                    .unwrap_or_else(|e| json_err(&format!("tailscale_status failed: {e}")));
                json_ok(json!({
                    "self_ip": status.self_ip,
                    "self_name": status.self_name,
                    "peers": status.peers.iter().map(|p| json!({
                        "hostname": p.hostname,
                        "ip": p.ip,
                        "online": p.online,
                        "os": p.os
                    })).collect::<Vec<_>>()
                }));
            }
        },

        // ── Auth ──────────────────────────────────────────────────────────────
        Commands::Auth { action } => match action {
            AuthAction::Export { output } => {
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

                json_ok(json!({
                    "exported": output,
                    "has_claude": bundle.claude_creds.is_some(),
                    "has_gh": bundle.gh_token.is_some(),
                    "has_azure": bundle.az_tokens.is_some()
                }));
            }
            AuthAction::Import { bundle } => {
                let bundle_path = PathBuf::from(&bundle);
                let encrypted = load_bundle(&bundle_path)
                    .unwrap_or_else(|e| json_err(&format!("load_bundle failed: {e}")));

                let token_str = std::env::var("CONVERGIO_MESH_TOKEN")
                    .unwrap_or_else(|_| "mesh-transfer".into());
                let password = rpassword::prompt_password("Decryption password: ")
                    .unwrap_or_else(|e| json_err(&format!("password prompt failed: {e}")));

                let auth_bundle = decrypt_bundle(&encrypted, &token_str, &password)
                    .unwrap_or_else(|e| json_err(&format!("decrypt_bundle failed: {e}")));

                import_credentials(&auth_bundle)
                    .unwrap_or_else(|e| json_err(&format!("import_credentials failed: {e}")));

                json_ok(json!({ "imported": bundle }));
            }
        },

        // ── Env ───────────────────────────────────────────────────────────────
        Commands::Env { action } => match action {
            EnvAction::Export { output } => {
                let github_dir = dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
                    .join("GitHub");
                let env_bundle = export_all(&github_dir, &[]);
                let json_bytes = serde_json::to_vec_pretty(&env_bundle)
                    .unwrap_or_else(|e| json_err(&format!("serialise env bundle: {e}")));
                std::fs::write(&output, &json_bytes)
                    .unwrap_or_else(|e| json_err(&format!("write env bundle: {e}")));
                json_ok(json!({
                    "exported": output,
                    "has_brew": env_bundle.brewfile.is_some(),
                    "has_vscode": env_bundle.vscode_extensions.is_some(),
                    "has_repos": env_bundle.repos.is_some(),
                    "has_shell": env_bundle.shell.is_some()
                }));
            }
            EnvAction::Import { bundle, profile } => {
                let data = std::fs::read(&bundle)
                    .unwrap_or_else(|e| json_err(&format!("read env bundle: {e}")));
                let env_bundle: convergiomesh_core::env::EnvBundle =
                    serde_json::from_slice(&data)
                    .unwrap_or_else(|e| json_err(&format!("parse env bundle: {e}")));

                let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
                let selections = if let Some(profile_name) = profile {
                    // Load profile and build Selections from its modules list
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

                json_ok(json!({ "imported": bundle }));
            }
            EnvAction::ListProfiles => {
                let profiles_dir = convergio_dir().join("profiles");
                let profiles = list_profiles(&profiles_dir);
                let list: Vec<_> = profiles.iter().map(|p| json!({
                    "name": p.name,
                    "description": p.description,
                    "modules": p.modules
                })).collect();
                json_ok(json!({ "profiles": list }));
            }
        },

        // ── Coordinator ───────────────────────────────────────────────────────
        Commands::Coordinator { action } => match action {
            CoordinatorAction::Migrate { to } => {
                let peers_path = default_peers_path();
                if !peers_path.exists() {
                    json_err("peers.conf not found — cannot migrate coordinator");
                }
                let mut registry = PeersRegistry::load(&peers_path)
                    .unwrap_or_else(|e| json_err(&format!("load peers failed: {e}")));

                let current_from = registry.get_coordinator()
                    .map(|(name, _)| name.to_string())
                    .unwrap_or_else(|| json_err("no current coordinator found in peers.conf"));

                let state = migrate_coordinator(&mut registry, &current_from, &to).await
                    .unwrap_or_else(|e| json_err(&format!("migrate_coordinator failed: {e}")));

                registry.save(&peers_path)
                    .unwrap_or_else(|e| json_err(&format!("save peers failed: {e}")));

                json_ok(json!({
                    "old_coordinator": state.old_coordinator,
                    "new_coordinator": state.new_coordinator,
                    "completed": state.completed,
                    "started_at": state.started_at
                }));
            }
            CoordinatorAction::Rollback { snapshot } => {
                let data = std::fs::read(&snapshot)
                    .unwrap_or_else(|e| json_err(&format!("read snapshot: {e}")));
                let state: convergiomesh_core::coordinator::MigrationState =
                    serde_json::from_slice(&data)
                    .unwrap_or_else(|e| json_err(&format!("parse snapshot: {e}")));

                rollback(&state).await
                    .unwrap_or_else(|e| json_err(&format!("rollback failed: {e}")));

                json_ok(json!({
                    "rolled_back_to": state.old_coordinator,
                    "from": state.new_coordinator
                }));
            }
            CoordinatorAction::Status => {
                match load_migration_state() {
                    Ok(state) => {
                        json_ok(json!({
                            "old_coordinator": state.old_coordinator,
                            "new_coordinator": state.new_coordinator,
                            "completed": state.completed,
                            "started_at": state.started_at,
                            "snapshots": state.snapshots.len()
                        }));
                    }
                    Err(_) => {
                        // No migration state file — that's normal
                        let peers_path = default_peers_path();
                        let coordinator = if peers_path.exists() {
                            PeersRegistry::load(&peers_path).ok()
                                .and_then(|r| r.get_coordinator().map(|(n, _)| n.to_string()))
                        } else {
                            None
                        };
                        json_ok(json!({
                            "migration_in_progress": false,
                            "current_coordinator": coordinator
                        }));
                    }
                }
            }
        },

        // ── Package ───────────────────────────────────────────────────────────
        Commands::Package { usb } => {
            let env_bundle = export_all(
                &dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp")).join("GitHub"),
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

            json_ok(json!({
                "package_dir": target,
                "env_bundle": env_path,
                "auth_exported": auth_bundle.is_some(),
                "usb": usb
            }));
        }
    }
}
