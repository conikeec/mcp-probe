#!/bin/bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if we have required tools
check_dependencies() {
    if ! command -v git >/dev/null 2>&1; then
        echo -e "${RED}❌ git is required but not installed${NC}"
        exit 1
    fi
    
    if ! command -v cargo >/dev/null 2>&1; then
        echo -e "${RED}❌ cargo is required but not installed${NC}"
        exit 1
    fi
}

# Function to get version from Cargo.toml
get_version() {
    grep '^version' Cargo.toml | head -1 | sed 's/version = "//;s/"//'
}

# Check if we're on a clean git state
check_git_clean() {
    if [ -n "$(git status --porcelain)" ]; then
        echo -e "${RED}❌ Git working directory is not clean. Please commit or stash changes.${NC}"
        exit 1
    fi
}

# Run all checks
run_checks() {
    echo -e "${BLUE}🔍 Running pre-release checks...${NC}"
    
    # Run the check script
    if ! ./scripts/check.sh; then
        echo -e "${RED}❌ Pre-release checks failed. Please fix issues before releasing.${NC}"
        exit 1
    fi
}

# Publish packages
publish_packages() {
    local version="$1"
    
    echo -e "\n${BLUE}📦 Publishing packages to crates.io...${NC}"
    
    # Publish mcp-core first (dependency of mcp-cli)
    echo -e "\n${YELLOW}📦 Publishing mcp-core...${NC}"
    (cd crates/mcp-core && cargo publish)
    
    # Wait a bit for crates.io to index the package
    echo -e "\n${YELLOW}⏳ Waiting 30 seconds for crates.io to index mcp-core...${NC}"
    sleep 30
    
    # Publish mcp-cli
    echo -e "\n${YELLOW}📦 Publishing mcp-cli...${NC}"
    (cd crates/mcp-cli && cargo publish)
    
    echo -e "\n${GREEN}✅ Packages published successfully!${NC}"
}

# Create git tag
create_tag() {
    local version="$1"
    
    echo -e "\n${BLUE}🏷️  Creating git tag...${NC}"
    git tag -a "v$version" -m "Release v$version"
    git push origin "v$version"
    echo -e "${GREEN}✅ Tag v$version created and pushed${NC}"
}

# Main function
main() {
    echo -e "${BLUE}🚀 Starting release process...${NC}"
    
    check_dependencies
    check_git_clean
    
    # Get current version
    local version
    version=$(get_version)
    echo -e "\n${BLUE}📋 Current version: $version${NC}"
    
    # Ask for confirmation
    echo -e "\n${YELLOW}❓ Are you sure you want to release version $version? (y/N)${NC}"
    read -r confirmation
    if [[ ! "$confirmation" =~ ^[Yy]$ ]]; then
        echo -e "${YELLOW}🚫 Release cancelled${NC}"
        exit 0
    fi
    
    # Check if CRATES_IO_TOKEN is set
    if [ -z "${CRATES_IO_TOKEN:-}" ]; then
        echo -e "\n${YELLOW}⚠️  CRATES_IO_TOKEN environment variable not set${NC}"
        echo -e "${YELLOW}💡 Please set it with: export CRATES_IO_TOKEN=your_token${NC}"
        echo -e "${YELLOW}📖 Get your token from: https://crates.io/me${NC}"
        exit 1
    fi
    
    run_checks
    publish_packages "$version"
    create_tag "$version"
    
    echo -e "\n${GREEN}🎉 Release v$version completed successfully!${NC}"
    echo -e "${BLUE}📖 View the release at: https://crates.io/crates/mcp-cli${NC}"
    echo -e "${BLUE}📖 And: https://crates.io/crates/mcp-core${NC}"
}

# Show help
show_help() {
    echo "Usage: $0 [options]"
    echo ""
    echo "Release script for mcp-probe to crates.io"
    echo ""
    echo "Options:"
    echo "  -h, --help    Show this help message"
    echo ""
    echo "Prerequisites:"
    echo "  - Set CRATES_IO_TOKEN environment variable"
    echo "  - Clean git working directory"
    echo "  - All tests and checks passing"
    echo ""
    echo "This script will:"
    echo "  1. Run all pre-release checks (format, clippy, tests)"
    echo "  2. Publish mcp-core to crates.io"
    echo "  3. Publish mcp-cli to crates.io"
    echo "  4. Create and push a git tag"
}

# Parse command line arguments
case "${1:-}" in
    -h|--help)
        show_help
        exit 0
        ;;
    "")
        main
        ;;
    *)
        echo -e "${RED}❌ Unknown option: $1${NC}"
        show_help
        exit 1
        ;;
esac 