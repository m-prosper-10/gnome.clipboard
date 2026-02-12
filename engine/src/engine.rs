use zbus::{interface, fdo, object_server::SignalEmitter};
use zvariant::Value;
use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use log::{error, debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize, zvariant::Type)]
pub struct Emoji {
    pub char: String,
    pub name: String,
    pub keywords: Vec<String>,
    #[serde(default)]
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmojiDatabase {
    pub version: String,
    pub emojis: Vec<Emoji>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub trigger_char: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            trigger_char: ":".to_string(),
        }
    }
}

impl EmojiDatabase {
    pub fn search(&self, query: &str, recents: &[String]) -> Vec<Emoji> {
        if query.is_empty() {
            return Vec::new();
        }
        
        let mut results = Vec::new();
        for e in &self.emojis {
            if e.name.starts_with(query) || e.keywords.iter().any(|k| k.starts_with(query)) {
                results.push(e.clone());
                for v in &e.variants {
                    let mut ve = e.clone();
                    ve.char = v.clone();
                    ve.variants = vec![]; // No nested variants
                    results.push(ve);
                }
            }
        }

        // Sort by recents: emojis in recents list come first
        results.sort_by_key(|e| {
            recents.iter().position(|r| r == &e.char).unwrap_or(usize::MAX)
        });

        results
    }
}

pub struct EmojiEngine {
    // Composition buffer - what the user is currently typing
    pub buffer: String,
    // Whether the engine is currently active
    pub enabled: bool,
    // Emoji database
    pub database: EmojiDatabase,
    // Current selection index in results
    pub selected_index: usize,
    // Recently used emoji characters
    pub recents: Vec<String>,
    // Settings (trigger character, etc.)
    pub settings: Settings,
    /// Channel to forward UpdateResults to session bus for UI
    picker_tx: Option<Arc<tokio::sync::mpsc::Sender<(Vec<Emoji>, u32)>>>,
}

impl EmojiEngine {
    pub fn new() -> Self {
        let mut engine = EmojiEngine {
            buffer: String::new(),
            enabled: false,
            database: EmojiDatabase::default(),
            selected_index: 0,
            recents: Vec::new(),
            settings: Settings::default(),
            picker_tx: None,
        };
        engine.load_recents();
        engine.load_settings();
        engine
    }

    #[allow(dead_code)]
    pub fn with_database(database: EmojiDatabase) -> Self {
        Self::with_database_and_picker(database, None)
    }

    pub fn with_database_and_picker(
        database: EmojiDatabase,
        picker_tx: Option<Arc<tokio::sync::mpsc::Sender<(Vec<Emoji>, u32)>>>,
    ) -> Self {
        let mut engine = EmojiEngine {
            buffer: String::new(),
            enabled: false,
            database,
            selected_index: 0,
            recents: Vec::new(),
            settings: Settings::default(),
            picker_tx,
        };
        engine.load_recents();
        engine.load_settings();
        engine
    }

