/// Shared helpers for libsql_adapter: column introspection and row serialization.
use rusqlite::Connection;
use super::libsql_adapter::SyncChange;

pub(crate) fn get_column_names(
    conn: &Connection,
    table_name: &str,
) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT name FROM pragma_table_info('{}')",
        table_name.replace('\'', "''")
    ))?;
    let cols: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(cols)
}

pub(crate) fn row_to_change(
    row: &rusqlite::Row<'_>,
    table_name: &str,
    columns: &[String],
) -> rusqlite::Result<SyncChange> {
    let pk: i64 = row.get(0)?;
    let mut data = serde_json::Map::new();
    for (i, col) in columns.iter().enumerate() {
        // Column index offset by 1 because id is at index 0 in the SELECT
        let val: rusqlite::types::Value = row.get(i + 1)?;
        let json_val = match val {
            rusqlite::types::Value::Null => serde_json::Value::Null,
            rusqlite::types::Value::Integer(n) => serde_json::Value::Number(n.into()),
            rusqlite::types::Value::Real(f) => serde_json::Number::from_f64(f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            rusqlite::types::Value::Text(s) => serde_json::Value::String(s),
            rusqlite::types::Value::Blob(b) => serde_json::Value::String(
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &b),
            ),
        };
        data.insert(col.clone(), json_val);
    }
    Ok(SyncChange {
        table_name: table_name.to_string(),
        pk,
        data: serde_json::Value::Object(data),
    })
}
