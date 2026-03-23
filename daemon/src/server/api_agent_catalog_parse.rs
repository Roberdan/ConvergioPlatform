// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// YAML frontmatter parser for .agent.md files — extracted from api_agent_catalog.rs (Plan F, T4-03).

pub(super) struct Frontmatter {
    pub(super) name: Option<String>,
    pub(super) description: Option<String>,
    pub(super) model: Option<String>,
    pub(super) tools: Option<String>,
}

pub(super) fn parse_yaml_frontmatter(content: &str) -> Option<Frontmatter> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let after_first = &trimmed[3..];
    let end = after_first.find("\n---")?;
    let yaml_block = &after_first[..end];

    let mut name = None;
    let mut description = None;
    let mut model = None;
    let mut tools_list: Vec<String> = Vec::new();
    let mut in_tools = false;

    for line in yaml_block.lines() {
        let trimmed_line = line.trim();
        if trimmed_line.starts_with("name:") {
            name = Some(extract_value(trimmed_line, "name:"));
            in_tools = false;
        } else if trimmed_line.starts_with("description:") {
            description = Some(extract_value(trimmed_line, "description:"));
            in_tools = false;
        } else if trimmed_line.starts_with("model:") {
            model = Some(extract_value(trimmed_line, "model:"));
            in_tools = false;
        } else if trimmed_line.starts_with("tools:") {
            in_tools = true;
        } else if in_tools && trimmed_line.starts_with("- ") {
            tools_list.push(trimmed_line[2..].trim().to_string());
        } else if !trimmed_line.starts_with('-') && trimmed_line.contains(':') {
            in_tools = false;
        }
    }

    let tools = if tools_list.is_empty() {
        None
    } else {
        Some(tools_list.join(", "))
    };

    Some(Frontmatter {
        name,
        description,
        model,
        tools,
    })
}

fn extract_value(line: &str, prefix: &str) -> String {
    line[prefix.len()..]
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}
