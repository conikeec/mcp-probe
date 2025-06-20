//! Error types for MCP (Model Context Protocol) operations.
//!
//! This module provides comprehensive error handling for all MCP operations,
//! including transport-specific errors, protocol errors, and validation errors.
//! 
//! # Design Philosophy
//! 
//! The error system is designed to be:
//! - **Informative**: Provide clear, actionable error messages
//! - **Structured**: Use strongly-typed error variants for programmatic handling  
//! - **Transport-aware**: Include transport-specific error context
//! - **Debuggable**: Include sufficient context for debugging
//! - **User-friendly**: Format appropriately for end-user display

use thiserror::Error;
use std::time::Duration;

/// The main error type for all MCP operations.
/// 
/// This enum covers all possible error conditions that can occur during
/// MCP client operations, from transport failures to protocol violations.
/// 
/// # Examples
/// 
/// ```rust
/// use mcp_core::error::{McpError, TransportError};
/// 
/// let error = McpError::Transport(TransportError::ConnectionFailed {
///     transport_type: "stdio".to_string(),
///     reason: "Process exited unexpectedly".to_string(),
/// });
/// 
/// println!("Error: {}", error);
/// ```
#[derive(Error, Debug)]
pub enum McpError {
    /// Transport-related errors (connection, communication, etc.)
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),

    /// Protocol-level errors (invalid messages, unsupported versions, etc.)
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    /// Validation errors (schema validation, capability mismatches, etc.)
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    /// Authentication and authorization errors
    #[error("Authentication error: {0}")]
    Auth(#[from] AuthError),

    /// Timeout errors for operations that exceed time limits
    #[error("Operation timed out after {duration_ms}ms: {operation}")]
    Timeout {
        /// The operation that timed out
        operation: String,
        /// The timeout duration in milliseconds
        duration_ms: u64,
    },

    /// Configuration errors (invalid config files, missing parameters, etc.)
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// Serialization/deserialization errors
    #[error("Serialization error: {source}")]
    Serialization {
        #[from]
        /// The underlying serde_json error
        source: serde_json::Error,
    },

    /// IO errors (file operations, network errors, etc.)
    #[error("IO error: {source}")]
    Io {
        #[from]
        /// The underlying IO error
        source: std::io::Error,
    },

    /// Generic errors for cases not covered by specific variants
    #[error("Internal error: {message}")]
    Internal { 
        /// Error message
        message: String 
    },
}

/// Transport-specific errors for different MCP transport mechanisms.
/// 
/// Each transport type (stdio, HTTP+SSE, HTTP streaming) can have
/// specific failure modes that need to be handled appropriately.
#[derive(Error, Debug, Clone)]
#[allow(missing_docs)]
pub enum TransportError {
    /// Failed to establish connection to the MCP server
    #[error("Failed to connect to {transport_type} server: {reason}")]
    ConnectionFailed {
        transport_type: String,
        reason: String,
    },

    /// Connection was lost during operation
    #[error("Connection lost to {transport_type} server: {reason}")]
    ConnectionLost {
        transport_type: String,
        reason: String,
    },

    /// Failed to send message to server
    #[error("Failed to send message via {transport_type}: {reason}")]
    SendFailed {
        transport_type: String,
        reason: String,
    },

    /// Failed to receive message from server
    #[error("Failed to receive message via {transport_type}: {reason}")]
    ReceiveFailed {
        transport_type: String,
        reason: String,
    },

    /// Transport-specific configuration is invalid
    #[error("Invalid {transport_type} configuration: {reason}")]
    InvalidConfig {
        transport_type: String,
        reason: String,
    },

    /// Process-related errors for stdio transport
    #[error("Process error: {reason}")]
    ProcessError { reason: String },

    /// HTTP-specific errors for HTTP transports
    #[error("HTTP error: {status_code} - {reason}")]
    HttpError {
        status_code: u16,
        reason: String,
    },

    /// Server-Sent Events specific errors
    #[error("SSE error: {reason}")]
    SseError { reason: String },

    /// Streaming protocol errors
    #[error("Streaming error: {reason}")]
    StreamingError { reason: String },

