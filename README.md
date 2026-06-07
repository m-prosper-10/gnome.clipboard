# GNOME Emoji Input Manager

A modern, fast, and native emoji input manager for GNOME, integrating with IBus and a GTK4/Libadwaita popup UI.

## Features

- **Trigger Character**: Insert emojis using a customizable trigger character (default `:`) or the **Super + ;** shortcut.
- **Searchable**: Real-time, case-insensitive emoji search as you type.
- **Recently Used**: Remembers your favorite emojis and puts them at the top of the list.
- **Emoji Variants**: Support for skin tones and other variants, easily accessible in the picker.
- **GTK4/Libadwaita UI**: A native-feeling popup and a dedicated preferences app.
- **IBus Integration**: Works as a standard input method in the GNOME desktop.
- **Scoped UI Commit Bridge**: The popup uses a per-launch token so only the matching engine instance can accept click-to-commit requests.

## Architecture

The project consists of three main components:

1.  **Engine** (`emoji-input-engine`): The core IBus logic, emoji search, and session-bus bridge.
2.  **UI** (`emoji-input-ui`): The GTK4 popup that appears when you type the trigger character.
3.  **Prefs** (`emoji-input-prefs`): The Libadwaita preferences app for customization.

## Installation

### Prerequisites

- Rust (latest stable)
- Meson & Ninja
- GTK4 & Libadwaita development headers
- IBus (installed and running)

### Build and Install

```bash
# Setup the build directory
meson setup build --prefix=$HOME/.local

# Build and install
ninja -C build install

# Restart IBus to pick up the new component
ibus restart
```

### Setup in GNOME

1. Open **Settings** -> **Keyboard** -> **Input Sources**.
2. Click **+** -> **Other** -> **Emoji Input**.
3. (Optional) Run `emoji-input-prefs` to change the trigger character or clear history.

## Usage

1. Switch to the **Emoji Input** source.
2. Press **Super + ;** or type `:` followed by the name of an emoji (e.g., `:smile`).
3. Use **Up/Down Arrows** to select the emoji.
4. Press **Enter** to insert it.
5. Press **Esc** or **Backspace** to cancel.

## Current State

- The engine loads emoji data from `data/emojis.json` or the installed data directory.
- Search is prefix-based and case-insensitive.
- The popup UI is launched by the engine and talks back over the session bus.
- Click-to-commit requests carry a per-launch token to avoid accidental cross-instance commits.
- Preferences currently persist to local JSON files under `~/.config/gnome-emoji-input/` and `~/.cache/gnome-emoji-input/`; the GSettings schema is present for packaging, but the prefs app has not been switched over to it yet.

## Debugging

To see detailed logs, run the engine or UI with the `RUST_LOG` environment variable:

```bash
RUST_LOG=debug emoji-input-engine --ibus
```

## License

MIT
