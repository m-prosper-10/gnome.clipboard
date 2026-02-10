use zbus::{interface, fdo, object_server::SignalEmitter};
use zvariant::Value;
use std::collections::HashMap;

pub struct EmojiEngine {
    // Composition buffer - what the user is currently typing
    pub buffer: String,
    // Whether the engine is currently active
    pub enabled: bool,
}

impl EmojiEngine {
    pub fn new() -> Self {
        EmojiEngine {
            buffer: String::new(),
            enabled: false,
        }
    }
    
    /// Processes a key event and returns an optional string to commit.
    /// Returns (handled, commit_text)
    pub fn internal_process_key_event(&mut self, keyval: u32, _keycode: u32, _state: u32) -> (bool, Option<String>) {
        if !self.enabled {
            return (false, None);
        }

        // Check if it's a printable character (rough check for ASCII/Basic Latin)
        // IBus keyvals for printable characters are usually their ASCII values
        if (0x20..=0x7E).contains(&keyval) {
            let c = (keyval as u8) as char;
            self.buffer.push(c);
            
            // Check for trigger sequence ":emoji:"
            if self.buffer.ends_with(":emoji:") {
                self.buffer.clear();
                return (true, Some("🙂".to_string()));
            }
            
            // If it starts with ':', we handle it but don't commit yet
            if self.buffer.starts_with(':') {
                return (true, None);
            } else {
                // If it doesn't start with ':', clear and pass through
                self.buffer.clear();
                return (false, None);
            }
        }

        // Handle Escape or Backspace
        match keyval {
            0xff1b => { // Esc
                self.internal_reset();
                (true, None)
            }
            0xff08 => { // Backspace
                if !self.buffer.is_empty() {
                    self.buffer.pop();
                    (true, None)
                } else {
                    (false, None)
                }
            }
            _ => {
                // For other keys, if we have a buffer, we might want to clear it
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
}

impl EmojiEngine {
    async fn emit_commit_text(&self, se: &SignalEmitter<'_>, text: String) -> zbus::Result<()> {
        // IBusText is (sava{sv}) wrapped in a variant
        let ibus_text = (text, Vec::<Value>::new(), HashMap::<String, Value>::new());
        let variant = Value::from(ibus_text);
        Self::commit_text_signal(se, variant.into()).await
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
        let mut engine = EmojiEngine::new();
        engine.internal_enable();
        
        // Type ':'
        let (handled, commit) = engine.internal_process_key_event(0x3a, 0, 0);
        assert!(handled);
        assert_eq!(commit, None);
        assert_eq!(engine.buffer, ":");
        
        // Type 'e'
        let (handled, commit) = engine.internal_process_key_event(0x65, 0, 0);
        assert!(handled);
        assert_eq!(commit, None);
        
        // Finish ":emoji:"
        for c in "moji".chars() {
            engine.internal_process_key_event(c as u32, 0, 0);
        }
        let (handled, commit) = engine.internal_process_key_event(0x3a, 0, 0);
        
        assert!(handled);
        assert_eq!(commit, Some("🙂".to_string()));
        assert_eq!(engine.buffer, "");
    }
}
