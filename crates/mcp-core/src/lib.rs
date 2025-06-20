//! # MCP Core Library
//!
//! `mcp-core` provides the foundational types, traits, and implementations for the
//! Model Context Protocol (MCP). This crate is the core building block for MCP
//! clients and debugging tools.
//!
//! ## Features
//!
//! - **Complete MCP Message Types**: All JSON-RPC message structures defined by the MCP specification
//! - **Transport Abstraction**: Unified interface for stdio, HTTP+SSE, and HTTP streaming transports  
//! - **Type-Safe Configuration**: Compile-time validated configuration for all transport types
//! - **Comprehensive Error Handling**: Structured error types for all failure modes
//! - **Async-First Design**: Built on tokio for high-performance async I/O
//! - **High-Level Client**: Ready-to-use MCP client with protocol handling
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use mcp_core::{
//!     client::{McpClient, ClientConfig},
//!     transport::TransportConfig,
//!     messages::Implementation,
//! };
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Configure a stdio transport
//!     let config = TransportConfig::stdio("python", &["server.py"]);
//!     
//!     // Create and connect client
//!     let mut client = McpClient::with_defaults(config).await?;
//!     
//!     let client_info = Implementation {
//!         name: "mcp-probe".to_string(),
//!         version: "0.1.0".to_string(),
//!         metadata: std::collections::HashMap::new(),
//!     };
//!     
//!     let server_info = client.connect(client_info).await?;
//!     println!("Connected to: {}", server_info.implementation.name);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! The library is organized into several key modules:
//!
//! - [`error`]: Comprehensive error types for all MCP operations
//! - [`messages`]: Complete MCP message type definitions  
//! - [`transport`]: Transport abstraction and implementations
//! - [`client`]: High-level MCP client interface
//!
//! ## Transport Support
//!
//! This crate supports all three MCP transport mechanisms:
//!
//! - **stdio**: Local process communication (enabled by default)
//! - **http-sse**: HTTP + Server-Sent Events (enabled by default)  
//! - **http-stream**: Full-duplex HTTP streaming (enabled by default)
//!
//! Transport support can be controlled via feature flags.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(clippy::module_name_repetitions)]

pub mod client;
pub mod error;
pub mod messages;
pub mod transport;

// Re-export commonly used types for convenience
pub use client::{ClientConfig, ClientState, ClientStats, McpClient, ServerInfo};
pub use error::{McpError, McpResult};
pub use messages::{
    Capabilities, Implementation, InitializeRequest, InitializeResponse, InitializedNotification,
    JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, ProtocolVersion,
};
pub use transport::{Transport, TransportConfig, TransportFactory, TransportInfo};

/// Current version of the mcp-core library
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Current MCP protocol version supported by this library
pub const PROTOCOL_VERSION: &str = "2024-11-05";
