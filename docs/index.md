---
layout: home
title: "MCP Probe - Production-grade MCP Debugger"
---

# MCP Probe
{: .text-center .hero-title}

**Production-grade Model Context Protocol debugger and client built in Rust**
{: .text-center .hero-subtitle}

<div class="text-center hero-buttons">
  <a href="{{ site.github.download_url }}" class="btn btn-primary">Download {{ site.current_version }}</a>
  <a href="getting-started.html" class="btn btn-secondary">Get Started</a>
  <a href="{{ site.github.repository_url }}" class="btn btn-outline">View on GitHub</a>
</div>

---

## What is MCP Probe?

MCP Probe is a powerful terminal-based debugger and CLI tool for **Model Context Protocol (MCP)** servers. It provides both an interactive TUI for real-time debugging and a comprehensive SDK for building MCP clients.

### 🎯 Perfect for:
- **MCP Server Developers** - Debug and validate your servers before deployment
- **LLM Application Builders** - Test MCP integrations with confidence  
- **DevOps Engineers** - Automate MCP server testing and monitoring
- **AI Researchers** - Explore and understand MCP protocol implementations

---

## 🚀 Quick Start

### Installation

{% for method in site.install_methods %}
**{{ method[0] | capitalize }}**
```bash
{{ method[1] }}
```
{% endfor %}

### Basic Usage

```bash
# Debug an MCP server via stdio
mcp-probe debug --stdio python server.py

# Debug via HTTP+SSE
mcp-probe debug --http-sse http://localhost:3000/sse

# Run automated tests
mcp-probe test --stdio python server.py

# Export capabilities
mcp-probe export --stdio python server.py --format json
```

---

## ✨ Key Features

<div class="features-grid">
{% for feature in site.features %}
<div class="feature-card">
  <h3>{{ feature.title }}</h3>
  <p>{{ feature.description }}</p>
</div>
{% endfor %}
</div>

### 🛠️ Comprehensive Toolset
- **Interactive TUI** - Rich terminal interface with real-time debugging
- **Non-interactive CLI** - Perfect for automation and CI/CD pipelines
- **Multi-transport Support** - stdio, HTTP+SSE, and HTTP streaming
- **Validation Engine** - Comprehensive MCP protocol compliance testing
- **Export Capabilities** - Generate reports in JSON, YAML, Markdown, and HTML
- **Session Management** - Save, replay, and share debugging sessions

### 🔧 Developer Experience
- **Smart Parameter Validation** - Auto-fix common issues (e.g., URL prefixing)
- **File Organization** - Automatic date-prefixed reports and organized logs
- **Version Management** - Automated release workflows and consistent versioning
- **Cross-platform** - Works seamlessly on Linux, macOS, and Windows

---

## 🏃‍♂️ Quick Demo

Debug the Playwright MCP server in seconds:

```bash
# Install MCP Probe
cargo install mcp-cli

# Debug Playwright MCP server
mcp-probe debug --stdio npx @playwright/mcp@latest

# Output:
# ✅ Connected to MCP server successfully!
# 📋 Tools (25): browser_navigate, browser_click, browser_type...
# 📁 Resources (0):
# 💬 Prompts (0):
# ✅ Debug session completed successfully!
```

---

## 📚 Protocol Support

MCP Probe implements the complete **Model Context Protocol specification**:

| Feature | Support | Description |
|---------|---------|-------------|
| **Tools** | ✅ Full | List, validate, and call MCP tools |
| **Resources** | ✅ Full | Access and manage MCP resources |
| **Prompts** | ✅ Full | List and execute MCP prompts |
| **Sampling** | ✅ Full | Handle LLM sampling requests |
| **Logging** | ✅ Full | Protocol-level logging and debugging |
| **Progress** | ✅ Full | Long-running operation progress tracking |

### Transport Layer Support
- **📡 Stdio** - Local process communication (most common)
- **🌐 HTTP+SSE** - Server-Sent Events for real-time updates  
- **🔄 HTTP Streaming** - Full-duplex HTTP communication
- **🔒 Authentication** - Bearer tokens, Basic auth, OAuth 2.0

---

## 🎥 Live Examples

### Interactive TUI Mode
Launch the rich terminal interface for real-time debugging:

```bash
mcp-probe debug --stdio python my_mcp_server.py
```

Features beautiful syntax highlighting, real-time message inspection, and intuitive navigation.

### Automated Testing
Perfect for CI/CD pipelines:

```bash
mcp-probe test --stdio python server.py --report --output-dir ./reports
```

Generates comprehensive test reports with protocol compliance analysis.

### Export & Analysis
Generate detailed capability reports:

```bash
mcp-probe export --stdio python server.py --format markdown --output server-capabilities.md
```

---

## 🤝 Community & Support

<div class="community-grid">
  <div class="community-card">
    <h3>📖 Documentation</h3>
    <p>Comprehensive guides and API reference</p>
    <a href="documentation.html">Read the Docs</a>
  </div>
  
  <div class="community-card">
    <h3>💡 Examples</h3>
    <p>Real-world usage examples and tutorials</p>
    <a href="examples.html">View Examples</a>
  </div>
  
  <div class="community-card">
    <h3>🐛 Issues</h3>
    <p>Report bugs and request features</p>
    <a href="{{ site.github.issues_url }}">GitHub Issues</a>
  </div>
  
  <div class="community-card">
    <h3>🤝 Contributing</h3>
    <p>Help improve MCP Probe</p>
    <a href="contributing.html">Contribute</a>
  </div>
</div>

---

## 🏆 Why Choose MCP Probe?

| Aspect | MCP Probe | Alternative Tools |
|--------|-----------|-------------------|
| **Performance** | ⚡ Rust-powered, sub-second responses | 🐌 Often slow, memory-heavy |
| **Reliability** | 🛡️ Production-tested, 160+ tests | ❓ Limited testing coverage |
| **Features** | 🎯 Complete MCP protocol support | 📝 Basic functionality only |
| **UX** | 💎 Beautiful TUI + powerful CLI | 🔧 Command-line only |
| **Cross-platform** | 🌍 Linux, macOS, Windows | 🏠 Platform-specific |
| **Maintenance** | 🔄 Active development, automated releases | ⏰ Sporadic updates |

---

<div class="text-center cta-section">
  <h2>Ready to start debugging MCP servers?</h2>
  <p>Get MCP Probe {{ site.current_version }} and experience the difference</p>
  <a href="getting-started.html" class="btn btn-primary btn-large">Get Started Now</a>
</div>

---

<div class="footer-note">
  <p><strong>MCP Probe {{ site.current_version }}</strong> - Built with ❤️ in Rust | <a href="{{ site.github.repository_url }}">Open Source</a> | <a href="https://github.com/conikeec/mcp-probe/blob/main/LICENSE">MIT License</a></p>
</div> 