# 🔍 MCP Probe - Advanced MCP Protocol Debugger & Interactive Client

![Terminal of week](https://terminaltrove.com/assets/media/terminal_trove_tool_of_the_week_green_on_dark_grey_bg.png)

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.1.0-green.svg)](Cargo.toml)

![CleanShot 2025-06-21 at 13 48 13@2x](https://github.com/user-attachments/assets/0d989e06-c852-4c02-a77a-9a451e366bbc)


**MCP Probe** is a powerful Terminal User Interface (TUI) for debugging, testing, and interacting with Model Context Protocol (MCP) servers. It provides an intuitive, feature-rich alternative to command-line MCP inspectors with real-time protocol analysis, capability discovery, and interactive tool execution.

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           🔍 MCP PROBE ARCHITECTURE                             │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐             │
│  │   🖥️  TUI        │    │  🔌 Transport   │    │  🔧 MCP Server  │             │
│  │   Interface     │◄──►│   Layer         │◄──►│   (Any impl.)   │             │
│  │                 │    │                 │    │                 │             │
│  │ • Capabilities  │    │ • HTTP/SSE      │    │ • Tools (373+)  │             │
│  │ • Search        │    │ • WebSocket     │    │ • Resources     │             │
│  │ • Response View │    │ • STDIO         │    │ • Prompts       │             │
│  │ • Debugging     │    │ • TCP           │    │                 │             │
│  └─────────────────┘    └─────────────────┘    └─────────────────┘             │
│           │                       │                       │                     │
│           ▼                       ▼                       ▼                     │
│  ┌─────────────────────────────────────────────────────────────────────────────┤
│  │                     📊 REAL-TIME PROTOCOL ANALYSIS                         │
│  │  • Message Tracing  • Session Management  • Error Detection               │
│  │  • JSON Validation  • Response Formatting • Performance Metrics           │
│  └─────────────────────────────────────────────────────────────────────────────┘
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## 🚀 Why MCP Probe?

### vs. Traditional MCP Inspector Tools

| Feature                  | Traditional CLI Tools  | MCP Probe TUI                                  |
| ------------------------ | ---------------------- | ---------------------------------------------- |
| **Capability Discovery** | Manual JSON parsing    | 🎯 Interactive browsing with search            |
| **Tool Execution**       | Complex curl commands  | 🖱️ Point-and-click with parameter forms        |
| **Response Analysis**    | Raw JSON dumps         | 📊 Multi-format viewer (Tree/Summary/Raw)      |
| **Error Debugging**      | Scattered logs         | 🔍 Centralized error tracking with suggestions |
| **Session Management**   | Stateless commands     | 💾 Persistent sessions with history            |
| **Multi-Transport**      | Single transport focus | 🔌 HTTP/SSE, WebSocket, STDIO, TCP support     |
| **Real-time Monitoring** | Snapshot-based         | ⚡ Live protocol stream analysis               |

### Key Advantages

- **🎮 Interactive**: Navigate 373+ tools with fuzzy search and auto-completion
- **🔍 Visual**: Color-coded responses, scrollable viewers, progress indicators
- **📊 Analytical**: Built-in protocol validation, message correlation, timing analysis
- **🛠️ Developer-Friendly**: Session export, parameter templates, debugging hints
- **🚀 Fast**: Rust-powered performance with async I/O and efficient TUI rendering

---

## 📦 Installation

MCP Probe offers multiple installation methods for your convenience:

### 📥 Pre-built Binaries (Recommended)

Download the latest binary for your platform from [GitHub Releases](https://github.com/conikeec/mcp-probe/releases/latest):

- **Linux (x86_64)**: `mcp-probe-x86_64-unknown-linux-gnu.tar.gz`
- **Linux (ARM64)**: `mcp-probe-aarch64-unknown-linux-gnu.tar.gz`
- **macOS (Intel)**: `mcp-probe-x86_64-apple-darwin.tar.gz`
- **macOS (Apple Silicon)**: `mcp-probe-aarch64-apple-darwin.tar.gz`
- **Windows (x86_64)**: `mcp-probe-x86_64-pc-windows-msvc.zip`

### 🌐 One-liner Install (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/conikeec/mcp-probe/master/install.sh | bash
```

**Custom installation directory:**

```bash
curl -fsSL https://raw.githubusercontent.com/conikeec/mcp-probe/master/install.sh | INSTALL_DIR=~/.local/bin bash
```

**Install specific version:**

```bash
curl -fsSL https://raw.githubusercontent.com/conikeec/mcp-probe/master/install.sh | VERSION=v0.1.55 bash
```

### 🍺 Homebrew (macOS/Linux)

```bash
# Add the tap
brew tap conikeec/tap

# Install mcp-probe
brew install mcp-probe

# Or in one command
brew install conikeec/tap/mcp-probe
```

**Update:**

```bash
brew upgrade mcp-probe
```

### 📦 Cargo Install

```bash
cargo install mcp-cli
```

**Note**: The binary will be named `mcp-probe` even though the crate is `mcp-cli`.

### 🔨 From Source

```bash
# Clone the repository
git clone https://github.com/conikeec/mcp-probe.git
cd mcp-probe

# Build and install
cargo build --release
cargo install --path .

# Or run directly
cargo run -- --help
```

### 🛡️ Verification

All binaries are provided with SHA256 checksums. You can verify your download:

```bash
# Download checksum file
curl -LO https://github.com/conikeec/mcp-probe/releases/latest/download/mcp-probe-x86_64-unknown-linux-gnu.tar.gz.sha256

# Verify (Linux/macOS)
sha256sum -c mcp-probe-x86_64-unknown-linux-gnu.tar.gz.sha256

# Verify (macOS alternative)
shasum -a 256 -c mcp-probe-x86_64-apple-darwin.tar.gz.sha256
```

### Quick Start

```bash
# Test with a local MCP server
cargo run -- debug --http-sse http://localhost:3000

# Connect to remote server
cargo run -- debug --http-sse https://api.example.com/mcp

# Use WebSocket transport
cargo run -- debug --websocket ws://localhost:8080/mcp

# STDIO mode for local development
cargo run -- debug --stdio python my_mcp_server.py
```

---

## 🎯 Section 1: MCP Client Usage

MCP Probe serves as a comprehensive MCP client for developers and integrators who need to interact with MCP servers programmatically or interactively.

### 🔧 Client Configuration

```bash
# Basic connection with default settings
mcp-probe debug --http-sse http://localhost:3000

# Advanced configuration
mcp-probe debug \
  --http-sse http://localhost:3000 \
  --timeout 30 \
  --max-retries 3 \
  --session-file my_session.json
```

### 💡 Interactive Workflow

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                        🎮 INTERACTIVE CLIENT WORKFLOW                          │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  1️⃣ DISCOVERY PHASE                                                           │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │ ┌─ Connection ─┐  ┌─ Capabilities ─┐  ┌─ Search & Filter ─┐             │   │
│  │ │• Auto-detect │  │• Tools: 373     │  │• Fuzzy matching   │             │   │
│  │ │• Protocol    │  │• Resources: 1   │  │• Category filter  │             │   │
│  │ │• Session ID  │  │• Prompts: 3     │  │• Real-time index  │             │   │
│  │ └──────────────┘  └─────────────────┘  └───────────────────┘             │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                    ▼                                            │
│  2️⃣ INTERACTION PHASE                                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │ ┌─ Parameter Input ─┐  ┌─ Execution ─┐  ┌─ Response Analysis ─┐          │   │
│  │ │• Smart forms      │  │• Real-time   │  │• Multi-format view  │          │   │
│  │ │• Type validation  │  │• Progress    │  │• Error highlighting │          │   │
│  │ │• Auto-completion  │  │• Correlation │  │• Export options     │          │   │
│  │ └───────────────────┘  └──────────────┘  └─────────────────────┘          │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                    ▼                                            │
│  3️⃣ ANALYSIS PHASE                                                            │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │ ┌─ Session Review ─┐  ┌─ Error Analysis ─┐  ┌─ Export & Share ─┐          │   │
│  │ │• Message history │  │• Root cause hints │  │• JSON export     │          │   │
│  │ │• Timing metrics  │  │• Fix suggestions  │  │• Session replay  │          │   │
│  │ │• Protocol trace  │  │• Debug logs       │  │• Report sharing  │          │   │
│  │ └─────────────────────└───────────────────┘  └─────────────────┘          │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 🔍 Smart Capability Discovery

**Fuzzy Search Engine**: Find tools instantly among hundreds of capabilities

```bash
# Search examples (press '/' to activate)
/github                    # Find GitHub-related tools
/repo list                 # Find repository listing functions
/add_numbers              # Direct tool name match
```

**Auto-Parameter Detection**: Intelligent form generation from JSON schemas

```bash
# Example: GitHub repo listing tool
┌─────────────────────────────────────┐
│ 📋 org (REQUIRED) [string]         │
│ 💡 The organization name...         │
│ > myorganization                    │
├─────────────────────────────────────┤
│ 📝 per_page (optional) [integer]   │
│ 💡 Results per page (max 100)      │
│ > 50                                │
└─────────────────────────────────────┘
```

### 🚀 Execution Patterns

**Direct Command Mode**:

```bash
# Syntax: category.name {"param": "value"}
tools.add_numbers {"a": 10, "b": 20}
resources.readme_content
prompts.generate_docs {"style": "technical"}
```

**Interactive Mode**: Use TUI navigation for guided execution

**Batch Mode**: Execute multiple operations with session scripts

---

## 🔍 Section 2: Advanced Protocol Discovery & Session Management

MCP Probe features a sophisticated protocol discovery system that automatically detects and adapts to different MCP protocol versions, providing seamless connectivity across the evolving MCP ecosystem.

### 🚀 Intelligent Protocol Discovery

**Automatic Protocol Detection**: MCP Probe automatically detects the protocol version based on endpoint patterns and server behavior, eliminating manual configuration.

```bash
# MCP Probe automatically detects the protocol version from these patterns:
mcp-probe debug --http-sse http://localhost:8931/mcp      # Modern Streamable HTTP
mcp-probe debug --http-sse http://localhost:8931/sse      # Legacy HTTP+SSE  
mcp-probe debug --stdio python server.py                 # Standard Transport
```

### 📊 Protocol Version Matrix

| Protocol Version | Spec Date | Endpoints | Session Management | Transport Method | Status |
|------------------|-----------|-----------|-------------------|------------------|---------|
| **Modern Streamable HTTP** | 2025-03-26 | `/mcp` | `Mcp-Session-Id` header | HTTP/SSE Streaming | ✅ Current |
| **Legacy HTTP+SSE** | 2024-11-05 | `/sse`, `/events` | `sessionId` query param | HTTP + Server-Sent Events | ✅ Supported |
| **Standard Transport** | 2025-03-26 | `stdio` | N/A (process-based) | Process I/O | ✅ Supported |
| **WebSocket** | 2025-03-26 | `/ws`, `/websocket` | Connection-based | WebSocket frames | 🔄 Planned |
| **TCP** | 2025-03-26 | Raw socket | Connection-based | TCP stream | 🔄 Planned |

### 🔧 Session Negotiation Workflows

#### Modern Streamable HTTP (Recommended)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                    🌟 MODERN STREAMABLE HTTP WORKFLOW                          │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  Client                    MCP Probe                      Server               │
│    │                          │                             │                  │
│    │                          │                             │                  │
│    │──── 1. Connection ──────►│────── POST /mcp ──────────►│                  │
│    │                          │   Mcp-Session-Id: [auto]   │                  │
│    │                          │                             │                  │
│    │◄─── Session Created ─────│◄───── 200 + Session ───────│                  │
│    │                          │   Mcp-Session-Id: abc123    │                  │
│    │                          │                             │                  │
│    │──── 2. Initialize ──────►│────── POST /mcp ──────────►│                  │
│    │                          │   Mcp-Session-Id: abc123    │                  │
│    │                          │   {"method": "initialize"}  │                  │
│    │                          │                             │                  │
│    │◄─── Capabilities ────────│◄───── 200 OK ──────────────│                  │
│    │                          │   Server capabilities       │                  │
│    │                          │                             │                  │
│    │──── 3. Ready State ─────►│──────── Persistent ────────►│                  │
│    │                          │      Session Active         │                  │
│                                                                                 │
│  ✅ Single endpoint simplicity        🔒 Header-based security                 │
│  ✅ Built-in session management       ⚡ Automatic resumability                │
│  ✅ Firewall-friendly                 📊 Full streaming support               │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

#### Legacy HTTP+SSE (Backward Compatibility)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                      📡 LEGACY HTTP+SSE WORKFLOW                               │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  Client                    MCP Probe                      Server               │
│    │                          │                             │                  │
│    │                          │                             │                  │
│    │──── 1. Discover ────────►│────── GET /events ────────►│                  │
│    │                          │   Accept: text/event-stream │                  │
│    │                          │                             │                  │
│    │◄─── Session Info ────────│◄───── SSE Stream ──────────│                  │
│    │                          │   data: {"sessionId": "xyz"}│                  │
│    │                          │                             │                  │
│    │──── 2. Initialize ──────►│─── POST /sse?sessionId=xyz─►│                  │
│    │                          │   {"method": "initialize"}  │                  │
│    │                          │                             │                  │
│    │◄─── Capabilities ────────│◄───── 200 OK ──────────────│                  │
│    │                          │   Server capabilities       │                  │
│    │                          │                             │                  │
│    │──── 3. SSE Listen ──────►│─── GET /sse?sessionId=xyz──►│                  │
│    │                          │   Accept: text/event-stream │                  │
│    │                          │                             │                  │
│    │◄─── Event Stream ────────│◄──── SSE Messages ─────────│                  │
│    │                          │   Continuous updates        │                  │
│                                                                                 │
│  🔄 Dual-endpoint architecture       📡 Query-based sessions                   │
│  🔄 Separate discovery phase         ⚡ Event-driven updates                   │
│  🔄 Legacy compatibility             📊 SSE streaming support                 │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

#### Standard Transport (Development)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                       🔧 STANDARD TRANSPORT WORKFLOW                           │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  Client                    MCP Probe                   Process                 │
│    │                          │                             │                  │
│    │                          │                             │                  │
│    │──── 1. Spawn Process ───►│────── exec/spawn ─────────►│                  │
│    │                          │   python server.py          │                  │
│    │                          │                             │                  │
│    │◄─── Process Ready ───────│◄───── stdin/stdout ────────│                  │
│    │                          │   Process initialization    │                  │
│    │                          │                             │                  │
│    │──── 2. Initialize ──────►│────── JSON-RPC ───────────►│                  │
│    │                          │   via stdin                  │                  │
│    │                          │                             │                  │
│    │◄─── Capabilities ────────│◄───── JSON-RPC ────────────│                  │
│    │                          │   via stdout                 │                  │
│    │                          │                             │                  │
│    │──── 3. Bidirectional ───►│──── stdin/stdout pipes ────►│                  │
│    │                          │   Full-duplex communication │                  │
│                                                                                 │
│  🚀 Direct process control          🔧 Perfect for development                  │
│  🚀 No network complexity           ⚡ Immediate debugging                      │
│  🚀 Local filesystem access        📊 Full protocol support                   │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 🎯 Interactive vs Non-Interactive Command Sequences

#### Interactive Mode (TUI) - Guided Discovery

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                         🖥️  INTERACTIVE TUI WORKFLOW                           │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  Phase 1: CONNECTION & DISCOVERY                                               │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │ 🔌 Auto-detect Protocol    → 📡 Establish Session    → 🔍 Discover Tools │   │
│  │ • Parse endpoint URL       • Header/query sessions   • Fuzzy search      │   │
│  │ • Detect /mcp vs /sse      • Auto-resume capability • Category filter    │   │
│  │ • Security validation     • Background monitoring   • Real-time index    │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                    ▼                                            │
│  Phase 2: INTERACTIVE EXECUTION                                                │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │ 📋 Parameter Forms         → ⚡ Real-time Execution → 📊 Response Analysis│   │
│  │ • Smart type detection     • Progress indicators    • Multi-format view  │   │
│  │ • Schema-driven hints      • Error correlation      • Error highlighting │   │
│  │ • Auto-completion          • Session persistence    • Export options     │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                    ▼                                            │
│  Phase 3: ANALYSIS & DEBUGGING                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │ 📈 Session Review         → 🔧 Error Investigation → 📤 Export & Share   │   │
│  │ • Message history         • Root cause analysis     • JSON export        │   │
│  │ • Timing metrics          • Fix suggestions         • Session replay     │   │
│  │ • Protocol trace          • Debug logs              • Report generation  │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
│  🎮 User Experience: Visual, guided, exploratory                               │
│  ⌨️  Hotkeys: Tab navigation, / search, Enter execute                          │
│  🔍 Features: Fuzzy search, parameter forms, real-time feedback               │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

**Interactive Mode Commands:**

```bash
# Launch TUI with automatic protocol detection
mcp-probe debug --http-sse http://localhost:8931/mcp

# TUI Navigation Flow:
# 1. Tab → Navigate between panels
# 2. /   → Activate fuzzy search
# 3. ↑↓  → Browse capabilities
# 4. Enter → Open parameter form
# 5. Tab → Execute with parameters
# 6. V   → Cycle response views
# 7. F2  → Save session

# Smart Parameter Forms:
# • Auto-detects field types from JSON Schema
# • Provides contextual hints and validation
# • Supports environment variable injection
# • Real-time syntax validation
```

#### Non-Interactive Mode (CLI) - Automation-Friendly

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                        ⚡ NON-INTERACTIVE CLI WORKFLOW                         │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  Phase 1: RAPID CONNECTION                                                     │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │ 🚀 Direct Connect         → 📊 Quick Capability Dump                      │   │
│  │ • Protocol auto-detection  • Structured output                            │   │
│  │ • No user interaction      • Machine-readable format                      │   │
│  │ • CI/CD friendly           • Error codes for automation                   │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                    ▼                                            │
│  Phase 2: BATCH EXECUTION                                                      │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │ 📝 Command Scripts        → ⚙️  Automated Testing    → 📄 Report Output  │   │
│  │ • Direct tool execution    • Comprehensive test suite • JSON/CSV export  │   │
│  │ • Parameter validation     • Protocol compliance     • CI integration    │   │
│  │ • Bulk operations          • Performance monitoring  • Success/fail codes│   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                    ▼                                            │
│  Phase 3: PRODUCTION VALIDATION                                                │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │ 🔍 Health Checks         → 📊 Performance Analysis → 🚨 Alert Integration│   │
│  │ • Endpoint discovery      • Response time metrics   • Monitoring systems │   │
│  │ • Protocol compliance     • Memory usage tracking   • Automated alerts   │   │
│  │ • Schema validation       • Error rate analysis     • Report webhooks    │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
│  🤖 Use Cases: CI/CD pipelines, monitoring, automation                         │
│  ⚡ Features: Zero interaction, structured output, exit codes                  │
│  🔧 Integration: Scripts, Docker, Kubernetes health checks                     │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

**Non-Interactive Mode Commands:**

```bash
# Quick capability overview
mcp-probe debug --http-sse http://localhost:8931/mcp --non-interactive

# Automated testing with reports
mcp-probe test --http-sse http://localhost:8931/mcp --report --output-dir ./reports

# Endpoint discovery for load balancers
mcp-probe test --discover http://api.company.com --report

# CI/CD integration examples
mcp-probe test --http-sse $MCP_SERVER_URL --fail-fast --timeout 30
if [ $? -eq 0 ]; then echo "✅ MCP server healthy"; else echo "❌ MCP server failed"; fi

# Batch operations for monitoring
mcp-probe validate --http-sse http://prod-server/mcp --suite compliance
```

### 🔍 Advanced Endpoint Discovery

**Multi-Endpoint Discovery**: MCP Probe can discover and test multiple MCP endpoints from a base URL, perfect for load balancers and multi-service deployments.

```bash
# Discover all MCP endpoints under a domain
mcp-probe test --discover https://api.company.com

# Discovery automatically tests these patterns:
# • https://api.company.com/mcp      (Modern)
# • https://api.company.com/sse      (Legacy)  
# • https://api.company.com/events   (Discovery)
# • https://api.company.com/v1/mcp   (Versioned)
# • https://api.company.com/api/mcp  (Nested)

# Output shows availability and capabilities:
✅ Modern Streamable HTTP - 47 tools, 3 resources, 2 prompts
✅ Legacy HTTP+SSE        - 47 tools, 3 resources, 2 prompts  
❌ Versioned API          - Connection failed
⚠️  Nested API            - Invalid response format
```

### 🛡️ Security & Session Management

#### Automatic Security Validation

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                          🔒 SECURITY VALIDATION                                │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  🛡️  Connection Security                                                       │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │ • HTTPS enforcement for production URLs                                  │   │
│  │ • Origin validation to prevent DNS rebinding                            │   │
│  │ • Session ID format validation (cryptographic strength)                 │   │
│  │ • Certificate verification for remote servers                           │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
│  🔐 Session Management                                                          │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │ • Automatic session discovery and renewal                               │   │
│  │ • Secure session ID generation and tracking                             │   │
│  │ • Background session monitoring for ephemeral servers                  │   │
│  │ • Session resumption after network interruptions                       │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
│  ⚡ Performance & Reliability                                                   │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │ • Connection pooling and keep-alive                                     │   │
│  │ • Automatic retry with exponential backoff                              │   │
│  │ • Request timeout and circuit breaker patterns                          │   │
│  │ • Memory-efficient streaming for large responses                        │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 📈 Protocol Compliance Testing

**Comprehensive Test Suites**: MCP Probe includes extensive test suites for validating protocol compliance across different versions.

```bash
# Full protocol compliance testing
mcp-probe validate --http-sse http://localhost:8931/mcp --suite all

# Specific compliance areas:
mcp-probe validate --suite initialization  # Connection & handshake
mcp-probe validate --suite capabilities    # Tool/resource discovery  
mcp-probe validate --suite schema         # JSON Schema validation
mcp-probe validate --suite security       # Security best practices
mcp-probe validate --suite performance    # Response time & throughput

# Generate detailed compliance reports
mcp-probe validate --suite all --report --output-dir ./compliance-reports
```

---

## 🔧 Section 3: MCP Deployment Troubleshooting

MCP Probe excels as a diagnostic tool for MCP deployments, providing deep insights into protocol behavior, performance bottlenecks, and integration issues.

### 🚨 Diagnostic Features

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                      🔍 TROUBLESHOOTING DASHBOARD                              │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  🔴 CONNECTION DIAGNOSTICS          🟡 PROTOCOL ANALYSIS                       │
│  ┌─────────────────────────────┐    ┌─────────────────────────────────────┐   │
│  │ • Transport validation      │    │ • Message correlation               │   │
│  │ • Authentication checks     │    │ • Response time analysis            │   │
│  │ • Firewall/proxy detection  │    │ • Error pattern recognition         │   │
│  │ • SSL/TLS verification      │    │ • Capability compatibility         │   │
│  └─────────────────────────────┘    └─────────────────────────────────────┘   │
│                                                                                 │
│  🟢 PERFORMANCE MONITORING          🟣 ERROR INVESTIGATION                     │
│  ┌─────────────────────────────┐    ┌─────────────────────────────────────┐   │
│  │ • Request/response latency  │    │ • Stack trace analysis             │   │
│  │ • Throughput measurement    │    │ • JSON schema validation           │   │
│  │ • Memory usage tracking     │    │ • Serialization debugging          │   │
│  │ • Connection stability      │    │ • Integration compatibility        │   │
│  └─────────────────────────────┘    └─────────────────────────────────────┘   │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 🛠️ Common Issues & Solutions

#### Issue 1: Connection Failures

**Symptoms**: "Transport connection failed", "Connection refused"

**Diagnosis with MCP Probe**:

```bash
# 1. Test basic connectivity
mcp-probe debug --http-sse http://localhost:3000

# 2. Check different transports
mcp-probe debug --websocket ws://localhost:8080/mcp
mcp-probe debug --stdio python server.py

# 3. Monitor protocol flow
# Look for: Connection status, SSL handshake, authentication
```

**Troubleshooting Guide**:

- ✅ Server is running and listening on correct port
- ✅ Firewall rules allow connections
- ✅ SSL certificates are valid (for HTTPS)
- ✅ Authentication credentials are correct

#### Issue 2: Tool Execution Failures

**Symptoms**: "Serialization error", "Invalid parameters", "Tool not found"

**Diagnosis with MCP Probe**:

```bash
# 1. Verify tool discovery
# Navigate to Tools section, check tool list

# 2. Inspect parameter schemas
# Select tool -> Parameter form should show required fields

# 3. Check raw response data
# Use 'V' key to cycle through response formats
```

**Common Root Causes**:

- 🔧 **Parameter Mismatch**: Use Parameter Form to validate inputs
- 🔧 **Tool Name Prefix Issues**: Check clean vs. full tool names
- 🔧 **JSON Format Errors**: Validate JSON in response viewer
- 🔧 **Server-Side Errors**: Review error messages in message history

#### Issue 3: Performance Problems

**Symptoms**: Slow responses, timeouts, memory issues

**Diagnosis with MCP Probe**:

```bash
# 1. Monitor timing metrics
# Check message history for response times

# 2. Analyze message sizes
# Use Raw JSON view to inspect payload sizes

# 3. Track connection stability
# Watch for reconnection attempts in logs
```

### 📊 Protocol Debugging

**Message Flow Analysis**:

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                         📈 PROTOCOL MESSAGE FLOW                               │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  Client                    MCP Probe                      Server               │
│    │                          │                             │                  │
│    │──── initialize ────────►│────── HTTP/POST ──────────►│                  │
│    │                          │                             │                  │
│    │◄─── init_response ──────│◄───── 200 OK ──────────────│                  │
│    │                          │                             │                  │
│    │──── tools/list ────────►│────── HTTP/POST ──────────►│                  │
│    │                          │                             │                  │
│    │◄─── tools_response ─────│◄───── 200 OK ──────────────│                  │
│    │                          │                             │                  │
│    │──── tools/call ────────►│────── HTTP/POST ──────────►│                  │
│    │                          │         (params)            │                  │
│    │                          │                             │                  │
│    │◄─── result/error ───────│◄───── 200/400/500 ─────────│                  │
│    │                          │                             │                  │
│                                                                                 │
│  🔍 MCP Probe captures and analyzes each step:                                 │
│  • Request correlation (session ID tracking)                                   │
│  • Response time measurement                                                    │
│  • Error classification and suggestions                                        │
│  • JSON schema validation                                                      │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 🎯 Environment Validation

**Development Environment Checklist**:

```bash
# 1. Server Implementation Validation
✅ Server responds to initialize request
✅ Capabilities are properly declared
✅ Tool schemas are valid JSON Schema
✅ Error responses include helpful messages

# 2. Integration Testing
✅ Authentication flow works correctly
✅ Session management is stable
✅ All transport types are supported
✅ Error handling is graceful

# 3. Performance Validation
✅ Response times are within SLA
✅ Memory usage is reasonable
✅ Concurrent requests are handled
✅ Rate limiting is implemented correctly
```

---

## 📚 Command Cheat Sheet

### 🔧 Basic Commands

```bash
# Connection
mcp-probe debug --http-sse <url>          # HTTP Server-Sent Events
mcp-probe debug --websocket <url>         # WebSocket connection
mcp-probe debug --stdio <command>         # STDIO transport
mcp-probe debug --tcp <host:port>         # Raw TCP connection

# Configuration
mcp-probe debug <transport> --timeout 30        # Request timeout
mcp-probe debug <transport> --max-retries 3     # Retry attempts
mcp-probe debug <transport> --session-file <f>  # Session persistence
```

### ⌨️ TUI Navigation Hotkeys

#### Global Navigation

| Key   | Action        | Description               |
| ----- | ------------- | ------------------------- |
| `Tab` | Cycle Focus   | Move between panels       |
| `F1`  | Help          | Show/hide help dialog     |
| `F2`  | Save Session  | Export current session    |
| `F3`  | Toggle JSON   | Switch JSON view mode     |
| `F4`  | Clear History | Reset message history     |
| `F5`  | Environment   | Set environment variables |
| `Q`   | Quit          | Exit application          |

#### Capability Browser

| Key     | Action   | Description                   |
| ------- | -------- | ----------------------------- |
| `Enter` | Select   | Open capability details       |
| `↑/↓`   | Navigate | Move through categories/items |
| `←/→`   | Page     | Previous/next page            |
| `/`     | Search   | Activate fuzzy search         |
| `Esc`   | Back     | Return to categories          |

#### Search Interface

| Key     | Action   | Description           |
| ------- | -------- | --------------------- |
| `/`     | Activate | Open search dialog    |
| `Type`  | Query    | Enter search terms    |
| `↑/↓`   | Navigate | Browse search results |
| `Enter` | Select   | Choose result         |
| `Esc`   | Cancel   | Close search          |

#### Parameter Forms

| Key     | Action   | Description                     |
| ------- | -------- | ------------------------------- |
| `↑/↓`   | Navigate | Move between fields (auto-edit) |
| `Type`  | Edit     | Enter parameter values          |
| `Enter` | Save     | Save field and move to next     |
| `Tab`   | Execute  | Run with current parameters     |
| `Esc`   | Cancel   | Close parameter form            |

#### Response Viewer

| Key         | Action      | Description                                |
| ----------- | ----------- | ------------------------------------------ |
| `R`         | Open        | View selected response                     |
| `V`         | View Mode   | Cycle formats (Formatted/Raw/Tree/Summary) |
| `↑/↓`       | Scroll V    | Vertical scrolling                         |
| `←/→`       | Scroll H    | Horizontal scrolling                       |
| `PgUp/PgDn` | Fast Scroll | Page up/down                               |
| `Home/End`  | Jump        | Go to top/bottom                           |
| `Esc`       | Close       | Exit response viewer                       |

### 🎮 Interactive Commands

#### Direct Tool Execution

```bash
# Syntax: category.name {json_params}
tools.add_numbers {"a": 10, "b": 20}
tools.github_list_repos {"org": "microsoft", "per_page": 10}
resources.readme_content
prompts.code_review {"language": "rust", "style": "detailed"}
```

#### Environment Variables

```bash
# Set variables for tool injection
KEY=value,API_TOKEN=secret123,ORG=myorg

# Variables automatically injected into tool calls
tools.api_call {}  # Will include ORG=myorg if tool expects it
```

---

## 📁 File System Organization

MCP Probe automatically organizes all generated files in a clean, structured directory hierarchy in your home directory.

### 🏠 Directory Structure

```
~/.mcp-probe/
├── logs/                    # All log files with timestamps
│   ├── mcp-probe-debug.log      # TUI mode debug log
│   └── mcp-probe-YYYYMMDD_HHMMSS.log  # CLI mode logs
├── reports/                 # All generated reports with date prefixes
│   ├── YYYYMMDD-test-report-HHMMSS.json
│   ├── YYYYMMDD-validation-report-HHMMSS.json
│   └── YYYYMMDD-discovery-report-HHMMSS.json
├── sessions/               # Saved session files
│   └── debug-session-YYYYMMDD_HHMMSS.json
└── config/                 # Configuration files
    └── mcp-probe.toml
```

### 🗂️ Path Management Commands

```bash
# Show directory structure and usage
mcp-probe paths show

# Clean up old files (dry run)
mcp-probe paths cleanup --days 30

# Actually clean up files older than 7 days
mcp-probe paths cleanup --days 7 --force

# Open MCP Probe directory in file manager
mcp-probe paths open
```

### 📅 Automatic Date Prefixing

All reports are automatically prefixed with dates for easy organization:

- **Format**: `YYYYMMDD-report-name-HHMMSS.extension`
- **Example**: `20250622-test-report-143052.json`
- **Benefits**: Chronological sorting, easy cleanup, no file conflicts

### 🧹 Automated Cleanup

MCP Probe includes intelligent cleanup features:

```bash
# Show what would be cleaned up
mcp-probe paths cleanup --days 30

# Clean files older than 30 days
mcp-probe paths cleanup --days 30 --force

# The paths show command gives cleanup recommendations
mcp-probe paths show
```

---

## 🔍 Advanced Features

### 📊 Response Analysis Modes

#### 1. **Formatted View** (Default)

- ✨ Syntax highlighting for JSON
- 📋 Structured analysis with field breakdown
- 🔍 Error highlighting and suggestions
- 📈 Content statistics and metadata

#### 2. **Raw JSON View**

- 📄 Pretty-printed JSON output
- 🔍 Full response data visibility
- 📋 Copy-friendly format
- 🛠️ Debug-oriented display

#### 3. **Tree View**

- 🌳 Hierarchical data visualization
- 📁 Collapsible object/array nodes
- 📊 Type indicators for each field
- 🎯 Easy navigation of nested structures

#### 4. **Summary View**

- 📈 High-level response overview
- 📊 Key metrics and statistics
- ⚡ Quick status assessment
- 🎯 Action-oriented insights

### 🔒 Session Management

```bash
# Auto-save sessions
mcp-probe debug --http-sse http://localhost:3000 --session-file debug.json

# Session contains:
# • Connection parameters
# • Message history with timing
# • Error logs and diagnostics
# • Environment variables
# • Response cache for offline analysis
```

### 🌐 Multi-Transport Support

#### HTTP Server-Sent Events (Recommended)

```bash
mcp-probe debug --http-sse http://localhost:3000
# ✅ Most compatible with web servers
# ✅ Firewall-friendly
# ✅ Built-in error handling
```

#### WebSocket

```bash
mcp-probe debug --websocket ws://localhost:8080/mcp
# ✅ Real-time bidirectional communication
# ✅ Lower latency
# ⚠️ May require proxy configuration
```

#### STDIO (Development)

```bash
mcp-probe debug --stdio python my_server.py
# ✅ Perfect for local testing
# ✅ Direct process communication
# ⚠️ Limited to local development
```

#### TCP (Advanced)

```bash
mcp-probe debug --tcp localhost:9000
# ✅ Low-level protocol access
# ✅ Custom transport implementations
# ⚠️ Requires manual protocol handling
```

---

## 🛠️ Development & Contributing

### Project Structure

```
mcp-probe/
├── crates/
│   ├── mcp-core/           # Core MCP protocol implementation
│   │   ├── src/
│   │   │   ├── client.rs   # High-level MCP client
│   │   │   ├── transport/  # Transport layer abstractions
│   │   │   └── messages/   # Protocol message definitions
│   │   └── Cargo.toml
│   └── mcp-cli/            # TUI application
│       ├── src/
│       │   ├── tui.rs      # Terminal UI implementation
│       │   ├── search.rs   # Capability search engine
│       │   └── main.rs     # CLI entry point
│       └── Cargo.toml
├── target/                 # Build artifacts
├── Cargo.toml              # Workspace configuration
└── README.md
```

### Building from Source

```bash
# Debug build
cargo build

# Release build (recommended for performance)
cargo build --release

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run -- debug --http-sse http://localhost:3000
```

### Contributing Guidelines

1. 🍴 Fork the repository
2. 🌿 Create a feature branch
3. ✅ Add tests for new functionality
4. 📝 Update documentation
5. 🔄 Submit a pull request

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🤝 Support & Community

- **📖 Documentation**: [docs.example.com/mcp-probe](docs.example.com/mcp-probe)
- **🐛 Issues**: [GitHub Issues](https://github.com/your-org/mcp-probe/issues)
- **💬 Discussions**: [GitHub Discussions](https://github.com/your-org/mcp-probe/discussions)
- **🔧 Contributing**: See [CONTRIBUTING.md](CONTRIBUTING.md)

---

## 🚀 Quick Examples

### Example 1: Debug API Integration

```bash
# Connect to your MCP server
mcp-probe debug --http-sse https://api.yourservice.com/mcp

# 1. Verify connection and capabilities
# 2. Search for relevant tools: /api or /user
# 3. Test tool execution with real parameters
# 4. Analyze responses for integration issues
# 5. Export session for team sharing
```

### Example 2: Performance Testing

```bash
# Set up performance monitoring
mcp-probe debug --http-sse http://localhost:3000 --session-file perf_test.json

# 1. Execute high-frequency tool calls
# 2. Monitor response times in message history
# 3. Check for memory leaks or connection issues
# 4. Review session file for timing analysis
```

### Example 3: Development Workflow

```bash
# Local development testing
mcp-probe debug --stdio python my_mcp_server.py

# 1. Rapid iteration on server code
# 2. Test tool schemas and validation
# 3. Debug parameter handling
# 4. Verify error response formats
```

---

## 🔧 Development Automation & CI/CD

### 🚀 Automated Quality Checks

MCP Probe includes comprehensive automation for code quality and CI/CD:

#### Development Scripts

```bash
# Run all checks (formatting, clippy, tests)
./scripts/check.sh

# Auto-fix formatting and clippy issues
./scripts/fix.sh

# Publish to crates.io (with all checks)
./scripts/release.sh
```

#### Continuous Integration

Our GitHub Actions CI pipeline automatically runs:

- ✅ **Code Formatting**: `rustfmt` ensures consistent code style
- ✅ **Linting**: `clippy` with warnings as errors for code quality
- ✅ **Testing**: Full test suite across Linux, macOS, and Windows
- ✅ **Security Audit**: `cargo audit` for vulnerability scanning
- ✅ **Code Coverage**: Coverage reporting with codecov.io
- ✅ **Documentation**: Automatic docs.rs generation

#### Publishing Workflow

Releases to crates.io are fully automated:

1. **Trigger**: Create a GitHub release
2. **Checks**: All CI checks must pass
3. **Publish**:
   - `mcp-core` published first (dependency)
   - `mcp-cli` published second
4. **Tagging**: Git tag created automatically

#### Local Development Setup

```bash
# Install development tools
cargo install cargo-audit cargo-tarpaulin

# Set up git hooks (optional)
cargo install cargo-husky
```

#### Code Quality Standards

- **Formatting**: Enforced via `rustfmt.toml` configuration
- **Linting**: Zero tolerance for clippy warnings
- **Testing**: Comprehensive test coverage required
- **Documentation**: All public APIs must be documented

### 📦 Crate Publishing

Both crates are available on crates.io:

- **[mcp-core](https://crates.io/crates/mcp-core)**: Core MCP protocol implementation
- **[mcp-cli](https://crates.io/crates/mcp-cli)**: Interactive TUI debugger

#### Installation

```bash
# Install the CLI tool
cargo install mcp-cli

# Add core library to your project
[dependencies]
mcp-core = "0.1.0"
```

---

## 💝 Credits & Acknowledgments

**Made with ❤️ in Rust**

This project wouldn't be possible without:

- 🦀 **[Rust Programming Language](https://rust-lang.org/)** - For memory safety, performance, and the amazing ecosystem that makes systems programming a joy
- 🤖 **[Anthropic](https://anthropic.com/)** - For creating the Model Context Protocol (MCP) specification and advancing the field of AI collaboration tools
- 🌟 **The Rust Community** - For the incredible crates that power MCP Probe: Ratatui, Tokio, Serde, Clap, and countless others
- 🛠️ **Open Source Contributors** - Every bug report, feature suggestion, and pull request makes this tool better

Special thanks to the Ratatui team for creating the foundation that makes our beautiful TUI possible! 🎨

---

**🎯 MCP Probe: Making MCP protocol debugging as intuitive as it should be.**
