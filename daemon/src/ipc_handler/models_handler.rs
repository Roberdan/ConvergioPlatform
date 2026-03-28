use std::path::PathBuf;

use super::types::{IpcHandlerError, SubCommands};
use super::utils::default_db_path;

pub fn handle_models(db_path: Option<PathBuf>) -> Result<(), IpcHandlerError> {
    let conn = open_db(db_path)?;
    match convergio_core::ipc::models::get_all_models(&conn) {
        Ok(models) => {
            println!(
                "{:<15} {:<10} {:<30} {:>8} {:<10} LAST_SEEN",
                "HOST", "PROVIDER", "MODEL", "SIZE_GB", "QUANT"
            );
            for m in &models {
                println!(
                    "{:<15} {:<10} {:<30} {:>8.1} {:<10} {}",
                    m.host, m.provider, m.model, m.size_gb, m.quantization, m.last_seen
                );
            }
            println!("\n{} model(s)", models.len());
        }
        Err(e) => {
            return Err(IpcHandlerError::OperationFailed(format!(
                "list models: {e}"
            )));
        }
    }
    Ok(())
}

pub fn handle_sub(command: SubCommands) -> Result<(), IpcHandlerError> {
    let db_path = match &command {
        SubCommands::Add { db_path, .. }
        | SubCommands::List { db_path }
        | SubCommands::Remove { db_path, .. } => db_path.clone().unwrap_or_else(default_db_path),
    };
    let conn = open_db(Some(db_path))?;
    match command {
        SubCommands::Add {
            name,
            provider,
            plan,
            budget,
            reset_day,
            models,
            ..
        } => {
            let sub = convergio_core::ipc::models::Subscription {
                name,
                provider,
                plan,
                budget_usd: budget,
                reset_day,
                models,
            };
            match convergio_core::ipc::models::add_subscription(&conn, &sub) {
                Ok(()) => println!("added subscription {}", sub.name),
                Err(e) => {
                    return Err(IpcHandlerError::OperationFailed(format!("add sub: {e}")));
                }
            }
        }
        SubCommands::List { .. } => match convergio_core::ipc::models::list_subscriptions(&conn) {
            Ok(subs) => {
                println!(
                    "{:<20} {:<12} {:<10} {:>10} {:>5} MODELS",
                    "NAME", "PROVIDER", "PLAN", "BUDGET", "DAY"
                );
                for s in &subs {
                    println!(
                        "{:<20} {:<12} {:<10} {:>10.2} {:>5} {}",
                        s.name,
                        s.provider,
                        s.plan,
                        s.budget_usd,
                        s.reset_day,
                        s.models.join(",")
                    );
                }
                println!("\n{} subscription(s)", subs.len());
            }
            Err(e) => {
                return Err(IpcHandlerError::OperationFailed(format!("list subs: {e}")));
            }
        },
        SubCommands::Remove { name, .. } => {
            match convergio_core::ipc::models::remove_subscription(&conn, &name) {
                Ok(n) => println!("removed {n} subscription(s)"),
                Err(e) => {
                    return Err(IpcHandlerError::OperationFailed(format!("remove sub: {e}")));
                }
            }
        }
    }
    Ok(())
}

pub fn handle_budget(db_path: Option<PathBuf>) -> Result<(), IpcHandlerError> {
    let conn = open_db(db_path)?;
    match convergio_core::ipc::models::list_subscriptions(&conn) {
        Ok(subs) => {
            println!(
                "{:<20} {:<10} {:>10} {:>10} {:>10} {:>6} {:>10} STATUS",
                "SUBSCRIPTION", "PROVIDER", "BUDGET", "SPENT", "REMAINING", "DAYS", "PROJECTED"
            );
            for s in &subs {
                if let Ok(Some(st)) = convergio_core::ipc::budget::get_budget_status(&conn, &s.name) {
                    let status = if st.usage_pct >= 95.0 {
                        "CRITICAL"
                    } else if st.usage_pct >= 85.0 {
                        "HIGH"
                    } else if st.usage_pct >= 70.0 {
                        "WARN"
                    } else {
                        "OK"
                    };
                    println!(
                        "{:<20} {:<10} {:>10.2} {:>10.2} {:>10.2} {:>6} {:>10.2} {}",
                        s.name,
                        s.provider,
                        st.budget_usd,
                        st.total_spent,
                        st.remaining_budget,
                        st.days_remaining,
                        st.projected_total,
                        status
                    );
                }
            }
        }
        Err(e) => {
            return Err(IpcHandlerError::OperationFailed(format!("budget: {e}")));
        }
    }
    Ok(())
}

fn open_db(db_path: Option<PathBuf>) -> Result<rusqlite::Connection, IpcHandlerError> {
    let path = db_path.unwrap_or_else(default_db_path);
    rusqlite::Connection::open(&path).map_err(|e| IpcHandlerError::DbOpen(format!("db open: {e}")))
}