    fn get_config_path() -> Option<std::path::PathBuf> {
        let home = std::env::var("HOME").ok()?;
        let path = std::path::PathBuf::from(home)
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

    fn get_recents_path() -> Option<std::path::PathBuf> {
        let home = std::env::var("HOME").ok()?;
        let path = std::path::PathBuf::from(home)
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
                    Ok(content) => {
                        match serde_json::from_str::<Vec<String>>(&content) {
                            Ok(recents) => self.recents = recents,
                            Err(e) => warn!("Failed to parse recents at {:?}: {}. Starting fresh.", path, e),
                        }
                    }
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
        // Remove if already present
        self.recents.retain(|c| c != &char);
        // Add to front
        self.recents.insert(0, char);
        // Keep only top 20
        self.recents.truncate(20);
        self.save_recents();
    }
    
    /// Processes a key event and returns an optional string to commit.
    /// Returns (handled, commit_text)
    pub fn internal_process_key_event(&mut self, keyval: u32, _keycode: u32, _state: u32) -> (bool, Option<String>) {
        if !self.enabled {
            return (false, None);
        }

        let trigger = self.settings.trigger_char.chars().next().unwrap_or(':');

        const SUPER_MASK: u32 = 1 << 26;
        const RELEASE_MASK: u32 = 1 << 30;

        // Super + ; (Semicolon is 0x3b)
        if (_state & SUPER_MASK) != 0 && (_state & RELEASE_MASK) == 0 && keyval == 0x3b {
            if self.buffer.is_empty() {
                self.buffer.push(trigger);
            }
            return (true, None);
        }

        // Check if it's a printable character (rough check for ASCII/Basic Latin)
        if (0x20..=0x7E).contains(&keyval) {
            let c = (keyval as u8) as char;
            
            if c == trigger && self.buffer.is_empty() {
                self.buffer.push(c);
                return (true, None);
            }
            
            if !self.buffer.is_empty() {
                self.buffer.push(c);
                return (true, None);
            }
            
            return (false, None);
        }

        // Handle Escape, Backspace, Enter
        match keyval {
            0xff1b => { // Esc
                self.internal_reset();
                (true, None)
            }
            0xff08 => { // Backspace
                if !self.buffer.is_empty() {
                    self.buffer.pop();
                    if self.buffer.is_empty() {
                        self.internal_reset();
                    }
                    (true, None)
                } else {
                    (false, None)
                }
            }
            0xff0d => { // Enter
                if !self.buffer.is_empty() && self.buffer.starts_with(trigger) {
                    let query = self.buffer.trim_start_matches(trigger);
                    let results = self.database.search(query, &self.recents);
                    if let Some(emoji) = results.get(self.selected_index) {
                        let text = emoji.char.clone();
                        self.record_usage(text.clone());
                        self.internal_reset();
                        return (true, Some(text));
                    }
                }
                (false, None)
            }
            0xff52 => { // Arrow Up
                if !self.buffer.is_empty() && self.buffer.starts_with(trigger) {
                    let query = self.buffer.trim_start_matches(trigger);
                    let count = self.database.search(query, &self.recents).len();
                    if count > 0 {
                        self.selected_index = (self.selected_index + count - 1) % count;
                        return (true, None);
                    }
                }
                (false, None)
            }
            0xff54 => { // Arrow Down
                if !self.buffer.is_empty() && self.buffer.starts_with(trigger) {
                    let query = self.buffer.trim_start_matches(trigger);
                    let count = self.database.search(query, &self.recents).len();
                    if count > 0 {
                        self.selected_index = (self.selected_index + 1) % count;
                        return (true, None);
                    }
                }
                (false, None)
            }
            _ => {
                if !self.buffer.is_empty() {
                    self.internal_reset();
                }
                (false, None)
            }
        }
    }
    
    pub fn internal_reset(&mut self) {
        self.buffer.clear();
    }
    
    pub fn internal_enable(&mut self) {
        self.enabled = true;
        self.internal_reset();
    }
    
    pub fn internal_disable(&mut self) {
        self.enabled = false;
        self.internal_reset();
    }
}

#[interface(name = "org.freedesktop.IBus.Engine")]
impl EmojiEngine {
    async fn process_key_event(
        &mut self,
        #[zbus(signal_emitter)] se: SignalEmitter<'_>,
        keyval: u32,
        _keycode: u32,
        state: u32,
    ) -> fdo::Result<bool> {
        // state & (1 << 30) is key release in IBus
        if (state & (1 << 30)) != 0 {
            return Ok(false);
        }

        let (handled, commit) = self.internal_process_key_event(keyval, 0, 0);
        
        // Update preedit text always if we are in composition
        let visible = !self.buffer.is_empty();
        let _ = self.emit_update_preedit_text(&se, self.buffer.clone(), self.buffer.len() as u32, visible).await;

        let trigger = self.settings.trigger_char.chars().next().unwrap_or(':');

        // Emit search results if in composition (or empty to hide popup)
        let (emojis, selected) = if visible && self.buffer.starts_with(trigger) {
            let query = self.buffer.trim_start_matches(trigger);
            let emojis = self.database.search(query, &self.recents);
            let count = emojis.len();
            if self.selected_index >= count && count > 0 {
                self.selected_index = 0;
            }
            (emojis, self.selected_index as u32)
        } else {
            (Vec::new(), 0)
        };
        if visible && self.buffer.starts_with(trigger) {
            let _ = self.emit_update_results(&se, emojis.clone(), selected).await;
        }
        // Forward to session bus for UI popup (IBus bus may not be visible to UI)
        if let Some(ref tx) = self.picker_tx {
            let _ = tx.try_send((emojis, selected));
        }

        if let Some(text) = commit {
            let _ = self.emit_commit_text(&se, text).await;
        }
        
        Ok(handled)
    }

