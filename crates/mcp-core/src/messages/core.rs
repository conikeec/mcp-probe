//! Core JSON-RPC 2.0 message structures for MCP communication.
//!
//! This module provides the fundamental JSON-RPC message types that form the
//! foundation of all MCP communication. These types strictly follow the
//! JSON-RPC 2.0 specification with MCP-specific extensions.
//!
//! # Message Types
//!
//! - **Request**: Client-to-server messages expecting a response
//! - **Response**: Server-to-client messages in reply to requests
//! - **Notification**: One-way messages that don't expect responses
//! - **Error**: Error responses for failed requests
//!
//! # Examples
//!
//! ```rust
//! use mcp_probe_core::messages::core::{JsonRpcRequest, JsonRpcResponse, JsonRpcError};
//! use serde_json::json;
//!
//! // Create a request
//! let request = JsonRpcRequest::new(
//!     "1".to_string(),
//!     "tools/list".to_string(),
//!     json!({}),
//! );
//!
//! // Create a success response
//! let response = JsonRpcResponse::success("1".to_string(), json!({"tools": []}));
//!
//! // Create an error response
//! let error_response = JsonRpcResponse::error(
//!     "1".to_string(),
//!     JsonRpcError::method_not_found("unknown_method"),
//! );
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// JSON-RPC 2.0 request message.
///
/// Represents a request from client to server that expects a response.
/// All MCP operations (except notifications) use this message type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,

    /// Unique identifier for request/response correlation
    pub id: RequestId,

    /// Method name being invoked
    pub method: String,

    /// Parameters for the method (can be object or array)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    /// Create a new JSON-RPC request with the given ID, method, and parameters.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::core::JsonRpcRequest;
    /// use serde_json::json;
    ///
    /// let request = JsonRpcRequest::new(
    ///     "1".to_string(),
    ///     "initialize".to_string(),
    ///     json!({"protocolVersion": "2024-11-05"}),
    /// );
    /// ```
    pub fn new(id: impl Into<RequestId>, method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            method: method.into(),
            params: Some(params),
        }
    }

    /// Create a new JSON-RPC request without parameters.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::core::JsonRpcRequest;
    ///
    /// let request = JsonRpcRequest::without_params("1", "tools/list");
    /// ```
    pub fn without_params(id: impl Into<RequestId>, method: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            method: method.into(),
            params: None,
        }
    }

    /// Generate a new request with a random UUID as the ID.
    ///
    /// This is useful when you don't need to track specific request IDs.
    pub fn with_random_id(method: impl Into<String>, params: Value) -> Self {
        Self::new(Uuid::new_v4().to_string(), method, params)
    }

    /// Check if this request has parameters.
    pub fn has_params(&self) -> bool {
        self.params.is_some()
    }

    /// Get the parameters as a specific type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::core::JsonRpcRequest;
    /// use serde_json::json;
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Deserialize, Serialize)]
    /// struct InitParams {
    ///     #[serde(rename = "protocolVersion")]
    ///     protocol_version: String,
    /// }
    ///
    /// let request = JsonRpcRequest::new(
    ///     "1",
    ///     "initialize",
    ///     json!({"protocolVersion": "2024-11-05"})
    /// );
    ///
    /// let params: InitParams = request.params_as().unwrap();
    /// assert_eq!(params.protocol_version, "2024-11-05");
    /// ```
    pub fn params_as<T>(&self) -> Result<T, serde_json::Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        match &self.params {
            Some(params) => serde_json::from_value(params.clone()),
            None => serde_json::from_value(Value::Null),
        }
    }
}

/// JSON-RPC 2.0 response message.
///
/// Represents a response from server to client for a previous request.
/// Can contain either a successful result or an error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,

    /// ID from the corresponding request
    pub id: RequestId,

    /// Success result (mutually exclusive with error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,

    /// Error result (mutually exclusive with result)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Create a successful response with the given result.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::core::JsonRpcResponse;
    /// use serde_json::json;
    ///
    /// let response = JsonRpcResponse::success("1", json!({"status": "ok"}));
    /// ```
    pub fn success(id: impl Into<RequestId>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response with the given error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::core::{JsonRpcResponse, JsonRpcError};
    ///
    /// let response = JsonRpcResponse::error(
    ///     "1",
    ///     JsonRpcError::method_not_found("unknown_method"),
    /// );
    /// ```
    pub fn error(id: impl Into<RequestId>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            result: None,
            error: Some(error),
        }
    }

    /// Check if this response represents a success.
    pub fn is_success(&self) -> bool {
        self.result.is_some() && self.error.is_none()
    }

    /// Check if this response represents an error.
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Get the result as a specific type.
    ///
    /// Returns an error if the response is an error response or if
    /// deserialization fails.
    pub fn result_as<T>(&self) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
    where
        T: for<'de> Deserialize<'de>,
    {
        match (&self.result, &self.error) {
            (Some(result), None) => Ok(serde_json::from_value(result.clone())?),
            (None, Some(error)) => Err(format!("JSON-RPC error: {error}").into()),
            _ => Err("Invalid response: both result and error are present or missing".into()),
        }
    }
}

