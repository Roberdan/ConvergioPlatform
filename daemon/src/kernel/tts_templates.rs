// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Kernel message templates in Italian for TTS synthesis.

/// Kernel message templates — produces Italian phrases for common kernel events.
pub struct KernelTemplates;

impl KernelTemplates {
    /// "Piano {name} completato. Costo {cost} dollari, durata {duration}."
    pub fn plan_completed(name: &str, cost: &str, duration: &str) -> String {
        format!("Piano {name} completato. Costo {cost} dollari, durata {duration}.")
    }

    /// "Attenzione: il daemon non risponde da {minutes} minuti."
    pub fn daemon_unresponsive(minutes: &str) -> String {
        format!("Attenzione: il daemon non risponde da {minutes} minuti.")
    }

    /// "Task {task_id} bloccato: {reason}."
    pub fn task_blocked(task_id: &str, reason: &str) -> String {
        format!("Task {task_id} bloccato: {reason}.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_completed_format() {
        let p = KernelTemplates::plan_completed("Alpha", "42", "3 ore");
        assert!(p.starts_with("Piano ") && p.contains("Alpha") && p.contains("42"));
    }

    #[test]
    fn daemon_unresponsive_format() {
        let d = KernelTemplates::daemon_unresponsive("15");
        assert!(d.contains("Attenzione") && d.contains("15"));
    }

    #[test]
    fn task_blocked_format() {
        let t = KernelTemplates::task_blocked("T2-01", "dipendenza mancante");
        assert!(t.starts_with("Task ") && t.contains("T2-01") && t.contains("dipendenza"));
    }
}
