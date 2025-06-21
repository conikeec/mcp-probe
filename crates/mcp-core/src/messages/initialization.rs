//! MCP initialization and protocol negotiation message types.
//!
//! This module contains message structures for the MCP initialization sequence:
//! 1. Client sends `initialize` request with capabilities and client info
//! 2. Server responds with its capabilities and server info
//! 3. Client sends `initialized` notification to complete handshake
//!
//! The initialization sequence establishes:
//! - Protocol version compatibility
//! - Mutual capability negotiation
//! - Client/server identification and metadata
//!
//! # Examples
//!
//! ```rust
//! use mcp_probe_core::messages::{InitializeRequest, ProtocolVersion, Capabilities, Implementation};
//! use serde_json::json;
//!
//! // Create client initialization request
//! let init_request = InitializeRequest {
//!     protocol_version: ProtocolVersion::V2024_11_05,
//!     capabilities: Capabilities::default(),
//!     client_info: Implementation::new("mcp-probe", "0.1.0"),
//! };
//! ```

use super::{Capabilities, Implementation, ProtocolVersion};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Client-to-server initialization request.
///
/// This is the first message sent by the client to establish the MCP session.
/// It includes the desired protocol version, client capabilities, and client metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitializeRequest {
    /// Protocol version requested by the client
    #[serde(rename = "protocolVersion")]
    pub protocol_version: ProtocolVersion,

    /// Capabilities offered by the client
    pub capabilities: Capabilities,

    /// Information about the client implementation
    #[serde(rename = "clientInfo")]
    pub client_info: Implementation,
}

impl InitializeRequest {
    /// Create a new initialization request.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::{InitializeRequest, ProtocolVersion, Capabilities, Implementation};
    ///
    /// let request = InitializeRequest::new(
    ///     ProtocolVersion::V2024_11_05,
    ///     Capabilities::default(),
    ///     Implementation::new("my-client", "1.0.0"),
    /// );
    /// ```
    pub fn new(
        protocol_version: ProtocolVersion,
        capabilities: Capabilities,
        client_info: Implementation,
    ) -> Self {
        Self {
            protocol_version,
            capabilities,
            client_info,
        }
    }

    /// Create a basic initialization request with default capabilities.
    ///
    /// This is a convenience method for simple clients that don't need
    /// to specify custom capabilities.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::InitializeRequest;
    ///
    /// let request = InitializeRequest::basic("my-client", "1.0.0");
    /// ```
    pub fn basic(client_name: impl Into<String>, client_version: impl Into<String>) -> Self {
        Self::new(
            ProtocolVersion::default(),
            Capabilities::default(),
            Implementation::new(client_name, client_version),
        )
    }

    /// Add custom client metadata to the initialization request.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::InitializeRequest;
    /// use serde_json::json;
    ///
    /// let request = InitializeRequest::basic("my-client", "1.0.0")
    ///     .with_client_metadata("platform", json!("rust"))
    ///     .with_client_metadata("os", json!("linux"));
    /// ```
    pub fn with_client_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.client_info.metadata.insert(key.into(), value);
        self
    }

    /// Check if the requested protocol version is supported.
    pub fn is_supported_version(&self) -> bool {
        self.protocol_version.is_supported()
    }
}

/// Server-to-client initialization response.
///
/// This is sent by the server in response to the client's initialization request.
/// It includes the server's capabilities, server metadata, and the negotiated protocol version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitializeResponse {
    /// Protocol version that will be used for the session
    #[serde(rename = "protocolVersion")]
    pub protocol_version: ProtocolVersion,

    /// Capabilities offered by the server
    pub capabilities: Capabilities,

    /// Information about the server implementation
    #[serde(rename = "serverInfo")]
    pub server_info: Implementation,

    /// Optional instructions or additional information for the client
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

