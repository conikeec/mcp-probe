#!/bin/bash
set -euo pipefail

# MCP Probe installer script
# Usage: curl -fsSL https://raw.githubusercontent.com/conikeec/mcp-probe/master/install.sh | bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# GitHub repository
REPO="conikeec/mcp-probe"
GITHUB_API="https://api.github.com/repos/$REPO"
GITHUB_RELEASES="https://github.com/$REPO/releases"

# Default installation directory
DEFAULT_INSTALL_DIR="/usr/local/bin"

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to detect platform
detect_platform() {
    local platform=""
    local arch=""
    
    # Detect OS
    case "$(uname -s)" in
        Linux*)
            platform="unknown-linux-gnu"
            ;;
        Darwin*)
            platform="apple-darwin"
            ;;
        CYGWIN*|MINGW*|MSYS*)
            platform="pc-windows-msvc"
            ;;
        *)
            print_error "Unsupported operating system: $(uname -s)"
            exit 1
            ;;
    esac
    
    # Detect architecture
    case "$(uname -m)" in
        x86_64|amd64)
            arch="x86_64"
            ;;
        aarch64|arm64)
            arch="aarch64"
            ;;
        *)
            print_error "Unsupported architecture: $(uname -m)"
            exit 1
            ;;
    esac
    
    echo "${arch}-${platform}"
}

# Function to get latest release version
get_latest_version() {
    print_status "Fetching latest release information..."
    
    if command -v curl >/dev/null 2>&1; then
        curl -s "$GITHUB_API/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- "$GITHUB_API/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
    else
        print_error "Neither curl nor wget is available. Please install one of them."
        exit 1
    fi
}

# Function to download and extract binary
download_and_install() {
    local version="$1"
    local platform="$2"
    local install_dir="$3"
    
    # Determine file extension based on platform
    local file_ext=""
    local extract_cmd=""
    
    if [[ "$platform" == *"windows"* ]]; then
        file_ext="zip"
        extract_cmd="unzip -q"
    else
        file_ext="tar.gz"
        extract_cmd="tar -xzf"
    fi
    
    local filename="mcp-probe-${platform}.${file_ext}"
    local download_url="$GITHUB_RELEASES/download/$version/$filename"
    local temp_dir=$(mktemp -d)
    
    print_status "Downloading $filename..."
    
    # Download the binary
    if command -v curl >/dev/null 2>&1; then
        if ! curl -L -o "$temp_dir/$filename" "$download_url"; then
            print_error "Failed to download $filename"
            exit 1
        fi
    elif command -v wget >/dev/null 2>&1; then
        if ! wget -O "$temp_dir/$filename" "$download_url"; then
            print_error "Failed to download $filename"
            exit 1
        fi
    fi
    
    print_status "Extracting binary..."
    
    # Extract the binary
    cd "$temp_dir"
    if ! $extract_cmd "$filename"; then
        print_error "Failed to extract $filename"
        exit 1
    fi
    
    # Determine binary name
    local binary_name="mcp-probe"
    if [[ "$platform" == *"windows"* ]]; then
        binary_name="mcp-probe.exe"
    fi
    
    # Make binary executable
    chmod +x "$binary_name"
    
    # Install binary
    if [[ -w "$install_dir" ]]; then
        print_status "Installing to $install_dir..."
        mv "$binary_name" "$install_dir/"
    else
        print_status "Installing to $install_dir (requires sudo)..."
        sudo mv "$binary_name" "$install_dir/"
    fi
    
    # Cleanup
    cd - >/dev/null
    rm -rf "$temp_dir"
    
    print_success "mcp-probe installed successfully!"
}

# Function to verify installation
verify_installation() {
    local install_dir="$1"
    local binary_path="$install_dir/mcp-probe"
    
    if [[ -x "$binary_path" ]]; then
        print_status "Verifying installation..."
        local version_output
        if version_output=$("$binary_path" --version 2>&1); then
            print_success "Installation verified: $version_output"
            return 0
        else
            print_warning "Binary installed but --version failed. Installation may be incomplete."
            return 1
        fi
    else
        print_error "Binary not found at $binary_path"
        return 1
    fi
}

# Function to show usage information
show_usage() {
    cat << EOF
🚀 MCP Probe Installer

This script installs the latest version of mcp-probe, a powerful CLI debugger 
for Model Context Protocol (MCP) servers.

Usage:
  curl -fsSL https://raw.githubusercontent.com/conikeec/mcp-probe/master/install.sh | bash
  
  # Or with custom install directory:
  curl -fsSL https://raw.githubusercontent.com/conikeec/mcp-probe/master/install.sh | INSTALL_DIR=~/.local/bin bash

Environment Variables:
  INSTALL_DIR    Installation directory (default: /usr/local/bin)
  VERSION        Specific version to install (default: latest)

Examples:
  # Install latest version to /usr/local/bin
  curl -fsSL https://raw.githubusercontent.com/conikeec/mcp-probe/master/install.sh | bash
  
  # Install to custom directory
  curl -fsSL https://raw.githubusercontent.com/conikeec/mcp-probe/master/install.sh | INSTALL_DIR=~/.local/bin bash
  
  # Install specific version
  curl -fsSL https://raw.githubusercontent.com/conikeec/mcp-probe/master/install.sh | VERSION=v0.1.55 bash

After installation, run:
  mcp-probe --help

EOF
}

# Main installation function
main() {
    echo "🚀 MCP Probe Installer"
    echo "======================================"
    
    # Check if help is requested
    if [[ "${1:-}" == "--help" ]] || [[ "${1:-}" == "-h" ]]; then
        show_usage
        exit 0
    fi
    
    # Get installation directory
    local install_dir="${INSTALL_DIR:-$DEFAULT_INSTALL_DIR}"
    
    # Validate installation directory
    if [[ ! -d "$install_dir" ]]; then
        print_error "Installation directory does not exist: $install_dir"
        print_status "Please create the directory or choose a different one:"
        print_status "  INSTALL_DIR=/path/to/dir curl -fsSL ... | bash"
        exit 1
    fi
    
    # Detect platform
    print_status "Detecting platform..."
    local platform
    platform=$(detect_platform)
    print_status "Detected platform: $platform"
    
    # Get version to install
    local version="${VERSION:-}"
    if [[ -z "$version" ]]; then
        version=$(get_latest_version)
        if [[ -z "$version" ]]; then
            print_error "Failed to get latest version information"
            exit 1
        fi
    fi
    print_status "Installing version: $version"
    
    # Check if already installed
    if command -v mcp-probe >/dev/null 2>&1; then
        local current_version
        current_version=$(mcp-probe --version 2>/dev/null | head -1 || echo "unknown")
        print_warning "mcp-probe is already installed: $current_version"
        print_status "Continuing with installation..."
    fi
    
    # Download and install
    download_and_install "$version" "$platform" "$install_dir"
    
    # Verify installation
    if verify_installation "$install_dir"; then
        echo ""
        print_success "🎉 mcp-probe has been installed successfully!"
        echo ""
        echo "📖 Quick start:"
        echo "  mcp-probe --help"
        echo "  mcp-probe debug --http-sse http://localhost:3000/sse"
        echo ""
        echo "📚 Documentation: https://github.com/$REPO"
        echo ""
    else
        print_error "Installation verification failed"
        exit 1
    fi
}

# Run main function with all arguments
main "$@" 