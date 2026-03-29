// Wave/task spec types and parse_waves extracted from api_plan_db_import.rs (250-line split).
use super::state::ApiError;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub struct WaveSpec {
    pub id: String,
    /// Wave name. Also accepts `title` as alias.
    #[serde(alias = "title")]
    pub name: String,
    #[serde(default)]
    pub depends_on: Option<String>,
    #[serde(default = "default_hours")]
    pub estimated_hours: i64,
    #[serde(default)]
    pub tasks: Vec<TaskSpec>,
}

fn default_hours() -> i64 {
    8
}

#[derive(Deserialize)]
pub struct TaskSpec {
    pub id: String,
    /// Primary task title. Also accepts `do` or `summary` as aliases.
    #[serde(alias = "do", alias = "summary")]
    pub title: String,
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(rename = "type", default = "default_type")]
    pub task_type: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub test_criteria: Option<Value>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub assignee: Option<String>,
    /// Output type drives validator_agent inference (pr, document, analysis, design, legal_opinion)
    #[serde(default)]
    pub output_type: Option<String>,
    /// Validator agent override — inferred from output_type if absent
    #[serde(default)]
    pub validator_agent: Option<String>,
    /// Files modified by this task — used to infer verify[] and effort_level
    #[serde(default)]
    pub files: Vec<String>,
    /// Verify commands — auto-generated as `test -f <file>` if files present and verify empty
    #[serde(default)]
    pub verify: Vec<String>,
    /// Effort level (1-3) — inferred from task type + file count if absent
    #[serde(default)]
    pub effort_level: Option<i64>,
}

fn default_priority() -> String {
    "P1".to_string()
}
fn default_type() -> String {
    "feature".to_string()
}

pub fn parse_waves(body: &Value) -> Result<Vec<WaveSpec>, ApiError> {
    // If "waves" array is provided directly
    if let Some(waves_val) = body.get("waves") {
        return parse_wave_array(waves_val, "body.waves");
    }

    // If "spec" is provided as a string (YAML), parse it
    if let Some(spec_str) = body.get("spec").and_then(Value::as_str) {
        let parsed: Value = serde_yaml::from_str(spec_str).map_err(|e| {
            let loc = format_yaml_location(&e);
            ApiError::bad_request(format!(
                "YAML parse error{loc}: {e}. Hint: check indentation and quoting."
            ))
        })?;
        if let Some(waves_val) = parsed.get("waves") {
            return parse_wave_array(waves_val, "spec.waves");
        }
        // serde_yaml parses into serde_json::Value via our pipeline
        let keys: Vec<String> = parsed
            .as_object()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        return Err(ApiError::bad_request(format!(
            "spec missing 'waves' key. Found top-level keys: [{}]. \
             Expected: waves: [{{id, name, tasks: [...]}}]",
            keys.join(", ")
        )));
    }

    // If "spec" is a JSON object
    if let Some(spec_obj) = body.get("spec") {
        if let Some(waves_val) = spec_obj.get("waves") {
            return parse_wave_array(waves_val, "spec.waves");
        }
        return Err(ApiError::bad_request(
            "spec object missing 'waves' key. Expected: {\"waves\": [{...}]}",
        ));
    }

    Err(ApiError::bad_request(
        "missing 'waves' or 'spec' in request body. \
         Use: {\"plan_id\": N, \"spec\": \"<yaml string>\"} or \
         {\"plan_id\": N, \"waves\": [...]}. \
         Run `cvg plan template` for a complete example.",
    ))
}

/// Parse a waves array with detailed per-wave error messages.
fn parse_wave_array(waves_val: &Value, path: &str) -> Result<Vec<WaveSpec>, ApiError> {
    let arr = waves_val.as_array().ok_or_else(|| {
        ApiError::bad_request(format!("{path} must be an array, got {}", value_type(waves_val)))
    })?;
    if arr.is_empty() {
        return Err(ApiError::bad_request(format!("{path} is empty — at least one wave required")));
    }
    for (i, wave) in arr.iter().enumerate() {
        if wave.get("id").is_none() {
            return Err(ApiError::bad_request(format!(
                "{path}[{i}] missing required field 'id'"
            )));
        }
        if wave.get("name").is_none() && wave.get("title").is_none() {
            return Err(ApiError::bad_request(format!(
                "{path}[{i}] missing required field 'name' (or alias 'title')"
            )));
        }
    }
    serde_json::from_value::<Vec<WaveSpec>>(waves_val.clone()).map_err(|e| {
        ApiError::bad_request(format!(
            "invalid wave/task structure in {path}: {e}. \
             Run `cvg plan template` for the expected format."
        ))
    })
}

fn value_type(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn format_yaml_location(e: &serde_yaml::Error) -> String {
    if let Some(loc) = e.location() {
        format!(" at line {}, column {}", loc.line(), loc.column())
    } else {
        String::new()
    }
}