impl InitializeResponse {
    /// Create a new initialization response.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::{InitializeResponse, ProtocolVersion, Capabilities, Implementation};
    ///
    /// let response = InitializeResponse::new(
    ///     ProtocolVersion::V2024_11_05,
    ///     Capabilities::default(),
    ///     Implementation::new("my-server", "1.0.0"),
    ///     None,
    /// );
    /// ```
    pub fn new(
        protocol_version: ProtocolVersion,
        capabilities: Capabilities,
        server_info: Implementation,
        instructions: Option<String>,
    ) -> Self {
        Self {
            protocol_version,
            capabilities,
            server_info,
            instructions,
        }
    }

    /// Create a basic initialization response with default capabilities.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::InitializeResponse;
    ///
    /// let response = InitializeResponse::basic("my-server", "1.0.0");
    /// ```
    pub fn basic(server_name: impl Into<String>, server_version: impl Into<String>) -> Self {
        Self::new(
            ProtocolVersion::default(),
            Capabilities::default(),
            Implementation::new(server_name, server_version),
            None,
        )
    }

    /// Add instructions for the client.
    ///
    /// Instructions can provide guidance to the client about how to use
    /// the server's capabilities effectively.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::InitializeResponse;
    ///
    /// let response = InitializeResponse::basic("my-server", "1.0.0")
    ///     .with_instructions("Use the 'calculator' tool for math operations");
    /// ```
    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Add custom server metadata to the initialization response.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::InitializeResponse;
    /// use serde_json::json;
    ///
    /// let response = InitializeResponse::basic("my-server", "1.0.0")
    ///     .with_server_metadata("supported_languages", json!(["python", "javascript"]));
    /// ```
    pub fn with_server_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.server_info.metadata.insert(key.into(), value);
        self
    }
}

/// Client-to-server initialization completion notification.
///
/// This notification is sent by the client after receiving and processing
/// the server's initialization response. It signals that the client is
/// ready to begin normal operations.
///
/// This is a notification (not a request), so the server should not respond.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitializedNotification {
    /// Optional metadata about the initialization completion
    #[serde(flatten)]
    pub metadata: std::collections::HashMap<String, Value>,
}

