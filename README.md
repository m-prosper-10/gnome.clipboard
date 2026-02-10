# GNOME Emoji Input Manager

A native IBus-based emoji input method for GNOME, designed for keyboard-driven workflow.

## What This Is

An input method engine (IME) that integrates with IBus to provide emoji insertion across all applications. Unlike GNOME Shell extensions, this works at the input stack level, making it stable and desktop-environment agnostic.

## Project Status

**Current Phase**: PHASE 2 - Minimal IBus Engine (Headless)

**Completed Phases**:
- ✅ PHASE 1: Repository + Build Skeleton

**Current Status**:
- Engine compiles and runs
- IBus component registration ready
- Awaiting manual testing in IBus

See `docs/tasks.md` for the complete roadmap.

## Build Instructions

### Prerequisites

- Meson (>= 0.59.0)
- Ninja
- Rust toolchain (rustc + cargo)
- IBus development files (>= 1.5.0)
- GTK 4 (for future UI components)
- GLib development files

On Fedora/RHEL:
```bash
sudo dnf install meson ninja-build rust cargo ibus-devel gtk4-devel glib2-devel
```

On Ubuntu/Debian:
```bash
sudo apt install meson ninja-build rustc cargo libibus-1.0-dev libgtk-4-dev libglib2.0-dev
```

### Building

```bash
meson setup build
ninja -C build
```

### Testing (PHASE 2)

Test the engine without installing:
```bash
./scripts/test-phase2.sh
```

Register with IBus for testing:
```bash
./scripts/register-ibus.sh
```

### Installing

```bash
sudo ninja -C build install
sudo ibus restart
```

After installation, add "Emoji Input" in `ibus-setup`.

### Uninstalling

```bash
sudo ninja -C build uninstall
```

Or use the provided script:
```bash
sudo ./scripts/uninstall.sh
```

## Architecture

- **engine/**: Rust-based IBus engine (core input method logic)
- **ui/**: GTK 4 preferences application (settings management)
- **data/**: Configuration files, emoji database, desktop integration
- **scripts/**: Installation and maintenance utilities

## Philosophy

- **CLI-first**: Every feature must work from the keyboard
- **Single repository**: No split packages or complex dependencies
- **Respect the stack**: Work with IBus, not against it
- **No premature polish**: Functionality before aesthetics
- **Compile or die**: If it doesn't build, it doesn't exist

## License

MIT License - See LICENSE file for details

## Contributing

This project is in early development. See `docs/tasks.md` for the implementation roadmap.

## Non-Goals

- ❌ GNOME Shell extensions
- ❌ Wayland protocol hacks
- ❌ Windows/macOS feature parity
- ❌ Unnecessary dependencies
