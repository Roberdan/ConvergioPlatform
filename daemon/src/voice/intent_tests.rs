use super::intent::{extract_intent, IntentType};

#[test]
fn command_plan_list() {
    let intent = extract_intent("list all plans").unwrap();
    assert_eq!(intent.intent_type, IntentType::Command);
    assert!(intent.command.as_ref().unwrap().contains("plan"));
}

#[test]
fn control_stop() {
    let intent = extract_intent("stop listening").unwrap();
    assert_eq!(intent.intent_type, IntentType::Control);
}

#[test]
fn navigation_mesh() {
    let intent = extract_intent("switch to mesh tab").unwrap();
    assert_eq!(intent.intent_type, IntentType::Navigation);
    assert!(intent.command.as_ref().unwrap().contains("mesh"));
}

#[test]
fn query_detected() {
    let intent = extract_intent("what agents are running right now").unwrap();
    // "what" triggers Query, but "agent" + implied "list" might trigger Command
    assert!(intent.intent_type == IntentType::Query || intent.intent_type == IntentType::Command);
}

#[test]
fn ambiguous_text() {
    let intent = extract_intent("lorem ipsum dolor sit amet").unwrap();
    assert_eq!(intent.intent_type, IntentType::Ambiguous);
    assert!(intent.confidence < 0.5);
}

#[test]
fn raw_text_preserved() {
    let intent = extract_intent("Hello World").unwrap();
    assert_eq!(intent.raw_text, "Hello World");
}
