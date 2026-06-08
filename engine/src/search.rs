//! Search helpers for the emoji engine.
//!
//! These helpers keep the query normalization and result shaping logic separate
//! from the engine event loop.

use super::{Emoji, EmojiDatabase, RecentEmoji};
use std::collections::HashMap;

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

pub fn sort_by_recents(results: &mut [Emoji], recents: &[RecentEmoji]) {
    let ranking: HashMap<&str, (u32, u64)> = recents
        .iter()
        .map(|recent| (recent.char.as_str(), (recent.count, recent.last_used)))
        .collect();

    results.sort_by(|a, b| match (ranking.get(a.char.as_str()), ranking.get(b.char.as_str())) {
        (Some(left), Some(right)) => right.cmp(left),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
}

pub fn search(database: &EmojiDatabase, query: &str, recents: &[RecentEmoji]) -> Vec<Emoji> {
    let normalized_query = normalize_query(query);
    if normalized_query.is_empty() {
        return Vec::new();
    }

    let mut results = Vec::new();
    for emoji in &database.emojis {
        if matches_query(emoji, &normalized_query) {
            results.push(emoji.clone());
        }
    }

    sort_by_recents(&mut results, recents);
    results
}
