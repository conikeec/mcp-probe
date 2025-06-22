---
layout: page
title: Documentation
permalink: /documentation/
---

# MCP Probe Documentation

Complete reference guide for MCP Probe features, configuration, and advanced usage.

## 📖 Overview

MCP Probe is a comprehensive debugging and testing tool for Model Context Protocol (MCP) servers. This documentation covers all aspects of using MCP Probe effectively.

## 🚀 Quick Navigation

- **[Getting Started](../getting-started/)** - Installation and first steps
- **[Examples](../examples/)** - Real-world usage scenarios  
- **[API Reference](../api-reference/)** - Complete CLI and SDK reference
- **[Contributing](../contributing/)** - Help improve MCP Probe

## 🔧 Core Features

### Interactive Debugging
- **TUI Interface** - Rich terminal interface for real-time debugging
- **Message Inspection** - View raw protocol messages and responses
- **Session Management** - Save, replay, and share debug sessions

### Automated Testing  
- **Protocol Compliance** - Comprehensive MCP specification testing
- **CI/CD Integration** - Automated testing in build pipelines
- **Custom Test Suites** - Target specific functionality areas

### Validation & Export
- **Parameter Validation** - Auto-fix common parameter issues
- **Capability Export** - Generate documentation in multiple formats
- **Report Generation** - HTML, Markdown, JSON, and YAML reports

## 🔌 Transport Support

### Stdio Transport
Local process communication via stdin/stdout pipes.

```bash
mcp-probe debug --stdio python server.py
```

### HTTP+SSE Transport  
HTTP requests with Server-Sent Events for real-time updates.

```bash
mcp-probe debug --http-sse http://localhost:3000/sse
```

### HTTP Streaming Transport
Full-duplex HTTP streaming communication.

```bash
mcp-probe debug --http-stream http://localhost:3000/stream
```

## ⚙️ Configuration

MCP Probe uses TOML configuration files for flexible setup:

```toml
[defaults]
transport = "stdio"
timeout = 30
max_retries = 3

[stdio]
working_dir = "/path/to/servers"
environment = { "DEBUG" = "1" }

[validation]
rules = ["schema-validation", "tool-parameters"]
severity = "warning"
```

## 📁 File Organization

Automatic file organization in `~/.mcp-probe/`:

```
~/.mcp-probe/
├── logs/           # Timestamped log files
├── reports/        # Generated reports with date prefixes
├── sessions/       # Saved debug sessions
└── config/         # Configuration files
```

## 🧪 Testing Framework

### Test Suites
- **Connection Tests** - Handshake and connection validation
- **Tool Tests** - Tool listing and parameter validation
- **Resource Tests** - Resource access and permissions
- **Protocol Tests** - General MCP compliance

### Validation Rules
- **Schema Validation** - JSON schema compliance
- **Parameter Validation** - Type checking and format validation
- **URI Validation** - Resource URI format validation

## 🔍 Troubleshooting

### Common Issues

**Connection Timeouts**
```bash
# Increase timeout
mcp-probe debug --stdio python server.py --timeout 60

# Enable debug logging
RUST_LOG=debug mcp-probe debug --stdio python server.py
```

**Permission Errors**
```bash
# Check file permissions
chmod +x server.py

# Verify working directory
mcp-probe debug --stdio python server.py --working-dir /correct/path
```

### Debug Logging

Enable detailed logging for troubleshooting:

```bash
# Debug level
RUST_LOG=debug mcp-probe debug --stdio python server.py

# Trace level (very verbose)
RUST_LOG=trace mcp-probe debug --stdio python server.py
```

## 📊 Performance

### Optimization Tips
- Use `--non-interactive` mode for automation
- Enable compression for HTTP transports  
- Limit session duration with `--timeout`
- Use specific test suites instead of `all`

### Benchmarking
```bash
# Time operations
time mcp-probe debug --non-interactive --stdio python server.py

# Memory usage analysis
/usr/bin/time -v mcp-probe debug --stdio python server.py
```

## 🔗 Integration

### CI/CD Pipelines
```yaml
# GitHub Actions example
- name: Test MCP Server
  run: |
    mcp-probe test --stdio python server.py \
      --report --output-dir ./test-reports
```

### Docker Integration
```dockerfile
FROM rust:1.75
RUN cargo install mcp-cli
WORKDIR /workspace
CMD ["mcp-probe", "--help"]
```

## 📚 Additional Resources

- **[MCP Specification](https://spec.modelcontextprotocol.io/)** - Official protocol documentation
- **[GitHub Repository](https://github.com/conikeec/mcp-probe)** - Source code and issues
- **[Examples Repository](https://github.com/modelcontextprotocol/servers)** - MCP server examples

---

*This documentation is for MCP Probe v0.2.4. For the latest updates, visit our [GitHub repository](https://github.com/conikeec/mcp-probe).*
