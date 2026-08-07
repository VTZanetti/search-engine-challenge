use std::sync::Arc;
use std::time::Instant;

use actix_cors::Cors;
use actix_web::{get, web, App, HttpServer, Responder};
use movie_search_backend::embed::EmbeddingIndex;
use movie_search_backend::index::FullTextIndex;
use movie_search_backend::model::{load_movies, Movie, MovieResult};
use serde::{Deserialize, Serialize};
use serde_json::json;

const DEFAULT_CSV_PATH: &str = "db/movie_metadata.csv";
const CACHE_DIR: &str = "db";
const DEFAULT_LIMIT: usize = 20;
const MAX_LIMIT: usize = 100;

/// Shared application state.
struct AppState {
    movies: Vec<Movie>,
    ft: FullTextIndex,
    emb: EmbeddingIndex,
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Deserialize)]
struct SuggestQuery {
    q: String,
    limit: Option<usize>,
}

#[derive(Serialize)]
struct SearchResponse {
    query: String,
    total: usize,
    elapsed_ms: f64,
    results: Vec<MovieResult>,
}

#[get("/api/search")]
async fn search(
    state: web::Data<Arc<AppState>>,
    params: web::Query<SearchQuery>,
) -> impl Responder {
    let started = Instant::now();
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = params.offset.unwrap_or(0);

    let q = params.q.trim();
    if q.is_empty() {
        return web::Json(SearchResponse {
            query: params.q.clone(),
            total: 0,
            elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
            results: vec![],
        });
    }

    match movie_search_backend::engine::hybrid_search(
        &state.ft,
        &state.emb,
        &state.movies,
        q,
        limit,
        offset,
    ) {
        Ok((ids, total)) => {
            let results: Vec<MovieResult> = ids
                .iter()
                .filter_map(|id| state.movies.get(*id))
                .map(MovieResult::from)
                .collect();
            let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
            web::Json(SearchResponse {
                query: params.q.clone(),
                total,
                elapsed_ms,
                results,
            })
        }
        Err(_) => web::Json(SearchResponse {
            query: params.q.clone(),
            total: 0,
            elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
            results: vec![],
        }),
    }
}

#[get("/api/suggest")]
async fn suggest(
    state: web::Data<Arc<AppState>>,
    params: web::Query<SuggestQuery>,
) -> impl Responder {
    let limit = params.limit.unwrap_or(8).clamp(1, 20);
    let suggestions = movie_search_backend::engine::suggest(&state.movies, &params.q, limit);
    web::Json(json!({ "suggestions": suggestions }))
}

#[get("/api/health")]
async fn health(state: web::Data<Arc<AppState>>) -> impl Responder {
    web::Json(json!({
        "status": "ok",
        "movies": state.movies.len()
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Configuration via environment variables (with sensible defaults).
    let csv_path = std::env::var("CSV_PATH").unwrap_or_else(|_| DEFAULT_CSV_PATH.to_string());
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080u16);

    // The embeddings cache lives next to the CSV so it works from any cwd.
    let cache_dir = std::path::Path::new(&csv_path)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| CACHE_DIR.to_string());

    println!("Loading movies from {csv_path}...");
    let movies = load_movies(&csv_path).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, format!("CSV load failed: {e}"))
    })?;
    println!("Loaded {} movies", movies.len());

    println!("Building full-text index...");
    let ft = FullTextIndex::build(&movies).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, format!("index build failed: {e}"))
    })?;

    println!("Building embedding index...");
    let emb = EmbeddingIndex::build(&movies, &cache_dir).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, format!("embed build failed: {e}"))
    })?;

    let state = Arc::new(AppState { movies, ft, emb });

    let addr = format!("127.0.0.1:{port}");
    println!("Server listening on http://{addr}");

    HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(state.clone()))
            .service(search)
            .service(suggest)
            .service(health)
    })
    .bind(addr)?
    .run()
    .await
}