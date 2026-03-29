use std::path::PathBuf;

use super::types::IpcHandlerError;
use super::utils::default_db_path;

pub fn handle_route(
    task_description: String,
    dry_run: bool,
    parallel: bool,
    db_path: Option<PathBuf>,
) -> Result<(), IpcHandlerError> {
    let path = db_path.unwrap_or_else(default_db_path);
    let conn = rusqlite::Connection::open(&path)
        .map_err(|e| IpcHandlerError::DbOpen(format!("db open: {e}")))?;
    if parallel {
        match convergio_core::ipc::router::plan_parallel_execution(&conn, &task_description, 3) {
            Ok(plan) => println!(
                "{}",
                serde_json::to_string_pretty(&plan).unwrap_or_default()
            ),
            Err(e) => {
                return Err(IpcHandlerError::OperationFailed(format!(
                    "parallel route: {e}"
                )));
            }
        }
    } else if dry_run {
        let analysis = convergio_core::ipc::router::analyze_task(&task_description);
        println!(
            "Analysis: {}",
            serde_json::to_string_pretty(&analysis).unwrap_or_default()
        );
        if let Ok(chain) = convergio_core::ipc::router::fallback_chain(&conn, "") {
            println!("\nFallback chain:");
            for f in &chain {
                println!(
                    "  #{}: {} {} @ {} (free={}, degraded={})",
                    f.priority, f.provider, f.model, f.host, f.is_free, f.degraded
                );
            }
        }
    } else {
        match convergio_core::ipc::router::route_task(&conn, &task_description) {
            Ok(Some(d)) => {
                println!("Model:      {}", d.model);
                println!("Provider:   {}", d.provider);
                println!("Host:       {}", d.host);
                println!("Reason:     {}", d.reason);
                println!("Confidence: {:.0}%", d.confidence * 100.0);
                println!("Est. Cost:  ${:.4}", d.estimated_cost);
            }
            Ok(None) => println!("No suitable model found"),
            Err(e) => {
                return Err(IpcHandlerError::OperationFailed(format!("route: {e}")));
            }
        }
    }
    Ok(())
}

pub fn handle_skills(
    agent: Option<String>,
    db_path: Option<PathBuf>,
) -> Result<(), IpcHandlerError> {
    let path = db_path.unwrap_or_else(default_db_path);
    let conn = rusqlite::Connection::open(&path)
        .map_err(|e| IpcHandlerError::DbOpen(format!("db open: {e}")))?;
    if let Some(agent_name) = agent {
        match convergio_core::ipc::skills::get_skills_for_agent(&conn, &agent_name) {
            Ok(skills) => {
                println!(
                    "{:<20} {:<15} {:<15} {:>10} LAST_USED",
                    "SKILL", "AGENT", "HOST", "CONFIDENCE"
                );
                for s in &skills {
                    println!(
                        "{:<20} {:<15} {:<15} {:>10.2} {}",
                        s.skill, s.agent, s.host, s.confidence, s.last_used
                    );
                }
            }
            Err(e) => {
                return Err(IpcHandlerError::OperationFailed(format!("skills: {e}")));
            }
        }
    } else {
        match convergio_core::ipc::skills::get_skill_pool(&conn) {
            Ok(pool) => {
                println!(
                    "{:<20} {:<15} {:<15} {:>10} LAST_USED",
                    "SKILL", "AGENT", "HOST", "CONFIDENCE"
                );
                for agents in pool.values() {
                    for s in agents {
                        println!(
                            "{:<20} {:<15} {:<15} {:>10.2} {}",
                            s.skill, s.agent, s.host, s.confidence, s.last_used
                        );
                    }
                }
            }
            Err(e) => {
                return Err(IpcHandlerError::OperationFailed(format!("skills: {e}")));
            }
        }
    }
    Ok(())
}

pub fn handle_request_skill(
    skill: String,
    payload: String,
    db_path: Option<PathBuf>,
) -> Result<(), IpcHandlerError> {
    let path = db_path.unwrap_or_else(default_db_path);
    let conn = rusqlite::Connection::open(&path)
        .map_err(|e| IpcHandlerError::DbOpen(format!("db open: {e}")))?;
    match convergio_core::ipc::skills::create_skill_request(&conn, &skill, &payload) {
        Ok(id) => {
            println!("Request created: {id}");
            if let Ok(Some((agent, host))) =
                convergio_core::ipc::skills::find_best_agent(&conn, &skill)
            {
                if let Err(e) = convergio_core::ipc::skills::assign_request(&conn, &id, &agent, &host) {
                    eprintln!("warning: assign_request failed: {e}");
                } else {
                    println!("Assigned to: {agent}@{host}");
                }
            }
        }
        Err(e) => {
            return Err(IpcHandlerError::OperationFailed(format!(
                "request-skill: {e}"
            )));
        }
    }
    Ok(())
}

pub fn handle_respond_skill(
    request_id: String,
    result: String,
    db_path: Option<PathBuf>,
) -> Result<(), IpcHandlerError> {
    let path = db_path.unwrap_or_else(default_db_path);
    let conn = rusqlite::Connection::open(&path)
        .map_err(|e| IpcHandlerError::DbOpen(format!("db open: {e}")))?;
    match convergio_core::ipc::skills::complete_skill_request(&conn, &request_id, &result) {
        Ok(()) => println!("Request {request_id} completed"),
        Err(e) => {
            return Err(IpcHandlerError::OperationFailed(format!(
                "respond-skill: {e}"
            )));
        }
    }
    Ok(())
}

pub fn handle_rate_skill(
    request_id: String,
    rating: f64,
    db_path: Option<PathBuf>,
) -> Result<(), IpcHandlerError> {
    let path = db_path.unwrap_or_else(default_db_path);
    let conn = rusqlite::Connection::open(&path)
        .map_err(|e| IpcHandlerError::DbOpen(format!("db open: {e}")))?;
    match convergio_core::ipc::skills::rate_skill_response(&conn, &request_id, rating) {
        Ok(()) => println!("Rated request {request_id}: {rating}"),
        Err(e) => {
            return Err(IpcHandlerError::OperationFailed(format!("rate-skill: {e}")));
        }
    }
    Ok(())
}
