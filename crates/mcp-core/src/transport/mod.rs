//! MCP transport layer abstraction and implementations.
//!
//! This module provides a unified interface for all MCP transport mechanisms:
//! - **stdio**: Local process communication via stdin/stdout
//! - **HTTP+SSE**: Remote servers using HTTP requests + Server-Sent Events
//! - **HTTP Streaming**: Full-duplex HTTP streaming for bidirectional communication
//!
//! The transport layer is designed to be:
//! - **Transport-agnostic**: Same interface for all transport types
//! - **Async-first**: Built on tokio for high-performance async I/O
//! - **Type-safe**: Leverages Rust's type system to prevent protocol violations
//! - **Extensible**: Easy to add new transport mechanisms
//! - **Robust**: Comprehensive error handling and recovery
//!
//! # Examples
//!
//! ```rust,no_run
//! use mcp_probe_core::transport::{Transport, TransportFactory, TransportConfig};
//! use mcp_probe_core::messages::JsonRpcRequest;
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a transport from configuration
//!     let config = TransportConfig::stdio("python", &["server.py"]);
//!         
//!     let mut transport = TransportFactory::create(config).await?;
//!     
//!     // Connect transport
//!     transport.connect().await?;
//!     
//!     // Send a request  
//!     let request = JsonRpcRequest::new("1", "initialize", json!({}));
//!     let response = transport.send_request(request, Some(std::time::Duration::from_secs(30))).await?;
//!     println!("Received: {:?}", response);
//!     
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod factory;

#[cfg(feature = "stdio")]
pub mod stdio;

#[cfg(feature = "http-sse")]
pub mod http_sse;

#[cfg(feature = "http-stream")]
pub mod http_stream;

pub use config::*;
pub use factory::*;

