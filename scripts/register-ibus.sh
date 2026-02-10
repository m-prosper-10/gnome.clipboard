#!/bin/bash
# Register the emoji input engine with IBus for testing
# This script sets up the engine without installing system-wide

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BINARY_PATH="$PROJECT_ROOT/engine/target/debug/emoji-input-engine"
COMPONENT_DIR="$HOME/.local/share/ibus/component"
COMPONENT_FILE="$COMPONENT_DIR/emoji-input.xml"

echo "=== Registering Emoji Input Engine with IBus ==="
echo ""

# Check if binary exists
if [ ! -f "$BINARY_PATH" ]; then
    echo "❌ Binary not found at: $BINARY_PATH"
    echo "   Run: cargo build --manifest-path engine/Cargo.toml"
    exit 1
fi

echo "✓ Binary found: $BINARY_PATH"

# Create component directory
mkdir -p "$COMPONENT_DIR"
echo "✓ Component directory: $COMPONENT_DIR"

# Create component XML with correct path
cat > "$COMPONENT_FILE" << EOF
<?xml version="1.0" encoding="utf-8"?>
<component>
  <name>org.example.EmojiInput</name>
  <description>Emoji Input Method</description>
  <exec>$BINARY_PATH --ibus</exec>
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
echo ""

# Restart IBus
echo "Restarting IBus..."
ibus restart

sleep 2

# Check if engine is registered
echo ""
echo "Checking if engine is registered..."
if ibus list-engine | grep -q "emoji-input"; then
    echo "✓ Engine successfully registered!"
    echo ""
    echo "=== Next Steps ==="
    echo "1. Open IBus preferences: ibus-setup"
    echo "2. Go to 'Input Method' tab"
    echo "3. Click 'Add' button"
    echo "4. Search for 'Emoji Input'"
    echo "5. Add it to your input methods"
    echo "6. Switch to it using Super+Space (or your configured shortcut)"
    echo ""
    echo "Note: PHASE 2 engine is minimal - it just runs but doesn't process keys yet"
else
    echo "❌ Engine not found in IBus list"
    echo ""
    echo "Debug information:"
    ibus list-engine | grep -i emoji || echo "  No emoji engines found"
    echo ""
    echo "Try running: ibus restart"
fi