    async fn enable(&mut self) -> fdo::Result<()> {
        self.internal_enable();
        Ok(())
    }

    async fn disable(&mut self) -> fdo::Result<()> {
        self.internal_disable();
        Ok(())
    }

    async fn reset(&mut self) -> fdo::Result<()> {
        self.internal_reset();
        Ok(())
    }

    /// Commit emoji from UI (e.g. mouse click). Called via D-Bus from session bus.
    async fn commit_emoji(
        &mut self,
        #[zbus(signal_emitter)] se: SignalEmitter<'_>,
        text: String,
    ) -> fdo::Result<()> {
        if !text.is_empty() {
            self.record_usage(text.clone());
            let _ = self.emit_commit_text(&se, text).await;
        }
        self.internal_reset();
        // Notify UI to hide popup
        if let Some(ref tx) = self.picker_tx {
            let _ = tx.try_send((Vec::new(), 0));
        }
        Ok(())
    }

    #[zbus(signal, name = "CommitText")]
    async fn commit_text_signal(se: &SignalEmitter<'_>, text: Value<'_>) -> zbus::Result<()>;

    #[zbus(signal, name = "UpdatePreeditText")]
    async fn update_preedit_text_signal(
        se: &SignalEmitter<'_>,
        text: Value<'_>,
        cursor_pos: u32,
        visible: bool,
    ) -> zbus::Result<()>;
    #[zbus(signal, name = "UpdateResults")]
    async fn update_results_signal(se: &SignalEmitter<'_>, results: Vec<Emoji>, selected_index: u32) -> zbus::Result<()>;
}

impl EmojiEngine {
    async fn emit_commit_text(&self, se: &SignalEmitter<'_>, text: String) -> zbus::Result<()> {
        let ibus_text = (text, Vec::<Value>::new(), HashMap::<String, Value>::new());
        let variant = Value::from(ibus_text);
        Self::commit_text_signal(se, variant.into()).await
    }

    async fn emit_update_preedit_text(&self, se: &SignalEmitter<'_>, text: String, cursor_pos: u32, visible: bool) -> zbus::Result<()> {
        let ibus_text = (text, Vec::<Value>::new(), HashMap::<String, Value>::new());
        let variant = Value::from(ibus_text);
        Self::update_preedit_text_signal(se, variant.into(), cursor_pos, visible).await
    }

    async fn emit_update_results(&self, se: &SignalEmitter<'_>, results: Vec<Emoji>, selected_index: u32) -> zbus::Result<()> {
        Self::update_results_signal(se, results, selected_index).await
    }
}

impl Default for EmojiEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_engine_creation() {
        let engine = EmojiEngine::new();
        assert_eq!(engine.buffer, "");
        assert_eq!(engine.enabled, false);
    }
    