    /// Transport is not connected
    #[error("Transport not connected ({transport_type}): {reason}")]
    NotConnected {
        transport_type: String,
        reason: String,
    },

    /// Generic network error
    #[error("Network error ({transport_type}): {reason}")]
    NetworkError {
        transport_type: String,
        reason: String,
    },

    /// Message serialization/deserialization error
    #[error("Serialization error ({transport_type}): {reason}")]
    SerializationError {
        transport_type: String,
        reason: String,
    },

    /// Operation timed out
    #[error("Operation timed out ({transport_type}): {reason}")]
    TimeoutError {
        transport_type: String,
        reason: String,
    },

    /// Transport unexpectedly disconnected
    #[error("Transport disconnected ({transport_type}): {reason}")]
    DisconnectedError {
        transport_type: String,
        reason: String,
    },

    /// Connection error (alias for ConnectionFailed for compatibility)
    #[error("Connection error ({transport_type}): {reason}")]
    ConnectionError {
        transport_type: String,
        reason: String,
    },
}

/// Protocol-level errors related to MCP message handling.
/// 
/// These errors occur when messages don't conform to the MCP specification
/// or when protocol violations are detected.
#[derive(Error, Debug, Clone)]
#[allow(missing_docs)]
pub enum ProtocolError {
    /// Invalid JSON-RPC message format
    #[error("Invalid JSON-RPC message: {reason}")]
    InvalidJsonRpc { reason: String },

    /// Unsupported MCP protocol version
    #[error("Unsupported protocol version: {version}, supported versions: {supported:?}")]
    UnsupportedVersion {
        version: String,
        supported: Vec<String>,
    },

    /// Message ID mismatch in request/response correlation
    #[error("Message ID mismatch: expected {expected}, got {actual}")]
    MessageIdMismatch { expected: String, actual: String },

    /// Unexpected message type received
    #[error("Unexpected message type: expected {expected}, got {actual}")]
    UnexpectedMessageType { expected: String, actual: String },

    /// Required field missing from message
    #[error("Missing required field '{field}' in {message_type}")]
    MissingField {
        field: String,
        message_type: String,
    },

    /// Invalid method name for MCP operation
    #[error("Invalid method name: {method}")]
    InvalidMethod { method: String },

    /// Server returned an error response
    #[error("Server error {code}: {message}")]
    ServerError { code: i32, message: String },

    /// Protocol state violation (e.g., calling method before initialization)
    #[error("Protocol state violation: {reason}")]
    StateViolation { reason: String },

    /// Protocol initialization failed
    #[error("Protocol initialization failed: {reason}")]
    InitializationFailed { reason: String },

    /// Operation attempted before protocol initialization
    #[error("Protocol not initialized: {reason}")]
    NotInitialized { reason: String },

    /// Invalid or malformed response
    #[error("Invalid response: {reason}")]
    InvalidResponse { reason: String },

    /// Configuration error in protocol settings
    #[error("Protocol configuration error: {reason}")]
    InvalidConfig { reason: String },

    /// Operation timeout
    #[error("Protocol operation '{operation}' timed out after {timeout:?}")]
    TimeoutError {
        operation: String,
        timeout: std::time::Duration,
    },

    /// Request failed
    #[error("Request failed: {reason}")]
    RequestFailed {
        reason: String,  
    },
    
    /// Request timed out
    #[error("Request timed out after {timeout:?}")]
    RequestTimeout {
        timeout: Duration,
    },
}

/// Validation errors for MCP capabilities and schemas.
/// 
/// These errors occur during validation of server capabilities,
/// tool parameters, resource schemas, etc.
#[derive(Error, Debug, Clone)]
#[allow(missing_docs)]
pub enum ValidationError {
    /// Schema validation failed
    #[error("Schema validation failed for {object_type}: {reason}")]
    SchemaValidation {
        object_type: String,
        reason: String,
    },

    /// Capability not supported by server
    #[error("Capability '{capability}' not supported by server")]
    UnsupportedCapability { capability: String },

    /// Tool parameter validation failed
    #[error("Invalid parameter '{parameter}' for tool '{tool}': {reason}")]
    InvalidToolParameter {
        tool: String,
        parameter: String,
        reason: String,
    },