use crate::error::{McpResult, TransportError};
use crate::messages::{JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use async_trait::async_trait;
use std::time::Duration;
use tokio::sync::mpsc;

/// Core transport trait for MCP communication.
///
/// This trait defines the interface that all MCP transports must implement.
/// It provides async methods for sending/receiving messages and managing
/// the transport connection lifecycle.
///
/// # Design Principles
///
/// - **Bidirectional**: Support both client-to-server and server-to-client messages
/// - **Message-oriented**: Work with high-level MCP messages, not raw bytes
/// - **Async**: All operations are async for maximum concurrency
/// - **Reliable**: Handle connection failures and provide retry mechanisms
/// - **Observable**: Provide hooks for monitoring and debugging
#[async_trait]
pub trait Transport: Send + Sync {
    /// Connect to the MCP server.
    ///
    /// This establishes the underlying connection (process spawn, HTTP connection, etc.)
    /// but does not perform MCP protocol initialization.
    async fn connect(&mut self) -> McpResult<()>;

    /// Disconnect from the MCP server.
    ///
    /// This cleanly closes the connection and releases any resources.
    /// Should be called when the MCP session is complete.
    async fn disconnect(&mut self) -> McpResult<()>;

    /// Check if the transport is currently connected.
    fn is_connected(&self) -> bool;

    /// Send a JSON-RPC request and wait for the response.
    ///
    /// This is the primary method for client-initiated request/response interactions.
    /// The method handles request correlation and timeout management.
    ///
    /// # Arguments
    ///
    /// * `request` - The JSON-RPC request to send
    /// * `timeout` - Optional timeout for the request (uses default if None)
    ///
    /// # Returns
    ///
    /// The corresponding JSON-RPC response, or an error if the request fails.
    async fn send_request(
        &mut self,
        request: JsonRpcRequest,
        timeout: Option<Duration>,
    ) -> McpResult<JsonRpcResponse>;

    /// Send a JSON-RPC notification (fire-and-forget).
    ///
    /// Notifications don't expect responses and are used for events,
    /// logging, and other one-way communications.
    ///
    /// # Arguments
    ///
    /// * `notification` - The JSON-RPC notification to send
    async fn send_notification(&mut self, notification: JsonRpcNotification) -> McpResult<()>;

    /// Receive the next message from the server.
    ///
    /// This method blocks until a message is received or an error occurs.
    /// It can return requests (from server to client), responses (to previous
    /// client requests), or notifications.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Optional timeout for receiving (blocks indefinitely if None)
    async fn receive_message(&mut self, timeout: Option<Duration>) -> McpResult<JsonRpcMessage>;

    /// Get transport-specific metadata and statistics.
    ///
    /// This can include connection info, performance metrics, error counts, etc.
    /// The exact contents depend on the transport implementation.
    fn get_info(&self) -> TransportInfo;

    /// Get the transport configuration used for this instance.
    fn get_config(&self) -> &TransportConfig;
}

/// Transport information and statistics.
///
/// This structure provides insight into the transport's current state,
/// performance characteristics, and any relevant metadata.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TransportInfo {
    /// Type of transport (stdio, http-sse, http-stream)
    pub transport_type: String,

    /// Whether the transport is currently connected
    pub connected: bool,

    /// Connection establishment time (if connected)
    pub connected_since: Option<std::time::SystemTime>,

    /// Number of requests sent
    pub requests_sent: u64,

    /// Number of responses received
    pub responses_received: u64,

    /// Number of notifications sent
    pub notifications_sent: u64,

    /// Number of notifications received
    pub notifications_received: u64,

    /// Number of errors encountered
    pub errors: u64,

    /// Transport-specific metadata
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl TransportInfo {
    /// Create a new transport info structure.
    pub fn new(transport_type: impl Into<String>) -> Self {
        Self {
            transport_type: transport_type.into(),
            connected: false,
            connected_since: None,
            requests_sent: 0,
            responses_received: 0,
            notifications_sent: 0,
            notifications_received: 0,
            errors: 0,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Mark the transport as connected.
    pub fn mark_connected(&mut self) {
        self.connected = true;
        self.connected_since = Some(std::time::SystemTime::now());
    }

    /// Mark the transport as disconnected.
    pub fn mark_disconnected(&mut self) {
        self.connected = false;
        self.connected_since = None;
    }

    /// Increment the request counter.
    pub fn increment_requests_sent(&mut self) {
        self.requests_sent += 1;
    }

    /// Increment the response counter.
    pub fn increment_responses_received(&mut self) {
        self.responses_received += 1;
    }

    /// Increment the notification sent counter.
    pub fn increment_notifications_sent(&mut self) {
        self.notifications_sent += 1;
    }

    /// Increment the notification received counter.
    pub fn increment_notifications_received(&mut self) {
        self.notifications_received += 1;
    }

    /// Increment the error counter.
    pub fn increment_errors(&mut self) {
        self.errors += 1;
    }

    /// Add transport-specific metadata.
    pub fn add_metadata(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.metadata.insert(key.into(), value);
    }

    /// Get the duration since connection was established.
    pub fn connection_duration(&self) -> Option<Duration> {
        self.connected_since.map(|since| {
            std::time::SystemTime::now()
                .duration_since(since)
                .unwrap_or_default()
        })
    }
}

/// Message sender for internal transport communication.
///
/// This type is used internally by transport implementations to send
/// messages between different async tasks (e.g., reader and writer tasks).
pub type MessageSender = mpsc::UnboundedSender<JsonRpcMessage>;

/// Message receiver for internal transport communication.
///
/// This type is used internally by transport implementations to receive
/// messages from different async tasks.
pub type MessageReceiver = mpsc::UnboundedReceiver<JsonRpcMessage>;

/// Helper trait for transport implementations.
///
/// This trait provides common functionality that most transport implementations
/// will need, such as message correlation, timeout handling, etc.
pub trait TransportHelper {
    /// Generate a unique request ID.
    fn generate_request_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// Create a timeout future for the given duration.
    fn timeout_future(duration: Duration) -> tokio::time::Sleep {
        tokio::time::sleep(duration)
    }

    /// Validate that a JSON-RPC message is well-formed.
    fn validate_message(message: &JsonRpcMessage) -> McpResult<()> {
        match message {
            JsonRpcMessage::Request(req) => {
                if req.jsonrpc != "2.0" {
                    return Err(TransportError::InvalidConfig {
                        transport_type: "generic".to_string(),
                        reason: format!("Invalid jsonrpc version: {}", req.jsonrpc),
                    }
                    .into());
                }
            }
            JsonRpcMessage::Response(resp) => {
                if resp.jsonrpc != "2.0" {
                    return Err(TransportError::InvalidConfig {
                        transport_type: "generic".to_string(),
                        reason: format!("Invalid jsonrpc version: {}", resp.jsonrpc),
                    }
                    .into());
                }
            }
            JsonRpcMessage::Notification(notif) => {
                if notif.jsonrpc != "2.0" {
                    return Err(TransportError::InvalidConfig {
                        transport_type: "generic".to_string(),
                        reason: format!("Invalid jsonrpc version: {}", notif.jsonrpc),
                    }
                    .into());
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_info_creation() {
        let mut info = TransportInfo::new("test");
        assert_eq!(info.transport_type, "test");
        assert!(!info.connected);
        assert_eq!(info.requests_sent, 0);

        info.mark_connected();
        assert!(info.connected);
        assert!(info.connected_since.is_some());

        info.increment_requests_sent();
        assert_eq!(info.requests_sent, 1);
    }

    #[test]
    fn test_transport_info_metadata() {
        let mut info = TransportInfo::new("test");
        info.add_metadata("version", serde_json::json!("1.0.0"));

        assert_eq!(
            info.metadata.get("version").unwrap(),
            &serde_json::json!("1.0.0")
        );
    }

    #[test]
    fn test_connection_duration() {
        let mut info = TransportInfo::new("test");
        assert!(info.connection_duration().is_none());

        info.mark_connected();
        let duration = info.connection_duration();
        assert!(duration.is_some());
        assert!(duration.unwrap().as_millis() < 100); // Should be very small

        info.mark_disconnected();
        assert!(info.connection_duration().is_none());
    }
}
