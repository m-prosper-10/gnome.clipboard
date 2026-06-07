//! Search helpers for the emoji engine.
//!
//! These helpers keep the query normalization and result shaping logic separate
//! from the engine event loop.

use super::{Emoji, EmojiDatabase};

pub fn normalize_query(query: &str) -> String {
    query.trim().to_lowercase()
}

pub fn matches_query(emoji: &Emoji, normalized_query: &str) -> bool {
    if normalized_query.is_empty() {
        return false;
    }

    let name = emoji.name.to_lowercase();
    let keywords_match = emoji
        .keywords
        .iter()
        .any(|k| k.to_lowercase().starts_with(normalized_query));

    name.starts_with(normalized_query) || keywords_match
}

pub fn expand_results(results: &mut Vec<Emoji>, emoji: &Emoji) {
    results.push(emoji.clone());
    for variant in &emoji.variants {
        let mut variant_emoji = emoji.clone();
        variant_emoji.char = variant.clone();
        variant_emoji.variants = Vec::new();
        results.push(variant_emoji);
    }
}

pub fn sort_by_recents(results: &mut [Emoji], recents: &[String]) {
    results.sort_by_key(|e| {
        recents
            .iter()
            .position(|r| r == &e.char)
            .unwrap_or(usize::MAX)
    });
}

pub fn search(database: &EmojiDatabase, query: &str, recents: &[String]) -> Vec<Emoji> {
    let normalized_query = normalize_query(query);
    if normalized_query.is_empty() {
        return Vec::new();
    }

    let mut results = Vec::new();
    for emoji in &database.emojis {
        if matches_query(emoji, &normalized_query) {
            expand_results(&mut results, emoji);
        }
    }

    sort_by_recents(&mut results, recents);
    results
}
