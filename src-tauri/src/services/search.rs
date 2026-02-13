use crate::db::DatabaseManager;
use crate::error::SunderError;
use crate::services::embedding::{embedding_to_blob, EmbeddingService};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    Hybrid,
    Fulltext,
    Semantic,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub snippet: String,
    pub score: f64,
    pub match_type: String, // "fulltext", "semantic", or "both"
}

struct ScoredNote {
    id: String,
    title: String,
    snippet: String,
    score: f64,
}

pub struct SearchService {
    db: Arc<DatabaseManager>,
    embedding_service: Arc<EmbeddingService>,
}

impl SearchService {
    pub fn new(db: Arc<DatabaseManager>, embedding_service: Arc<EmbeddingService>) -> Self {
        Self {
            db,
            embedding_service,
        }
    }

    pub fn search(
        &self,
        query: &str,
        mode: &SearchMode,
        limit: u32,
    ) -> Result<Vec<SearchResult>, SunderError> {
        let query = query.trim();
        if query.is_empty() {
            return Err(SunderError::EmptyQuery);
        }

        match mode {
            SearchMode::Fulltext => {
                let results = self.fulltext_search(query, limit)?;
                Ok(results
                    .into_iter()
                    .map(|r| SearchResult {
                        id: r.id,
                        title: r.title,
                        snippet: r.snippet,
                        score: r.score,
                        match_type: "fulltext".to_string(),
                    })
                    .collect())
            }
            SearchMode::Semantic => {
                let embedding = self.embedding_service.embed_text(query)?;
                let results = self.semantic_search(&embedding, limit)?;
                Ok(results
                    .into_iter()
                    .map(|r| SearchResult {
                        id: r.id,
                        title: r.title,
                        snippet: r.snippet,
                        score: r.score,
                        match_type: "semantic".to_string(),
                    })
                    .collect())
            }
            SearchMode::Hybrid => self.hybrid_search(query, limit),
        }
    }

    fn fulltext_search(&self, query: &str, limit: u32) -> Result<Vec<ScoredNote>, SunderError> {
        let sanitized = sanitize_fts_query(query);
        if sanitized.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.db.get_read_conn()?;
        let mut stmt = conn.prepare(
            "SELECT n.id, n.title, n.content, bm25(notes_fts) as rank
             FROM notes_fts
             JOIN notes n ON n.rowid = notes_fts.rowid
             WHERE notes_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let results = stmt
            .query_map(rusqlite::params![sanitized, limit], |row| {
                let content: String = row.get(2)?;
                Ok(ScoredNote {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    snippet: make_snippet(&content),
                    score: row.get::<_, f64>(3)?.abs(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    fn semantic_search(
        &self,
        query_embedding: &[f32],
        limit: u32,
    ) -> Result<Vec<ScoredNote>, SunderError> {
        let blob = embedding_to_blob(query_embedding);
        let conn = self.db.get_read_conn()?;

        let mut stmt = conn.prepare(
            "SELECT v.note_id, v.distance, n.title, n.content
             FROM vec_embeddings v
             JOIN notes n ON n.id = v.note_id
             WHERE v.embedding MATCH ?1
             ORDER BY v.distance
             LIMIT ?2",
        )?;

        let results = stmt
            .query_map(rusqlite::params![blob, limit], |row| {
                let content: String = row.get(3)?;
                let distance: f64 = row.get(1)?;
                Ok(ScoredNote {
                    id: row.get(0)?,
                    title: row.get(2)?,
                    snippet: make_snippet(&content),
                    score: 1.0 - distance, // Convert distance to similarity
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    fn hybrid_search(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<SearchResult>, SunderError> {
        let fts_results = self.fulltext_search(query, limit * 2)?;
        let embedding = self.embedding_service.embed_text(query)?;
        let sem_results = self.semantic_search(&embedding, limit * 2)?;

        // Reciprocal Rank Fusion (RRF) with k=60
        let k = 60.0;
        let mut rrf_scores: HashMap<String, (f64, String, String, String)> = HashMap::new();
        // Track which result sets each ID appeared in
        let mut fts_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut sem_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

        for (rank, result) in fts_results.iter().enumerate() {
            let rrf_score = 1.0 / (k + rank as f64 + 1.0);
            fts_ids.insert(result.id.clone());
            rrf_scores
                .entry(result.id.clone())
                .and_modify(|(score, _, _, _)| *score += rrf_score)
                .or_insert((
                    rrf_score,
                    result.title.clone(),
                    result.snippet.clone(),
                    "fulltext".to_string(),
                ));
        }

        for (rank, result) in sem_results.iter().enumerate() {
            let rrf_score = 1.0 / (k + rank as f64 + 1.0);
            sem_ids.insert(result.id.clone());
            rrf_scores
                .entry(result.id.clone())
                .and_modify(|(score, _, _, match_type)| {
                    *score += rrf_score;
                    if match_type == "fulltext" {
                        *match_type = "both".to_string();
                    }
                })
                .or_insert((
                    rrf_score,
                    result.title.clone(),
                    result.snippet.clone(),
                    "semantic".to_string(),
                ));
        }

        let mut combined: Vec<SearchResult> = rrf_scores
            .into_iter()
            .map(|(id, (score, title, snippet, match_type))| SearchResult {
                id,
                title,
                snippet,
                score,
                match_type,
            })
            .collect();

        combined.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        combined.truncate(limit as usize);

        Ok(combined)
    }
}

/// Sanitize FTS5 query: escape special characters, wrap words in quotes.
fn sanitize_fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .filter(|word| {
            let upper = word.to_uppercase();
            !matches!(
                upper.as_str(),
                "OR" | "AND" | "NOT" | "NEAR"
            ) && !word.contains('*')
                && !word.contains(':')
        })
        .map(|word| {
            let escaped = word.replace('"', "");
            format!("\"{escaped}\"")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn make_snippet(content: &str) -> String {
    let stripped: String = content
        .chars()
        .take(250)
        .collect::<String>()
        .lines()
        .map(|line| {
            line.trim()
                .trim_start_matches('#')
                .trim()
                .replace("**", "")
                .replace('*', "")
        })
        .collect::<Vec<_>>()
        .join(" ");

    if stripped.len() > 200 {
        format!("{}...", &stripped[..200])
    } else {
        stripped
    }
}
