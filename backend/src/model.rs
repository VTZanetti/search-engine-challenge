use serde::{Deserialize, Serialize};

/// A single movie row, normalized from `db/movie_metadata.csv`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Movie {
    pub id: usize,
    pub title: String,
    pub year: Option<i64>,
    pub genres: Vec<String>,
    pub director: String,
    pub actors: Vec<String>,
    pub imdb_score: Option<f64>,
    pub plot_keywords: Vec<String>,
    pub link: String,
    pub language: String,
    pub country: String,
}

impl Movie {
    /// Combined text used to build the embedding (semantic representation).
    pub fn embed_text(&self) -> String {
        let mut parts = vec![self.title.clone()];
        if !self.genres.is_empty() {
            parts.push(format!("genres: {}", self.genres.join(", ")));
        }
        if !self.director.is_empty() {
            parts.push(format!("director: {}", self.director));
        }
        if !self.actors.is_empty() {
            parts.push(format!("cast: {}", self.actors.join(", ")));
        }
        if !self.plot_keywords.is_empty() {
            parts.push(format!("keywords: {}", self.plot_keywords.join(", ")));
        }
        parts.join(" | ")
    }
}

/// Subset of a movie returned by the search API.
#[derive(Debug, Clone, Serialize)]
pub struct MovieResult {
    pub id: usize,
    pub title: String,
    pub year: Option<i64>,
    pub genres: Vec<String>,
    pub director: String,
    pub actors: Vec<String>,
    pub imdb_score: Option<f64>,
    pub plot_keywords: Vec<String>,
    pub link: String,
}

impl From<&Movie> for MovieResult {
    fn from(m: &Movie) -> Self {
        MovieResult {
            id: m.id,
            title: m.title.clone(),
            year: m.year,
            genres: m.genres.clone(),
            director: m.director.clone(),
            actors: m.actors.clone(),
            imdb_score: m.imdb_score,
            plot_keywords: m.plot_keywords.clone(),
            link: m.link.clone(),
        }
    }
}

/// Normalize a raw CSV value: trim and convert `NaN`/empty to `None`.
fn clean(raw: &str) -> Option<String> {
    let t = raw.trim();
    if t.is_empty() || t == "NaN" || t == "nan" || t == "NULL" {
        None
    } else {
        Some(t.to_string())
    }
}

fn clean_number(raw: &str) -> Option<f64> {
    clean(raw).and_then(|s| s.parse::<f64>().ok())
}

fn split_pipe(raw: &str) -> Vec<String> {
    raw.split('|')
        .map(|s| s.trim().to_string())
        .filter(|s| {
            !s.is_empty()
                && s != "NaN"
                && s.to_lowercase() != "nan"
                && s != "NULL"
                && s != "N/A"
        })
        .collect()
}

