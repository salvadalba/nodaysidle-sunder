use serde::Serialize;
use std::fmt;

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "kind", content = "message")]
pub enum SunderError {
    NotFound(String),
    AlreadyExists(String),
    ValidationError(String),
    DatabaseError(String),
    EmbeddingError(String),
    IoError(String),
    Internal(String),
    ContentTooLarge(String),
    EmptyQuery,
    ContentTooShort(String),
    AlreadyRunning,
    DirectoryNotFound(String),
    NotADirectory(String),
    PermissionDenied(String),
    InvalidValue(String),
}

impl fmt::Display for SunderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SunderError::NotFound(msg) => write!(f, "Not found: {msg}"),
            SunderError::AlreadyExists(msg) => write!(f, "Already exists: {msg}"),
            SunderError::ValidationError(msg) => write!(f, "Validation error: {msg}"),
            SunderError::DatabaseError(msg) => write!(f, "Database error: {msg}"),
            SunderError::EmbeddingError(msg) => write!(f, "Embedding error: {msg}"),
            SunderError::IoError(msg) => write!(f, "IO error: {msg}"),
            SunderError::Internal(msg) => write!(f, "Internal error: {msg}"),
            SunderError::ContentTooLarge(msg) => write!(f, "Content too large: {msg}"),
            SunderError::EmptyQuery => write!(f, "Query cannot be empty"),
            SunderError::ContentTooShort(msg) => write!(f, "Content too short: {msg}"),
            SunderError::AlreadyRunning => write!(f, "Operation already running"),
            SunderError::DirectoryNotFound(msg) => write!(f, "Directory not found: {msg}"),
            SunderError::NotADirectory(msg) => write!(f, "Not a directory: {msg}"),
            SunderError::PermissionDenied(msg) => write!(f, "Permission denied: {msg}"),
            SunderError::InvalidValue(msg) => write!(f, "Invalid value: {msg}"),
        }
    }
}

impl std::error::Error for SunderError {}

impl From<rusqlite::Error> for SunderError {
    fn from(err: rusqlite::Error) -> Self {
        SunderError::DatabaseError(err.to_string())
    }
}

impl From<std::io::Error> for SunderError {
    fn from(err: std::io::Error) -> Self {
        SunderError::IoError(err.to_string())
    }
}

impl From<r2d2::Error> for SunderError {
    fn from(err: r2d2::Error) -> Self {
        SunderError::DatabaseError(format!("Connection pool error: {err}"))
    }
}
