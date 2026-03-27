use super::types::VoiceError;
use serde::{Deserialize, Serialize};

/// Structured intent extracted from transcribed text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub intent_type: IntentType,
    pub command: Option<String>,
    pub confidence: f32,
    pub raw_text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IntentType {
    /// Mapped to a cvg CLI command (e.g., "show me the plans" → cvg plan list).
    Command,
    /// Information request (e.g., "what agents are running?").
    Query,
    /// Control action (e.g., "start", "stop", "status").
    Control,
    /// Navigation (e.g., "switch to mesh tab").
    Navigation,
    /// Unclear — needs disambiguation.
    Ambiguous,
}

/// Extract structured intent from transcribed text.
pub fn extract_intent(text: &str) -> Result<Intent, VoiceError> {
    let lower = text.to_lowercase();
    let (intent_type, command) = classify(&lower);
    let confidence = if intent_type == IntentType::Ambiguous {
        0.3
    } else {
        0.85
    };

    Ok(Intent {
        intent_type,
        command,
        confidence,
        raw_text: text.to_string(),
    })
}

fn classify(text: &str) -> (IntentType, Option<String>) {
    // Control intents.
    if text.contains("start") && text.contains("voice") {
        return (IntentType::Control, Some("cvg voice start".to_string()));
    }
    if text.contains("stop") {
        return (IntentType::Control, Some("cvg voice stop".to_string()));
    }
    if text.contains("status") {
        return (IntentType::Control, Some("cvg voice status".to_string()));
    }

    // Navigation intents.
    for tab in ["mesh", "plans", "agents", "chat", "terminal", "brain"] {
        if text.contains(tab) && (text.contains("show") || text.contains("switch") || text.contains("go to")) {
            return (IntentType::Navigation, Some(format!("navigate:{tab}")));
        }
    }

    // Command intents.
    if text.contains("plan") && (text.contains("list") || text.contains("show")) {
        return (IntentType::Command, Some("cvg plan list".to_string()));
    }
    if text.contains("agent") && (text.contains("list") || text.contains("who")) {
        return (IntentType::Command, Some("cvg who agents".to_string()));
    }
    if text.contains("memory") && text.contains("recall") {
        return (IntentType::Command, Some("cvg memory recall".to_string()));
    }

    // Query intents.
    if text.contains("what") || text.contains("how") || text.contains("which") {
        return (IntentType::Query, None);
    }

    (IntentType::Ambiguous, None)
}
