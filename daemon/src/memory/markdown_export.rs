use super::types::{Memory, MemoryError};
use std::fs;
use std::path::{Path, PathBuf};

/// Export memories to Markdown files in the project memory directory.
/// Pattern: `~/.claude/projects/<slug>/memory/<type>_<topic>.md`
/// Each file has YAML frontmatter matching the Claude Code memory format.
pub fn export_memory(memory: &Memory, memory_dir: &Path) -> Result<PathBuf, MemoryError> {
    fs::create_dir_all(memory_dir)
        .map_err(|e| MemoryError::StorageError(format!("create memory dir: {e}")))?;

    let type_prefix = match memory.memory_type {
        super::types::MemoryType::Fact => "fact",
        super::types::MemoryType::Decision => "decision",
        super::types::MemoryType::Preference => "preference",
        super::types::MemoryType::Observation => "observation",
    };

    // Derive topic from first tag or content slug.
    let topic = memory
        .tags
        .first()
        .cloned()
        .unwrap_or_else(|| slug_from_content(&memory.content));
    let filename = format!("{type_prefix}_{topic}.md");
    let path = memory_dir.join(&filename);

    let md = format_memory_markdown(memory);

    // Append if file exists (multiple memories per topic), create otherwise.
    if path.exists() {
        let existing = fs::read_to_string(&path)
            .map_err(|e| MemoryError::StorageError(format!("read existing: {e}")))?;
        if !existing.contains(&memory.id) {
            fs::write(&path, format!("{existing}\n---\n\n{md}"))
                .map_err(|e| MemoryError::StorageError(format!("append: {e}")))?;
        }
    } else {
        fs::write(&path, md)
            .map_err(|e| MemoryError::StorageError(format!("write: {e}")))?;
    }

    Ok(path)
}

/// Export all memories to a directory, one file per type+topic group.
pub fn export_all(memories: &[Memory], memory_dir: &Path) -> Result<usize, MemoryError> {
    let mut count = 0;
    for mem in memories {
        export_memory(mem, memory_dir)?;
        count += 1;
    }
    Ok(count)
}

/// Parse memories from Markdown files in a directory.
/// Returns memories that can be re-imported into SQLite.
pub fn import_from_markdown(memory_dir: &Path) -> Result<Vec<Memory>, MemoryError> {
    let mut memories = Vec::new();
    let entries = fs::read_dir(memory_dir)
        .map_err(|e| MemoryError::StorageError(format!("read dir: {e}")))?;

    for entry in entries.filter_map(|e| match e {
        Ok(v) => Some(v),
        Err(e) => { tracing::warn!("import_from_markdown: readdir entry: {e}"); None }
    }) {
        let path = entry.path();
        if path.extension().map(|e| e == "md").unwrap_or(false) {
            let content = fs::read_to_string(&path)
                .map_err(|e| MemoryError::StorageError(format!("read {}: {e}", path.display())))?;
            let parsed = parse_markdown_memories(&content);
            memories.extend(parsed);
        }
    }
    Ok(memories)
}

fn format_memory_markdown(mem: &Memory) -> String {
    let tags_str = mem.tags.join(", ");
    let access = format!("{:?}", mem.access_level);
    format!(
        "---\nid: {}\nagent_id: {}\ntype: {:?}\ntags: [{}]\naccess: {}\ncreated_at: {}\n---\n\n{}\n",
        mem.id, mem.agent_id, mem.memory_type, tags_str, access,
        mem.created_at.to_rfc3339(), mem.content,
    )
}

fn parse_markdown_memories(content: &str) -> Vec<Memory> {
    let mut result = Vec::new();
    let mut id = String::new();
    let mut agent_id = String::new();
    let mut body_lines: Vec<String> = Vec::new();
    let mut in_frontmatter = false;
    let mut dash_count = 0;

    for line in content.lines() {
        if line.trim() == "---" {
            dash_count += 1;
            if dash_count % 2 == 1 {
                // Entering frontmatter — flush previous block if any.
                if !id.is_empty() && !body_lines.is_empty() {
                    result.push(build_memory(&id, &agent_id, &body_lines));
                    body_lines.clear();
                }
                in_frontmatter = true;
                id.clear();
                agent_id.clear();
            } else {
                in_frontmatter = false;
            }
            continue;
        }
        if in_frontmatter {
            if let Some(val) = line.strip_prefix("id: ") {
                id = val.trim().to_string();
            } else if let Some(val) = line.strip_prefix("agent_id: ") {
                agent_id = val.trim().to_string();
            }
        } else if !line.trim().is_empty() {
            body_lines.push(line.to_string());
        }
    }
    // Flush last block.
    if !id.is_empty() && !body_lines.is_empty() {
        result.push(build_memory(&id, &agent_id, &body_lines));
    }
    result
}

fn build_memory(id: &str, agent_id: &str, body: &[String]) -> Memory {
    Memory {
        id: id.to_string(),
        agent_id: agent_id.to_string(),
        memory_type: super::types::MemoryType::Fact,
        content: body.join("\n"),
        tags: vec![],
        created_at: chrono::Utc::now(),
        expires_at: None,
        access_level: super::types::AccessLevel::Private,
        attestations: vec![],
    }
}

fn slug_from_content(content: &str) -> String {
    content
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join("_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .take(30)
        .collect::<String>()
        .to_lowercase()
}

#[cfg(test)]
#[path = "markdown_export_tests.rs"]
mod tests;
