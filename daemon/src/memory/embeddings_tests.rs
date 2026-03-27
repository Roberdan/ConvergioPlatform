use super::*;

#[test]
fn deterministic_output() {
    let a = generate_embedding("hello world");
    let b = generate_embedding("hello world");
    assert_eq!(a, b, "same input must produce identical embeddings");
}

#[test]
fn correct_dimensionality() {
    let emb = generate_embedding("the quick brown fox");
    assert_eq!(emb.len(), EMBEDDING_DIM);
}

#[test]
fn unit_norm() {
    let emb = generate_embedding("normalisation test");
    let norm: f64 = emb.iter().map(|x| (*x as f64) * (*x as f64)).sum();
    assert!(
        (norm.sqrt() - 1.0).abs() < 1e-5,
        "embedding should be L2-normalised, got norm={:.6}",
        norm.sqrt()
    );
}

#[test]
fn similar_texts_higher_similarity() {
    let a = generate_embedding("the deployment pipeline runs on kubernetes");
    let b = generate_embedding("the deployment pipeline uses kubernetes pods");
    let c = generate_embedding("fresh pasta recipe with tomato sauce");
    let sim_ab = cosine_similarity(&a, &b);
    let sim_ac = cosine_similarity(&a, &c);
    assert!(
        sim_ab > sim_ac,
        "related texts ({sim_ab:.4}) should be more similar than unrelated ({sim_ac:.4})"
    );
}

#[test]
fn self_similarity_is_one() {
    let emb = generate_embedding("self similarity check");
    let sim = cosine_similarity(&emb, &emb);
    assert!(
        (sim - 1.0).abs() < 1e-5,
        "self-similarity should be 1.0, got {sim:.6}"
    );
}

#[test]
fn empty_input_produces_zero_vector() {
    let emb = generate_embedding("");
    // All zeros after normalisation of a zero vector stays zero.
    let norm: f64 = emb.iter().map(|x| (*x as f64) * (*x as f64)).sum();
    assert!(norm < 1e-10, "empty input should produce zero vector");
}

#[test]
fn cosine_mismatched_lengths_returns_zero() {
    let a = vec![1.0_f32; 10];
    let b = vec![1.0_f32; 5];
    assert_eq!(cosine_similarity(&a, &b), 0.0);
}

#[test]
fn roundtrip_bytes() {
    let emb = generate_embedding("roundtrip serialisation");
    let bytes = embedding_to_bytes(&emb);
    let restored = embedding_from_bytes(&bytes);
    assert_eq!(emb, restored, "byte roundtrip must be lossless");
}

#[test]
fn bytes_length_correct() {
    let emb = generate_embedding("length check");
    let bytes = embedding_to_bytes(&emb);
    assert_eq!(bytes.len(), EMBEDDING_DIM * 4);
}
