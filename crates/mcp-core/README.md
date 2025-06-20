# mcp-core

[![Crates.io](https://img.shields.io/crates/v/mcp-core.svg)](https://crates.io/crates/mcp-core)
[![Documentation](https://docs.rs/mcp-core/badge.svg)](https://docs.rs/mcp-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Core MCP (Model Context Protocol) types, traits, and transport implementations for Rust.

## Overview

`mcp-core` provides the fundamental building blocks for implementing MCP clients and servers in Rust. It includes:

- **Protocol Types**: All MCP message types, including initialization, tools, resources, and prompts
- **Transport Layer**: Pluggable transport implementations (stdio, HTTP SSE, HTTP streaming)
- **Client Implementation**: High-level MCP client with automatic session management
- **Error Handling**: Comprehensive error types for robust MCP implementations

## Features

- 🚀 **High Performance**: Built on Tokio for async I/O
- 🔌 **Pluggable Transports**: Support for stdio, HTTP SSE, and HTTP streaming
- 🛡️ **Type Safety**: Fully typed MCP protocol implementation
- 🔄 **Session Management**: Automatic session handling and reconnection
- 📝 **Comprehensive Logging**: Detailed tracing for debugging

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
mcp-core = "0.1.0"
```

### Basic Client Usage

```rust
use mcp_core::{
    client::McpClient,
    transport::TransportConfig,
    messages::Implementation,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure transport (stdio example)
    let config = TransportConfig::stdio("my-mcp-server", &["--arg1", "value1"]);

    // Create client
    let mut client = McpClient::with_defaults(config).await?;

    // Connect with client info
    let client_info = Implementation {
        name: "my-client".to_string(),
        version: "1.0.0".to_string(),
        metadata: Default::default(),
    };

    let _server_info = client.connect(client_info).await?;

    // List available tools
    let tools = client.list_tools().await?;
    println!("Available tools: {}", tools.len());

    // Call a tool
    if let Some(tool) = tools.first() {
        let params = serde_json::json!({"param": "value"});
        let result = client.call_tool(&tool.name, params).await?;
        println!("Tool result: {:?}", result);
    }

    Ok(())
}
```

### Transport Configuration

#### Stdio Transport

```rust
let config = TransportConfig::stdio("python", &["-m", "my_mcp_server"]);
```

#### HTTP SSE Transport

```rust
let config = TransportConfig::http_sse("http://localhost:3000/sse")?;
```

#### HTTP Stream Transport

```rust
let config = TransportConfig::http_stream("http://localhost:3000/stream")?;
```

## Architecture

The crate is organized into several modules:

- **`client`**: High-level MCP client implementation
- **`messages`**: All MCP protocol message types and serialization
- **`transport`**: Transport layer abstractions and implementations
- **`error`**: Error types and handling

### Transport Layer

The transport layer uses a trait-based design for extensibility:

```rust
#[async_trait]
pub trait Transport: Send + Sync {
    async fn send_request(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse>;
    async fn start_session(&mut self, client_info: Implementation) -> Result<TransportInfo>;
    // ... other methods
}
```

This allows for easy implementation of custom transports while maintaining a consistent interface.

## Contributing

Contributions are welcome! Please see the [main repository](https://github.com/conikeec/mcp-probe) for contribution guidelines.

## License

This project is licensed under the MIT License - see the [LICENSE](../../LICENSE) file for details.
