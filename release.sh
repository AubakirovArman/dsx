#!/usr/bin/env bash
# DSX Code — Release Compilation Automation Script.
#
# This script handles compilation of the optimized release binary for your platform.

set -e

PLATFORM=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

echo "📦 DSX CODE RELEASE TOOL"
echo "─────────────────────────────────────────"
echo "Active Host OS: $PLATFORM"
echo "Active Host Arch: $ARCH"
echo "─────────────────────────────────────────"

if [ "$PLATFORM" = "linux" ]; then
    echo "🔨 Compiling native Linux release binary..."
    cargo build --release
    echo "✓ Success! Binary built at: ./target/release/dsx"
    echo "👉 Install globally using: cargo install --path . --force"
elif [ "$PLATFORM" = "darwin" ]; then
    echo "🔨 Compiling native macOS (Apple Silicon/ARM or Intel) release binary..."
    cargo build --release
    echo "✓ Success! Binary built at: ./target/release/dsx"
    echo "👉 Install globally using: cargo install --path . --force"
else
    echo "🔨 Compiling release binary..."
    cargo build --release
    echo "✓ Success! Binary built at: ./target/release/dsx"
fi

echo "─────────────────────────────────────────"
echo "🌐 Note on cross-compiling to other platforms:"
echo "Since DSX includes SQLite and Rustls (TLS) C-assembly optimizations (libsqlite3-sys & ring),"
echo "cross-compiling from Linux to macOS requires a complete osxcross/Clang SDK toolchain."
echo "For absolute safety, performance, and simplicity, we highly recommend natively building"
echo "on your Mac by cloning this repository and running this script or: cargo build --release"
echo "─────────────────────────────────────────"
