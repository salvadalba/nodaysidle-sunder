use crate::db::DatabaseManager;
use crate::error::SunderError;
use crate::services::embedding::{embedding_to_blob, EmbeddingService};
use lru::LruCache;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize)]
pub struct LatentLink {
    pub note_id: String,
    pub title: String,
    pub similarity: f64,
    pub snippet: String,
}

pub struct LinkService {
    db: Arc<DatabaseManager>,
    embedding_service: Arc<EmbeddingService>,
    cache: Mutex<LruCache<String, Vec<LatentLink>>>,
}

impl LinkService {
    pub fn new(db: Arc<DatabaseManager>, embedding_service: Arc<EmbeddingService>) -> Self {
        Self {
            db,
            embedding_service,
            cache: Mutex::new(LruCache::new(NonZeroUsize::new(64).unwrap())),
        }
    }

    pub fn compute_latent_links(
        &self,
        content: &str,
        exclude_note_id: Option<&str>,
        threshold: f64,
        limit: u32,
    ) -> Result<Vec<LatentLink>, SunderError> {
        if content.split_whitespace().count() < 3 {
            return Ok(Vec::new());
        }

        // Check cache
        let cache_key = content_hash(content);
        if let Ok(mut cache) = self.cache.lock() {
            if let Some(cached) = cache.get(&cache_key) {
                let mut results = cached.clone();
                if let Some(exclude) = exclude_note_id {
                    results.retain(|l| l.note_id != exclude);
                }
                results.retain(|l| l.similarity >= threshold);
                results.truncate(limit as usize);
                return Ok(results);
            }
        }

        let embedding = self.embedding_service.embed_text(content)?;
        let blob = embedding_to_blob(&embedding);

        let conn = self.db.get_read_conn()?;
        let mut stmt = conn.prepare(
            "SELECT v.note_id, v.distance, n.title, n.content
             FROM vec_embeddings v
             JOIN notes n ON n.id = v.note_id
             WHERE v.embedding MATCH ?1
             ORDER BY v.distance
             LIMIT ?2",
        )?;

        // Fetch more than needed so we can filter
        let fetch_limit = (limit * 3).max(20);
        let links: Vec<LatentLink> = stmt
            .query_map(rusqlite::params![blob, fetch_limit], |row| {
                let content: String = row.get(3)?;
                let distance: f64 = row.get(1)?;
                Ok(LatentLink {
                    note_id: row.get(0)?,
                    title: row.get(2)?,
                    similarity: 1.0 - distance,
                    snippet: make_snippet(&content),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // Cache before filtering
        if let Ok(mut cache) = self.cache.lock() {
            cache.put(cache_key, links.clone());
        }

        let mut results = links;
        if let Some(exclude) = exclude_note_id {
            results.retain(|l| l.note_id != exclude);
        }
        results.retain(|l| l.similarity >= threshold);
        results.truncate(limit as usize);

        Ok(results)
    }
}

fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
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
