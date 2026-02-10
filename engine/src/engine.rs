use zbus::{interface, fdo, object_server::SignalEmitter};
use zvariant::Value;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, zvariant::Type)]
pub struct Emoji {
    pub char: String,
    pub name: String,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmojiDatabase {
    pub version: String,
    pub emojis: Vec<Emoji>,
}

impl EmojiDatabase {
    pub fn search(&self, query: &str) -> Vec<&Emoji> {
        if query.is_empty() {
            return Vec::new();
        }
        
        self.emojis.iter()
            .filter(|e| {
                e.name.starts_with(query) || 
                e.keywords.iter().any(|k| k.starts_with(query))
            })
            .collect()
    }
}

pub struct EmojiEngine {
    // Composition buffer - what the user is currently typing
    pub buffer: String,
    // Whether the engine is currently active
    pub enabled: bool,
    // Emoji database
    pub database: EmojiDatabase,
}

impl EmojiEngine {
    pub fn new() -> Self {
        EmojiEngine {
            buffer: String::new(),
            enabled: false,
            database: EmojiDatabase::default(),
        }
    }
    
    pub fn with_database(database: EmojiDatabase) -> Self {
        EmojiEngine {
            buffer: String::new(),
            enabled: false,
            database,
        }
    }
    
    /// Processes a key event and returns an optional string to commit.
    /// Returns (handled, commit_text)
    pub fn internal_process_key_event(&mut self, keyval: u32, _keycode: u32, _state: u32) -> (bool, Option<String>) {
        if !self.enabled {
            return (false, None);
        }

        // Check if it's a printable character (rough check for ASCII/Basic Latin)
        if (0x20..=0x7E).contains(&keyval) {
            let c = (keyval as u8) as char;
            
            if c == ':' && self.buffer.is_empty() {
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
                if !self.buffer.is_empty() && self.buffer.starts_with(':') {
                    let query = self.buffer.trim_start_matches(':');
                    let results = self.database.search(query);
                    if let Some(emoji) = results.first() {
                        let text = emoji.char.clone();
                        self.internal_reset();
                        return (true, Some(text));
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

        // Emit search results if in composition
        if visible && self.buffer.starts_with(':') {
            let query = self.buffer.trim_start_matches(':');
            let results = self.database.search(query);
            let emojis: Vec<Emoji> = results.into_iter().cloned().collect();
            let _ = self.emit_update_results(&se, emojis).await;
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
    async fn update_results_signal(se: &SignalEmitter<'_>, results: Vec<Emoji>) -> zbus::Result<()>;
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

    async fn emit_update_results(&self, se: &SignalEmitter<'_>, results: Vec<Emoji>) -> zbus::Result<()> {
        Self::update_results_signal(se, results).await
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
                Emoji { char: "🙂".to_string(), name: "smile".to_string(), keywords: vec![] },
                Emoji { char: "❤️".to_string(), name: "heart".to_string(), keywords: vec![] },
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
                Emoji { char: "🙂".to_string(), name: "smile".to_string(), keywords: vec![] },
                Emoji { char: "😊".to_string(), name: "blush".to_string(), keywords: vec!["happy".to_string()] },
            ],
        };
        
        assert_eq!(db.search("smi").len(), 1);
        assert_eq!(db.search("smi")[0].char, "🙂");
        
        // Search by keyword
        assert_eq!(db.search("hap").len(), 1);
        assert_eq!(db.search("hap")[0].char, "😊");
        
        // No match
        assert_eq!(db.search("xyz").len(), 0);
    }
}
