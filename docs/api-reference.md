---
layout: page
title: API Reference
permalink: /api-reference/
---

# API Reference

Complete API reference for MCP Probe SDK and command-line interface.

## 📚 SDK Reference

### Core Types

#### `McpClient`
Main client for MCP protocol communication.

```rust
pub struct McpClient {
    transport: Box<dyn Transport>,
    client_info: Implementation,
    server_info: Option<ServerInfo>,
}

impl McpClient {
    pub async fn connect(&mut self, client_info: Implementation) -> McpResult<ServerInfo>;
    pub async fn list_tools(&mut self) -> McpResult<Vec<Tool>>;
    pub async fn call_tool(&mut self, name: &str, params: Value) -> McpResult<CallToolResult>;
    pub async fn list_resources(&mut self) -> McpResult<Vec<Resource>>;
    pub async fn read_resource(&mut self, uri: &str) -> McpResult<ReadResourceResult>;
    pub async fn list_prompts(&mut self) -> McpResult<Vec<Prompt>>;
}
```

## 🔧 CLI Reference

### Global Options

| Option | Short | Description | Default |
|--------|--------|-------------|---------|
| `--verbose` | `-v` | Enable verbose logging | `false` |
| `--no-color` | | Disable colored output | `false` |
| `--config <FILE>` | `-c` | Configuration file path | `~/.mcp-probe/config/` |
| `--help` | `-h` | Show help information | |
| `--version` | `-V` | Show version information | |

### Commands

#### `debug`
Interactive and non-interactive debugging of MCP servers.

**Usage:**
```bash
mcp-probe debug [OPTIONS] <TRANSPORT_OPTIONS>
```

#### `test`
Automated testing of MCP server compliance.

**Usage:**
```bash
mcp-probe test [OPTIONS] <TRANSPORT_OPTIONS>
```

#### `validate`
Validate MCP server against protocol specifications.

**Usage:**
```bash
mcp-probe validate [OPTIONS] <TRANSPORT_OPTIONS>
```

#### `export`
Export MCP server capabilities and session data.

**Usage:**
```bash
mcp-probe export [OPTIONS] <TRANSPORT_OPTIONS>
```

For detailed documentation, see our [Documentation page](documentation.html).

---

*For more examples and detailed usage, see our [Examples page](examples.html) and [Documentation](documentation.html).*
