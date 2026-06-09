# PHASE 2 Status: Minimal IBus Engine

## Completed ✅

### 1. Engine Implementation
- **File**: `engine/src/main.rs`
  - Standalone mode: Displays version and usage info
  - IBus mode: Runs with `--ibus` flag, starts GLib main loop
  - Session-bus UI bridge: launches the popup and forwards commit requests with a per-launch token
  - Signal handling: Responds to SIGINT and SIGTERM
  
- **File**: `engine/src/engine.rs`
  - `EmojiEngine` struct with composition buffer
  - Key event handler with trigger, navigation, and commit handling
  - Enable/disable/reset methods
  - Unit tests

### 2. IBus Component Registration
- **File**: `data/ibus-component.xml`
  - Engine metadata defined
  - Executable path configured
  - Language and layout settings

### 3. Build System
- Cargo.toml updated with GLib dependencies
- Meson build compiles successfully
- IBus component installation enabled by default
- Emoji data is installed from `data/emojis.json`

### 4. Testing Infrastructure
- **File**: `scripts/test-phase2.sh`
  - Tests binary compilation
  - Tests standalone mode
  - Tests IBus mode process lifecycle
  
- **File**: `scripts/register-ibus.sh`
  - Registers engine with IBus for testing
  - Creates component file with correct paths
  - Restarts IBus and verifies registration
- Popup commit bridge is protected by a per-launch token in the engine/UI handshake

## Pending ⏳

### Critical: Manual Testing Required

The engine is ready but needs manual verification:

1. **Registration Test**
   ```bash
   ./scripts/register-ibus.sh
   ```
   Expected: Engine appears in `ibus list-engine`

2. **IBus Setup Test**
   ```bash
   ibus-setup
   ```
   Expected: "Emoji Input" appears in available input methods

3. **Selection Test**
   - Add "Emoji Input" to active input methods
   - Switch to it using Super+Space
   Expected: Engine process starts, no crashes

4. **Typing Test**
   - With engine selected, try typing in any application
   Expected: Keys pass through normally (no processing yet)

### Not Yet Implemented

- **Manual IBus verification**: The code path is implemented, but the engine still needs to be exercised in a live GNOME/IBus session
- **Full packaging polish**: Desktop integration and schema installation are still partial

## Known Limitations

1. **Partial GSettings migration**: trigger and variant preferences use GSettings, but recents/cache data still use local files
2. **Session-bus bridge is instance-scoped, not authenticated**: The token prevents accidental cross-instance commits, not hostile same-user access
3. **Packaging assets are still evolving**: The main desktop file is not present yet

## Next Steps

### If Manual Testing Succeeds ✅
1. Decide whether any remaining settings should move into GSettings or stay in the local cache
2. Add the main desktop entry and finish packaging polish
3. Expand search and selection behavior
4. Test emoji insertion in real applications

### If Manual Testing Fails ❌
1. Check IBus logs: `journalctl --user -u ibus`
2. Verify component XML syntax
3. Check binary permissions and path
4. Test with `ibus-daemon -xvr` for verbose output
5. Review IBus documentation for protocol requirements

## Architecture Notes

### Why GLib Instead of Direct IBus Bindings?

The Rust `ibus` crate (v0.2.0) is minimal and unmaintained. Instead:
- Using `glib` and `gio` crates for GObject integration
- These are well-maintained and used by GNOME projects
- Provides foundation for future GTK UI (PHASE 4)
- More stable than abandoned IBus-specific bindings

### Current Approach

```
User Input → IBus Daemon → emoji-input-engine (GLib main loop)
                                    ↓
                       [EmojiEngine key handling + search]
                                    ↓
                        [IBus commit_text / preedit updates]
                                    ↓
                 [Session-bus popup updates + tokenized commit bridge]
```

## Files Modified in PHASE 2

- `engine/Cargo.toml` - Added glib, gio, libc dependencies
- `engine/src/main.rs` - Implemented main loop, popup launch, and tokenized UI bridge
- `engine/src/engine.rs` - Created EmojiEngine struct and search/selection logic
- `data/ibus-component.xml` - Defined engine registration
- `scripts/test-phase2.sh` - Testing script
- `scripts/register-ibus.sh` - Registration helper
- `meson_options.txt` - Enabled IBus component installation
- `README.md` - Updated with PHASE 2 instructions
- `docs/tasks.md` - Marked PHASE 2 progress

## Testing Commands

```bash
# Build
cargo build --manifest-path engine/Cargo.toml

# Test standalone
./engine/target/debug/emoji-input-engine

# Test IBus mode (Ctrl+C to stop)
./engine/target/debug/emoji-input-engine --ibus

# Register with IBus
./scripts/register-ibus.sh

# Check registration
ibus list-engine | grep emoji

# Open preferences
ibus-setup
```

## Success Criteria

PHASE 2 is complete when:
- ✅ Engine compiles without errors
- ✅ Engine contains the `:emoji:` hardcoded trigger path
- ⏳ Engine appears in `ibus list-engine`
- ⏳ Engine appears in `ibus-setup` GUI
- ⏳ Engine can be selected and activated
- ⏳ Typing `:emoji:` inserts 🙂 in a live IBus session

**Current Status**: Implementation complete. Awaiting manual IBus testing.
