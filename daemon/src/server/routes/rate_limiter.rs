// Rate limiter and endpoint categorisation — extracted from api_routes.rs.
// WHY: api_routes.rs holds route constants; mixing runtime state there inflated the file.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct RateLimiter {
    pub(super) buckets: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl RateLimiter {
    pub(super) async fn allow(&self, category: String, limit: usize, window: Duration) -> bool {
        let now = Instant::now();
        let mut buckets = self.buckets.lock().await;
        let entries = buckets.entry(category).or_default();
        entries.retain(|seen| now.duration_since(*seen) <= window);
        if entries.len() >= limit {
            return false;
        }
        entries.push(now);
        true
    }
}

pub fn endpoint_category(path: &str) -> String {
    let mut segments = path.split('/').filter(|segment| !segment.is_empty());
    match (segments.next(), segments.next()) {
        (Some("api"), Some(category)) => format!("api:{category}"),
        (Some(segment), _) => segment.to_string(),
        _ => "root".to_string(),
    }
}
