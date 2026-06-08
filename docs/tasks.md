# GNOME Emoji Input Manager - Implementation Tasks

> **Project Philosophy**: Single repo, CLI-first, no premature polish. Every phase ends with something runnable. If it does not compile, it does not exist.

---

## PHASE 1: Repository + Build Skeleton ✅ COMPLETE

**Goal**: Empty project that builds and installs cleanly.

- [x] Create GitLab repository
- [x] Add license file
- [x] Initialize Meson build system at root
  - [x] Create `meson.build` at project root
  - [x] Configure project metadata and dependencies
- [x] Stub subprojects
  - [x] Create `engine/` directory structure
  - [x] Create `ui/` directory structure
  - [x] Create `data/` directory structure
- [x] Wire Meson build system
  - [x] Configure `engine/` subproject build
  - [x] Configure `ui/` subproject build
  - [x] Configure `data/` subproject build
  - [x] Ensure `ninja install` works end-to-end
- [x] Create README.md
  - [x] Document what this project is
  - [x] Document build instructions
  - [x] Document uninstall procedure

**Deliverables**:

- ✅ `meson setup build && ninja -C build` succeeds
- ✅ `ninja -C build install` installs something
- ✅ README with essential information

**Completed**: 2026-02-10

---

## PHASE 2: Minimal IBus Engine (Headless) 🚧 IN PROGRESS

**Goal**: Register a valid IBus engine that can commit text.

**⚠️ CRITICAL**: If this fails, stop. Everything depends on this.

- [x] Implement IBus engine in `engine/`
  - [x] Create engine main file
  - [x] Implement engine activation handler (stub)
  - [x] Implement key event handler (stub)
  - [x] Implement `commit_text()` function (stub)
  - [x] Add hardcoded test trigger (e.g., `:emoji:` → 🙂)
- [x] Create `ibus-component.xml`
  - [x] Define engine metadata
  - [x] Register engine with IBus
  - [x] Configure engine executable path
- [ ] Integration testing
  - [ ] Verify engine appears in `ibus-setup`
  - [ ] Test engine selection
  - [ ] Test hardcoded emoji insertion

**Deliverables**:

- ✅ Engine shows up in `ibus-setup` (ready to test)
- ⏳ Selecting it allows typing
- ⏳ Pressing hardcoded key inserts single emoji

**Status**: Engine implementation is complete. Component XML ready. Needs manual IBus testing.

---

## PHASE 3: Composition Buffer + Search Core

**Goal**: Real input method behavior (still headless).

**Current code note**: The composition buffer, search/filtering, recents, and commit path already exist in `engine/src/engine.rs`. The remaining phase 3 work is mostly about keeping that logic cleanly separated, validating it in a live IBus session, and tightening edge cases as they show up.

- [x] Implement composition buffer
  - [x] Create composition state management
  - [x] Capture typed characters
  - [x] Display preedit text
  - [x] Implement cancel logic (Esc)
  - [x] Implement confirm logic (Enter)
- [x] Emoji database system
  - [x] Create `data/emojis.json` structure
  - [x] Implement JSON loader
  - [x] Build in-memory index
  - [x] Add keyword-based filtering
  - [x] Implement search algorithm
- [x] Search integration
  - [x] Connect composition buffer to search
  - [x] Filter emoji list based on input
  - [x] Commit first match on Enter
  - [x] Add debug logging for filtering

**Deliverables**:

- ✅ Typing `:sm` narrows emoji list internally
- ✅ Basic popup bridge exists, but the phase remains focused on engine-side behavior
- ✅ Commit first match on Enter works

---

## PHASE 4: Popup GTK UI (Engine-Controlled)

**Goal**: Polish the popup GTK UI and harden engine-UI interaction.

**Current code note**: The popup now exists as a functional session-bus-driven picker. Phase 4 work is focused on placement, explicit keyboard handling, closure behavior, and interaction polish rather than protocol changes.

See also: [docs/phase4-plan.md](/home/polo/Documents/GNOME%20Input%20Manager/docs/phase4-plan.md)