    /// Resource validation failed
    #[error("Invalid resource '{resource}': {reason}")]
    InvalidResource { resource: String, reason: String },

    /// Prompt validation failed
    #[error("Invalid prompt '{prompt}': {reason}")]
    InvalidPrompt { prompt: String, reason: String },

    /// Constraint violation (size limits, rate limits, etc.)
    #[error("Constraint violation: {constraint} - {reason}")]
    ConstraintViolation { constraint: String, reason: String },
}

/// Authentication and authorization errors.
/// 
/// These errors cover all aspects of authentication and authorization
/// for different transport types and authentication schemes.
#[derive(Error, Debug, Clone)]
#[allow(missing_docs)]
pub enum AuthError {
    /// Missing required authentication credentials
    #[error("Missing authentication credentials for {auth_type}")]
    MissingCredentials { auth_type: String },

    /// Invalid authentication credentials
    #[error("Invalid {auth_type} credentials: {reason}")]
    InvalidCredentials { auth_type: String, reason: String },

    /// Authentication expired and needs renewal
    #[error("Authentication expired for {auth_type}")]
    Expired { auth_type: String },

    /// Access denied for requested operation
    #[error("Access denied: {reason}")]
    AccessDenied { reason: String },

    /// OAuth-specific errors
    #[error("OAuth error: {error_code} - {description}")]
    OAuth {
        error_code: String,
        description: String,
    },

    /// JWT token errors
    #[error("JWT error: {reason}")]
    Jwt { reason: String },
}

/// Configuration-related errors.
/// 
/// These errors occur when configuration files are invalid,
/// missing required parameters, or contain conflicting settings.
#[derive(Error, Debug, Clone)]
#[allow(missing_docs)]
pub enum ConfigError {
    /// Configuration file not found
    #[error("Configuration file not found: {path}")]
    FileNotFound { path: String },

    /// Configuration file has invalid format
    #[error("Invalid configuration format in {path}: {reason}")]
    InvalidFormat { path: String, reason: String },

    /// Required configuration parameter is missing
    #[error("Missing required configuration parameter: {parameter}")]
    MissingParameter { parameter: String },

    /// Configuration parameter has invalid value
    #[error("Invalid value for parameter '{parameter}': {value} - {reason}")]
    InvalidValue {
        parameter: String,
        value: String,
        reason: String,
    },

    /// Conflicting configuration parameters
    #[error("Conflicting configuration: {reason}")]
    Conflict { reason: String },
}

/// Convenience type alias for Results using McpError.
pub type McpResult<T> = Result<T, McpError>;

impl McpError {
    /// Create a new internal error with a custom message.
    /// 
    /// This is useful for creating errors from string messages when
    /// a more specific error type is not available.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use mcp_core::error::McpError;
    /// 
    /// let error = McpError::internal("Something went wrong");
    /// ```
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Create a new timeout error.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use mcp_core::error::McpError;
    /// use std::time::Duration;
    /// 
    /// let error = McpError::timeout("server connection", Duration::from_secs(30));
    /// ```
    pub fn timeout(operation: impl Into<String>, duration: std::time::Duration) -> Self {
        Self::Timeout {
            operation: operation.into(),
            duration_ms: duration.as_millis() as u64,
        }
    }

    /// Check if this error is retryable.
    /// 
    /// Some errors (like network timeouts) may be worth retrying,
    /// while others (like invalid credentials) are permanent failures.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use mcp_core::error::{McpError, TransportError};
    /// 
    /// let timeout_error = McpError::timeout("connection", std::time::Duration::from_secs(30));
    /// assert!(timeout_error.is_retryable());
    /// 
    /// let auth_error = McpError::Auth(
    ///     mcp_core::error::AuthError::InvalidCredentials {
    ///         auth_type: "Bearer".to_string(),
    ///         reason: "Invalid token".to_string(),
    ///     }
    /// );
    /// assert!(!auth_error.is_retryable());
    /// ```
    pub fn is_retryable(&self) -> bool {
        match self {
            McpError::Transport(transport_err) => transport_err.is_retryable(),
            McpError::Timeout { .. } => true,
            McpError::Io { .. } => true,
            McpError::Auth(_) => false,
            McpError::Protocol(_) => false,
            McpError::Validation(_) => false,
            McpError::Config(_) => false,
            McpError::Serialization { .. } => false,
            McpError::Internal { .. } => false,
        }
    }

