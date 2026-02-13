use crate::db::DatabaseManager;
use crate::error::SunderError;
use ort::session::Session;
use ort::value::Tensor;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokenizers::Tokenizer;

const EMBEDDING_DIM: usize = 384;
const MAX_TOKENS: usize = 512;
const OVERLAP_TOKENS: usize = 256;

pub struct EmbeddingService {
    session: Mutex<Session>,
    tokenizer: Tokenizer,
    db: Arc<DatabaseManager>,
    reindexing: AtomicBool,
}

impl EmbeddingService {
    pub fn new(resource_dir: &Path, db: Arc<DatabaseManager>) -> Result<Self, SunderError> {
        let model_path = resource_dir.join("model_quantized.onnx");
        let tokenizer_path = resource_dir.join("tokenizer.json");

        if !model_path.exists() {
            return Err(SunderError::EmbeddingError(format!(
                "ONNX model not found: {}",
                model_path.display()
            )));
        }
        if !tokenizer_path.exists() {
            return Err(SunderError::EmbeddingError(format!(
                "Tokenizer not found: {}",
                tokenizer_path.display()
            )));
        }

        let session = Session::builder()
            .map_err(|e| SunderError::EmbeddingError(format!("Session builder: {e}")))?
            .with_intra_threads(2)
            .map_err(|e| SunderError::EmbeddingError(format!("Set threads: {e}")))?
            .commit_from_file(&model_path)
            .map_err(|e| SunderError::EmbeddingError(format!("Load ONNX: {e}")))?;

        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| SunderError::EmbeddingError(format!("Load tokenizer: {e}")))?;

