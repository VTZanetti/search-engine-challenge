use std::path::Path;
use std::sync::{Mutex, OnceLock};

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::model::Movie;

/// Cache file where pre-computed embeddings are stored (bincode).
const CACHE_FILE: &str = "embeddings.bin";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Cache {
    /// Embedding vectors, one per movie, aligned with `movies` order.
    vectors: Vec<Vec<f32>>,
}

/// Semantic index: normalized embeddings + cosine similarity search.
pub struct EmbeddingIndex {
    /// Row id -> normalized embedding.
    vectors: Vec<Vec<f32>>,
}

impl EmbeddingIndex {
    /// Load from cache if present, otherwise compute with the ONNX model
    /// (downloads once on first run) and persist to disk.
    pub fn build(movies: &[Movie], cache_dir: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let cache_path = Path::new(cache_dir).join(CACHE_FILE);

        if let Some(vectors) = Self::read_cache(&cache_path, movies.len()) {
            println!("Loaded {} embeddings from cache", vectors.len());
            return Ok(Self { vectors });
        }

        println!("Computing embeddings with fastembed (first run, may take a while)...");
        let texts: Vec<String> = movies.iter().map(|m| m.embed_text()).collect();
        let mut model_guard = model_lock()?.lock().unwrap();
        let embeddings = model_guard.embed(texts, None)?;
        drop(model_guard);

        let vectors: Vec<Vec<f32>> = embeddings.into_iter().map(normalize).collect();

        Self::write_cache(&cache_path, &vectors)?;
        println!("Saved {} embeddings to cache", vectors.len());

        Ok(Self { vectors })
    }

    /// Rank movies by cosine similarity to the query text.
    pub fn search(
        &self,
        query_text: &str,
        limit: usize,
    ) -> Result<Vec<(usize, f32)>, Box<dyn std::error::Error>> {
        let mut model_guard = model_lock()?.lock().unwrap();
        let q = model_guard.embed(vec![query_text.to_string()], None)?;
        drop(model_guard);
        let q_vec = normalize(q.into_iter().next().ok_or("no embedding")?);

        // Parallel cosine scan over all movies; rayon splits across cores.
        let mut scored: Vec<(usize, f32)> = self
            .vectors
            .par_iter()
            .enumerate()
            .map(|(i, v)| (i, cosine(&q_vec, v)))
            .collect();

        // Sort descending by score.
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        Ok(scored)
    }

    fn read_cache(path: &Path, expected: usize) -> Option<Vec<Vec<f32>>> {
        if !path.exists() {
            return None;
        }
        let bytes = std::fs::read(path).ok()?;
        let cache: Cache = bincode::deserialize(&bytes).ok()?;
        if cache.vectors.len() != expected || cache.vectors.iter().any(|v| v.is_empty()) {
            return None;
        }
        Some(cache.vectors)
    }

    fn write_cache(path: &Path, vectors: &[Vec<f32>]) -> Result<(), Box<dyn std::error::Error>> {
        let bytes = bincode::serialize(&Cache { vectors: vectors.to_vec() })?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, bytes)?;
        Ok(())
    }
}

/// Cache the loaded ONNX model process-wide so queries don't reload it.
static MODEL: OnceLock<Mutex<TextEmbedding>> = OnceLock::new();

fn model_lock() -> Result<&'static Mutex<TextEmbedding>, Box<dyn std::error::Error>> {
    Ok(MODEL.get_or_init(|| {
        Mutex::new(
            TextEmbedding::try_new(
                InitOptions::new(EmbeddingModel::AllMiniLML6V2)
                    .with_show_download_progress(false),
            )
            .expect("failed to load embedding model"),
        )
    }))
}

fn normalize(mut v: Vec<f32>) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
    v
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}