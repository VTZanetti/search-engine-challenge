//! Benchmark harness: measures `elapsed_ms` for hybrid vs BM25-only vs
//! embeddings-only search over a fixed set of queries.
//!
//! Run from the repo root so the relative CSV path resolves:
//!   cargo run --release --bin bench -- ../db/movie_metadata.csv
//! or with an explicit CSV path as the first argument.

use std::time::Instant;

use movie_search_backend::embed::EmbeddingIndex;
use movie_search_backend::index::FullTextIndex;
use movie_search_backend::model::load_movies;

const QUERIES: &[&str] = &[
    "inception",
    "sci-fi space",
    "Christopher Nolan",
    "pirate",
    "love story",
    "matrix",
    "war drama",
    "comedy",
    "alien",
    "romance",
];

fn bench(label: &str, f: impl Fn()) {
    // Warm-up to populate caches / load model.
    f();
    let mut times = Vec::new();
    for _ in 0..5 {
        let start = Instant::now();
        f();
        times.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    let avg = times.iter().sum::<f64>() / times.len() as f64;
    println!("{label:<22} avg {:>8.2} ms   (runs: {})", avg, times.len());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let csv_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "db/movie_metadata.csv".to_string());
    let cache_dir = std::path::Path::new(&csv_path)
        .parent()
        .unwrap_or(std::path::Path::new("db"));

    println!("Loading movies from {csv_path}...");
    let movies = load_movies(&csv_path)?;
    println!("Loaded {} movies", movies.len());

    println!("Building full-text index...");
    let ft = FullTextIndex::build(&movies)?;

    println!("Building embedding index...");
    let emb = EmbeddingIndex::build(&movies, &cache_dir.to_string_lossy())?;

    println!("\n--- Benchmark (5 warm runs, averaged) ---\n");

    for q in QUERIES {
        println!("Query: {q:?}");
        bench("hybrid", || {
            let _ = movie_search_backend::engine::hybrid_search(&ft, &emb, &movies, q, 20, 0);
        });
        bench("bm25-only", || {
            let _ = ft.search(q, 20);
        });
        bench("embeddings-only", || {
            let _ = emb.search(q, 20);
        });
        println!();
    }

    Ok(())
}