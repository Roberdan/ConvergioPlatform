use super::super::types::{AccessLevel, Attestation, Memory, MemoryType};
use chrono::Utc;

pub fn encode_type(t: &MemoryType) -> &'static str {
    match t {
        MemoryType::Fact => "Fact",
        MemoryType::Decision => "Decision",
        MemoryType::Preference => "Preference",
        MemoryType::Observation => "Observation",
    }
}

pub fn decode_type(s: &str) -> MemoryType {
    match s {
        "Decision" => MemoryType::Decision,
        "Preference" => MemoryType::Preference,
        "Observation" => MemoryType::Observation,
        _ => MemoryType::Fact,
    }
}

pub fn encode_access(a: &AccessLevel) -> &'static str {
    match a {
        AccessLevel::Private => "Private",
        AccessLevel::Shared => "Shared",
        AccessLevel::Public => "Public",
    }
}

pub fn decode_access(s: &str) -> AccessLevel {
    match s {
        "Shared" => AccessLevel::Shared,
        "Public" => AccessLevel::Public,
        _ => AccessLevel::Private,
    }
}

pub fn row_to_memory(row: &rusqlite::Row<'_>) -> rusqlite::Result<Memory> {
    let id: String = row.get(0)?;
    let agent_id: String = row.get(1)?;
    let mt_str: String = row.get(2)?;
    let content: String = row.get(3)?;
    let tags_json: String = row.get(4)?;
    let created_at_str: String = row.get(5)?;
    let expires_at_str: Option<String> = row.get(6)?;
    let access_str: String = row.get(7)?;
    let attestations_json: String = row.get(8)?;

    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let expires_at = expires_at_str.and_then(|s| {
        match chrono::DateTime::parse_from_rfc3339(&s) {
            Ok(dt) => Some(dt.with_timezone(&Utc)),
            Err(_) => None,
        }
    });
    let attestations: Vec<Attestation> =
        serde_json::from_str(&attestations_json).unwrap_or_default();

    Ok(Memory {
        id,
        agent_id,
        memory_type: decode_type(&mt_str),
        content,
        tags,
        created_at,
        expires_at,
        access_level: decode_access(&access_str),
        attestations,
    })
}

/// Build extra WHERE conditions for recall queries (all inputs are caller-validated types).
pub fn recall_conditions(
    memory_type: Option<&MemoryType>,
    agent_id: Option<&str>,
    time_range: Option<(&chrono::DateTime<Utc>, &chrono::DateTime<Utc>)>,
    text_search: Option<&str>,
    tags: Option<&[String]>,
) -> Vec<String> {
    let mut conds: Vec<String> = vec![];
    if let Some(mt) = memory_type {
        conds.push(format!("memory_type = '{}'", encode_type(mt)));
    }
    if let Some(aid) = agent_id {
        conds.push(format!("agent_id = '{}'", aid.replace('\'', "''")));
    }
    if let Some((from, to)) = time_range {
        conds.push(format!(
            "created_at BETWEEN '{}' AND '{}'",
            from.to_rfc3339(),
            to.to_rfc3339()
        ));
    }
    if let Some(search) = text_search {
        let safe = search.replace('\'', "''");
        conds.push(format!(
            "rowid IN (SELECT rowid FROM agent_memories_fts \
             WHERE agent_memories_fts MATCH '{safe}')"
        ));
    }
    if let Some(tags) = tags {
        for tag in tags {
            let safe = tag.replace('\'', "''");
            conds.push(format!("tags LIKE '%\"{safe}\"%'"));
        }
    }
    conds
}