impl InitializedNotification {
    /// Create a new initialized notification.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::InitializedNotification;
    ///
    /// let notification = InitializedNotification::new();
    /// ```
    pub fn new() -> Self {
        Self {
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create an initialized notification with custom metadata.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::InitializedNotification;
    /// use serde_json::json;
    /// use std::collections::HashMap;
    ///
    /// let mut metadata = HashMap::new();
    /// metadata.insert("client_ready_time".to_string(), json!("2024-01-15T10:30:00Z"));
    ///
    /// let notification = InitializedNotification::with_metadata(metadata);
    /// ```
    pub fn with_metadata(metadata: std::collections::HashMap<String, Value>) -> Self {
        Self { metadata }
    }

    /// Add a metadata field to the notification.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::InitializedNotification;
    /// use serde_json::json;
    ///
    /// let notification = InitializedNotification::new()
    ///     .add_metadata("timestamp", json!("2024-01-15T10:30:00Z"));
    /// ```
    pub fn add_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

impl Default for InitializedNotification {
    fn default() -> Self {
        Self::new()
    }
}

/// Ping request for connection health checking.
///
/// Either client or server can send ping requests to verify that the
/// connection is still active and responsive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PingRequest {
    /// Optional metadata for the ping
    #[serde(flatten)]
    pub metadata: std::collections::HashMap<String, Value>,
}

impl PingRequest {
    /// Create a new ping request.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::PingRequest;
    ///
    /// let ping = PingRequest::new();
    /// ```
    pub fn new() -> Self {
        Self {
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create a ping request with a timestamp.
    ///
    /// The timestamp can be used to measure round-trip time.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::PingRequest;
    /// use serde_json::json;
    ///
    /// let ping = PingRequest::with_timestamp("2024-01-15T10:30:00Z");
    /// ```
    pub fn with_timestamp(timestamp: impl Into<String>) -> Self {
        let mut ping = Self::new();
        ping.metadata
            .insert("timestamp".to_string(), Value::String(timestamp.into()));
        ping
    }

    /// Add metadata to the ping request.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::PingRequest;
    /// use serde_json::json;
    ///
    /// let ping = PingRequest::new()
    ///     .add_metadata("source", json!("health_check"));
    /// ```
    pub fn add_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

impl Default for PingRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Pong response to ping requests.
///
/// This is sent in response to ping requests and can include the original
/// ping metadata for round-trip time calculation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PongResponse {
    /// Optional metadata echoed from the ping or added by the responder
    #[serde(flatten)]
    pub metadata: std::collections::HashMap<String, Value>,
}

impl PongResponse {
    /// Create a new pong response.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::PongResponse;
    ///
    /// let pong = PongResponse::new();
    /// ```
    pub fn new() -> Self {
        Self {
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create a pong response echoing ping metadata.
    ///
    /// This is useful for round-trip time calculation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::{PingRequest, PongResponse};
    /// use serde_json::json;
    ///
    /// let ping = PingRequest::with_timestamp("2024-01-15T10:30:00Z");
    /// let pong = PongResponse::echo(&ping);
    /// ```
    pub fn echo(ping: &PingRequest) -> Self {
        Self {
            metadata: ping.metadata.clone(),
        }
    }

    /// Add metadata to the pong response.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::PongResponse;
    /// use serde_json::json;
    ///
    /// let pong = PongResponse::new()
    ///     .add_metadata("response_time", json!("2024-01-15T10:30:01Z"));
    /// ```
    pub fn add_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

impl Default for PongResponse {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_initialize_request_creation() {
        let request = InitializeRequest::basic("test-client", "1.0.0");

        assert_eq!(request.protocol_version, ProtocolVersion::default());
        assert_eq!(request.client_info.name, "test-client");
        assert_eq!(request.client_info.version, "1.0.0");
        assert!(request.is_supported_version());
    }

    #[test]
    fn test_initialize_request_with_metadata() {
        let request = InitializeRequest::basic("test-client", "1.0.0")
            .with_client_metadata("platform", json!("rust"))
            .with_client_metadata("os", json!("linux"));

        assert_eq!(
            request.client_info.metadata.get("platform").unwrap(),
            &json!("rust")
        );
        assert_eq!(
            request.client_info.metadata.get("os").unwrap(),
            &json!("linux")
        );
    }

    #[test]
    fn test_initialize_response_creation() {
        let response = InitializeResponse::basic("test-server", "2.0.0")
            .with_instructions("Use tools carefully")
            .with_server_metadata("max_tools", json!(10));

        assert_eq!(response.server_info.name, "test-server");
        assert_eq!(
            response.instructions,
            Some("Use tools carefully".to_string())
        );
        assert_eq!(
            response.server_info.metadata.get("max_tools").unwrap(),
            &json!(10)
        );
    }

    #[test]
    fn test_initialized_notification() {
        let notification =
            InitializedNotification::new().add_metadata("timestamp", json!("2024-01-15T10:30:00Z"));

        assert_eq!(
            notification.metadata.get("timestamp").unwrap(),
            &json!("2024-01-15T10:30:00Z")
        );
    }

    #[test]
    fn test_ping_pong() {
        let ping =
            PingRequest::with_timestamp("2024-01-15T10:30:00Z").add_metadata("sequence", json!(1));

        let pong =
            PongResponse::echo(&ping).add_metadata("response_time", json!("2024-01-15T10:30:01Z"));

        assert_eq!(
            pong.metadata.get("timestamp").unwrap(),
            &json!("2024-01-15T10:30:00Z")
        );
        assert_eq!(pong.metadata.get("sequence").unwrap(), &json!(1));
        assert_eq!(
            pong.metadata.get("response_time").unwrap(),
            &json!("2024-01-15T10:30:01Z")
        );
    }

    #[test]
    fn test_serialization_roundtrip() {
        let request = InitializeRequest::basic("test", "1.0.0");
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: InitializeRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request, deserialized);
    }
}