/// JSON-RPC 2.0 notification message.
///
/// Represents a one-way message that doesn't expect a response.
/// Used for events, logging, and other fire-and-forget communications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,

    /// Method name being invoked
    pub method: String,

    /// Parameters for the method (can be object or array)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcNotification {
    /// Create a new JSON-RPC notification with the given method and parameters.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::core::JsonRpcNotification;
    /// use serde_json::json;
    ///
    /// let notification = JsonRpcNotification::new(
    ///     "notifications/cancelled",
    ///     json!({"requestId": "1"}),
    /// );
    /// ```
    pub fn new(method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params: Some(params),
        }
    }

    /// Create a new JSON-RPC notification without parameters.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::core::JsonRpcNotification;
    ///
    /// let notification = JsonRpcNotification::without_params("ping");
    /// ```
    pub fn without_params(method: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params: None,
        }
    }

    /// Check if this notification has parameters.
    pub fn has_params(&self) -> bool {
        self.params.is_some()
    }

    /// Get the parameters as a specific type.
    pub fn params_as<T>(&self) -> Result<T, serde_json::Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        match &self.params {
            Some(params) => serde_json::from_value(params.clone()),
            None => serde_json::from_value(Value::Null),
        }
    }
}

/// JSON-RPC 2.0 error object.
///
/// Represents an error that occurred during request processing.
/// Includes standard error codes and optional additional data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Numeric error code
    pub code: i32,

    /// Human-readable error message
    pub message: String,

    /// Additional error data (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    /// Create a new JSON-RPC error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::core::JsonRpcError;
    /// use serde_json::json;
    ///
    /// let error = JsonRpcError::new(-32000, "Custom error", Some(json!({"details": "More info"})));
    /// ```
    pub fn new(code: i32, message: impl Into<String>, data: Option<Value>) -> Self {
        Self {
            code,
            message: message.into(),
            data,
        }
    }

    /// Create a "Parse error" (-32700).
    ///
    /// Used when the JSON cannot be parsed.
    pub fn parse_error() -> Self {
        Self::new(-32700, "Parse error", None)
    }

    /// Create an "Invalid Request" error (-32600).
    ///
    /// Used when the request is not a valid JSON-RPC request.
    pub fn invalid_request(details: impl Into<String>) -> Self {
        Self::new(
            -32600,
            "Invalid Request",
            Some(Value::String(details.into())),
        )
    }

    /// Create a "Method not found" error (-32601).
    ///
    /// Used when the requested method doesn't exist.
    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self::new(
            -32601,
            "Method not found",
            Some(Value::String(format!(
                "Method '{}' not found",
                method.into()
            ))),
        )
    }

    /// Create an "Invalid params" error (-32602).
    ///
    /// Used when method parameters are invalid.
    pub fn invalid_params(details: impl Into<String>) -> Self {
        Self::new(
            -32602,
            "Invalid params",
            Some(Value::String(details.into())),
        )
    }

    /// Create an "Internal error" (-32603).
    ///
    /// Used for server-side internal errors.
    pub fn internal_error(details: impl Into<String>) -> Self {
        Self::new(
            -32603,
            "Internal error",
            Some(Value::String(details.into())),
        )
    }

    /// Create a custom application error.
    ///
    /// Custom error codes should be in the range -32000 to -32099.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::messages::core::JsonRpcError;
    ///
    /// let error = JsonRpcError::application_error(
    ///     -32000,
    ///     "Tool execution failed",
    ///     "Tool 'calculator' returned non-zero exit code",
    /// );
    /// ```
    pub fn application_error(
        code: i32,
        message: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self::new(code, message, Some(Value::String(details.into())))
    }

    /// Check if this is a standard JSON-RPC error (vs application-specific).
    pub fn is_standard_error(&self) -> bool {
        matches!(self.code, -32700..=-32600)
    }

    /// Check if this is an application-specific error.
    pub fn is_application_error(&self) -> bool {
        matches!(self.code, -32099..=-32000)
    }
}

impl std::fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSON-RPC Error {}: {}", self.code, self.message)?;
        if let Some(data) = &self.data {
            write!(f, " ({data})")?;
        }
        Ok(())
    }
}

impl std::error::Error for JsonRpcError {}

