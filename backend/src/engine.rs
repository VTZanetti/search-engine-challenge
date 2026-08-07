use std::collections::HashMap;

use crate::embed::EmbeddingIndex;
use crate::index::FullTextIndex;
use crate::model::Movie;

/// Reciprocal Rank Fusion constant (standard value: 60).
const RRF_K: f32 = 60.0;

/// Fuse multiple ranked lists (each is `(movie_id, score)`) into one ranking.
fn rrf_fuse(lists: &[Vec<(usize, f32)>]) -> Vec<(usize, f32)> {
    rrf_fuse_with_k(lists, RRF_K)
}

/// Core RRF fusion with an explicit `k` (kept `pub` for unit tests).
pub fn rrf_fuse_with_k(lists: &[Vec<(usize, f32)>], k: f32) -> Vec<(usize, f32)> {
    let mut scores: HashMap<usize, f32> = HashMap::new();

    for list in lists {
        for (rank, (id, _)) in list.iter().enumerate() {
            let entry = scores.entry(*id).or_insert(0.0);
            *entry += 1.0 / (k + rank as f32);
        }
    }

    let mut ranked: Vec<(usize, f32)> = scores.into_iter().collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    ranked
}

/// Strip characters that break Tantivy's query parser (operators, quotes,
/// parens, brackets, colons, etc.) while keeping letters, digits, spaces and
/// simple punctuation. Empty results fall back to the trimmed original text.
fn sanitize_query(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-' || *c == '_')
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        raw.trim().to_string()
    } else {
        trimmed.to_string()
    }
}

/// Combine BM25 + embeddings using RRF and return the top results.
pub fn hybrid_search(
    ft: &FullTextIndex,
    emb: &EmbeddingIndex,
    movies: &[Movie],
    query: &str,
    limit: usize,
    offset: usize,
) -> Result<(Vec<usize>, usize), Box<dyn std::error::Error>> {
    let k = (limit + offset) * 4; // fetch extra candidates before pagination

    // Sanitize before handing to the Tantivy parser.
    let safe = sanitize_query(query);
    if safe.is_empty() {
        return Ok((Vec::new(), 0));
    }

    let bm25 = ft.search(&safe, k)?;
    let semantic = emb.search(&safe, k)?;

    let fused = rrf_fuse(&[bm25, semantic]);

    let total = fused.len();
    let page: Vec<usize> = fused
        .iter()
        .skip(offset)
        .take(limit)
        .map(|(id, _)| *id)
        .collect();

    let _ = movies; // used by callers for details
    Ok((page, total))
}

/// Autocomplete: prefix matches on titles + top plot keywords.
pub fn suggest(
    movies: &[Movie],
    query: &str,
    limit: usize,
) -> Vec<serde_json::Value> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();

    // Titles starting with the prefix (case-insensitive, ignore accents lightly).
    let mut title_hits: Vec<(&str, f64)> = movies
        .iter()
        .filter_map(|m| {
            let t = m.title.trim();
            let tl = t.to_lowercase();
            if tl.starts_with(&q) {
                Some((t, m.imdb_score.unwrap_or(0.0) as f64))
            } else {
                None
            }
        })
        .collect();
    title_hits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    for (title, _) in title_hits.into_iter().take(limit) {
        out.push(serde_json::json!({ "type": "title", "text": title }));
    }

    // Keywords containing the prefix, deduplicated.
    let mut seen = std::collections::HashSet::new();
    for movie in movies {
        for kw in &movie.plot_keywords {
            let kl = kw.to_lowercase();
            if kl.contains(&q) && seen.insert(kl.clone()) {
                out.push(serde_json::json!({ "type": "keyword", "text": kw }));
                if out.len() >= limit * 2 {
                    break;
                }
            }
        }
        if out.len() >= limit * 2 {
            break;
        }
    }

    out.truncate(limit);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(list: &[(usize, f32)]) -> Vec<usize> {
        list.iter().map(|(id, _)| *id).collect()
    }

    #[test]
    fn rrf_fuses_single_list_in_order() {
        let lists = vec![vec![(3, 1.0), (1, 0.5), (2, 0.1)]];
        let fused = rrf_fuse(&lists);
        assert_eq!(ids(&fused), vec![3, 1, 2]);
    }

    #[test]
    fn rrf_prefers_items_ranked_high_in_both_lists() {
        // 42 ranks #1 in both lists -> highest RRF score.
        // 7 ranks #2 in both lists -> next.
        // 3 ranks #3 in the second list only -> last.
        let lists = vec![
            vec![(42, 9.0), (7, 8.0)],
            vec![(42, 0.2), (7, 0.1), (3, 0.05)],
        ];
        let fused = rrf_fuse(&lists);
        assert_eq!(ids(&fused), vec![42, 7, 3]);
    }

    #[test]
    fn rrf_deduplicates_ids_across_lists() {
        let lists = vec![vec![(5, 1.0)], vec![(5, 1.0)], vec![(9, 1.0)]];
        let fused = rrf_fuse(&lists);
        assert_eq!(ids(&fused), vec![5, 9]);
    }

    #[test]
    fn rrf_empty_lists_yield_empty_result() {
        assert!(rrf_fuse(&[]).is_empty());
        assert!(rrf_fuse(&[vec![], vec![]]).is_empty());
    }

    #[test]
    fn rrf_k_changes_scores_but_not_ranking() {
        let lists = vec![vec![(1, 1.0), (2, 0.5)]];
        let a = rrf_fuse_with_k(&lists, 1.0);
        let b = rrf_fuse_with_k(&lists, 60.0);
        assert_eq!(ids(&a), ids(&b));
        // Smaller k amplifies differences.
        assert!(a[0].1 - a[1].1 > b[0].1 - b[1].1);
    }

    #[test]
    fn sanitize_removes_tantivy_operators() {
        assert_eq!(sanitize_query("inception"), "inception");
        assert_eq!(sanitize_query("sci-fi space"), "sci-fi space");
        // '+' ':' '"' '()' are stripped; '-' is kept for hyphenated words.
        assert_eq!(sanitize_query("star+: wars :episode"), "star wars episode");
        assert_eq!(sanitize_query("\"matrix\" (1999)"), "matrix 1999");
        assert_eq!(sanitize_query("a:b c"), "ab c");
    }

    #[test]
    fn sanitize_keeps_alphanumerics_and_falls_back() {
        // All-special input becomes empty (caller handles it).
        assert_eq!(sanitize_query("  "), "");
        // Nothing to strip -> falls back to the original text.
        assert_eq!(sanitize_query("!!!"), "!!!");
        assert_eq!(sanitize_query("münchen"), "münchen");
    }
}