    #[test]
    fn test_trigger_logic() {
        let db = EmojiDatabase {
            version: "test".to_string(),
            emojis: vec![
                Emoji { char: "🙂".to_string(), name: "smile".to_string(), keywords: vec![], variants: vec![] },
                Emoji { char: "❤️".to_string(), name: "heart".to_string(), keywords: vec![], variants: vec![] },
            ],
        };
        let mut engine = EmojiEngine::with_database(db);
        engine.internal_enable();
        
        // Type ':'
        let (handled, commit) = engine.internal_process_key_event(0x3a, 0, 0);
        assert!(handled);
        assert_eq!(commit, None);
        assert_eq!(engine.buffer, ":");
        
        // Type 's'
        let (handled, commit) = engine.internal_process_key_event(0x73, 0, 0);
        assert!(handled);
        assert_eq!(commit, None);
        assert_eq!(engine.buffer, ":s");
        
        // Press Enter
        let (handled, commit) = engine.internal_process_key_event(0xff0d, 0, 0);
        assert!(handled);
        assert_eq!(commit, Some("🙂".to_string()));
        assert_eq!(engine.buffer, "");
    }
    
    #[test]
    fn test_search_logic() {
        let db = EmojiDatabase {
            version: "test".to_string(),
            emojis: vec![
                Emoji { char: "🙂".to_string(), name: "smile".to_string(), keywords: vec![], variants: vec![] },
                Emoji { char: "😊".to_string(), name: "blush".to_string(), keywords: vec!["happy".to_string()], variants: vec![] },
            ],
        };
        
        assert_eq!(db.search("smi", &[]).len(), 1);
        assert_eq!(db.search("smi", &[])[0].char, "🙂");
        
        // Search by keyword
        assert_eq!(db.search("hap", &[]).len(), 1);
        assert_eq!(db.search("hap", &[])[0].char, "😊");
        
        // No match
        assert_eq!(db.search("xyz", &[]).len(), 0);
    }

    #[test]
    fn test_variant_expansion() {
        let db = EmojiDatabase {
            version: "test".to_string(),
            emojis: vec![
                Emoji { 
                    char: "👍".to_string(), 
                    name: "thumbsup".to_string(), 
                    keywords: vec![], 
                    variants: vec!["👍🏻".to_string(), "👍🏼".to_string()] 
                },
            ],
        };

        let results = db.search("thumb", &[]);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].char, "👍");
        assert_eq!(results[1].char, "👍🏻");
        assert_eq!(results[2].char, "👍🏼");
    }

    #[test]
    fn test_recents_prioritization() {
        let db = EmojiDatabase {
            version: "test".to_string(),
            emojis: vec![
                Emoji { char: "🙂".to_string(), name: "smile1".to_string(), keywords: vec![], variants: vec![] },
                Emoji { char: "😊".to_string(), name: "smile2".to_string(), keywords: vec![], variants: vec![] },
                Emoji { char: "😄".to_string(), name: "smile3".to_string(), keywords: vec![], variants: vec![] },
            ],
        };

        // Initially sorted by database order
        let results = db.search("smile", &[]);
        assert_eq!(results[0].char, "🙂");
        assert_eq!(results[1].char, "😊");

        // Prioritize smile2
        let results = db.search("smile", &["😊".to_string()]);
        assert_eq!(results[0].char, "😊");
        assert_eq!(results[1].char, "🙂");
        assert_eq!(results[2].char, "😄");

        // Prioritize smile3 then smile2
        let results = db.search("smile", &["😄".to_string(), "😊".to_string()]);
        assert_eq!(results[0].char, "😄");
        assert_eq!(results[1].char, "😊");
        assert_eq!(results[2].char, "🙂");
    }

    #[test]
    fn test_recents_recording() {
        let mut engine = EmojiEngine::new();
        engine.recents = vec![];
        
        engine.record_usage("👍".to_string());
        assert_eq!(engine.recents, vec!["👍".to_string()]);

        engine.record_usage("❤️".to_string());
        assert_eq!(engine.recents, vec!["❤️".to_string(), "👍".to_string()]);

        // Move to front if repeated
        engine.record_usage("👍".to_string());
        assert_eq!(engine.recents, vec!["👍".to_string(), "❤️".to_string()]);
    }
}
