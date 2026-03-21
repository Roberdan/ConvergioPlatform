// plans: mission, plan detail, tokens, history, timeline
// tasks/projects/notifications/status handlers → plans_detail.rs
pub use super::plans_detail::{
    api_notifications, api_plan_status, api_plans_assignable, api_project_create, api_projects,
    api_tasks_blocked, api_tasks_distribution,
};

use super::super::state::{query_one, query_rows, ApiError, ServerState};
use axum::extract::{Path, Query, State};
use axum::Json;
use serde_json::{json, Value};
use std::collections::HashMap;

pub async fn api_mission(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let plans = query_rows(
        &conn,
        "SELECT p.id,p.name,p.status,p.tasks_done,p.tasks_total,p.project_id,p.execution_host,p.human_summary,p.started_at,p.created_at,p.lines_added,p.lines_removed,p.description,pr.name AS project_name FROM plans p LEFT JOIN projects pr ON p.project_id=pr.id WHERE p.status IN ('todo','doing') UNION ALL SELECT * FROM (SELECT p.id,p.name,p.status,p.tasks_done,p.tasks_total,p.project_id,p.execution_host,p.human_summary,p.started_at,p.created_at,p.lines_added,p.lines_removed,p.description,pr.name AS project_name FROM plans p LEFT JOIN projects pr ON p.project_id=pr.id WHERE p.status='cancelled' ORDER BY p.id DESC LIMIT 10)",
        [],
    )?;
    let mut result = Vec::new();
    for plan in plans {
        let plan_id = plan.get("id").and_then(Value::as_i64).unwrap_or(0);
        let waves = query_rows(
            &conn,
            "SELECT wave_id,name,status,tasks_done,tasks_total,position,completed_at AS validated_at,pr_number,pr_url,started_at,completed_at,theme,depends_on FROM waves WHERE plan_id=?1 ORDER BY position",
            rusqlite::params![plan_id],
        ).unwrap_or_default();
        let tasks = query_rows(
            &conn,
            "SELECT task_id,title,status,executor_agent,executor_host,tokens,validated_at,model,wave_id,started_at,completed_at,duration_minutes FROM tasks WHERE plan_id=?1 ORDER BY id",
            rusqlite::params![plan_id],
        ).unwrap_or_default();
        result.push(json!({"plan": plan, "waves": waves, "tasks": tasks}));
    }
    Ok(Json(json!({"plans": result})))
}

pub async fn api_tokens_daily(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    Ok(Json(Value::Array(query_rows(
        &conn,
        "SELECT date(created_at) AS day, SUM(input_tokens) AS input, SUM(output_tokens) AS output, SUM(cost_usd) AS cost FROM token_usage GROUP BY day ORDER BY day",
        [],
    )?)))
}

pub async fn api_tokens_models(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    Ok(Json(Value::Array(query_rows(
        &conn,
        "SELECT model, SUM(input_tokens + output_tokens) AS tokens, SUM(cost_usd) AS cost FROM token_usage WHERE model IS NOT NULL GROUP BY model ORDER BY tokens DESC LIMIT 8",
        [],
    )?)))
}

pub async fn api_history(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    Ok(Json(Value::Array(query_rows(
        &conn,
        "SELECT p.id,p.name,p.status,p.tasks_done,p.tasks_total,p.project_id,p.started_at,p.completed_at,p.human_summary,p.lines_added,p.lines_removed,pr.name AS project_name FROM plans p LEFT JOIN projects pr ON p.project_id=pr.id WHERE p.status IN ('done','cancelled') ORDER BY p.id DESC LIMIT 20",
        [],
    )?)))
}

