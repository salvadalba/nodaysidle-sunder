pub mod migrations;

use crate::error::SunderError;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct DatabaseManager {
    read_pool: Pool<SqliteConnectionManager>,
    write_conn: Mutex<Connection>,
    db_path: PathBuf,
}

/// Register sqlite-vec as an auto-extension so every new connection loads it.
/// Must be called before any connections are opened.
#[allow(clippy::missing_transmute_annotations)]
fn load_sqlite_vec() {
    unsafe {
        rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
            sqlite_vec::sqlite3_vec_init as *const (),
        )));
    }
}

impl DatabaseManager {
    pub fn initialize(app_data_dir: &Path) -> Result<Self, SunderError> {
        std::fs::create_dir_all(app_data_dir)?;
        let db_path = app_data_dir.join("sunder.db");

        // Register sqlite-vec as an auto-extension before opening connections
        load_sqlite_vec();

        // Set up the write connection
        let write_conn = Connection::open(&db_path)?;
        write_conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;

        // Set up the read connection pool
        let manager = SqliteConnectionManager::file(&db_path);
        let read_pool = Pool::builder()
            .max_size(4)
            .build(manager)
            .map_err(|e| SunderError::DatabaseError(format!("Failed to create pool: {e}")))?;

        // Configure each read connection
        let test_conn = read_pool.get()?;
        test_conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;",
        )?;
        drop(test_conn);

        let db = Self {
            read_pool,
            write_conn: Mutex::new(write_conn),
            db_path,
        };

        // Run migrations
        db.run_migrations()?;

        Ok(db)
    }

    pub fn get_read_conn(
        &self,
    ) -> Result<r2d2::PooledConnection<SqliteConnectionManager>, SunderError> {
        Ok(self.read_pool.get()?)
    }

    pub fn get_write_conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, SunderError> {
        self.write_conn
            .lock()
            .map_err(|e| SunderError::Internal(format!("Write lock poisoned: {e}")))
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    fn run_migrations(&self) -> Result<u32, SunderError> {
        let conn = self.get_write_conn()?;
        migrations::run_all(&conn)
    }
}
