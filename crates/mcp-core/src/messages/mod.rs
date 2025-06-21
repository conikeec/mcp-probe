//! MCP (Model Context Protocol) message types and JSON-RPC structures.
//!
//! This module provides complete type definitions for all MCP messages according to the
//! MCP specification. All message types are designed to be:
//!
//! - **Spec-compliant**: Exact adherence to MCP JSON-RPC 2.0 protocol
//! - **Type-safe**: Leverage Rust's type system to prevent protocol violations
//! - **Serializable**: Full serde support for JSON serialization/deserialization
//! - **Extensible**: Support for custom extensions and future protocol versions
//! - **Debuggable**: Rich Debug implementations for development and debugging
//!
//! # Message Categories
//!
//! - **Core Messages**: Basic JSON-RPC request/response/notification structures
//! - **Initialization**: Protocol version negotiation and capability discovery
//! - **Tools**: Tool discovery, schema definition, and execution
//! - **Resources**: Resource listing, reading, and subscription
//! - **Prompts**: Prompt templates and completion requests
//! - **Sampling**: LLM completion requests from server to client
//! - **Logging**: Server-to-client logging messages
//!
//! # Examples
//!
//! ```rust
//! use mcp_probe_core::messages::{JsonRpcRequest, InitializeRequest, ProtocolVersion, Implementation};
//! use serde_json::json;
//!
//! // Create an initialization request
//! let init_request = InitializeRequest {
//!     protocol_version: ProtocolVersion::V2024_11_05,
//!     capabilities: Default::default(),
//!     client_info: Implementation {
//!         name: "mcp-probe".to_string(),
//!         version: "0.1.0".to_string(),
//!         metadata: std::collections::HashMap::new(),
//!     },
//! };
//!
//! // Wrap in JSON-RPC request
//! let request = JsonRpcRequest::new(
//!     "1".to_string(),
//!     "initialize".to_string(),
//!     serde_json::to_value(init_request).unwrap(),
//! );
//! ```

pub mod core;
pub mod initialization;
pub mod logging;
pub mod prompts;
pub mod resources;
pub mod sampling;
pub mod tools;

pub use core::*;
pub use initialization::*;
pub use logging::{
    LogLevel, LoggingNotification, ProgressNotification,
    PromptListChangedNotification as LoggingPromptListChangedNotification,
    ResourceListChangedNotification as LoggingResourceListChangedNotification,
    ResourceUpdatedNotification as LoggingResourceUpdatedNotification, SetLevelRequest,
    ToolListChangedNotification as LoggingToolListChangedNotification,
};
pub use prompts::{
    GetPromptRequest, GetPromptResponse, ListPromptsRequest, ListPromptsResponse,
    MessageRole as PromptMessageRole, Prompt, PromptContent, PromptListChangedNotification,
    PromptMessage, ResourceReference as PromptResourceReference,
};
pub use resources::{
    ListResourcesRequest, ListResourcesResponse, ReadResourceRequest, ReadResourceResponse,
    Resource, ResourceContent, ResourceListChangedNotification, ResourceUpdatedNotification,
    SubscribeRequest, UnsubscribeRequest,
};
pub use sampling::{
    CompleteRequest, CompleteResponse, CompletionArgument, CompletionResult, CostPriority,
    IntelligencePriority, MessageRole, ModelPreferences, SamplingContent, SamplingMessage,
    SpeedPriority, StopReason,
};
pub use tools::{
    CallToolRequest, CallToolResponse, ListToolsRequest, ListToolsResponse,
    ResourceReference as ToolResourceReference, Tool, ToolListChangedNotification, ToolResult,
};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP protocol version identifier.
///
/// The MCP protocol uses semantic versioning with date-based versions.
/// This enum provides type-safe handling of supported protocol versions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProtocolVersion {
    /// MCP Protocol version 2024-11-05 (legacy)
    #[serde(rename = "2024-11-05")]
    V2024_11_05,

    /// MCP Protocol version 2025-03-26 (current stable)
    #[serde(rename = "2025-03-26")]
    V2025_03_26,

    /// Future protocol versions can be added here
    /// Custom version string for forward compatibility
    #[serde(untagged)]
    Custom(String),
}

impl ProtocolVersion {
    /// Get the string representation of the protocol version.
    pub fn as_str(&self) -> &str {
        match self {
            Self::V2024_11_05 => "2024-11-05",
            Self::V2025_03_26 => "2025-03-26",
            Self::Custom(version) => version,
        }
    }

    /// Check if this version is supported by the current implementation.
    pub fn is_supported(&self) -> bool {
        matches!(self, Self::V2024_11_05 | Self::V2025_03_26)
    }

    /// Get all supported protocol versions.
    pub fn supported_versions() -> Vec<Self> {
        vec![Self::V2024_11_05, Self::V2025_03_26]
    }
}

impl Default for ProtocolVersion {
    fn default() -> Self {
        Self::V2025_03_26
    }
}

impl std::fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Generic capability structure for extensible capability negotiation.
///
/// Capabilities are used during initialization to negotiate what features
/// both client and server support. This structure allows for both standard
/// and custom capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Capabilities {
    /// Standard MCP capabilities
    #[serde(flatten)]
    pub standard: StandardCapabilities,

    /// Custom or experimental capabilities
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

