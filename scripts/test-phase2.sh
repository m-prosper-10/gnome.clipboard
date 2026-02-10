#!/bin/bash
# PHASE 2 Testing Script
# Tests IBus engine registration and basic functionality

set -e

echo "=== PHASE 2: IBus Engine Testing ==="
echo ""

# Check if binary exists
if [ ! -f "./engine/target/debug/emoji-input-engine" ]; then
    echo "❌ Binary not found. Run: cargo build --manifest-path engine/Cargo.toml"
    exit 1
fi

echo "✓ Binary exists"
echo ""

# Test standalone mode
echo "Testing standalone mode:"
./engine/target/debug/emoji-input-engine
echo ""

# Test IBus mode (will run in background)
echo "Testing IBus mode (Ctrl+C to stop):"
echo "Starting engine with --ibus flag..."
./engine/target/debug/emoji-input-engine --ibus &
ENGINE_PID=$!

sleep 2

if ps -p $ENGINE_PID > /dev/null; then
    echo "✓ Engine process is running (PID: $ENGINE_PID)"
    kill $ENGINE_PID
    wait $ENGINE_PID 2>/dev/null || true
    echo "✓ Engine stopped cleanly"
else
    echo "❌ Engine process failed to start"
    exit 1
fi

echo ""
echo "=== Manual Testing Steps ==="
echo ""
echo "To register with IBus:"
echo "  1. Create component directory:"
echo "     mkdir -p ~/.local/share/ibus/component"
echo ""
echo "  2. Copy component file:"
echo "     cp data/ibus-component.xml ~/.local/share/ibus/component/emoji-input.xml"
echo ""
echo "  3. Update the exec path in the XML to point to your binary:"
echo "     sed -i 's|/usr/local/libexec/emoji-input-engine|$(pwd)/engine/target/debug/emoji-input-engine|' \\"
echo "       ~/.local/share/ibus/component/emoji-input.xml"
echo ""
echo "  4. Restart IBus:"
echo "     ibus restart"
echo ""
echo "  5. Check if engine appears:"
echo "     ibus list-engine | grep emoji"
echo ""
echo "  6. Open IBus preferences:"
echo "     ibus-setup"
echo ""
echo "  7. Add 'Emoji Input' to your input methods"
echo ""
echo "  8. Switch to the emoji input method and test typing"
echo ""