        Ok(Self {
            session: Mutex::new(session),
            tokenizer,
            db,
            reindexing: AtomicBool::new(false),
        })
    }

    /// Embed a text string into a 384-dimensional unit vector.
    pub fn embed_text(&self, text: &str) -> Result<Vec<f32>, SunderError> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| SunderError::EmbeddingError(format!("Tokenization failed: {e}")))?;

        let token_count = encoding.get_ids().len();

        if token_count <= MAX_TOKENS {
            return self.embed_tokens(encoding.get_ids(), encoding.get_attention_mask());
        }

        // Chunk long texts with overlap
        let ids = encoding.get_ids();
        let mask = encoding.get_attention_mask();
        let mut chunk_embeddings: Vec<Vec<f32>> = Vec::new();

        let mut start = 0;
        while start < ids.len() {
            let end = (start + MAX_TOKENS).min(ids.len());
            let chunk_ids = &ids[start..end];
            let chunk_mask = &mask[start..end];

            let emb = self.embed_tokens(chunk_ids, chunk_mask)?;
            chunk_embeddings.push(emb);

            if end >= ids.len() {
                break;
            }
            start += MAX_TOKENS - OVERLAP_TOKENS;
        }

        // Average chunk embeddings
        let mut avg = vec![0.0f32; EMBEDDING_DIM];
        for emb in &chunk_embeddings {
            for (i, v) in emb.iter().enumerate() {
                avg[i] += v;
            }
        }
        let n = chunk_embeddings.len() as f32;
        for v in &mut avg {
            *v /= n;
        }

        l2_normalize(&mut avg);
        Ok(avg)
    }

    /// Run ONNX inference on token IDs with attention mask, mean pool, and normalize.
    fn embed_tokens(&self, ids: &[u32], attention_mask: &[u32]) -> Result<Vec<f32>, SunderError> {
        let seq_len = ids.len();

        let input_ids: Vec<i64> = ids.iter().map(|&x| x as i64).collect();
        let attn_mask: Vec<i64> = attention_mask.iter().map(|&x| x as i64).collect();
        let token_type_ids: Vec<i64> = vec![0i64; seq_len];

        let input_ids_tensor = Tensor::from_array(([1usize, seq_len], input_ids.into_boxed_slice()))
            .map_err(|e| SunderError::EmbeddingError(format!("input_ids tensor: {e}")))?;
        let attn_mask_tensor = Tensor::from_array(([1usize, seq_len], attn_mask.into_boxed_slice()))
            .map_err(|e| SunderError::EmbeddingError(format!("attention_mask tensor: {e}")))?;
        let token_type_tensor = Tensor::from_array(([1usize, seq_len], token_type_ids.into_boxed_slice()))
            .map_err(|e| SunderError::EmbeddingError(format!("token_type_ids tensor: {e}")))?;

        let mut session = self.session.lock()
            .map_err(|e| SunderError::EmbeddingError(format!("Session lock: {e}")))?;
        let outputs = session
            .run([
                input_ids_tensor.into(),
                attn_mask_tensor.into(),
                token_type_tensor.into(),
            ])
            .map_err(|e| SunderError::EmbeddingError(format!("Inference failed: {e}")))?;

        // Output shape: [1, seq_len, 384]
        let (shape, data) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| SunderError::EmbeddingError(format!("Output extraction: {e}")))?;

        let dims: &[i64] = shape;
        let hidden_dim = if dims.len() == 3 { dims[2] as usize } else { EMBEDDING_DIM };

        // Mean pooling with attention mask
        let mut pooled = vec![0.0f32; hidden_dim];
        let mut total_weight = 0.0f32;

        for (t, &mask_val) in attention_mask.iter().enumerate().take(seq_len) {
            let w = mask_val as f32;
            total_weight += w;
            let offset = t * hidden_dim;
            for d in 0..hidden_dim {
                pooled[d] += data[offset + d] * w;
            }
        }

        if total_weight > 0.0 {
            for v in &mut pooled {
                *v /= total_weight;
            }
        }

        l2_normalize(&mut pooled);
        Ok(pooled)
    }

    /// Store an embedding for a note in both embeddings table and vec_embeddings virtual table.
    pub fn index_note(&self, note_id: &str, content: &str) -> Result<(), SunderError> {
        let embedding = self.embed_text(content)?;
        let blob = embedding_to_blob(&embedding);
        let now = chrono::Utc::now().to_rfc3339();

        let conn = self.db.get_write_conn()?;

        conn.execute(
            "INSERT OR REPLACE INTO embeddings (note_id, vector, model_version, updated_at)
             VALUES (?1, ?2, 'minilm-v2-q8', ?3)",
            rusqlite::params![note_id, blob, now],
        )?;

        conn.execute(
            "DELETE FROM vec_embeddings WHERE note_id = ?1",
            [note_id],
        )?;
        conn.execute(
            "INSERT INTO vec_embeddings (note_id, embedding) VALUES (?1, ?2)",
            rusqlite::params![note_id, blob],
        )?;

        Ok(())
    }

    /// Remove embedding for a note from both tables.
    pub fn remove_embedding(&self, note_id: &str) -> Result<(), SunderError> {
        let conn = self.db.get_write_conn()?;
        conn.execute("DELETE FROM embeddings WHERE note_id = ?1", [note_id])?;
        conn.execute("DELETE FROM vec_embeddings WHERE note_id = ?1", [note_id])?;
        Ok(())
    }

    /// Reindex all notes. Progress reported via callback.
    pub fn reindex_all<F>(&self, progress_callback: F) -> Result<u32, SunderError>
    where
        F: Fn(u32, u32, &str),
    {
        if self
            .reindexing
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(SunderError::AlreadyRunning);
        }

        let result = self.do_reindex_all(&progress_callback);
        self.reindexing.store(false, Ordering::SeqCst);
        result
    }

    fn do_reindex_all<F>(&self, progress_callback: &F) -> Result<u32, SunderError>
    where
        F: Fn(u32, u32, &str),
    {
        let conn = self.db.get_read_conn()?;
        let mut stmt = conn.prepare("SELECT id, title, content FROM notes")?;
        let notes: Vec<(String, String, String)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(stmt);
        drop(conn);

        let total = notes.len() as u32;
        let mut indexed = 0u32;

        for (id, title, content) in &notes {
            if content.split_whitespace().count() < 3 {
                indexed += 1;
                continue;
            }

            self.index_note(id, content)?;
            indexed += 1;
            progress_callback(indexed, total, title);

            if indexed.is_multiple_of(10) {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }

        Ok(indexed)
    }
}

fn l2_normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

pub fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(embedding.len() * 4);
    for &v in embedding {
        blob.extend_from_slice(&v.to_le_bytes());
    }
    blob
}

pub fn blob_to_embedding(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}
