use crate::db::DatabaseManager;
use crate::error::SunderError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub watch_directory: Option<String>,
    pub similarity_threshold: f64,
    pub debounce_ms: u32,
    pub theme: String,
}

#[derive(Debug, Deserialize)]
pub struct SettingsPatch {
    pub similarity_threshold: Option<f64>,
    pub debounce_ms: Option<u32>,
    pub theme: Option<String>,
}

pub struct SettingsService {
    db: Arc<DatabaseManager>,
}

impl SettingsService {
    pub fn new(db: Arc<DatabaseManager>) -> Self {
        Self { db }
    }

    pub fn get_settings(&self) -> Result<Settings, SunderError> {
        let conn = self.db.get_read_conn()?;

        let get_value = |key: &str, default: &str| -> String {
            conn.query_row(
                "SELECT value FROM settings WHERE key = ?1",
                [key],
                |row| row.get::<_, String>(0),
            )
            .unwrap_or_else(|_| default.to_string())
        };

        let watch_directory = {
            let v = get_value("watch_directory", "");
            if v.is_empty() {
                None
            } else {
                Some(v)
            }
        };

        let similarity_threshold: f64 = get_value("similarity_threshold", "0.65")
            .parse()
            .unwrap_or(0.65);

        let debounce_ms: u32 = get_value("debounce_ms", "300")
            .parse()
            .unwrap_or(300);

        let theme = get_value("theme", "dark");

        Ok(Settings {
            watch_directory,
            similarity_threshold,
            debounce_ms,
            theme,
        })
    }

    pub fn update_settings(&self, patch: SettingsPatch) -> Result<(), SunderError> {
        let conn = self.db.get_write_conn()?;

        if let Some(threshold) = patch.similarity_threshold {
            if !(0.0..=1.0).contains(&threshold) {
                return Err(SunderError::InvalidValue(
                    "similarity_threshold must be between 0.0 and 1.0".to_string(),
                ));
            }
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES ('similarity_threshold', ?1)",
                [threshold.to_string()],
            )?;
        }

        if let Some(debounce) = patch.debounce_ms {
            if !(100..=2000).contains(&debounce) {
                return Err(SunderError::InvalidValue(
                    "debounce_ms must be between 100 and 2000".to_string(),
                ));
            }
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES ('debounce_ms', ?1)",
                [debounce.to_string()],
            )?;
        }

        if let Some(theme) = &patch.theme {
            if theme != "dark" && theme != "light" {
                return Err(SunderError::InvalidValue(
                    "theme must be 'dark' or 'light'".to_string(),
                ));
            }
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES ('theme', ?1)",
                [theme.as_str()],
            )?;
        }

        Ok(())
    }
}
