use super::types::{AuthCommands, IpcHandlerError};
use super::utils::default_db_path;

pub async fn handle_auth(command: AuthCommands) -> Result<(), IpcHandlerError> {
    let db_path = match &command {
        AuthCommands::Store { db_path, .. }
        | AuthCommands::List { db_path }
        | AuthCommands::Get { db_path, .. }
        | AuthCommands::Revoke { db_path, .. }
        | AuthCommands::Rotate { db_path, .. } => db_path.clone().unwrap_or_else(default_db_path),
    };
    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| IpcHandlerError::DbOpen(format!("db open failed: {e}")))?;
    match command {
        AuthCommands::Store {
            service,
            token,
            secret,
            ..
        } => match convergio_core::ipc::auth_sync::store_token(&conn, &service, &token, &secret) {
            Ok(()) => println!("stored token for {service}"),
            Err(e) => {
                return Err(IpcHandlerError::OperationFailed(format!(
                    "store failed: {e}"
                )));
            }
        },
        AuthCommands::List { .. } => match convergio_core::ipc::auth_sync::list_tokens(&conn) {
            Ok(tokens) => {
                println!("{:<20} {:<20} UPDATED", "SERVICE", "HOST");
                for t in &tokens {
                    println!("{:<20} {:<20} {}", t.service, t.host, t.updated_at);
                }
                println!("\n{} token(s)", tokens.len());
            }
            Err(e) => {
                return Err(IpcHandlerError::OperationFailed(format!(
                    "list failed: {e}"
                )));
            }
        },
        AuthCommands::Get {
            service, secret, ..
        } => match convergio_core::ipc::auth_sync::get_token(&conn, &service, &secret) {
            Ok(Some(val)) => println!("{val}"),
            Ok(None) => {
                return Err(IpcHandlerError::NotFound(format!(
                    "no token found for {service}"
                )));
            }
            Err(e) => {
                return Err(IpcHandlerError::OperationFailed(format!("get failed: {e}")));
            }
        },
        AuthCommands::Revoke { service, host, .. } => {
            let h = host.unwrap_or_else(|| {
                hostname::get()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "unknown".to_string())
            });
            match convergio_core::ipc::auth_sync::revoke_token(&conn, &service, &h) {
                Ok(n) => println!("revoked {n} token(s) for {service}@{h}"),
                Err(e) => {
                    return Err(IpcHandlerError::OperationFailed(format!(
                        "revoke failed: {e}"
                    )));
                }
            }
        }
        AuthCommands::Rotate {
            old_secret,
            new_secret,
            ..
        } => match convergio_core::ipc::auth_sync::rotate_keys(&conn, &old_secret, &new_secret) {
            Ok(n) => println!("rotated {n} token(s)"),
            Err(e) => {
                return Err(IpcHandlerError::OperationFailed(format!(
                    "rotate failed: {e}"
                )));
            }
        },
    }
    Ok(())
}
