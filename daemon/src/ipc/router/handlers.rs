use rusqlite::Connection;
use serde::Serialize;

use super::dispatch::{analyze_task, route_task};

// --- T8070: Distributed execution ---

#[derive(Debug, Clone, Serialize)]
pub struct SubtaskAssignment {
    pub subtask: String,
    pub host: String,
    pub model: String,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionPlan {
    pub subtasks: Vec<SubtaskAssignment>,
    pub parallel: bool,
    pub estimated_total_cost: f64,
}

pub fn plan_parallel_execution(
    conn: &Connection,
    description: &str,
    subtask_count: usize,
) -> rusqlite::Result<ExecutionPlan> {
    let analysis = analyze_task(description);
    let words: Vec<&str> = description
        .split('.')
        .filter(|s| !s.trim().is_empty())
        .collect();
    let parts = if words.len() >= subtask_count {
        words
    } else {
        vec![description; subtask_count]
    };

    let mut assignments = Vec::new();
    let mut total_cost = 0.0;

    for (i, part) in parts.iter().enumerate().take(subtask_count) {
        if let Ok(Some(decision)) = route_task(conn, part) {
            total_cost += decision.estimated_cost;
            assignments.push(SubtaskAssignment {
                subtask: format!("subtask-{}: {}", i, part.trim()),
                host: decision.host,
                model: decision.model,
                provider: decision.provider,
            });
        } else {
            assignments.push(SubtaskAssignment {
                subtask: format!("subtask-{}: {}", i, part.trim()),
                host: "local".into(),
                model: format!("{:?}", analysis.task_type),
                provider: "fallback".into(),
            });
        }
    }

    Ok(ExecutionPlan {
        subtasks: assignments,
        parallel: true,
        estimated_total_cost: total_cost,
    })
}
