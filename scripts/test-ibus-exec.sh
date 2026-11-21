#!/bin/bash
# Test if IBus can execute our engine

echo "=== Testing IBus Engine Execution ==="
echo ""

BINARY_PATH="./engine/target/debug/emoji-input-engine"

echo "1. Direct execution test:"
"$BINARY_PATH" --ibus &
PID=$!
echo "   Started with PID: $PID"
sleep 2

if ps -p $PID > /dev/null; then
    echo "   ✓ Process is running"
    kill $PID
    wait $PID 2>/dev/null
    echo "   ✓ Process stopped cleanly"
else
    echo "   ✗ Process died immediately"
fi
echo ""

echo "2. Testing with escaped path:"
ESCAPED_PATH=$(printf '%q' "$BINARY_PATH")
echo "   Escaped: $ESCAPED_PATH"
echo ""

echo "3. Checking for IBus component validation:"
# IBus might validate the component XML
xmllint --noout ~/.local/share/ibus/component/emoji-input.xml 2>&1 && echo "   ✓ XML is valid" || echo "   ✗ XML validation failed"
echo ""

echo "4. Checking IBus component directories:"
echo "   System: /usr/share/ibus/component/"
ls /usr/share/ibus/component/ 2>/dev/null | head -5 || echo "   (empty or not accessible)"
echo ""
echo "   User: ~/.local/share/ibus/component/"
ls ~/.local/share/ibus/component/ 2>/dev/null || echo "   (empty)"
echo ""

echo "5. Comparing with a working component:"
if [ -f "/usr/share/ibus/component/simple.xml" ]; then
    echo "   Found system component: simple.xml"
    grep -A 2 "<exec>" /usr/share/ibus/component/simple.xml | head -3
else
    echo "   No reference component found"
fi
echo ""

echo "=== Test complete ==="