/// Request ID for JSON-RPC messages.
///
/// Can be a string, number, or null according to JSON-RPC 2.0 specification.
/// MCP typically uses string IDs for better traceability.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    /// String identifier
    String(String),
    /// Numeric identifier
    Number(i64),
    /// Null identifier (discouraged in MCP)
    Null,
}

impl From<String> for RequestId {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for RequestId {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<i64> for RequestId {
    fn from(n: i64) -> Self {
        Self::Number(n)
    }
}

impl From<i32> for RequestId {
    fn from(n: i32) -> Self {
        Self::Number(n as i64)
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => write!(f, "{s}"),
            Self::Number(n) => write!(f, "{n}"),
            Self::Null => write!(f, "null"),
        }
    }
}

/// Enum for any JSON-RPC message type.
///
/// This is useful for generic message handling where you need to
/// differentiate between requests, responses, and notifications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    /// Request message
    Request(JsonRpcRequest),
    /// Response message
    Response(JsonRpcResponse),
    /// Notification message
    Notification(JsonRpcNotification),
}

impl JsonRpcMessage {
    /// Get the method name if this is a request or notification.
    pub fn method(&self) -> Option<&str> {
        match self {
            Self::Request(req) => Some(&req.method),
            Self::Notification(notif) => Some(&notif.method),
            Self::Response(_) => None,
        }
    }

    /// Get the request ID if this is a request or response.
    pub fn id(&self) -> Option<&RequestId> {
        match self {
            Self::Request(req) => Some(&req.id),
            Self::Response(resp) => Some(&resp.id),
            Self::Notification(_) => None,
        }
    }

    /// Check if this message expects a response.
    pub fn expects_response(&self) -> bool {
        matches!(self, Self::Request(_))
    }
}

impl From<JsonRpcRequest> for JsonRpcMessage {
    fn from(req: JsonRpcRequest) -> Self {
        Self::Request(req)
    }
}

impl From<JsonRpcResponse> for JsonRpcMessage {
    fn from(resp: JsonRpcResponse) -> Self {
        Self::Response(resp)
    }
}

impl From<JsonRpcNotification> for JsonRpcMessage {
    fn from(notif: JsonRpcNotification) -> Self {
        Self::Notification(notif)
    }
}

/// Type alias for request IDs to match client code expectations.
pub type JsonRpcId = RequestId;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_creation() {
        let request = JsonRpcRequest::new("1", "test_method", json!({"param": "value"}));

        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, RequestId::String("1".to_string()));
        assert_eq!(request.method, "test_method");
        assert!(request.has_params());
    }

    #[test]
    fn test_request_without_params() {
        let request = JsonRpcRequest::without_params("1", "test_method");

        assert!(!request.has_params());
        assert_eq!(request.params, None);
    }

    #[test]
    fn test_success_response() {
        let response = JsonRpcResponse::success("1", json!({"result": "ok"}));

        assert!(response.is_success());
        assert!(!response.is_error());
        assert_eq!(response.id, RequestId::String("1".to_string()));
    }

    #[test]
    fn test_error_response() {
        let error = JsonRpcError::method_not_found("unknown");
        let response = JsonRpcResponse::error("1", error);

        assert!(!response.is_success());
        assert!(response.is_error());
        assert_eq!(response.error.as_ref().unwrap().code, -32601);
    }

    #[test]
    fn test_notification_creation() {
        let notification = JsonRpcNotification::new("event", json!({"data": "value"}));

        assert_eq!(notification.method, "event");
        assert!(notification.has_params());
    }

    #[test]
    fn test_json_rpc_error_types() {
        let parse_error = JsonRpcError::parse_error();
        assert_eq!(parse_error.code, -32700);
        assert!(parse_error.is_standard_error());

        let app_error = JsonRpcError::application_error(-32000, "App error", "Details");
        assert_eq!(app_error.code, -32000);
        assert!(app_error.is_application_error());
    }

    #[test]
    fn test_request_id_variants() {
        let string_id = RequestId::from("test");
        let number_id = RequestId::from(42i64);
        let null_id = RequestId::Null;

        assert_eq!(string_id.to_string(), "test");
        assert_eq!(number_id.to_string(), "42");
        assert_eq!(null_id.to_string(), "null");
    }

    #[test]
    fn test_message_serialization() {
        let request = JsonRpcRequest::new("1", "test", json!({}));
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: JsonRpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request, deserialized);
    }

    #[test]
    fn test_generic_message_handling() {
        let request = JsonRpcMessage::Request(JsonRpcRequest::new("1", "test", json!({})));
        let notification =
            JsonRpcMessage::Notification(JsonRpcNotification::new("event", json!({})));

        assert_eq!(request.method(), Some("test"));
        assert_eq!(notification.method(), Some("event"));

        assert!(request.expects_response());
        assert!(!notification.expects_response());
    }
}
