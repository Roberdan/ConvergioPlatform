use crate::resilience::notify::ChannelResult;
use crate::server::state::{query_rows, ApiError};
use rusqlite::Connection;
use serde_json::{json, Value};

pub fn record_delivery_attempts(
    conn: &Connection,
    notification_id: i64,
    trace_id: &str,
    attempts: &[ChannelResult],
) -> Result<(), ApiError> {
    for attempt in attempts {
        conn.execute(
            "INSERT INTO notification_deliveries \
             (notification_id, trace_id, channel, success, error_message, duration_ms) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                notification_id,
                trace_id,
                attempt.channel,
                if attempt.success { 1 } else { 0 },
                attempt.error,
                attempt.duration_ms
            ],
        )
        .map_err(|error| ApiError::internal(format!("notification delivery insert failed: {error}")))?;
    }
    Ok(())
}

pub fn telemetry_summary(conn: &Connection) -> Result<Value, ApiError> {
    let totals = query_rows(
        conn,
        "SELECT COUNT(*) AS total_attempts,
                SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) AS successful_attempts,
                SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) AS failed_attempts,
                COUNT(DISTINCT trace_id) AS trace_count,
                COALESCE(AVG(duration_ms), 0) AS avg_duration_ms
         FROM notification_deliveries",
        [],
    )?
    .into_iter()
    .next()
    .unwrap_or_else(|| json!({}));
    let channels = query_rows(
        conn,
        "SELECT channel,
                COUNT(*) AS attempts,
                SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) AS successes,
                SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) AS failures,
                COALESCE(AVG(duration_ms), 0) AS avg_duration_ms
         FROM notification_deliveries
         GROUP BY channel
         ORDER BY attempts DESC, channel ASC",
        [],
    )?;
    Ok(json!({
        "total_attempts": totals.get("total_attempts").cloned().unwrap_or(Value::from(0)),
        "successful_attempts": totals.get("successful_attempts").cloned().unwrap_or(Value::from(0)),
        "failed_attempts": totals.get("failed_attempts").cloned().unwrap_or(Value::from(0)),
        "trace_count": totals.get("trace_count").cloned().unwrap_or(Value::from(0)),
        "avg_duration_ms": totals.get("avg_duration_ms").cloned().unwrap_or(Value::from(0)),
        "channels": channels,
    }))
}