- [x] Create GTK popup window
  - [x] Initialize GTK application
  - [x] Create popup window widget
  - [x] Configure window properties (borderless, floating)
  - [x] Implement window positioning (near cursor or centered)
- [x] Build UI components
  - [x] Keep the popup display-only and engine-driven
  - [x] Create emoji grid/list view
  - [x] Implement emoji rendering
  - [x] Add visual selection indicator
- [x] Keyboard navigation
  - [x] Arrow key navigation (up/down/left/right)
  - [x] Enter key to commit selection
  - [x] Esc key to close popup
  - [x] Tab/Shift+Tab navigation
- [x] Engine-UI integration
  - [x] Connect engine to popup lifecycle
  - [x] Pass search results to UI
  - [x] Handle selection events
  - [x] Clean popup closure

**Deliverables**:

- ✅ Popup appears on trigger
- ✅ Arrow keys move selection
- ✅ Enter commits selected emoji
- ✅ Esc closes popup cleanly

**Status**: Popup implementation is complete. Remaining work is live IBus/compositor verification and any polish that shows up during that testing.

---

## PHASE 5: Recents + Variants

**Goal**: Usability improvements, not decoration.

**Current code note**: Recents tracking already exists in the engine, and the variant support surface is still minimal. Phase 5 is now about tightening that behavior, choosing a stable persistence story, and deciding how much of the picker UI should own variants versus the engine.

See also: [docs/phase5-plan.md](/home/polo/Documents/GNOME%20Input%20Manager/docs/phase5-plan.md)

- [ ] Recent emojis system
  - [ ] Design persistence format (GSettings or local file)
  - [ ] Implement recent emoji tracking
  - [ ] Sort recents by frequency
  - [ ] Display recents in UI (separate section or priority)
  - [ ] Limit recents history size
- [ ] Emoji variants support
  - [ ] Identify emojis with skin tone modifiers
  - [ ] Implement variant selection UI
  - [ ] Add variant picker (long-press or submenu)
  - [ ] Persist variant preferences
- [ ] Cache management
  - [ ] Implement cache storage
  - [ ] Load cache on startup
  - [ ] Save cache on changes
  - [ ] Handle cache corruption gracefully

**Deliverables**:

- ✅ Recents update correctly
- ✅ Emoji variants selectable
- ✅ No crashes after restart

---

## PHASE 6: Preferences App

**Goal**: Separation of concerns - dedicated settings UI.

**Current code note**: The prefs app already exists, but it still persists settings to local JSON files in `~/.config/gnome-emoji-input/` and `~/.cache/gnome-emoji-input/`. The GSettings schema file is present for packaging, but the app is not yet wired to it.

- [ ] Implement `ui/` GTK preferences app
  - [ ] Create standalone GTK application
  - [ ] Design preferences window layout
  - [ ] Implement settings categories
- [ ] Create GSettings schema
  - [ ] Define schema XML
  - [ ] Compile schema
  - [ ] Install schema to system
- [ ] Expose settings
  - [ ] Trigger sequence configuration
  - [ ] Emoji display size setting
  - [ ] History depth setting
  - [ ] Theme preferences (if applicable)
  - [ ] Keyboard shortcuts configuration
- [ ] Settings integration
  - [ ] Bind UI controls to GSettings
  - [ ] Ensure engine reads settings
  - [ ] Implement live settings reload
  - [ ] Add settings validation

**Deliverables**:

- ✅ Preferences app launches
- ✅ Changes affect engine behavior
- ✅ Settings persist across reboots

---

## PHASE 7: Packaging + System Integration

**Goal**: Feels native to GNOME.

**Current code note**: The packaged component file for the IBus engine is installed, and the main desktop/schema placeholder state has been cleaned up. The remaining packaging work is around finishing the desktop integration story and moving prefs to GSettings if that is still the intended direction.

- [ ] Desktop integration
  - [ ] Create `.desktop` file
  - [ ] Design application icon
  - [ ] Install icon to system paths
  - [ ] Register MIME types (if needed)
