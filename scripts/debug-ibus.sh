#!/bin/bash
# Debug script to check IBus integration

echo "=== IBus Integration Debug ==="
echo ""

# 1. Check binary
BINARY_PATH="$(pwd)/engine/target/debug/emoji-input-engine"
echo "1. Binary check:"
if [ -f "$BINARY_PATH" ]; then
    echo "   ✓ Binary exists: $BINARY_PATH"
    echo "   Size: $(ls -lh "$BINARY_PATH" | awk '{print $5}')"
else
    echo "   ✗ Binary missing: $BINARY_PATH"
    exit 1
fi
echo ""

# 2. Test binary execution
echo "2. Binary execution test:"
"$BINARY_PATH" 2>&1 | head -3
echo ""

# 3. Check component directory
COMPONENT_DIR="$HOME/.local/share/ibus/component"
COMPONENT_FILE="$COMPONENT_DIR/emoji-input.xml"
echo "3. Component file check:"
if [ -f "$COMPONENT_FILE" ]; then
    echo "   ✓ Component file exists"
    echo "   Path: $COMPONENT_FILE"
    echo "   Size: $(ls -lh "$COMPONENT_FILE" | awk '{print $5}')"
else
    echo "   ✗ Component file missing"
    echo "   Expected: $COMPONENT_FILE"
fi
echo ""

# 4. Show component file content
if [ -f "$COMPONENT_FILE" ]; then
    echo "4. Component file content:"
    cat "$COMPONENT_FILE"
    echo ""
fi

# 5. Check IBus daemon
echo "5. IBus daemon status:"
if pgrep -x ibus-daemon > /dev/null; then
    echo "   ✓ IBus daemon is running"
    pgrep -a ibus-daemon
else
    echo "   ✗ IBus daemon not running"
    echo "   Try: ibus-daemon -drx"
fi
echo ""

# 6. List all IBus engines
echo "6. All registered IBus engines:"
ibus list-engine | head -10
echo "   ..."
echo ""

# 7. Search for emoji engine
echo "7. Search for emoji engine:"
if ibus list-engine | grep -q "emoji"; then
    echo "   ✓ Found emoji engine:"
    ibus list-engine | grep emoji
else
    echo "   ✗ Emoji engine not found in IBus"
fi
echo ""

# 8. Check IBus logs
echo "8. Recent IBus logs (last 20 lines):"
journalctl --user -u ibus -n 20 --no-pager 2>/dev/null || echo "   (journalctl not available or no logs)"
echo ""

echo "=== Debug complete ==="
