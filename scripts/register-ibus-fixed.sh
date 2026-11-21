#!/bin/bash
# Register the emoji input engine with IBus (fixed for spaces in path)
# Creates a symlink to avoid path issues

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BINARY_PATH="$PROJECT_ROOT/engine/target/debug/emoji-input-engine"
COMPONENT_DIR="$HOME/.local/share/ibus/component"
COMPONENT_FILE="$COMPONENT_DIR/emoji-input.xml"

# Create a symlink in a path without spaces
SYMLINK_DIR="$HOME/.local/bin"
SYMLINK_PATH="$SYMLINK_DIR/emoji-input-engine"

echo "=== Registering Emoji Input Engine with IBus (Fixed) ==="
echo ""

# Check if binary exists
if [ ! -f "$BINARY_PATH" ]; then
    echo "❌ Binary not found at: $BINARY_PATH"
    echo "   Run: cargo build --manifest-path engine/Cargo.toml"
    exit 1
fi

echo "✓ Binary found: $BINARY_PATH"

# Create symlink directory
mkdir -p "$SYMLINK_DIR"
echo "✓ Symlink directory: $SYMLINK_DIR"

# Create or update symlink
ln -sf "$BINARY_PATH" "$SYMLINK_PATH"
echo "✓ Symlink created: $SYMLINK_PATH -> $BINARY_PATH"

# Make sure it's executable
chmod +x "$SYMLINK_PATH"

# Test the symlink
if [ -x "$SYMLINK_PATH" ]; then
    echo "✓ Symlink is executable"
else
    echo "❌ Symlink is not executable"
    exit 1
fi

# Create component directory
mkdir -p "$COMPONENT_DIR"
echo "✓ Component directory: $COMPONENT_DIR"

# Create component XML with symlink path (no spaces!)
cat > "$COMPONENT_FILE" << EOF
<?xml version="1.0" encoding="utf-8"?>
<component>
  <name>org.example.EmojiInput</name>
  <description>Emoji Input Method</description>
  <exec>$SYMLINK_PATH --ibus</exec>
  <version>0.1.0</version>
  <author>GNOME Emoji Input Manager</author>
  <license>MIT</license>
  <homepage>https://example.org</homepage>
  <textdomain>emoji-input</textdomain>

  <engines>
    <engine>
      <name>emoji-input</name>
      <language>en</language>
      <license>MIT</license>
      <author>GNOME Emoji Input Manager</author>
      <icon>input-keyboard</icon>
      <layout>us</layout>
      <longname>Emoji Input</longname>
      <description>Emoji input method for GNOME</description>
      <rank>0</rank>
    </engine>
  </engines>
</component>
EOF

echo "✓ Component file created: $COMPONENT_FILE"
echo "  Using path: $SYMLINK_PATH (no spaces!)"
echo ""

# Clear IBus cache
echo "Clearing IBus cache..."
rm -rf ~/.cache/ibus/bus/ 2>/dev/null || true

# Restart IBus
echo "Restarting IBus..."
ibus restart

sleep 3

# Check if engine is registered
echo ""
echo "Checking if engine is registered..."
if ibus list-engine | grep -q "emoji-input"; then
    echo "✓ Engine successfully registered!"
    echo ""
    ibus list-engine | grep emoji-input
    echo ""
    echo "=== Next Steps ==="
    echo "1. Open IBus preferences: ibus-setup"
    echo "2. Go to 'Input Method' tab"
    echo "3. Click 'Add' button"
    echo "4. Search for 'Emoji Input'"
    echo "5. Add it to your input methods"
    echo "6. Switch to it using Super+Space (or your configured shortcut)"
    echo ""
    echo "Note: PHASE 2 engine now handles trigger/search/commit behavior; manual verification is still required"
else
    echo "❌ Engine not found in IBus list"
    echo ""
    echo "Debug information:"
    echo "Component file:"
    cat "$COMPONENT_FILE"
    echo ""
    echo "Trying manual daemon restart..."
    ibus exit
    sleep 2
    ibus-daemon -drx &
    sleep 3
    ibus list-engine | grep -i emoji || echo "  Still not found"
fi
