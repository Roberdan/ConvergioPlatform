// Inference fallback configuration — loaded from [inference.fallback] in config.toml.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(default)]
pub struct InferenceFallbackConfig {
    pub max_attempts: usize,
    pub t1: Vec<String>,
    pub t2: Vec<String>,
    pub t3: Vec<String>,
    pub t4: Vec<String>,
}

impl Default for InferenceFallbackConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            t1: vec!["local".into(), "haiku".into(), "sonnet".into()],
            t2: vec!["haiku".into(), "local".into(), "sonnet".into()],
            t3: vec!["sonnet".into(), "opus".into()],
            t4: vec!["opus".into(), "sonnet".into()],
        }
    }
}
