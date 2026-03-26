use super::types::{InferenceTier, InferenceRequest};

/// Classifies an inference request into a tier.
///
/// Priority:
/// 1. Use caller-supplied tier_hint when present.
/// 2. Base tier by prompt length: <100 → T1, <500 → T2, <2000 → T3, else T4.
/// 3. Apply keyword adjustments (case-insensitive):
///    - "architecture", "security", "review" → +1 tier
///    - "format", "list" → -1 tier
/// 4. Clamp result to T1..T4.
pub fn classify(request: &InferenceRequest) -> InferenceTier {
    // Tier hint overrides all heuristics.
    if let Some(hint) = &request.tier_hint {
        return hint.clone();
    }

    let prompt_lower = request.prompt.to_lowercase();
    let len = request.prompt.len();

    let base = if len < 100 {
        InferenceTier::T1Trivial
    } else if len < 500 {
        InferenceTier::T2Standard
    } else if len < 2000 {
        InferenceTier::T3Complex
    } else {
        InferenceTier::T4Critical
    };

    // Count keyword adjustments.
    const BOOSTERS: &[&str] = &["architecture", "security", "review"];
    const REDUCERS: &[&str] = &["format", "list"];

    let boost: i32 = BOOSTERS.iter().filter(|&&kw| prompt_lower.contains(kw)).count() as i32;
    let reduce: i32 = REDUCERS.iter().filter(|&&kw| prompt_lower.contains(kw)).count() as i32;
    let delta = boost - reduce;

    apply_delta(base, delta)
}

/// Converts tier to numeric index, applies delta, clamps, converts back.
fn apply_delta(tier: InferenceTier, delta: i32) -> InferenceTier {
    let idx = tier_to_index(tier);
    let clamped = (idx + delta).clamp(0, 3);
    index_to_tier(clamped)
}

fn tier_to_index(tier: InferenceTier) -> i32 {
    match tier {
        InferenceTier::T1Trivial => 0,
        InferenceTier::T2Standard => 1,
        InferenceTier::T3Complex => 2,
        InferenceTier::T4Critical => 3,
    }
}

fn index_to_tier(idx: i32) -> InferenceTier {
    match idx {
        0 => InferenceTier::T1Trivial,
        1 => InferenceTier::T2Standard,
        2 => InferenceTier::T3Complex,
        _ => InferenceTier::T4Critical,
    }
}
