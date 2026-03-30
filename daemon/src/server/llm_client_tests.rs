use super::*;

#[test]
fn parse_claude_content_block_delta() {
    let block = "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}";
    assert!(matches!(parse_claude_sse(block), Some(StreamChunk::Text(t)) if t == "Hello"));
}

#[test]
fn parse_claude_message_start_usage() {
    let block = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":42,\"output_tokens\":0}}}";
    assert!(
        matches!(parse_claude_sse(block), Some(StreamChunk::Usage(u)) if u.input_tokens == 42)
    );
}

#[test]
fn parse_openai_delta_content() {
    let block = "data: {\"choices\":[{\"delta\":{\"content\":\"world\"},\"index\":0}]}";
    assert!(matches!(parse_openai_sse(block), Some(StreamChunk::Text(t)) if t == "world"));
}

#[test]
fn parse_openai_done() {
    assert!(parse_openai_sse("data: [DONE]").is_none());
}

#[test]
fn parse_openai_usage_chunk() {
    let block = "data: {\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":20}}";
    assert!(
        matches!(parse_openai_sse(block), Some(StreamChunk::Usage(u)) if u.input_tokens == 10 && u.output_tokens == 20)
    );
}

#[test]
fn build_prompt_from_messages() {
    let msgs = vec![
        ChatMessage { role: "system".into(), content: "You are helpful".into() },
        ChatMessage { role: "user".into(), content: "Hello".into() },
    ];
    let prompt = build_prompt(&msgs);
    assert!(prompt.contains("[System]: You are helpful"));
    assert!(prompt.contains("Hello"));
}

#[test]
fn provider_enum_variants() {
    assert_ne!(Provider::ClaudeSubscription, Provider::LocalLLM);
    assert_ne!(Provider::CopilotSubscription, Provider::LocalLLM);
    assert_ne!(Provider::ClaudeSubscription, Provider::CopilotSubscription);
}
