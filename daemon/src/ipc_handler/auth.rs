use super::types::AuthCommands;
use super::utils::default_db_path;

pub async fn handle_auth(command: AuthCommands) {
    let db_path = match &command {
        AuthCommands::Store { db_path, .. }
        | AuthCommands::List { db_path }
        | AuthCommands::Get { db_path, .. }
        | AuthCommands::Revoke { db_path, .. }
        | AuthCommands::Rotate { db_path, .. } => db_path.clone().unwrap_or_else(default_db_path),
    };
    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("db open failed: {e}");
            std::process::exit(2);
        }
    };
    match command {
        AuthCommands::Store {
            service,
            token,
            secret,
            ..
        } => {
            match claude_core::ipc::auth_sync::store_token(&conn, &service, &token, &secret) {
                Ok(()) => println!("stored token for {service}"),
                Err(e) => {
                    eprintln!("store failed: {e}");
                    std::process::exit(2);
                }
            }
        }
        AuthCommands::List { .. } => {
            match claude_core::ipc::auth_sync::list_tokens(&conn) {
                Ok(tokens) => {
                    println!("{:<20} {:<20} {}", "SERVICE", "HOST", "UPDATED");
                    for t in &tokens {
                        println!("{:<20} {:<20} {}", t.service, t.host, t.updated_at);
                    }
                    println!("\n{} token(s)", tokens.len());
                }
                Err(e) => {
                    eprintln!("list failed: {e}");
                    std::process::exit(2);
                }
            }
        }
        AuthCommands::Get {
            service, secret, ..
        } => match claude_core::ipc::auth_sync::get_token(&conn, &service, &secret) {
            Ok(Some(val)) => println!("{val}"),
            Ok(None) => {
                eprintln!("no token found for {service}");
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("get failed: {e}");
                std::process::exit(2);
            }
        },
        AuthCommands::Revoke { service, host, .. } => {
            let h = host.unwrap_or_else(|| {
                hostname::get()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "unknown".to_string())
            });
            match claude_core::ipc::auth_sync::revoke_token(&conn, &service, &h) {
                Ok(n) => println!("revoked {n} token(s) for {service}@{h}"),
                Err(e) => {
                    eprintln!("revoke failed: {e}");
                    std::process::exit(2);
                }
            }
        }
        AuthCommands::Rotate {
            old_secret,
            new_secret,
            ..
        } => {
            match claude_core::ipc::auth_sync::rotate_keys(&conn, &old_secret, &new_secret) {
                Ok(n) => println!("rotated {n} token(s)"),
                Err(e) => {
                    eprintln!("rotate failed: {e}");
                    std::process::exit(2);
                }
            }
        }
    }
}

