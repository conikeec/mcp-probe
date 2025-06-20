# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Automated release builds with GitHub Actions
- Binary distributions for Linux, macOS, and Windows
- Homebrew formula for easy installation on macOS/Linux
- Curl install script for one-line installation
- Comprehensive CI/CD pipeline with rustfmt, clippy, and tests
- Automated crates.io publishing

### Changed

- Improved release process with automated version management

## [0.1.0] - 2025-01-03

### Added

- **Interactive TUI**: Beautiful terminal interface built with Ratatui
- **Real-time Discovery**: Automatic discovery of tools, resources, and prompts from MCP servers
- **Advanced Search**: Fuzzy search across all capabilities with instant results and relevance scoring
- **Response Viewer**: Multiple view modes (Formatted, Raw JSON, Tree View, Summary) for inspecting server responses
- **Parameter Forms**: Dynamic forms for tool execution with auto-edit mode
- **Comprehensive Scrolling**: Full scrollbar support for all UI panels
- **Clean Tool Names**: Automatic prefix stripping for better UX while maintaining API compatibility
- **Session Management**: Proper MCP session handling with session ID tracking
- **Multiple Transport Types**: Support for stdio, HTTP SSE, and HTTP streaming
- **Tool Execution**: Execute tools with proper parameter handling and response formatting
- **Message History**: Complete message log with timestamps and response indicators
- **Environment Variables**: Configurable environment variables panel
- **Export Functionality**: Export discovered capabilities to JSON format
- **Validation**: Comprehensive parameter and response validation
- **Error Handling**: Detailed error messages with troubleshooting suggestions

### Technical Features

- **Transport Layer**: HTTP SSE transport with proper session management
- **Protocol Support**: Full MCP protocol implementation with initialization, tools, resources, and prompts
- **Search Engine**: In-memory indexing with multiple matching algorithms (exact, prefix, contains, description, token, keyword, fuzzy)
- **Response Parsing**: Robust parsing that handles both MCP and non-MCP format responses
- **Parameter Detection**: Fixed critical bug in parameter schema detection (inputSchema vs parametersSchema)
- **Tool Name Handling**: Dual naming system (clean display names vs full API names)

### User Experience

- **Enhanced Navigation**: Improved panel focus management and keyboard controls
- **Visual Indicators**: Clear visual feedback for selected items, success/error states
- **Help System**: Comprehensive help documentation accessible via F1
- **Keyboard Shortcuts**: Full keyboard navigation support
- **Auto-focus**: Smart focus management for optimal user workflow
- **Message Types**: Clear distinction between success and error messages

### Performance

- **Efficient Search**: Fast in-memory search across 373+ tools
- **Responsive UI**: Smooth scrolling and navigation
- **Memory Management**: Efficient capability caching and indexing

### Infrastructure

- **CI/CD**: GitHub Actions for testing, linting, and building
- **Code Quality**: rustfmt and clippy integration with warnings as errors
- **Testing**: Comprehensive test suite with multiple platform support
- **Documentation**: Complete API documentation and user guides

[unreleased]: https://github.com/conikeec/mcp-probe/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/conikeec/mcp-probe/releases/tag/v0.1.0
