// Wave/task spec types and parse_waves extracted from api_plan_db_import.rs (250-line split).
use super::state::ApiError;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
pub struct WaveSpec {
    pub id: String,
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
        return serde_json::from_value::<Vec<WaveSpec>>(waves_val.clone())
            .map_err(|e| ApiError::bad_request(format!("invalid waves: {e}")));
    }

    // If "spec" is provided as a string (YAML), parse it
    if let Some(spec_str) = body.get("spec").and_then(Value::as_str) {
        let parsed: Value = serde_yaml::from_str(spec_str)
            .map_err(|e| ApiError::bad_request(format!("YAML parse failed: {e}")))?;
        if let Some(waves_val) = parsed.get("waves") {
            return serde_json::from_value::<Vec<WaveSpec>>(waves_val.clone())
                .map_err(|e| ApiError::bad_request(format!("invalid waves in spec: {e}")));
        }
        return Err(ApiError::bad_request("spec missing 'waves' key"));
    }

    // If "spec" is a JSON object
    if let Some(spec_obj) = body.get("spec") {
        if let Some(waves_val) = spec_obj.get("waves") {
            return serde_json::from_value::<Vec<WaveSpec>>(waves_val.clone())
                .map_err(|e| ApiError::bad_request(format!("invalid waves in spec: {e}")));
        }
        return Err(ApiError::bad_request("spec missing 'waves' key"));
    }

    Err(ApiError::bad_request(
        "missing 'waves' or 'spec' in request body",
    ))
}
