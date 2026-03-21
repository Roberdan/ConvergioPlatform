// T3-04: In-memory log buffer for aggregation API (O(1) push via VecDeque)

use std::collections::VecDeque;
use std::sync::Mutex;

#[derive(Clone, serde::Serialize)]
pub struct LogEntry {
    pub ts: u64,
    pub level: String,
    pub target: String,
    pub message: String,
    pub node: String,
}

pub struct LogBuffer {
    entries: Mutex<VecDeque<LogEntry>>,
    capacity: usize,
}

impl LogBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Mutex::new(VecDeque::with_capacity(capacity)),
            capacity,
        }
    }

    pub fn push(&self, entry: LogEntry) {
        if self.capacity == 0 {
            return;
        }
        let mut entries = self.entries.lock().unwrap();
        if entries.len() >= self.capacity {
            entries.pop_front(); // O(1) vs Vec::remove(0) O(n)
        }
        entries.push_back(entry);
    }

    pub fn recent(&self, limit: usize) -> Vec<LogEntry> {
        let entries = self.entries.lock().unwrap();
        let skip = entries.len().saturating_sub(limit);
        entries.iter().skip(skip).cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_buffer_evicts_oldest() {
        let buf = LogBuffer::new(3);
        for i in 0..5 {
            buf.push(LogEntry {
                ts: i,
                level: "INFO".into(),
                target: "test".into(),
                message: format!("msg-{i}"),
                node: "n1".into(),
            });
        }
        let recent = buf.recent(10);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].message, "msg-2"); // oldest surviving
    }

    #[test]
    fn log_buffer_zero_capacity() {
        let buf = LogBuffer::new(0);
        buf.push(LogEntry {
            ts: 1,
            level: "ERROR".into(),
            target: "test".into(),
            message: "msg".into(),
            node: "n".into(),
        });
        assert_eq!(buf.recent(10).len(), 0);
    }

    #[test]
    fn log_buffer_recent_limit() {
        let buf = LogBuffer::new(100);
        for i in 0..50 {
            buf.push(LogEntry {
                ts: i,
                level: "INFO".into(),
                target: "t".into(),
                message: format!("m{i}"),
                node: "n".into(),
            });
        }
        assert_eq!(buf.recent(5).len(), 5);
        assert_eq!(buf.recent(5).last().unwrap().message, "m49");
    }
}
