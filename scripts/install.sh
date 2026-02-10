#!/bin/bash
# Installation script for GNOME Emoji Input Manager

set -e

echo "Installing GNOME Emoji Input Manager..."
echo ""
echo "This script is a convenience wrapper."
echo "For manual installation, use:"
echo "  meson setup build"
echo "  ninja -C build"
echo "  sudo ninja -C build install"
echo ""

# Check if build directory exists
if [ ! -d "build" ]; then
    echo "Setting up build directory..."
    meson setup build
fi

echo "Building..."
ninja -C build

echo ""
echo "Ready to install. Run: sudo ninja -C build install"

exit 0
