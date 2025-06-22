#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_step() {
    echo -e "${BLUE}🔄 $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️ $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Help function
show_help() {
    cat << EOF
Usage: $0 <version_type> [options]

Version Types:
  major    - Bump major version (1.0.0 -> 2.0.0)
  minor    - Bump minor version (1.0.0 -> 1.1.0)  
  patch    - Bump patch version (1.0.0 -> 1.0.1)
  <version> - Set specific version (e.g. 1.2.3)

Options:
  --dry-run    - Show what would be changed without making changes
  --no-commit  - Update files but don't commit changes
  --no-tag     - Don't create git tag
  --help       - Show this help message

Examples:
  $0 patch                    # Bump patch version and commit + tag
  $0 minor --dry-run          # Show what minor bump would do
  $0 1.5.0 --no-commit        # Set version to 1.5.0 without committing
  $0 major --no-tag           # Bump major version, commit but don't tag

EOF
}

# Parse arguments
VERSION_TYPE=""
DRY_RUN=false
NO_COMMIT=false
NO_TAG=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --no-commit)
            NO_COMMIT=true
            shift
            ;;
        --no-tag)
            NO_TAG=true
            shift
            ;;
        --help|-h)
            show_help
            exit 0
            ;;
        *)
            if [[ -z "$VERSION_TYPE" ]]; then
                VERSION_TYPE="$1"
            else
                print_error "Unknown option: $1"
                show_help
                exit 1
            fi
            shift
            ;;
    esac
done

# Validate arguments
if [[ -z "$VERSION_TYPE" ]]; then
    print_error "Version type is required"
    show_help
    exit 1
fi

# Get current version from workspace Cargo.toml
CURRENT_VERSION=$(grep -E "^version = " Cargo.toml | head -1 | sed 's/version = "//;s/"//')

if [[ -z "$CURRENT_VERSION" ]]; then
    print_error "Could not find current version in Cargo.toml"
    exit 1
fi

print_step "Current version: $CURRENT_VERSION"

# Calculate new version
calculate_new_version() {
    local current="$1"
    local bump_type="$2"
    
    # Split version into parts
    IFS='.' read -ra VERSION_PARTS <<< "$current"
    local major="${VERSION_PARTS[0]}"
    local minor="${VERSION_PARTS[1]}"
    local patch="${VERSION_PARTS[2]}"
    
    case "$bump_type" in
        major)
            echo "$((major + 1)).0.0"
            ;;
        minor)
            echo "$major.$((minor + 1)).0"
            ;;
        patch)
            echo "$major.$minor.$((patch + 1))"
            ;;
        *)
            # Assume it's a specific version
            if [[ "$bump_type" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
                echo "$bump_type"
            else
                print_error "Invalid version format: $bump_type"
                print_error "Use 'major', 'minor', 'patch', or a specific version like '1.2.3'"
                exit 1
            fi
            ;;
    esac
}

NEW_VERSION=$(calculate_new_version "$CURRENT_VERSION" "$VERSION_TYPE")
print_step "New version: $NEW_VERSION"

# Function to update workspace version
update_workspace_version() {
    local file="Cargo.toml"
    print_step "Updating workspace version in $file"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        echo "  Would change: version = \"$CURRENT_VERSION\" -> version = \"$NEW_VERSION\""
    else
        sed -i.bak "s/version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" "$file"
        rm "$file.bak"
    fi
}

# Function to update dependency version  
update_dependency_version() {
    local file="crates/mcp-cli/Cargo.toml"
    print_step "Updating mcp-probe-core dependency version in $file"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        echo "  Would change: mcp-probe-core = { version = \"$CURRENT_VERSION\" -> version = \"$NEW_VERSION\""
    else
        sed -i.bak "s/mcp-probe-core = { version = \"$CURRENT_VERSION\"/mcp-probe-core = { version = \"$NEW_VERSION\"/" "$file"
        rm "$file.bak"
    fi
}

# Update all files
update_workspace_version
update_dependency_version

# Verify changes if not dry run
if [[ "$DRY_RUN" == "false" ]]; then
    print_step "Verifying changes..."
    
    # Check workspace version
    NEW_WORKSPACE_VERSION=$(grep -E "^version = " Cargo.toml | head -1 | sed 's/version = "//;s/"//')
    if [[ "$NEW_WORKSPACE_VERSION" != "$NEW_VERSION" ]]; then
        print_error "Workspace version update failed: expected $NEW_VERSION, got $NEW_WORKSPACE_VERSION"
        exit 1
    fi
    
    # Check dependency version
    NEW_DEP_VERSION=$(grep "mcp-probe-core = { version = " crates/mcp-cli/Cargo.toml | sed 's/.*version = "//;s/".*//')
    if [[ "$NEW_DEP_VERSION" != "$NEW_VERSION" ]]; then
        print_error "Dependency version update failed: expected $NEW_VERSION, got $NEW_DEP_VERSION"
        exit 1
    fi
    
    print_success "All version updates verified successfully"
    
    # Test that the project builds
    print_step "Testing build with new version..."
    if ! cargo build --quiet; then
        print_error "Build failed with new version"
        exit 1
    fi
    print_success "Build successful with new version"
    
    # Run tests
    print_step "Running tests..."
    if ! cargo test --quiet; then
        print_error "Tests failed with new version"
        exit 1
    fi
    print_success "All tests passed"
fi

# Git operations
if [[ "$DRY_RUN" == "false" ]] && [[ "$NO_COMMIT" == "false" ]]; then
    print_step "Committing changes..."
    
    git add Cargo.toml crates/mcp-cli/Cargo.toml
    git commit -m "bump: update workspace version to $NEW_VERSION

- Update workspace version from $CURRENT_VERSION to $NEW_VERSION
- Update mcp-probe-core dependency version in mcp-cli
- Automated version bump via scripts/version-bump.sh

All tests passing with new version."
    
    print_success "Changes committed"
    
    if [[ "$NO_TAG" == "false" ]]; then
        print_step "Creating git tag v$NEW_VERSION..."
        
        git tag -a "v$NEW_VERSION" -m "Release v$NEW_VERSION

🚀 Automated version bump from $CURRENT_VERSION to $NEW_VERSION

This release maintains all existing functionality while updating
version numbers consistently across the workspace.

Generated by scripts/version-bump.sh"
        
        print_success "Git tag v$NEW_VERSION created"
        
        echo ""
        print_success "Version bump complete!"
        echo ""
        echo "Next steps:"
        echo "  1. Review changes: git show HEAD"
        echo "  2. Push changes: git push origin master"
        echo "  3. Push tag: git push origin v$NEW_VERSION"
        echo "  4. GitHub Actions will automatically publish to crates.io"
        echo ""
    fi
fi

if [[ "$DRY_RUN" == "true" ]]; then
    echo ""
    print_warning "DRY RUN - No changes were made"
    echo "To apply these changes, run: $0 $VERSION_TYPE"
fi 