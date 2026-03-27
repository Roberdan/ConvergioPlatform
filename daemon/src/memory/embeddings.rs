use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Dimensionality for the hash-based fallback embeddings.
pub const EMBEDDING_DIM: usize = 384;

/// Generates a deterministic pseudo-embedding from text.
///
/// Uses a sliding-window hash approach to produce a fixed-size f32 vector.
/// Serves as a fallback until a real model endpoint is available.
pub fn generate_embedding(text: &str) -> Vec<f32> {
    let normalised = text.to_lowercase();
    let tokens: Vec<&str> = normalised.split_whitespace().collect();
    let mut embedding = vec![0.0_f32; EMBEDDING_DIM];

    // Hash each token and n-gram, distributing values across dimensions.
    for (i, token) in tokens.iter().enumerate() {
        hash_into(&mut embedding, token, i);

        // Bigrams for positional context.
        if i + 1 < tokens.len() {
            let bigram = format!("{} {}", token, tokens[i + 1]);
            hash_into(&mut embedding, &bigram, i + EMBEDDING_DIM / 2);
        }
    }

    l2_normalise(&mut embedding);
    embedding
}

/// Cosine similarity between two vectors. Returns 0.0 if either is zero-length.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let (mut dot, mut norm_a, mut norm_b) = (0.0_f64, 0.0_f64, 0.0_f64);
    for (x, y) in a.iter().zip(b.iter()) {
        let (fx, fy) = (*x as f64, *y as f64);
        dot += fx * fy;
        norm_a += fx * fx;
        norm_b += fy * fy;
    }
    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < 1e-12 {
        return 0.0;
    }
    (dot / denom) as f32
}

/// Serialise embedding to bytes (little-endian packed f32).
pub fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Deserialise embedding from bytes (little-endian packed f32).
pub fn embedding_from_bytes(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

fn hash_into(embedding: &mut [f32], token: &str, offset: usize) {
    let mut hasher = DefaultHasher::new();
    token.hash(&mut hasher);
    let h = hasher.finish();
    let idx = (h as usize ^ offset) % embedding.len();
    // Mix the hash bits into a value in [-1, 1].
    let val = ((h as f64) / (u64::MAX as f64)) * 2.0 - 1.0;
    embedding[idx] += val as f32;
}

fn l2_normalise(v: &mut [f32]) {
    let norm: f64 = v.iter().map(|x| (*x as f64) * (*x as f64)).sum();
    let norm = norm.sqrt();
    if norm > 1e-12 {
        for x in v.iter_mut() {
            *x = (*x as f64 / norm) as f32;
        }
    }
}

#[cfg(test)]
#[path = "embeddings_tests.rs"]
mod tests;