/// Load all movies from the CSV file into memory.
pub fn load_movies(path: &str) -> Result<Vec<Movie>, Box<dyn std::error::Error>> {
    let mut rdr = csv::Reader::from_path(path)?;
    let headers = rdr.headers()?.clone();
    let mut movies = Vec::new();

    // Resolve column indexes once, by header name.
    let col = |name: &str| -> Option<usize> { headers.iter().position(|h| h == name) };

    fn cell<'a>(rec: &'a csv::StringRecord, col: impl Fn(&str) -> Option<usize>, name: &str) -> Option<&'a str> {
        col(name).and_then(|i| rec.get(i))
    }

    for (i, record) in rdr.records().enumerate() {
        let rec = record?;
        let title = cell(&rec, &col, "movie_title")
            .and_then(clean)
            .unwrap_or_else(|| format!("Movie {}", i));

        movies.push(Movie {
            id: i,
            title,
            year: cell(&rec, &col, "title_year")
                .and_then(clean_number)
                .map(|v| v as i64),
            genres: cell(&rec, &col, "genres").map(split_pipe).unwrap_or_default(),
            director: cell(&rec, &col, "director_name").and_then(clean).unwrap_or_default(),
            actors: ["actor_1_name", "actor_2_name", "actor_3_name"]
                .iter()
                .filter_map(|c| cell(&rec, &col, c).and_then(clean))
                .collect(),
            imdb_score: cell(&rec, &col, "imdb_score").and_then(clean_number),
            plot_keywords: cell(&rec, &col, "plot_keywords").map(split_pipe).unwrap_or_default(),
            link: cell(&rec, &col, "movie_imdb_link").and_then(clean).unwrap_or_default(),
            language: cell(&rec, &col, "language").and_then(clean).unwrap_or_default(),
            country: cell(&rec, &col, "country").and_then(clean).unwrap_or_default(),
        });
    }

    Ok(movies)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADERS: &str = "movie_title,title_year,genres,director_name,actor_1_name,actor_2_name,actor_3_name,imdb_score,plot_keywords,movie_imdb_link,language,country";

    fn write_csv(name: &str, rows: &[&str]) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        // Unique per test to avoid parallel-test collisions on the same PID.
        path.push(format!("movie_test_{name}_{}.csv", std::process::id()));
        let mut content = String::from(HEADERS);
        content.push('\n');
        for r in rows {
            content.push_str(r);
            content.push('\n');
        }
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn loads_rows_and_assigns_ids() {
        let path = write_csv("loads", &[
            "Inception,2010,Action|Sci-Fi,Christopher Nolan,Leonardo DiCaprio,,,8.8,dream|heist,http://tt1375666/,English,USA",
            "The Matrix,1999,Action|Sci-Fi,Lana Wachowski,Keanu Reeves,,,8.7,virtual|reality,http://tt0133093/,English,USA",
        ]);
        let movies = load_movies(path.to_str().unwrap()).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(movies.len(), 2);
        assert_eq!(movies[0].id, 0);
        assert_eq!(movies[0].title, "Inception");
        assert_eq!(movies[0].year, Some(2010));
        assert_eq!(movies[0].genres, vec!["Action", "Sci-Fi"]);
        assert_eq!(movies[0].director, "Christopher Nolan");
        assert_eq!(movies[0].actors, vec!["Leonardo DiCaprio"]);
        assert_eq!(movies[0].imdb_score, Some(8.8));
        assert_eq!(movies[0].plot_keywords, vec!["dream", "heist"]);
        assert_eq!(movies[0].link, "http://tt1375666/");
    }

    #[test]
    fn empty_and_nan_fields_are_handled_without_panic() {
        let path = write_csv("nan", &[
            "Pulp Fiction,1994,Crime,,,,,8.9,,,English,USA",
            "NaN,NaN,NaN,NaN,NaN,NaN,NaN,NaN,NaN,NaN,NaN,NaN",
        ]);
        let movies = load_movies(path.to_str().unwrap()).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(movies.len(), 2);
        // Missing title falls back to a placeholder.
        assert_eq!(movies[1].title, "Movie 1");
        assert_eq!(movies[1].year, None);
        assert!(movies[1].genres.is_empty());
        assert!(movies[1].actors.is_empty());
        assert_eq!(movies[1].imdb_score, None);
        assert!(movies[1].plot_keywords.is_empty());
        assert_eq!(movies[1].language, "");
        assert_eq!(movies[1].country, "");
    }

    #[test]
    fn embed_text_joins_fields() {
        let path = write_csv("embed", &[
            "Inception,2010,Action|Sci-Fi,Christopher Nolan,Leonardo DiCaprio,,,8.8,dream|heist,http://tt1375666/,English,USA",
        ]);
        let movies = load_movies(path.to_str().unwrap()).unwrap();
        std::fs::remove_file(&path).ok();

        let text = movies[0].embed_text();
        assert!(text.contains("Inception"));
        assert!(text.contains("genres: Action, Sci-Fi"));
        assert!(text.contains("director: Christopher Nolan"));
        assert!(text.contains("cast: Leonardo DiCaprio"));
        assert!(text.contains("keywords: dream, heist"));
    }
}