use librush::ibus::{IBusEngine, IBusEngineBackend, IBusModifierState};
use xkeysym::{Keysym, KeyCode};
use zbus::{fdo, object_server::SignalEmitter, ObjectServer};

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
    pub fn process_key_event(&mut self, keyval: u32, _keycode: u32, _state: u32) -> (bool, Option<String>) {
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
                self.reset();
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
                    self.reset();
                }
                (false, None)
            }
        }
    }
    
    pub fn reset(&mut self) {
        self.buffer.clear();
    }
    
    pub fn enable(&mut self) {
        self.enabled = true;
        self.reset();
    }
    
    pub fn disable(&mut self) {
        self.enabled = false;
        self.reset();
    }
}

impl IBusEngine for EmojiEngine {
    async fn process_key_event(
        &mut self,
        se: SignalEmitter<'_>,
        _server: &ObjectServer,
        keyval: Keysym,
        _keycode: KeyCode,
        state: IBusModifierState,
    ) -> fdo::Result<bool> {
        // Only handle key press events (ignore releases)
        if state.release() {
            return Ok(false);
        }

        let (handled, commit) = self.process_key_event(u32::from(keyval), 0, 0);
        
        if let Some(text) = commit {
            let _ = Self::commit_text(&se, text).await;
        }
        
        Ok(handled)
    }

    async fn enable(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
    ) -> fdo::Result<()> {
        self.enable();
        Ok(())
    }

    async fn disable(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
    ) -> fdo::Result<()> {
        self.disable();
        Ok(())
    }

    async fn reset(
        &mut self,
        _se: SignalEmitter<'_>,
        _server: &ObjectServer,
    ) -> fdo::Result<()> {
        self.reset();
        Ok(())
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
        engine.enable();
        
        // Type ':'
        let (handled, commit) = engine.process_key_event(0x3a, 0, 0);
        assert!(handled);
        assert_eq!(commit, None);
        assert_eq!(engine.buffer, ":");
        
        // Type 'e'
        let (handled, commit) = engine.process_key_event(0x65, 0, 0);
        assert!(handled);
        assert_eq!(commit, None);
        
        // Finish ":emoji:"
        for c in "moji".chars() {
            engine.process_key_event(c as u32, 0, 0);
        }
        let (handled, commit) = engine.process_key_event(0x3a, 0, 0);
        
        assert!(handled);
        assert_eq!(commit, Some("🙂".to_string()));
        assert_eq!(engine.buffer, "");
    }
}
