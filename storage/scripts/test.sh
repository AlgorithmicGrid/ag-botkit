#!/usr/bin/env bash
# Test script for ag-storage

set -e

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

echo "=== ag-storage Test Suite ==="
echo ""

# Check if TimescaleDB is running
echo "1. Checking TimescaleDB..."
if docker-compose ps | grep -q "timescaledb.*Up"; then
    echo "   ✓ TimescaleDB is running"
else
    echo "   ✗ TimescaleDB is not running"
    echo "   Starting TimescaleDB..."
    docker-compose up -d
    echo "   Waiting for database to be ready..."
    sleep 5
fi
echo ""

# Run unit tests
echo "2. Running unit tests..."
cargo test --lib
echo "   ✓ Unit tests passed"
echo ""

# Run integration tests (requires TimescaleDB)
echo "3. Running integration tests..."
cargo test --test integration -- --test-threads=1 --nocapture
echo "   ✓ Integration tests passed"
echo ""

# Run clippy
echo "4. Running clippy..."
cargo clippy --all-targets -- -D warnings
echo "   ✓ No clippy warnings"
echo ""

# Check formatting
echo "5. Checking code formatting..."
cargo fmt -- --check
echo "   ✓ Code is formatted correctly"
echo ""

# Build examples
echo "6. Building examples..."
cargo build --examples
echo "   ✓ Examples built successfully"
echo ""

echo "=== All tests passed! ==="
