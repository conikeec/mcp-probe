---
layout: page
title: Getting Started with MCP Probe
permalink: /getting-started/
---

# Getting Started with MCP Probe

Welcome to MCP Probe! This guide will help you install and start using MCP Probe to debug and test Model Context Protocol (MCP) servers.

## 📦 Installation

### Option 1: Homebrew (Recommended for macOS/Linux)

```bash
# Add the tap and install
brew install conikeec/tap/mcp-probe

# Verify installation
mcp-probe --version
```

### Option 2: Cargo (Rust Package Manager)

```bash
# Install from crates.io
cargo install mcp-cli

# Verify installation
mcp-probe --version
```

### Option 3: Pre-built Binaries

1. Visit the [releases page](https://github.com/conikeec/mcp-probe/releases/latest)
2. Download the binary for your platform:
   - **Linux**: `mcp-probe-x86_64-unknown-linux-gnu.tar.gz`
   - **macOS (Intel)**: `mcp-probe-x86_64-apple-darwin.tar.gz`
   - **macOS (Apple Silicon)**: `mcp-probe-aarch64-apple-darwin.tar.gz`
   - **Windows**: `mcp-probe-x86_64-pc-windows-msvc.zip`
3. Extract and add to your PATH

### Option 4: One-liner Install Script

```bash
# Linux/macOS
curl -fsSL https://raw.githubusercontent.com/conikeec/mcp-probe/main/install.sh | bash
```

## 🚀 Your First MCP Debug Session

Let's start with a simple example using the Playwright MCP server:

### Step 1: Install Prerequisites

For the Playwright example, you'll need Node.js:

```bash
# macOS
brew install node

# Ubuntu/Debian
sudo apt install nodejs npm

# Windows
# Download from https://nodejs.org
```

### Step 2: Run Your First Debug Session

```bash
# Debug the Playwright MCP server
mcp-probe debug --stdio npx @playwright/mcp@latest
```

You should see output like:

```
🔍 MCP Probe - Non-Interactive Debug Mode
🔌 Transport: stdio
📡 Client: mcp-probe v0.2.4

✅ Connected to MCP server successfully!

🛠️  Server Capabilities:
📋 Tools (25):
  → browser_navigate - Navigate to a URL
  → browser_click - Perform click on a web page
  → browser_type - Type text into editable element
  ... (and 22 more tools)
📁 Resources (0):
💬 Prompts (0):

✅ Debug session completed successfully!
```

### Step 3: Try Interactive Mode

For a richer debugging experience, run without `--non-interactive`:

```bash
mcp-probe debug --stdio npx @playwright/mcp@latest
```

This launches the interactive TUI where you can:
- Browse tools, resources, and prompts
- Call tools with parameters
- View real-time message logs
- Export session data

## 🛠️ Common Usage Patterns

### Debug Different Transport Types

**Stdio (Local Process)**
```bash
# Python MCP server
mcp-probe debug --stdio python --args my_server.py

# Node.js MCP server
mcp-probe debug --stdio node --args server.js

# Custom command with arguments
mcp-probe debug --stdio npx --args @my-org/mcp-server@latest
```

**HTTP+SSE (Server-Sent Events)**
```bash
# Debug HTTP+SSE server
mcp-probe debug --http-sse http://localhost:3000/sse

# With authentication
mcp-probe debug --http-sse https://api.example.com/mcp \
  --auth-header "Bearer your-token-here"
```

**HTTP Streaming**
```bash
# Debug HTTP streaming server
mcp-probe debug --http-stream http://localhost:3000/stream
```

### Run Automated Tests

```bash
# Basic test run
mcp-probe test --stdio python server.py

# Generate detailed report
mcp-probe test --stdio python server.py --report --output-dir ./test-reports

# Run specific test suite
mcp-probe test --stdio python server.py --suite tools-validation
```

### Export Server Capabilities

```bash
# Export as JSON
mcp-probe export --stdio python server.py --format json --output capabilities.json

# Export as Markdown report
mcp-probe export --stdio python server.py --format markdown --output server-docs.md

# Export as HTML
mcp-probe export --stdio python server.py --format html --output report.html
```

## 📁 File Organization

MCP Probe automatically organizes files in `~/.mcp-probe/`:

```
~/.mcp-probe/
├── logs/           # Date-timestamped log files
├── reports/        # Generated reports with date prefixes
├── sessions/       # Saved debug sessions
└── config/         # Configuration files
```

### View and Manage Files

```bash
# Show directory structure and usage
mcp-probe paths show

# Clean up old files (dry run)
mcp-probe paths cleanup --days 30

# Actually perform cleanup
mcp-probe paths cleanup --days 30 --force

# Open in file manager
mcp-probe paths open
```

## ⚙️ Configuration

### Create Configuration File

```bash
# Generate initial config
mcp-probe config init --template full --output ~/.mcp-probe/config/default.toml

# Validate configuration
mcp-probe config validate ~/.mcp-probe/config/default.toml

# Show current configuration
mcp-probe config show
```

### Example Configuration

```toml
# ~/.mcp-probe/config/default.toml
[defaults]
transport = "stdio"
timeout = 30
max_retries = 3

[stdio]
working_dir = "/path/to/mcp/servers"
environment = { "DEBUG" = "1", "LOG_LEVEL" = "info" }

[http]
headers = { "User-Agent" = "mcp-probe/0.2.4" }
timeout = 60

[output]
format = "pretty"
include_timing = true
```

## 🔧 Advanced Usage

### Environment Variables

```bash
# Enable debug logging
export RUST_LOG=debug
mcp-probe debug --stdio python server.py

# Custom MCP Probe home directory
export MCP_PROBE_HOME=/custom/path
mcp-probe paths show
```

### Session Management

```bash
# Save a session
mcp-probe debug --stdio python server.py --save-session debug-session-1

# Replay a session
mcp-probe debug --replay-session ~/.mcp-probe/sessions/debug-session-1.json

# Export session data
mcp-probe export session.json --format json --include-raw --include-timing
```

### Validation Rules

```bash
# Run with specific validation rules
mcp-probe validate --stdio python server.py \
  --rules schema-validation,tool-parameters,resource-uris \
  --severity error \
  --report validation-report.md
```

## 🏃‍♂️ Quick Reference

### Most Common Commands

```bash
# Quick debug (non-interactive)
mcp-probe debug --non-interactive --stdio COMMAND

# Interactive debugging
mcp-probe debug --stdio COMMAND

# Run tests
mcp-probe test --stdio COMMAND --report

# Export capabilities
mcp-probe export --stdio COMMAND --format json

# Show help
mcp-probe --help
mcp-probe debug --help
```

### Keyboard Shortcuts (Interactive Mode)

| Key | Action |
|-----|--------|
| `q` | Quit |
| `h` | Help |
| `Tab` | Switch panels |
| `Enter` | Execute/Select |
| `↑↓` | Navigate lists |
| `Ctrl+C` | Force quit |

## 🔍 Troubleshooting

### Common Issues

**Connection Timeouts**
```bash
# Increase timeout
mcp-probe debug --stdio python server.py --timeout 60

# Check server logs
mcp-probe debug --stdio python server.py --show-raw
```

**Permission Errors**
```bash
# Ensure script is executable
chmod +x your_server_script.py

# Check working directory
mcp-probe debug --stdio python server.py --working-dir /correct/path
```

**Port Already in Use**
```bash
# For HTTP transports, ensure port is available
lsof -i :3000
```

### Debug Logging

```bash
# Enable verbose logging
RUST_LOG=debug mcp-probe debug --stdio python server.py 2>&1 | tee debug.log

# View structured logs
jq . ~/.mcp-probe/logs/mcp-probe-$(date +%Y%m%d)*.log
```

### Getting Help

- **Documentation**: [Read the full documentation](../documentation/)
- **Examples**: [View usage examples](../examples/)
- **Issues**: [Report bugs on GitHub](https://github.com/conikeec/mcp-probe/issues)
- **Discussions**: [Join GitHub Discussions](https://github.com/conikeec/mcp-probe/discussions)

## 🎯 Next Steps

Now that you have MCP Probe installed and working:

1. **[Explore Examples](../examples/)** - See real-world usage scenarios
2. **[Read Documentation](../documentation/)** - Learn about advanced features
3. **[API Reference](../api-reference/)** - Dive into the technical details
4. **[Contributing](../contributing/)** - Help improve MCP Probe

---

**Need more help?** Check out our [examples page](../examples/) for detailed walkthroughs of common scenarios! 