pub async fn api_recent_missions(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let plans = query_rows(
        &conn,
        "SELECT p.id,p.name,p.status,p.tasks_done,p.tasks_total,p.project_id,p.execution_host,p.human_summary,p.completed_at,p.cancelled_at,COALESCE(p.completed_at,p.cancelled_at) AS finished_at,pr.name AS project_name
         FROM plans p
         LEFT JOIN projects pr ON p.project_id=pr.id
         WHERE p.status = 'done'
           AND datetime(p.completed_at) >= datetime('now','-1 day')
           AND LOWER(COALESCE(p.name,'')) NOT LIKE '%test%'
           AND LOWER(COALESCE(pr.name,'')) NOT LIKE '%test%'
           AND LOWER(COALESCE(p.name,'')) NOT LIKE '%hyperdemo%'
           AND LOWER(COALESCE(pr.name,'')) NOT LIKE '%hyperdemo%'
         ORDER BY datetime(p.completed_at) DESC, p.id DESC",
        [],
    )?;
    let mut result = Vec::new();
    for plan in plans {
        let plan_id = plan.get("id").and_then(Value::as_i64).unwrap_or(0);
        let waves = query_rows(
            &conn,
            "SELECT wave_id,name,status,tasks_done,tasks_total,position,completed_at AS validated_at,pr_number,pr_url,started_at,completed_at,theme,depends_on FROM waves WHERE plan_id=?1 ORDER BY position",
            rusqlite::params![plan_id],
        )
        .unwrap_or_default();
        let tasks = query_rows(
            &conn,
            "SELECT task_id,title,status,executor_agent,executor_host,tokens,validated_at,model,wave_id,started_at,completed_at,duration_minutes FROM tasks WHERE plan_id=?1 ORDER BY id",
            rusqlite::params![plan_id],
        )
        .unwrap_or_default();
        result.push(json!({"plan": plan, "waves": waves, "tasks": tasks}));
    }
    Ok(Json(json!({"plans": result})))
}

pub async fn api_plan_detail(
    State(state): State<ServerState>,
    Path(plan_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let plan = query_one(
        &conn,
        "SELECT p.id,p.name,p.status,p.tasks_done,p.tasks_total,p.project_id,p.execution_host,p.human_summary,p.started_at,p.completed_at,p.parallel_mode,p.lines_added,p.lines_removed,pr.name AS project_name FROM plans p LEFT JOIN projects pr ON p.project_id=pr.id WHERE p.id=?1",
        rusqlite::params![plan_id],
    )?.ok_or_else(|| ApiError::bad_request(format!("plan {plan_id} not found")))?;
    let waves = query_rows(
        &conn,
        "SELECT wave_id,name,status,tasks_done,tasks_total,position,completed_at AS validated_at,pr_number,pr_url FROM waves WHERE plan_id=?1 ORDER BY position",
        rusqlite::params![plan_id],
    ).unwrap_or_default();
    let tasks = query_rows(
        &conn,
        "SELECT task_id,title,status,executor_agent,executor_host,tokens,validated_at,model,wave_id FROM tasks WHERE plan_id=?1 ORDER BY id",
        rusqlite::params![plan_id],
    ).unwrap_or_default();
    let cost = query_one(
        &conn,
        "SELECT COALESCE(SUM(input_tokens+output_tokens),0) AS tokens, COALESCE(SUM(cost_usd),0) AS cost FROM token_usage WHERE plan_id=?1",
        rusqlite::params![plan_id],
    )?.unwrap_or_else(|| json!({"tokens":0,"cost":0}));
    Ok(Json(json!({"plan": plan, "waves": waves, "tasks": tasks, "cost": cost})))
}

pub async fn api_plans_timeline(
    State(state): State<ServerState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let days = params
        .get("days")
        .and_then(|d| d.parse::<i64>().ok())
        .unwrap_or(30);
    let plans = query_rows(
        &conn,
        "SELECT p.id, p.name, p.status, p.project_id, p.created_at, p.started_at, p.completed_at, p.cancelled_at,
                p.tasks_total, p.tasks_done, p.execution_host, p.lines_added, p.lines_removed, p.description
         FROM plans p
         WHERE p.created_at >= datetime('now', '-' || ?1 || ' days')
         ORDER BY p.created_at DESC",
        rusqlite::params![days],
    )?;
    let mut result = Vec::new();
    for plan in plans {
        let plan_id = plan.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let tasks = query_rows(
            &conn,
            "SELECT t.id, t.task_id, t.title, t.status, t.started_at, t.completed_at, t.wave_id, t.tokens, t.assignee, t.priority, t.type AS task_type, t.executor_host, t.model, w.name AS wave_name
             FROM tasks t LEFT JOIN waves w ON t.wave_id_fk = w.id
             WHERE t.plan_id = ?1 ORDER BY t.id",
            rusqlite::params![plan_id],
        ).unwrap_or_default();
        let mut plan_obj = plan;
        if let Some(obj) = plan_obj.as_object_mut() {
            obj.insert("tasks".to_string(), json!(tasks));
        }
        result.push(plan_obj);
    }
    Ok(Json(json!({ "plans": result })))
}

