//! Compatibility search helpers for the emoji engine.
//!
//! The core search implementation currently lives on `EmojiDatabase` in
//! `engine.rs`, but this module gives us a stable place to grow search-specific
//! helpers as Phase 3 is modularized.

use super::{Emoji, EmojiDatabase};

pub fn search(database: &EmojiDatabase, query: &str, recents: &[String]) -> Vec<Emoji> {
    database.search(query, recents)
}
