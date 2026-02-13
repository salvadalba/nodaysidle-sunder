use crate::db::DatabaseManager;
use crate::error::SunderError;
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Serialize, Clone)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    pub file_path: Option<String>,
    pub word_count: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct NoteListItem {
    pub id: String,
    pub title: String,
    pub snippet: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct NoteList {
    pub notes: Vec<NoteListItem>,
    pub total: u32,
}

pub struct NoteService {
    db: Arc<DatabaseManager>,
}

impl NoteService {
    pub fn new(db: Arc<DatabaseManager>) -> Self {
        Self { db }
    }

    pub fn create_note(
        &self,
        title: String,
        content: String,
        file_path: Option<String>,
    ) -> Result<Note, SunderError> {
        // Validate title
        let title = title.trim().to_string();
        if title.is_empty() {
            return Err(SunderError::ValidationError(
                "Title cannot be empty".to_string(),
            ));
        }
        if title.len() > 500 {
            return Err(SunderError::ValidationError(
                "Title must be 500 characters or fewer".to_string(),
            ));
        }

        // Validate content size (max 2MB)
        if content.len() > 2 * 1024 * 1024 {
            return Err(SunderError::ContentTooLarge(
                "Content exceeds 2MB limit".to_string(),
            ));
        }

        let id = uuid::Uuid::now_v7().to_string();
        let word_count = content.split_whitespace().count() as u32;
        let now = chrono::Utc::now().to_rfc3339();

        let conn = self.db.get_write_conn()?;
        conn.execute(
            "INSERT INTO notes (id, title, content, file_path, word_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![id, title, content, file_path, word_count, now, now],
        )?;

        Ok(Note {
            id,
            title,
            content,
            file_path,
            word_count,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn get_note(&self, id: &str) -> Result<Note, SunderError> {
        let conn = self.db.get_read_conn()?;
        conn.query_row(
            "SELECT id, title, content, file_path, word_count, created_at, updated_at
             FROM notes WHERE id = ?1",
            [id],
            |row| {
                Ok(Note {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    file_path: row.get(3)?,
                    word_count: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                SunderError::NotFound(format!("Note not found: {id}"))
            }
            _ => SunderError::from(e),
        })
    }

    pub fn get_note_by_file_path(&self, path: &str) -> Result<Option<Note>, SunderError> {
        let conn = self.db.get_read_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, title, content, file_path, word_count, created_at, updated_at
             FROM notes WHERE file_path = ?1",
        )?;

        let result = stmt
            .query_row([path], |row| {
                Ok(Note {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    file_path: row.get(3)?,
                    word_count: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .optional()?;

        Ok(result)
    }

    pub fn update_note(
        &self,
        id: &str,
        title: Option<String>,
        content: Option<String>,
    ) -> Result<Note, SunderError> {
        // Verify note exists
        let existing = self.get_note(id)?;

        let new_title = match title {
            Some(t) => {
                let t = t.trim().to_string();
                if t.is_empty() {
                    return Err(SunderError::ValidationError(
                        "Title cannot be empty".to_string(),
                    ));
                }
                if t.len() > 500 {
                    return Err(SunderError::ValidationError(
                        "Title must be 500 characters or fewer".to_string(),
                    ));
                }
                t
            }
            None => existing.title,
        };

        let new_content = match content {
            Some(c) => {
                if c.len() > 2 * 1024 * 1024 {
                    return Err(SunderError::ContentTooLarge(
                        "Content exceeds 2MB limit".to_string(),
                    ));
                }
                c
            }
            None => existing.content,
        };

        let word_count = new_content.split_whitespace().count() as u32;
        let now = chrono::Utc::now().to_rfc3339();

        let conn = self.db.get_write_conn()?;
        conn.execute(
            "UPDATE notes SET title = ?1, content = ?2, word_count = ?3, updated_at = ?4
             WHERE id = ?5",
            rusqlite::params![new_title, new_content, word_count, now, id],
        )?;

        Ok(Note {
            id: id.to_string(),
            title: new_title,
            content: new_content,
            file_path: existing.file_path,
            word_count,
            created_at: existing.created_at,
            updated_at: now,
        })
    }

    pub fn delete_note(&self, id: &str) -> Result<(), SunderError> {
        let note = self.get_note(id)?;

        let conn = self.db.get_write_conn()?;
        conn.execute("DELETE FROM notes WHERE id = ?1", [id])?;

        // Delete associated file if it exists
        if let Some(path) = &note.file_path {
            let path = std::path::Path::new(path);
            if path.exists() {
                std::fs::remove_file(path)?;
            }
        }

        Ok(())
    }

    pub fn list_notes(
        &self,
        offset: u32,
        limit: u32,
        sort_by: &str,
    ) -> Result<NoteList, SunderError> {
        let conn = self.db.get_read_conn()?;

        let order_clause = match sort_by {
            "created_at" => "created_at DESC",
            "title" => "title ASC",
            _ => "updated_at DESC",
        };

        let total: u32 =
            conn.query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))?;

        let query = format!(
            "SELECT id, title, content, updated_at FROM notes ORDER BY {order_clause} LIMIT ?1 OFFSET ?2"
        );

        let mut stmt = conn.prepare(&query)?;
        let notes = stmt
            .query_map(rusqlite::params![limit, offset], |row| {
                let content: String = row.get(2)?;
                let snippet = make_snippet(&content);
                Ok(NoteListItem {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    snippet,
                    updated_at: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(NoteList { notes, total })
    }
}

/// Create a snippet from content: first 200 chars with markdown stripped
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

// Needed for optional query results
trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
