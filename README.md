# GNOME Emoji Input Manager

A modern, fast, and native emoji input manager for GNOME, integrating seamlessly with IBus and featuring a beautiful GTK4/Libadwaita interface.

## Features

- **Trigger Character**: Swiftly insert emojis using a customizable trigger character (default `:`).
- **Searchable**: Real-time emoji search as you type.
- **Recently Used**: Remembers your favorite emojis and puts them at the top of the list.
- **Emoji Variants**: Support for skin tones and other variants, easily accessible in the picker.
- **GTK4/Libadwaita UI**: A native-feeling popup and a dedicated preferences app.
- **IBus Integration**: Works as a standard input method in the GNOME desktop.

## Architecture

The project consists of three main components:

1.  **Engine** (`emoji-input-engine`): The core IBus logic and emoji search engine.
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
2. Type `:` followed by the name of an emoji (e.g., `:smile`).
3. Use **Up/Down Arrows** to select the emoji.
4. Press **Enter** to insert it.
5. Press **Esc** or **Backspace** to cancel.

## Debugging

To see detailed logs, run the engine or UI with the `RUST_LOG` environment variable:

```bash
RUST_LOG=debug emoji-input-engine --ibus
```

## License

MIT
