# mcp-cli

[![Crates.io](https://img.shields.io/crates/v/mcp-cli.svg)](https://crates.io/crates/mcp-cli)
[![Documentation](https://docs.rs/mcp-cli/badge.svg)](https://docs.rs/mcp-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Interactive CLI debugger and TUI for MCP (Model Context Protocol) servers.

## Overview

`mcp-cli` provides a powerful terminal-based interface for debugging and testing MCP servers. It features:

- **Interactive TUI**: Beautiful terminal interface built with Ratatui
- **Real-time Discovery**: Automatic discovery of tools, resources, and prompts
- **Advanced Search**: Fuzzy search across all capabilities with instant results
- **Response Viewer**: Multiple view modes for inspecting server responses
- **Parameter Forms**: Dynamic forms for tool/prompt parameter input
- **Session Management**: Automatic session handling and reconnection

## Installation

### From crates.io

```bash
cargo install mcp-cli
```

### From source

```bash
git clone https://github.com/conikeec/mcp-probe
cd mcp-probe
cargo install --path crates/mcp-cli
```

## Usage

The CLI tool is named `mcp-probe` and provides multiple commands:

### Debug Mode (TUI)

Launch the interactive TUI for debugging an MCP server:

```bash
# Connect to stdio server
mcp-probe debug --stdio python -- -m my_mcp_server

# Connect to HTTP SSE server
mcp-probe debug --http-sse http://localhost:3000/sse

# Connect to HTTP streaming server
mcp-probe debug --http-stream http://localhost:3000/stream
```

### Export Configuration

Export discovered capabilities to various formats:

```bash
# Export to JSON
mcp-probe export --stdio python -- -m my_mcp_server --format json --output capabilities.json

# Export to YAML
mcp-probe export --stdio python -- -m my_mcp_server --format yaml --output capabilities.yaml
```

### Validate Server

Validate an MCP server implementation:

```bash
mcp-probe validate --stdio python -- -m my_mcp_server
```

## TUI Features

### Navigation

- **Tab**: Switch between panels
- **↑/↓**: Navigate lists and menus
- **Enter**: Select/activate items
- **Esc**: Go back or close dialogs
- **Q**: Quit application

### Capabilities Browser

- **Categories View**: Browse tools (🔧), resources (📁), and prompts (💬)
- **Detailed Lists**: Paginated view of capabilities with descriptions
- **Parameter Detection**: Automatic detection of required/optional parameters

### Advanced Search

- **Instant Search**: Press `/` to open search, results appear as you type
- **Fuzzy Matching**: Handles typos and partial matches
- **Multiple Criteria**: Search by name, description, keywords
- **Relevance Scoring**: Results ranked by match quality

### Response Viewer

- **Multiple View Modes**: Formatted, Raw JSON, Tree View, Summary
- **Scrolling Support**: Full horizontal and vertical scrolling
- **Response History**: View responses from previous tool calls
- **Quick Access**: Press `R` to view latest response

### Parameter Forms

- **Dynamic Forms**: Automatically generated from tool/prompt schemas
- **Type Validation**: Input validation based on parameter types
- **Required/Optional**: Clear indication of required vs optional fields
- **Auto-edit Mode**: Start typing immediately without pressing Enter

## Configuration

Create a configuration file at `~/.config/mcp-probe/config.yaml`:

```yaml
# Default transport configuration
transport:
  type: "stdio"
  command: "python"
  args: ["-m", "my_mcp_server"]
  working_dir: "/path/to/server"

# Environment variables
environment:
  DEBUG: "true"
  API_KEY: "your-api-key"

# UI preferences
ui:
  theme: "dark"
  page_size: 10
  auto_scroll: true

# Logging configuration
logging:
  level: "info"
  file: "mcp-probe.log"
```

## Examples

### Testing a GitHub MCP Server

```bash
# Connect to GitHub MCP server
mcp-probe debug --stdio npx -- -y @modelcontextprotocol/server-github

# In the TUI:
# 1. Press '/' to search
# 2. Type "repo" to find repository-related tools
# 3. Select a tool and fill in parameters
# 4. Press Tab to execute
# 5. Press 'R' to view the response
```

### Debugging a Custom Server

```bash
# Connect with debug logging
RUST_LOG=debug mcp-probe debug --stdio python -- -m my_server --debug

# The TUI will show:
# - All discovered capabilities
# - Real-time protocol messages
# - Detailed error information
# - Session state tracking
```

## Development

### Building from Source

```bash
git clone https://github.com/conikeec/mcp-probe
cd mcp-probe
cargo build --release
```

### Running Tests

```bash
cargo test --all-features
```

### Code Formatting

```bash
cargo fmt --all
```

### Linting

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

## Contributing

Contributions are welcome! Please see the [main repository](https://github.com/conikeec/mcp-probe) for contribution guidelines.

## License

This project is licensed under the MIT License - see the [LICENSE](../../LICENSE) file for details.
