#!/usr/bin/env bash
# Build script for exec module

set -e

echo "=== Building ag-exec module ==="
echo ""

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo not found in PATH"
    echo "Please install Rust from https://rustup.rs/"
    exit 1
fi

echo "1. Checking Rust version..."
rustc --version
cargo --version
echo ""

echo "2. Building library (release mode)..."
cargo build --release
echo "   ✓ Build complete"
echo ""

echo "3. Running clippy (linter)..."
cargo clippy --all-targets -- -D warnings
echo "   ✓ No clippy warnings"
echo ""

echo "4. Running unit tests..."
cargo test --lib
echo "   ✓ Unit tests passed"
echo ""

echo "5. Running integration tests..."
cargo test --test integration
echo "   ✓ Integration tests passed"
echo ""

echo "6. Generating documentation..."
cargo doc --no-deps
echo "   ✓ Documentation generated"
echo ""

echo "=== Build Summary ==="
echo "Library: target/release/libag_exec.rlib"
echo "Documentation: target/doc/ag_exec/index.html"
echo ""
echo "To run the example:"
echo "  export POLYMARKET_API_KEY=your_key"
echo "  export POLYMARKET_API_SECRET=your_secret"
echo "  cargo run --example place_order"
echo ""
echo "=== Build Complete ==="
