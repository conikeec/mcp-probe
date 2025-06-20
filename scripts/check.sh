#!/bin/bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔍 Running all checks...${NC}"

# Function to run a check and report status
run_check() {
    local name="$1"
    local command="$2"
    
    echo -e "\n${YELLOW}📋 Running $name...${NC}"
    if eval "$command"; then
        echo -e "${GREEN}✅ $name passed${NC}"
    else
        echo -e "${RED}❌ $name failed${NC}"
        exit 1
    fi
}

# Check formatting
run_check "rustfmt" "cargo fmt --all -- --check"

# Run clippy
run_check "clippy" "cargo clippy --all-targets --all-features -- -D warnings"

# Run tests
run_check "tests" "cargo test --all-targets --all-features"

# Check that everything builds
run_check "build" "cargo build --all-targets --all-features"

# Security audit (optional, don't fail on this)
echo -e "\n${YELLOW}🔒 Running security audit...${NC}"
if command -v cargo-audit >/dev/null 2>&1; then
    if cargo audit; then
        echo -e "${GREEN}✅ Security audit passed${NC}"
    else
        echo -e "${YELLOW}⚠️  Security audit found issues (not failing build)${NC}"
    fi
else
    echo -e "${YELLOW}⚠️  cargo-audit not installed, skipping security audit${NC}"
    echo -e "${YELLOW}💡 Install with: cargo install cargo-audit${NC}"
fi

echo -e "\n${GREEN}🎉 All checks passed!${NC}" 