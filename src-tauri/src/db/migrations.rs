use crate::error::SunderError;
use rusqlite::Connection;

struct Migration {
    version: u32,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: "
            CREATE TABLE IF NOT EXISTS notes (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                file_path TEXT UNIQUE,
                word_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_notes_updated_at ON notes(updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_notes_file_path ON notes(file_path);
        ",
    },
    Migration {
        version: 2,
        sql: "
            CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
                title,
                content,
                content=notes,
                content_rowid=rowid,
                tokenize='unicode61'
            );

            CREATE TRIGGER IF NOT EXISTS notes_ai AFTER INSERT ON notes BEGIN
                INSERT INTO notes_fts(rowid, title, content) VALUES (new.rowid, new.title, new.content);
            END;

            CREATE TRIGGER IF NOT EXISTS notes_ad AFTER DELETE ON notes BEGIN
                INSERT INTO notes_fts(notes_fts, rowid, title, content) VALUES('delete', old.rowid, old.title, old.content);
            END;

            CREATE TRIGGER IF NOT EXISTS notes_au AFTER UPDATE ON notes BEGIN
                INSERT INTO notes_fts(notes_fts, rowid, title, content) VALUES('delete', old.rowid, old.title, old.content);
                INSERT INTO notes_fts(rowid, title, content) VALUES (new.rowid, new.title, new.content);
            END;
        ",
    },
    Migration {
        version: 3,
        sql: "
            CREATE TABLE IF NOT EXISTS embeddings (
                note_id TEXT PRIMARY KEY REFERENCES notes(id) ON DELETE CASCADE,
                vector BLOB NOT NULL,
                model_version TEXT NOT NULL DEFAULT 'minilm-v2-q8',
                updated_at TEXT NOT NULL
            );
        ",
    },
    Migration {
        version: 4,
        sql: "
            CREATE VIRTUAL TABLE IF NOT EXISTS vec_embeddings USING vec0(
                note_id TEXT PRIMARY KEY,
                embedding float[384]
            );
        ",
    },
    Migration {
        version: 5,
        sql: "
            CREATE TABLE IF NOT EXISTS similarity_cache (
                note_id_a TEXT NOT NULL,
                note_id_b TEXT NOT NULL,
                similarity REAL NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (note_id_a, note_id_b),
                CHECK (note_id_a < note_id_b)
            );

            CREATE INDEX IF NOT EXISTS idx_similarity_cache_a ON similarity_cache(note_id_a);
            CREATE INDEX IF NOT EXISTS idx_similarity_cache_b ON similarity_cache(note_id_b);

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            INSERT OR IGNORE INTO settings (key, value) VALUES ('similarity_threshold', '0.65');
            INSERT OR IGNORE INTO settings (key, value) VALUES ('debounce_ms', '300');
            INSERT OR IGNORE INTO settings (key, value) VALUES ('theme', 'dark');
        ",
    },
];

pub fn run_all(conn: &Connection) -> Result<u32, SunderError> {
    // Create migrations tracking table
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        );",
    )?;

    let mut applied: u32 = 0;

    for migration in MIGRATIONS {
        let already_applied: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM migrations WHERE version = ?1",
                [migration.version],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !already_applied {
            conn.execute_batch(migration.sql)?;
            conn.execute(
                "INSERT INTO migrations (version, applied_at) VALUES (?1, datetime('now'))",
                [migration.version],
            )?;
            applied += 1;
            tracing::info!("Applied migration v{}", migration.version);
        }
    }

    if applied > 0 {
        tracing::info!("Applied {applied} migration(s)");
    }

    Ok(applied)
}
