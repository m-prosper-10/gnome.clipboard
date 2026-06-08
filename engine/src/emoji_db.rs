//! Database and persistence helpers for the emoji engine.
//!
//! The concrete `Emoji`, `EmojiDatabase`, `EmojiEngine`, and `Settings` types
//! currently live in `engine.rs`; this module owns the data-layer behavior so
//! the engine event loop can stay focused on IBus state transitions.

use super::{search, EmojiEngine};
use log::{debug, error, warn};
use std::path::PathBuf;

pub use super::{Emoji, EmojiDatabase, RecentEmoji, Settings};

impl EmojiDatabase {
    pub fn search(&self, query: &str, recents: &[RecentEmoji]) -> Vec<Emoji> {
        search::search(self, query, recents)
    }
}

impl EmojiEngine {
    fn get_config_path() -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        let path = PathBuf::from(home)
            .join(".config")
            .join("gnome-emoji-input")
            .join("settings.json");
        Some(path)
    }

    pub fn load_settings(&mut self) {
        if let Some(path) = Self::get_config_path() {
            if path.exists() {
                debug!("Loading settings from {:?}", path);
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        match serde_json::from_str::<Settings>(&content) {
                            Ok(settings) => self.settings = settings,
                            Err(e) => warn!("Failed to parse settings at {:?}: {}. Using defaults.", path, e),
                        }
                    }
                    Err(e) => error!("Failed to read settings file at {:?}: {}", path, e),
                }
            } else {
                debug!("Settings file {:?} not found, using defaults.", path);
            }
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
                    if let Err(e) = std::fs::write(&path, content) {
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
