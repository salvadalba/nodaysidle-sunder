use crate::db::DatabaseManager;
use crate::error::SunderError;
use crate::services::embedding::blob_to_embedding;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct GraphNode {
    pub id: String,
    pub title: String,
    pub cluster: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

pub struct GraphService {
    db: Arc<DatabaseManager>,
}

impl GraphService {
    pub fn new(db: Arc<DatabaseManager>, _embedding_service: Arc<crate::services::embedding::EmbeddingService>) -> Self {
        Self { db }
    }

    pub fn get_graph(
        &self,
        _center_note_id: Option<&str>,
        threshold: f64,
    ) -> Result<GraphData, SunderError> {
        let conn = self.db.get_read_conn()?;

        // Get all notes
        let mut stmt = conn.prepare("SELECT id, title FROM notes")?;
        let notes: Vec<(String, String)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);

        if notes.is_empty() {
            return Ok(GraphData {
                nodes: Vec::new(),
                edges: Vec::new(),
            });
        }

        // Get all edges from similarity_cache above threshold
        let mut edge_stmt = conn.prepare(
            "SELECT note_id_a, note_id_b, similarity FROM similarity_cache WHERE similarity >= ?1",
        )?;
        let edges: Vec<GraphEdge> = edge_stmt
            .query_map([threshold], |row| {
                Ok(GraphEdge {
                    source: row.get(0)?,
                    target: row.get(1)?,
                    weight: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // Assign clusters via union-find
        let note_ids: Vec<&str> = notes.iter().map(|(id, _)| id.as_str()).collect();
        let clusters = union_find_clusters(&note_ids, &edges);

        let nodes: Vec<GraphNode> = notes
            .iter()
            .map(|(id, title)| GraphNode {
                id: id.clone(),
                title: title.clone(),
                cluster: *clusters.get(id.as_str()).unwrap_or(&0),
            })
            .collect();

        Ok(GraphData { nodes, edges })
    }

    /// Rebuild similarity cache for a single note against all other notes.
    pub fn rebuild_cache_for_note(&self, note_id: &str) -> Result<(), SunderError> {
        let conn = self.db.get_read_conn()?;

        // Get this note's embedding
        let note_vec: Option<Vec<u8>> = conn
            .query_row(
                "SELECT vector FROM embeddings WHERE note_id = ?1",
                [note_id],
                |row| row.get(0),
            )
            .ok();

        let note_embedding = match note_vec {
            Some(blob) => blob_to_embedding(&blob),
            None => return Ok(()), // No embedding yet
        };

        // Get all other embeddings
        let mut stmt = conn.prepare("SELECT note_id, vector FROM embeddings WHERE note_id != ?1")?;
        let others: Vec<(String, Vec<f32>)> = stmt
            .query_map([note_id], |row| {
                let id: String = row.get(0)?;
                let blob: Vec<u8> = row.get(1)?;
                Ok((id, blob_to_embedding(&blob)))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);
        drop(conn);

        let write_conn = self.db.get_write_conn()?;
        let now = chrono::Utc::now().to_rfc3339();

        // Delete old cache entries for this note
        write_conn.execute(
            "DELETE FROM similarity_cache WHERE note_id_a = ?1 OR note_id_b = ?1",
            [note_id],
        )?;

        for (other_id, other_embedding) in &others {
            let similarity = cosine_similarity(&note_embedding, other_embedding);

            // Enforce note_id_a < note_id_b
            let (id_a, id_b) = if note_id < other_id.as_str() {
                (note_id, other_id.as_str())
            } else {
                (other_id.as_str(), note_id)
            };

            write_conn.execute(
                "INSERT OR REPLACE INTO similarity_cache (note_id_a, note_id_b, similarity, updated_at)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![id_a, id_b, similarity, now],
            )?;
        }

        Ok(())
    }

    /// Rebuild the full similarity cache for all note pairs.
    pub fn rebuild_full_cache(&self) -> Result<u32, SunderError> {
        let conn = self.db.get_read_conn()?;
        let mut stmt = conn.prepare("SELECT note_id, vector FROM embeddings")?;
        let all: Vec<(String, Vec<f32>)> = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let blob: Vec<u8> = row.get(1)?;
                Ok((id, blob_to_embedding(&blob)))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);
        drop(conn);

        let write_conn = self.db.get_write_conn()?;
        write_conn.execute("DELETE FROM similarity_cache", [])?;

        let now = chrono::Utc::now().to_rfc3339();
        let mut count = 0u32;

        for i in 0..all.len() {
            for j in (i + 1)..all.len() {
                let (id_a, emb_a) = &all[i];
                let (id_b, emb_b) = &all[j];
                let similarity = cosine_similarity(emb_a, emb_b);

                // Enforce ordering
                let (a, b) = if id_a < id_b {
                    (id_a.as_str(), id_b.as_str())
                } else {
                    (id_b.as_str(), id_a.as_str())
                };

                write_conn.execute(
                    "INSERT INTO similarity_cache (note_id_a, note_id_b, similarity, updated_at)
                     VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![a, b, similarity, now],
                )?;
                count += 1;
            }
        }

        Ok(count)
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| (*x as f64) * (*y as f64)).sum();
    let norm_a: f64 = a.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>().sqrt();
    if norm_a > 0.0 && norm_b > 0.0 {
        dot / (norm_a * norm_b)
    } else {
        0.0
    }
}

/// Union-find clustering based on edges above threshold.
fn union_find_clusters(ids: &[&str], edges: &[GraphEdge]) -> HashMap<String, u32> {
    let mut id_to_idx: HashMap<&str, usize> = HashMap::new();
    for (i, id) in ids.iter().enumerate() {
        id_to_idx.insert(id, i);
    }

    let n = ids.len();
    let mut parent: Vec<usize> = (0..n).collect();

    fn find(parent: &mut [usize], i: usize) -> usize {
        if parent[i] != i {
            parent[i] = find(parent, parent[i]);
        }
        parent[i]
    }

    fn union(parent: &mut [usize], a: usize, b: usize) {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra != rb {
            parent[ra] = rb;
        }
    }

    for edge in edges {
        if let (Some(&a), Some(&b)) = (
            id_to_idx.get(edge.source.as_str()),
            id_to_idx.get(edge.target.as_str()),
        ) {
            union(&mut parent, a, b);
        }
    }

    // Map roots to cluster IDs
    let mut root_to_cluster: HashMap<usize, u32> = HashMap::new();
    let mut next_cluster = 0u32;
    let mut result: HashMap<String, u32> = HashMap::new();

    for (i, id) in ids.iter().enumerate() {
        let root = find(&mut parent, i);
        let cluster = *root_to_cluster.entry(root).or_insert_with(|| {
            let c = next_cluster;
            next_cluster += 1;
            c
        });
        result.insert(id.to_string(), cluster);
    }

    result
}