/// Standard MCP capabilities as defined in the specification.
///
/// These capabilities control what features are available during the MCP session.
/// Both client and server declare their capabilities during initialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StandardCapabilities {
    /// Server capability: Can provide tools for execution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolCapabilities>,

    /// Server capability: Can provide resources for reading
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceCapabilities>,

    /// Server capability: Can provide prompt templates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptCapabilities>,

    /// Client capability: Can handle sampling requests from server
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapabilities>,

    /// Server capability: Can send log messages to client
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingCapabilities>,

    /// Client capability: Can provide root directories for server operations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapabilities>,
}

/// Tool-related capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ToolCapabilities {
    /// Whether the server supports listing changed tools
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Resource-related capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ResourceCapabilities {
    /// Whether the server supports subscribing to resource changes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,

    /// Whether the server supports listing changed resources
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Prompt-related capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PromptCapabilities {
    /// Whether the server supports listing changed prompts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Sampling-related capabilities (client-side).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SamplingCapabilities {
    /// Whether the client supports receiving sampling requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

/// Logging-related capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LoggingCapabilities {
    /// Whether the server supports different log levels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<bool>,
}

/// Roots-related capabilities (client-side).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RootsCapabilities {
    /// Whether the client supports providing root directories
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Implementation information for client or server.
///
/// This provides metadata about the MCP implementation, useful for
/// debugging, telemetry, and compatibility checking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Implementation {
    /// Name of the implementation (e.g., "mcp-probe")
    pub name: String,

    /// Version of the implementation (e.g., "0.1.0")
    pub version: String,

    /// Additional implementation metadata
    #[serde(flatten)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Implementation {
    /// Create a new implementation info structure.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            metadata: HashMap::new(),
        }
    }

    /// Add custom metadata to the implementation info.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Progress token for long-running operations.
///
/// Operations that may take significant time can include progress tokens
/// to allow clients to track progress and provide user feedback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProgressToken {
    /// String-based progress token
    String(String),
    /// Numeric progress token
    Number(i64),
}

impl From<String> for ProgressToken {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for ProgressToken {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<i64> for ProgressToken {
    fn from(n: i64) -> Self {
        Self::Number(n)
    }
}

impl std::fmt::Display for ProgressToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => write!(f, "{}", s),
            Self::Number(n) => write!(f, "{}", n),
        }
    }
}

/// Pagination cursor for list operations.
///
/// Used to support efficient pagination of large result sets in list operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaginationCursor {
    /// Opaque cursor value for pagination
    pub cursor: String,
}

impl PaginationCursor {
    /// Create a new pagination cursor.
    pub fn new(cursor: impl Into<String>) -> Self {
        Self {
            cursor: cursor.into(),
        }
    }
}

impl From<String> for PaginationCursor {
    fn from(cursor: String) -> Self {
        Self::new(cursor)
    }
}

impl From<&str> for PaginationCursor {
    fn from(cursor: &str) -> Self {
        Self::new(cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_protocol_version_serialization() {
        let version = ProtocolVersion::V2024_11_05;
        let json = serde_json::to_string(&version).unwrap();
        assert_eq!(json, "\"2024-11-05\"");

        let deserialized: ProtocolVersion = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, version);
    }

    #[test]
    fn test_protocol_version_custom() {
        let custom = ProtocolVersion::Custom("2025-01-01".to_string());
        assert_eq!(custom.as_str(), "2025-01-01");
        assert!(!custom.is_supported());
    }

    #[test]
    fn test_capabilities_serialization() {
        let capabilities = Capabilities {
            standard: StandardCapabilities {
                tools: Some(ToolCapabilities {
                    list_changed: Some(true),
                }),
                resources: Some(ResourceCapabilities {
                    subscribe: Some(true),
                    list_changed: Some(false),
                }),
                ..Default::default()
            },
            custom: {
                let mut custom = HashMap::new();
                custom.insert("experimental".to_string(), serde_json::json!(true));
                custom
            },
        };

        let json = serde_json::to_value(&capabilities).unwrap();
        let deserialized: Capabilities = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, capabilities);
    }

    #[test]
    fn test_implementation_creation() {
        let impl_info = Implementation::new("mcp-probe", "0.1.0")
            .with_metadata("platform", serde_json::json!("rust"));

        assert_eq!(impl_info.name, "mcp-probe");
        assert_eq!(impl_info.version, "0.1.0");
        assert_eq!(
            impl_info.metadata.get("platform").unwrap(),
            &serde_json::json!("rust")
        );
    }

    #[test]
    fn test_progress_token_variants() {
        let string_token = ProgressToken::from("progress-1");
        let number_token = ProgressToken::from(42i64);

        assert_eq!(string_token.to_string(), "progress-1");
        assert_eq!(number_token.to_string(), "42");

        // Test serialization
        let json_string = serde_json::to_string(&string_token).unwrap();
        let json_number = serde_json::to_string(&number_token).unwrap();

        assert_eq!(json_string, "\"progress-1\"");
        assert_eq!(json_number, "42");
    }
}
