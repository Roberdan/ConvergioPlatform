#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json;

    use crate::memory::types::{
        AccessLevel, Attestation, Memory, MemoryError, MemoryType, RecallQuery,
    };

    // MemoryType serialization
    #[test]
    fn memory_type_serializes_to_string() {
        let cases = [
            (MemoryType::Fact, "\"Fact\""),
            (MemoryType::Decision, "\"Decision\""),
            (MemoryType::Preference, "\"Preference\""),
            (MemoryType::Observation, "\"Observation\""),
        ];
        for (variant, expected) in cases {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn memory_type_deserializes_from_string() {
        let fact: MemoryType = serde_json::from_str("\"Fact\"").unwrap();
        assert_eq!(fact, MemoryType::Fact);

        let decision: MemoryType = serde_json::from_str("\"Decision\"").unwrap();
        assert_eq!(decision, MemoryType::Decision);
    }

    #[test]
    fn memory_type_clone_and_debug() {
        let mt = MemoryType::Preference;
        let cloned = mt.clone();
        assert_eq!(mt, cloned);
        assert!(!format!("{:?}", mt).is_empty());
    }

    // AccessLevel serialization
    #[test]
    fn access_level_serializes_correctly() {
        let cases = [
            (AccessLevel::Private, "\"Private\""),
            (AccessLevel::Shared, "\"Shared\""),
            (AccessLevel::Public, "\"Public\""),
        ];
        for (variant, expected) in cases {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn access_level_deserializes_correctly() {
        let public: AccessLevel = serde_json::from_str("\"Public\"").unwrap();
        assert_eq!(public, AccessLevel::Public);
    }

    #[test]
    fn access_level_clone_and_debug() {
        let al = AccessLevel::Shared;
        assert_eq!(al.clone(), AccessLevel::Shared);
        assert!(!format!("{:?}", al).is_empty());
    }

    // Attestation
    #[test]
    fn attestation_serializes_roundtrip() {
        let now = Utc::now();
        let att = Attestation {
            attesting_agent_id: "agent-42".to_string(),
            timestamp: now,
            confidence: 0.95,
        };
        let json = serde_json::to_string(&att).unwrap();
        let back: Attestation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.attesting_agent_id, "agent-42");
        assert!((back.confidence - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn attestation_confidence_range() {
        // Confidence stored as-is — boundary values round-trip correctly
        let att_zero = Attestation {
            attesting_agent_id: "a".to_string(),
            timestamp: Utc::now(),
            confidence: 0.0,
        };
        let att_one = Attestation {
            attesting_agent_id: "a".to_string(),
            timestamp: Utc::now(),
            confidence: 1.0,
        };
        assert!((att_zero.confidence - 0.0).abs() < f64::EPSILON);
        assert!((att_one.confidence - 1.0).abs() < f64::EPSILON);
    }

    // Memory struct
    #[test]
    fn memory_serializes_roundtrip() {
        let now = Utc::now();
        let m = Memory {
            id: "mem-001".to_string(),
            agent_id: "agent-1".to_string(),
            memory_type: MemoryType::Fact,
            content: "The sky is blue".to_string(),
            tags: vec!["nature".to_string(), "color".to_string()],
            created_at: now,
            expires_at: None,
            access_level: AccessLevel::Private,
            attestations: vec![],
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: Memory = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "mem-001");
        assert_eq!(back.agent_id, "agent-1");
        assert_eq!(back.memory_type, MemoryType::Fact);
        assert_eq!(back.content, "The sky is blue");
        assert_eq!(back.tags, vec!["nature", "color"]);
        assert!(back.expires_at.is_none());
        assert_eq!(back.access_level, AccessLevel::Private);
        assert!(back.attestations.is_empty());
    }

    #[test]
    fn memory_with_expiry_roundtrip() {
        let now = Utc::now();
        let expires = now + chrono::Duration::hours(24);
        let m = Memory {
            id: "mem-002".to_string(),
            agent_id: "agent-2".to_string(),
            memory_type: MemoryType::Observation,
            content: "Temporary note".to_string(),
            tags: vec![],
            created_at: now,
            expires_at: Some(expires),
            access_level: AccessLevel::Public,
            attestations: vec![],
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: Memory = serde_json::from_str(&json).unwrap();
        assert!(back.expires_at.is_some());
    }

    #[test]
    fn memory_with_attestations_roundtrip() {
        let now = Utc::now();
        let att = Attestation {
            attesting_agent_id: "verifier-1".to_string(),
            timestamp: now,
            confidence: 0.8,
        };
        let m = Memory {
            id: "mem-003".to_string(),
            agent_id: "agent-3".to_string(),
            memory_type: MemoryType::Decision,
            content: "Use SQLite for storage".to_string(),
            tags: vec!["architecture".to_string()],
            created_at: now,
            expires_at: None,
            access_level: AccessLevel::Shared,
            attestations: vec![att],
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: Memory = serde_json::from_str(&json).unwrap();
        assert_eq!(back.attestations.len(), 1);
        assert_eq!(back.attestations[0].attesting_agent_id, "verifier-1");
    }

    // RecallQuery
    #[test]
    fn recall_query_default_values() {
        let q = RecallQuery::default();
        assert!(q.memory_type.is_none());
        assert!(q.tags.is_none());
        assert!(q.time_range.is_none());
        assert!(q.text_search.is_none());
        assert!(q.agent_id.is_none());
        assert_eq!(q.limit, 100);
    }

    #[test]
    fn recall_query_with_all_fields() {
        let now = Utc::now();
        let q = RecallQuery {
            memory_type: Some(MemoryType::Fact),
            tags: Some(vec!["tag1".to_string()]),
            time_range: Some((now, now + chrono::Duration::hours(1))),
            text_search: Some("search term".to_string()),
            agent_id: Some("agent-x".to_string()),
            limit: 50,
        };
        assert_eq!(q.memory_type, Some(MemoryType::Fact));
        assert_eq!(q.limit, 50);
        assert!(q.time_range.is_some());
    }

    // MemoryError
    #[test]
    fn memory_error_variants_debug() {
        let e1 = MemoryError::NotFound("id-1".to_string());
        let e2 = MemoryError::AccessDenied("no permission".to_string());
        let e3 = MemoryError::StorageError("disk full".to_string());
        let e4 = MemoryError::Expired("id-2".to_string());

        assert!(format!("{:?}", e1).contains("NotFound"));
        assert!(format!("{:?}", e2).contains("AccessDenied"));
        assert!(format!("{:?}", e3).contains("StorageError"));
        assert!(format!("{:?}", e4).contains("Expired"));
    }

    #[test]
    fn memory_error_display() {
        let e = MemoryError::NotFound("mem-999".to_string());
        let msg = format!("{}", e);
        assert!(msg.contains("mem-999"));
    }

    #[test]
    fn memory_error_implements_std_error() {
        let e: Box<dyn std::error::Error> =
            Box::new(MemoryError::StorageError("oops".to_string()));
        assert!(!e.to_string().is_empty());
    }
}
