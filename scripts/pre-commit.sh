#!/bin/bash
set -e

echo "🔍 Running pre-commit checks (same as CI)..."

echo "📐 Checking code formatting..."
cargo fmt --all -- --check

echo "📎 Running clippy with warnings as errors..."
cargo clippy --all-targets --all-features -- -D warnings

echo "🧪 Running all tests with all features..."
cargo test --all-targets --all-features

echo "✅ All pre-commit checks passed! Safe to commit." 