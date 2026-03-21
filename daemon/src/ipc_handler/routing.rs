use std::path::PathBuf;

use super::utils::default_db_path;

pub fn handle_route(
    task_description: String,
    dry_run: bool,
    parallel: bool,
    db_path: Option<PathBuf>,
) {
    let path = db_path.unwrap_or_else(default_db_path);
    let conn = match rusqlite::Connection::open(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("db open: {e}");
            std::process::exit(2);
        }
    };
    if parallel {
        match claude_core::ipc::router::plan_parallel_execution(&conn, &task_description, 3) {
            Ok(plan) => println!(
                "{}",
                serde_json::to_string_pretty(&plan).unwrap_or_default()
            ),
            Err(e) => {
                eprintln!("parallel route: {e}");
                std::process::exit(2);
            }
        }
    } else if dry_run {
        let analysis = claude_core::ipc::router::analyze_task(&task_description);
        println!(
            "Analysis: {}",
            serde_json::to_string_pretty(&analysis).unwrap_or_default()
        );
        if let Ok(chain) = claude_core::ipc::router::fallback_chain(&conn, "") {
            println!("\nFallback chain:");
            for f in &chain {
                println!(
                    "  #{}: {} {} @ {} (free={}, degraded={})",
                    f.priority, f.provider, f.model, f.host, f.is_free, f.degraded
                );
            }
        }
    } else {
        match claude_core::ipc::router::route_task(&conn, &task_description) {
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
                eprintln!("route: {e}");
                std::process::exit(2);
            }
        }
    }
}

pub fn handle_skills(agent: Option<String>, db_path: Option<PathBuf>) {
    let path = db_path.unwrap_or_else(default_db_path);
    let conn = match rusqlite::Connection::open(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("db open: {e}");
            std::process::exit(2);
        }
    };
    if let Some(agent_name) = agent {
        match claude_core::ipc::skills::get_skills_for_agent(&conn, &agent_name) {
            Ok(skills) => {
                println!(
                    "{:<20} {:<15} {:<15} {:>10} {}",
                    "SKILL", "AGENT", "HOST", "CONFIDENCE", "LAST_USED"
                );
                for s in &skills {
                    println!(
                        "{:<20} {:<15} {:<15} {:>10.2} {}",
                        s.skill, s.agent, s.host, s.confidence, s.last_used
                    );
                }
            }
            Err(e) => {
                eprintln!("skills: {e}");
                std::process::exit(2);
            }
        }
    } else {
        match claude_core::ipc::skills::get_skill_pool(&conn) {
            Ok(pool) => {
                println!(
                    "{:<20} {:<15} {:<15} {:>10} {}",
                    "SKILL", "AGENT", "HOST", "CONFIDENCE", "LAST_USED"
                );
                for (_, agents) in &pool {
                    for s in agents {
                        println!(
                            "{:<20} {:<15} {:<15} {:>10.2} {}",
                            s.skill, s.agent, s.host, s.confidence, s.last_used
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("skills: {e}");
                std::process::exit(2);
            }
        }
    }
}

pub fn handle_request_skill(skill: String, payload: String, db_path: Option<PathBuf>) {
    let path = db_path.unwrap_or_else(default_db_path);
    let conn = match rusqlite::Connection::open(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("db open: {e}");
            std::process::exit(2);
        }
    };
    match claude_core::ipc::skills::create_skill_request(&conn, &skill, &payload) {
        Ok(id) => {
            println!("Request created: {id}");
            if let Ok(Some((agent, host))) =
                claude_core::ipc::skills::find_best_agent(&conn, &skill)
            {
                let _ = claude_core::ipc::skills::assign_request(&conn, &id, &agent, &host);
                println!("Assigned to: {agent}@{host}");
            }
        }
        Err(e) => {
            eprintln!("request-skill: {e}");
            std::process::exit(2);
        }
    }
}

pub fn handle_respond_skill(request_id: String, result: String, db_path: Option<PathBuf>) {
    let path = db_path.unwrap_or_else(default_db_path);
    let conn = match rusqlite::Connection::open(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("db open: {e}");
            std::process::exit(2);
        }
    };
    match claude_core::ipc::skills::complete_skill_request(&conn, &request_id, &result) {
        Ok(()) => println!("Request {request_id} completed"),
        Err(e) => {
            eprintln!("respond-skill: {e}");
            std::process::exit(2);
        }
    }
}

pub fn handle_rate_skill(request_id: String, rating: f64, db_path: Option<PathBuf>) {
    let path = db_path.unwrap_or_else(default_db_path);
    let conn = match rusqlite::Connection::open(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("db open: {e}");
            std::process::exit(2);
        }
    };
    match claude_core::ipc::skills::rate_skill_response(&conn, &request_id, rating) {
        Ok(()) => println!("Rated request {request_id}: {rating}"),
        Err(e) => {
            eprintln!("rate-skill: {e}");
            std::process::exit(2);
        }
    }
}
