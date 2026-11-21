//! Database and persistence helpers for the emoji engine.
//!
//! The concrete `Emoji`, `EmojiDatabase`, `EmojiEngine`, and `Settings` types
//! currently live in `engine.rs`; this module owns the data-layer behavior so
//! the engine event loop can stay focused on IBus state transitions.

use super::{search, EmojiEngine};
use gio::prelude::*;
use log::{debug, error, warn};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::UNIX_EPOCH;

pub use super::{Emoji, EmojiDatabase, RecentEmoji, Settings};

const SETTINGS_SCHEMA_ID: &str = "org.example.EmojiInput";
const SETTINGS_TRIGGER_CHAR: &str = "trigger-char";
static MISSING_SETTINGS_SCHEMA_WARNED: AtomicBool = AtomicBool::new(false);

fn settings_available() -> bool {
    gio::SettingsSchemaSource::default()
        .and_then(|source| source.lookup(SETTINGS_SCHEMA_ID, true))
        .is_some()
}

fn file_mtime(path: &Path) -> Option<u64> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    modified.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs())
}

fn write_atomic(path: &Path, content: &str) -> std::io::Result<()> {
    let tmp_path = path.with_extension("tmp");
    let mut file = std::fs::File::create(&tmp_path)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?;
    std::fs::rename(tmp_path, path)
}

impl EmojiDatabase {
    pub fn search(&self, query: &str, recents: &[RecentEmoji]) -> Vec<Emoji> {
        search::search(self, query, recents)
    }

    pub fn load_from_source_with_cache(source_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let source = Path::new(source_path);
        let source_exists = source.exists();
        let source_mtime = file_mtime(source).unwrap_or(0);

        if let Some(cache_path) = Self::get_cache_path() {
            if cache_path.exists() {
                match std::fs::read_to_string(&cache_path) {
                    Ok(content) => match serde_json::from_str::<EmojiDatabaseCache>(&content) {
                        Ok(cache) if !source_exists || cache.source_mtime == source_mtime => {
                            debug!("Loaded emoji database from cache {:?}", cache_path);
                            return Ok(cache.database);
                        }
                        Ok(_) => {
                            debug!("Emoji database cache {:?} is stale, rebuilding.", cache_path);
                        }
                        Err(e) => {
                            warn!("Failed to parse emoji cache at {:?}: {}. Rebuilding.", cache_path, e);
                        }
                    },
                    Err(e) => {
                        warn!("Failed to read emoji cache at {:?}: {}. Rebuilding.", cache_path, e);
                    }
                }
            }
        }

        let content = std::fs::read_to_string(source)?;
        let database: EmojiDatabase = serde_json::from_str(&content)?;
        Self::save_cache(&database, source_mtime);
        Ok(database)
    }

    fn get_cache_path() -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        let path = PathBuf::from(home)
            .join(".cache")
            .join("gnome-emoji-input")
            .join("emoji-db-cache.json");

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        Some(path)
    }

    fn save_cache(database: &EmojiDatabase, source_mtime: u64) {
        if let Some(cache_path) = Self::get_cache_path() {
            let cache = EmojiDatabaseCache {
                source_mtime,
                database: database.clone(),
            };
            match serde_json::to_string(&cache) {
                Ok(content) => {
                    if let Err(e) = write_atomic(&cache_path, &content) {
                        warn!("Failed to save emoji cache to {:?}: {}", cache_path, e);
                    }
                }
                Err(e) => warn!("Failed to serialize emoji cache: {}", e),
            }
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct EmojiDatabaseCache {
    source_mtime: u64,
    database: EmojiDatabase,
}

impl EmojiEngine {
    pub fn load_settings(&mut self) {
        self.settings = Settings::default();
        if !settings_available() {
            if !MISSING_SETTINGS_SCHEMA_WARNED.swap(true, Ordering::Relaxed) {
                warn!(
                    "GSettings schema '{}' is unavailable; using default settings",
                    SETTINGS_SCHEMA_ID
                );
            }
            return;
        }

        let settings = gio::Settings::new(SETTINGS_SCHEMA_ID);
        let trigger_char = settings.string(SETTINGS_TRIGGER_CHAR).to_string();
        if !trigger_char.is_empty() {
            self.settings.trigger_char = trigger_char;
            debug!("Loaded trigger char from GSettings");
        }
    }

    fn get_recents_path() -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        let path = PathBuf::from(home)
            .join(".cache")
            .join("gnome-emoji-input")
            .join("recents.json");

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        Some(path)
    }

    pub fn load_recents(&mut self) {
        if let Some(path) = Self::get_recents_path() {
            if path.exists() {
                debug!("Loading recents from {:?}", path);
                match std::fs::read_to_string(&path) {
                    Ok(content) => match serde_json::from_str::<Vec<RecentEmoji>>(&content) {
                        Ok(recents) => {
                            self.recent_tick = recents.iter().map(|entry| entry.last_used).max().unwrap_or(0);
                            self.recents = recents;
                        }
                        Err(_) => match serde_json::from_str::<Vec<String>>(&content) {
                            Ok(recents) => {
                                self.recent_tick = recents.len() as u64;
                                self.recents = recents
                                    .into_iter()
                                    .enumerate()
                                    .map(|(index, char)| RecentEmoji {
                                        char,
                                        count: 1,
                                        last_used: (self.recent_tick.saturating_sub(index as u64)),
                                    })
                                    .collect();
                            }
                            Err(e) => warn!("Failed to parse recents at {:?}: {}. Starting fresh.", path, e),
                        },
                    },
                    Err(e) => error!("Failed to read recents file at {:?}: {}", path, e),
                }
            }
        }
    }

    pub fn save_recents(&self) {
        if let Some(path) = Self::get_recents_path() {
            match serde_json::to_string(&self.recents) {
                Ok(content) => {
                    if let Err(e) = write_atomic(&path, &content) {
                        error!("Failed to save recents to {:?}: {}", path, e);
                    }
                }
                Err(e) => error!("Failed to serialize recents: {}", e),
            }
        }
    }

    pub fn record_usage(&mut self, char: String) {
        let canonical = self.canonical_usage_char(&char);
        self.recent_tick = self.recent_tick.saturating_add(1);

        if let Some(entry) = self.recents.iter_mut().find(|entry| entry.char == canonical) {
            entry.count = entry.count.saturating_add(1);
            entry.last_used = self.recent_tick;
        } else {
            self.recents.push(RecentEmoji {
                char: canonical,
                count: 1,
                last_used: self.recent_tick,
            });
        }

        self.recents.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then_with(|| b.last_used.cmp(&a.last_used))
        });
        self.recents.truncate(20);
        self.save_recents();
    }
}