- [ ] Autostart handling
  - [ ] Create autostart desktop entry (optional)
  - [ ] Add autostart toggle in preferences
  - [ ] Test autostart on login
- [ ] Installation refinement
  - [ ] Verify install paths follow FHS
  - [ ] Create uninstall script
  - [ ] Test clean removal
  - [ ] Document manual installation steps

**Deliverables**:

- ✅ Appears in GNOME app grid
- ✅ Survives logout/reboot
- ✅ Can be removed without residue

---

## PHASE 8: Hardening

**Goal**: Production-ready longevity.

- [ ] Error handling
  - [ ] Add comprehensive error checking
  - [ ] Implement graceful degradation
  - [ ] Add user-friendly error messages
  - [ ] Log errors appropriately
- [ ] Performance tuning
  - [ ] Profile search performance
  - [ ] Optimize emoji database loading
  - [ ] Reduce UI rendering lag
  - [ ] Test with large emoji datasets
- [ ] Unicode updates
  - [ ] Document emoji update procedure
  - [ ] Create script to update `emojis.json`
  - [ ] Test with new Unicode versions
- [ ] Accessibility review
  - [ ] Ensure keyboard-only operation
  - [ ] Test with screen readers
  - [ ] Add ARIA labels where needed
  - [ ] Verify high contrast mode support
- [ ] Compatibility testing
  - [ ] Test in terminals (GNOME Terminal, etc.)
  - [ ] Test in browsers (Firefox, Chrome)
  - [ ] Test in GTK apps (Gedit, Files)
  - [ ] Test in Qt apps (if applicable)

**Deliverables**:

- ✅ No noticeable lag
- ✅ No crashes on bad input
- ✅ Works in terminals, browsers, GTK apps

---

## PHASE 9: Versioning + Maintenance

**Goal**: Future-proofing and sustainability.

- [ ] Version control
  - [ ] Tag `v0.1` (minimal viable)
  - [ ] Tag `v0.5` (feature complete)
  - [ ] Tag `v1.0` (production ready)
  - [ ] Create release notes for each version
- [ ] Documentation
  - [ ] Write architecture overview
  - [ ] Document known limitations
  - [ ] Create upgrade guide
  - [ ] Add troubleshooting section
- [ ] Contribution guidelines
  - [ ] Create CONTRIBUTING.md
  - [ ] Define code style
  - [ ] Set up issue templates
  - [ ] Document testing procedures
- [ ] Maintenance procedures
  - [ ] Document emoji update process
  - [ ] Create dependency update checklist
  - [ ] Define support policy
  - [ ] Plan deprecation strategy

**Deliverables**:

- ✅ Clean release history
- ✅ Clear contribution rules
- ✅ Low-maintenance baseline

---

## Maintenance Model (Ongoing)

### Monitor:

- [ ] IBus API changes
- [ ] GTK major version bumps
- [ ] Unicode emoji releases (annual)
- [ ] Security vulnerabilities in dependencies

### Ignore:

- ❌ GNOME Shell extension drama
- ❌ Extension API churn
- ❌ Desktop environment fashion cycles
- ❌ Windows/macOS feature parity

---

## Project Constraints (Non-Negotiable)

### DO:

- ✅ Respect the input stack (IBus)
- ✅ Keep the engine simple
- ✅ Treat UI as replaceable
- ✅ Focus on keyboard-driven workflow
- ✅ Maintain backward compatibility

### DO NOT:

- ❌ Touch GNOME Shell
- ❌ Fight Wayland
- ❌ Chase Windows parity
- ❌ Add unnecessary dependencies
- ❌ Implement features before core works

---

## Current Status

**Active Phase**: PHASE 2 - Minimal IBus Engine (Headless)

**Last Updated**: 2026-06-07

**Notes**: 
- PHASE 1 completed successfully
- Build system functional with Rust + Meson integration
- Phase 2 implementation is complete; manual IBus validation remains
- Note: `ibus` crate version 0.2.0 available on crates.io for PHASE 2
