//! Search helpers for the emoji engine.
//!
//! These helpers keep the query normalization and result shaping logic separate
//! from the engine event loop.

use super::{Emoji, EmojiDatabase, RecentEmoji};
use std::collections::HashMap;

pub fn normalize_query(query: &str) -> String {
    query.trim().to_lowercase()
}

/// Returns match priority tier (lower is better), or `None` if no match.
fn match_tier(emoji: &Emoji, normalized_query: &str) -> Option<u8> {
    if normalized_query.is_empty() {
        return Some(0);
    }

    let name = emoji.name.to_lowercase();
    if name == normalized_query {
        return Some(0);
    }
    if name.starts_with(normalized_query) {
        return Some(1);
    }

    let mut kw_prefix = false;
    let mut kw_contains = false;
    for kw in &emoji.keywords {
        let lower = kw.to_lowercase();
        if lower == normalized_query || lower.starts_with(normalized_query) {
            kw_prefix = true;
            break;
        }
        if lower.contains(normalized_query) {
            kw_contains = true;
        }
    }

    if kw_prefix {
        return Some(2);
    }
    if name.contains(normalized_query) || kw_contains {
        return Some(3);
    }

    None
}

#[allow(dead_code)]
pub fn matches_query(emoji: &Emoji, normalized_query: &str) -> bool {
    if normalized_query.is_empty() {
        return false;
    }
    match_tier(emoji, normalized_query).is_some()
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

/// Returns the `Emoji` entries for the user's recently used emojis, ordered by most recently used.
pub fn recent_emojis(database: &EmojiDatabase, recents: &[RecentEmoji]) -> Vec<Emoji> {
    let mut ordered: Vec<&RecentEmoji> = recents.iter().collect();
    ordered.sort_by(|a, b| b.last_used.cmp(&a.last_used).then_with(|| b.count.cmp(&a.count)));

    ordered
        .into_iter()
        .filter_map(|recent| {
            database
                .emojis
                .iter()
                .find(|emoji| emoji.char == recent.char)
                .cloned()
        })
        .collect()
}

pub fn search(database: &EmojiDatabase, query: &str, recents: &[RecentEmoji]) -> Vec<Emoji> {
    let normalized_query = normalize_query(query);
    if normalized_query.is_empty() {
        let mut results = database.emojis.clone();
        sort_by_recents(&mut results, recents);
        return results;
    }

    let ranking: HashMap<&str, (u32, u64)> = recents
        .iter()
        .map(|recent| (recent.char.as_str(), (recent.count, recent.last_used)))
        .collect();

    let mut tiered: Vec<(u8, Emoji)> = database
        .emojis
        .iter()
        .filter_map(|emoji| {
            match_tier(emoji, &normalized_query).map(|tier| (tier, emoji.clone()))
        })
        .collect();

    tiered.sort_by(|(tier_a, a), (tier_b, b)| {
        tier_a.cmp(tier_b).then_with(|| {
            match (ranking.get(a.char.as_str()), ranking.get(b.char.as_str())) {
                (Some(left), Some(right)) => right.cmp(left),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        })
    });

    tiered.into_iter().map(|(_, emoji)| emoji).collect()
}
