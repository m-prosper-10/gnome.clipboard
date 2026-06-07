//! Compatibility helpers for the emoji database layer.
//!
//! The concrete `Emoji`, `EmojiDatabase`, and `Settings` types currently live in
//! `engine.rs`. This module keeps the phase 3 data layer visible and provides
//! a natural landing spot for future database loading helpers.

pub use super::{Emoji, EmojiDatabase, Settings};

pub fn default_settings() -> Settings {
    Settings::default()
}