    /// Get the error category for this error.
    /// 
    /// This is useful for error reporting and metrics collection.
    pub fn category(&self) -> &'static str {
        match self {
            McpError::Transport(_) => "transport",
            McpError::Protocol(_) => "protocol",
            McpError::Validation(_) => "validation",
            McpError::Auth(_) => "auth",
            McpError::Timeout { .. } => "timeout",
            McpError::Config(_) => "config",
            McpError::Serialization { .. } => "serialization",
            McpError::Io { .. } => "io",
            McpError::Internal { .. } => "internal",
        }
    }
}

impl TransportError {
    /// Check if this transport error is retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            TransportError::ConnectionFailed { .. } => true,
            TransportError::ConnectionLost { .. } => true,
            TransportError::ConnectionError { .. } => true,
            TransportError::SendFailed { .. } => true,
            TransportError::ReceiveFailed { .. } => true,
            TransportError::NetworkError { .. } => true,
            TransportError::TimeoutError { .. } => true,
            TransportError::DisconnectedError { .. } => true,
            TransportError::HttpError { status_code, .. } => {
                // 5xx errors are generally retryable, 4xx are not
                *status_code >= 500
            }
            TransportError::SseError { .. } => true,
            TransportError::StreamingError { .. } => true,
            TransportError::ProcessError { .. } => false,
            TransportError::InvalidConfig { .. } => false,
            TransportError::NotConnected { .. } => false,
            TransportError::SerializationError { .. } => false,
        }
    }
}

impl From<reqwest::Error> for McpError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            McpError::timeout("HTTP request", std::time::Duration::from_secs(30))
        } else if err.is_connect() {
            McpError::Transport(TransportError::ConnectionFailed {
                transport_type: "http".to_string(),
                reason: err.to_string(),
            })
        } else if let Some(status) = err.status() {
            McpError::Transport(TransportError::HttpError {
                status_code: status.as_u16(),
                reason: err.to_string(),
            })
        } else {
            McpError::Transport(TransportError::HttpError {
                status_code: 0,
                reason: err.to_string(),
            })
        }
    }
}

impl From<url::ParseError> for McpError {
    fn from(err: url::ParseError) -> Self {
        McpError::Config(ConfigError::InvalidValue {
            parameter: "url".to_string(),
            value: err.to_string(),
            reason: "Invalid URL format".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_error_display() {
        let error = McpError::timeout("test operation", Duration::from_secs(30));
        assert_eq!(
            error.to_string(),
            "Operation timed out after 30000ms: test operation"
        );
    }

    #[test]
    fn test_retryable_errors() {
        let timeout = McpError::timeout("test", Duration::from_secs(30));
        assert!(timeout.is_retryable());

        let auth_error = McpError::Auth(AuthError::InvalidCredentials {
            auth_type: "Bearer".to_string(),
            reason: "Invalid token".to_string(),
        });
        assert!(!auth_error.is_retryable());
    }

    #[test]
    fn test_error_categories() {
        let timeout = McpError::timeout("test", Duration::from_secs(30));
        assert_eq!(timeout.category(), "timeout");

        let transport_error = McpError::Transport(TransportError::ConnectionFailed {
            transport_type: "stdio".to_string(),
            reason: "Process failed".to_string(),
        });
        assert_eq!(transport_error.category(), "transport");
    }

    #[test]
    fn test_transport_error_retryable() {
        let connection_failed = TransportError::ConnectionFailed {
            transport_type: "stdio".to_string(),
            reason: "Process failed".to_string(),
        };
        assert!(connection_failed.is_retryable());

        let invalid_config = TransportError::InvalidConfig {
            transport_type: "stdio".to_string(),
            reason: "Missing command".to_string(),
        };
        assert!(!invalid_config.is_retryable());
    }
} 