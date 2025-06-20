#!/bin/bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔧 Fixing code issues...${NC}"

# Fix formatting
echo -e "\n${YELLOW}📝 Formatting code with rustfmt...${NC}"
cargo fmt --all
echo -e "${GREEN}✅ Code formatted${NC}"

# Fix clippy issues (where possible)
echo -e "\n${YELLOW}🔧 Fixing clippy issues...${NC}"
cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged
echo -e "${GREEN}✅ Clippy fixes applied${NC}"

# Run a quick check to see if there are remaining issues
echo -e "\n${YELLOW}🔍 Checking for remaining issues...${NC}"

if cargo clippy --all-targets --all-features -- -D warnings >/dev/null 2>&1; then
    echo -e "${GREEN}✅ No remaining clippy issues${NC}"
else
    echo -e "${YELLOW}⚠️  Some clippy issues remain (may require manual fixing)${NC}"
    echo -e "${YELLOW}💡 Run 'cargo clippy --all-targets --all-features' to see details${NC}"
fi

echo -e "\n${GREEN}🎉 Auto-fixes completed!${NC}"
echo -e "${BLUE}💡 Run './scripts/check.sh' to verify all checks pass${NC}" 