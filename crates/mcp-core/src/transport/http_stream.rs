//! HTTP streaming transport implementation for MCP communication.
//!
//! This transport implements the MCP Streamable HTTP protocol (2025-03-26):
//! - Single /mcp endpoint for all communication
//! - Session management via mcp-session-id headers
//! - Simple request/response pattern

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use tokio::sync::{oneshot, Mutex};
use tokio::time::timeout;
use tracing::{debug, info};

use super::{Transport, TransportConfig, TransportInfo};
use crate::error::{McpError, McpResult, TransportError};
use crate::messages::{
    JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, RequestId,
};

/// MCP Streamable HTTP transport implementation (2025-03-26)
pub struct HttpStreamTransport {
    /// HTTP client for making requests
    client: Client,
    /// Base URL for the MCP server
    base_url: String,
    /// Optional authentication header
    auth_header: Option<String>,
    /// Transport configuration
    config: TransportConfig,
    /// Current session ID from server
    session_id: Option<String>,
    /// Transport information
    info: TransportInfo,
    /// Pending requests awaiting responses
    pending_requests: Arc<Mutex<HashMap<String, oneshot::Sender<JsonRpcResponse>>>>,
    /// Whether we're connected
    connected: bool,
}

impl HttpStreamTransport {
    /// Create a new MCP Streamable HTTP transport.
    pub fn new(base_url: String, auth_header: Option<String>) -> Self {
        let client = Client::new();

        Self {
            client,
            base_url: base_url.clone(),
            auth_header: auth_header.clone(),
            config: TransportConfig::HttpStream(crate::transport::config::HttpStreamConfig {
                base_url: base_url
                    .parse()
                    .unwrap_or_else(|_| "http://localhost".parse().unwrap()),
                timeout: Duration::from_secs(300),
                headers: std::collections::HashMap::new(),
                auth: auth_header.map(crate::transport::config::AuthConfig::bearer),
                compression: true,
                flow_control_window: 65536,
            }),
            session_id: None,
            info: TransportInfo::new("http-stream"),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            connected: false,
        }
    }

    /// Get the MCP endpoint URL
    fn get_mcp_url(&self) -> String {
        // Ensure URL ends with /mcp
        let url = if self.base_url.ends_with('/') {
            format!("{}mcp", self.base_url)
        } else {
            format!("{}/mcp", self.base_url)
        };
        url
    }

    /// Send a JSON-RPC message to the MCP server and parse response
    async fn send_mcp_request(&self, message: &JsonRpcMessage) -> McpResult<JsonRpcResponse> {
        let url = self.get_mcp_url();
        let json_body = serde_json::to_string(message).map_err(|e| {
            McpError::Transport(TransportError::SerializationError {
                transport_type: "http-stream".to_string(),
                reason: format!("Failed to serialize message: {}", e),
            })
        })?;

        debug!("Sending MCP request to {}: {}", url, json_body);

        let mut request_builder = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .body(json_body);

        // Add authentication if provided
        if let Some(auth) = &self.auth_header {
            request_builder = request_builder.header("Authorization", auth);
        }

        // Add session ID if we have one (Modern Streamable HTTP)
        if let Some(session_id) = &self.session_id {
            request_builder = request_builder.header("mcp-session-id", session_id);
        }

        let response = request_builder.send().await.map_err(|e| {
            McpError::Transport(TransportError::NetworkError {
                transport_type: "http-stream".to_string(),
                reason: format!("HTTP request failed: {}", e),
            })
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(McpError::Transport(TransportError::HttpError {
                status_code: status.as_u16(),
                reason: body,
            }));
        }

        // Extract session ID from headers for initialization requests
        if let Some(session_id) = response.headers().get("mcp-session-id") {
            if let Ok(session_str) = session_id.to_str() {
                debug!("Received session ID: {}", session_str);
                // Note: Can't modify self here since this is &self
            }
        }

        let response_text = response.text().await.map_err(|e| {
            McpError::Transport(TransportError::NetworkError {
                transport_type: "http-stream".to_string(),
                reason: format!("Failed to read response body: {}", e),
            })
        })?;

        debug!("Received MCP response: {}", response_text);

        // Parse response - handle both JSON and simple SSE formats
        self.parse_response(&response_text)
    }

    /// Parse response text that may be JSON or SSE format
    fn parse_response(&self, response_text: &str) -> McpResult<JsonRpcResponse> {
        // Try JSON first
        if let Ok(json_response) = serde_json::from_str::<serde_json::Value>(response_text) {
            return self.parse_json_response(&json_response);
        }

        // If not JSON, try SSE format
        if response_text.contains("data: ") {
            return self.parse_sse_response(response_text);
        }

        Err(McpError::Transport(TransportError::SerializationError {
            transport_type: "http-stream".to_string(),
            reason: format!("Could not parse response as JSON or SSE: {}", response_text),
        }))
    }

    /// Parse a JSON response value into JsonRpcResponse
    fn parse_json_response(&self, json_response: &serde_json::Value) -> McpResult<JsonRpcResponse> {
        if let Some(result) = json_response.get("result") {
            Ok(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(result.clone()),
                error: None,
                id: self.extract_request_id(json_response),
            })
        } else if let Some(error) = json_response.get("error") {
            Err(McpError::Transport(TransportError::HttpError {
                status_code: 400,
                reason: format!("Server returned error: {}", error),
            }))
        } else {
            Err(McpError::Transport(TransportError::SerializationError {
                transport_type: "http-stream".to_string(),
                reason: "Invalid JSON-RPC response format".to_string(),
            }))
        }
    }

    /// Parse SSE response and extract JSON-RPC from data lines
    fn parse_sse_response(&self, response_text: &str) -> McpResult<JsonRpcResponse> {
        // Look for data lines in SSE format
        for line in response_text.lines() {
            if let Some(json_text) = line.strip_prefix("data: ") {
                if let Ok(json_response) = serde_json::from_str::<serde_json::Value>(json_text) {
                    if json_response.get("id").is_some() {
                        // Found a JSON-RPC response
                        return self.parse_json_response(&json_response);
                    }
                }
            }
        }

        Err(McpError::Transport(TransportError::SerializationError {
            transport_type: "http-stream".to_string(),
            reason: "No valid JSON-RPC response found in SSE data".to_string(),
        }))
    }

    /// Extract RequestId from JSON response
    fn extract_request_id(&self, json_response: &serde_json::Value) -> RequestId {
        json_response
            .get("id")
            .and_then(|id| match id {
                serde_json::Value::String(s) => Some(RequestId::String(s.clone())),
                serde_json::Value::Number(n) => n.as_i64().map(RequestId::Number),
                serde_json::Value::Null => Some(RequestId::Null),
                _ => None,
            })
            .unwrap_or(RequestId::Null)
    }

    /// Send initialization request and extract session ID
    async fn send_initialize_request(
        &mut self,
        request: JsonRpcRequest,
    ) -> McpResult<JsonRpcResponse> {
        let url = self.get_mcp_url();
        let json_body = serde_json::to_string(&JsonRpcMessage::Request(request)).map_err(|e| {
            McpError::Transport(TransportError::SerializationError {
                transport_type: "http-stream".to_string(),
                reason: format!("Failed to serialize init request: {}", e),
            })
        })?;

        debug!("Sending initialization request to {}: {}", url, json_body);

        let mut request_builder = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .body(json_body);

        if let Some(auth) = &self.auth_header {
            request_builder = request_builder.header("Authorization", auth);
        }

        let response = request_builder.send().await.map_err(|e| {
            McpError::Transport(TransportError::NetworkError {
                transport_type: "http-stream".to_string(),
                reason: format!("Initialization request failed: {}", e),
            })
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(McpError::Transport(TransportError::HttpError {
                status_code: status.as_u16(),
                reason: format!("Initialization failed: {}", body),
            }));
        }

        // Extract session ID from headers (CRITICAL for Modern Streamable HTTP)
        if let Some(session_id) = response.headers().get("mcp-session-id") {
            if let Ok(session_str) = session_id.to_str() {
                info!("Session established with ID: {}", session_str);
                self.session_id = Some(session_str.to_string());
            }
        }

        let response_text = response.text().await.map_err(|e| {
            McpError::Transport(TransportError::NetworkError {
                transport_type: "http-stream".to_string(),
                reason: format!("Failed to read init response: {}", e),
            })
        })?;

        debug!("Initialization response: {}", response_text);

        // Parse the response
        self.parse_response(&response_text)
    }
}

#[async_trait]
impl Transport for HttpStreamTransport {
    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn connect(&mut self) -> McpResult<()> {
        info!(
            "Connecting MCP Streamable HTTP transport to {}",
            self.base_url
        );

        // Just mark as connected - initialization happens in first request
        self.connected = true;
        self.info.mark_connected();

        info!("MCP Streamable HTTP transport connected successfully");
        Ok(())
    }

    async fn send_request(
        &mut self,
        request: JsonRpcRequest,
        timeout_duration: Option<Duration>,
    ) -> McpResult<JsonRpcResponse> {
        if !self.is_connected() {
            return Err(McpError::Transport(TransportError::NotConnected {
                transport_type: "http-stream".to_string(),
                reason: "Transport not connected".to_string(),
            }));
        }

        let timeout_duration = timeout_duration.unwrap_or(Duration::from_secs(30));
        let is_initialize = request.method == "initialize";

        let result = timeout(timeout_duration, async {
            if is_initialize {
                // Special handling for initialization to extract session ID
                self.send_initialize_request(request).await
            } else {
                // Regular request using existing session ID
                self.send_mcp_request(&JsonRpcMessage::Request(request))
                    .await
            }
        })
        .await;

        match result {
            Ok(response) => {
                self.info.increment_requests_sent();
                self.info.increment_responses_received();
                response
            }
            Err(_) => Err(McpError::Transport(TransportError::TimeoutError {
                transport_type: "http-stream".to_string(),
                reason: format!("Request timed out after {timeout_duration:?}"),
            })),
        }
    }

    async fn send_notification(&mut self, notification: JsonRpcNotification) -> McpResult<()> {
        if !self.is_connected() {
            return Err(McpError::Transport(TransportError::NotConnected {
                transport_type: "http-stream".to_string(),
                reason: "Transport not connected".to_string(),
            }));
        }

        // Send notification (no response expected)
        let url = self.get_mcp_url();
        let json_body = serde_json::to_string(&JsonRpcMessage::Notification(notification))
            .map_err(|e| {
                McpError::Transport(TransportError::SerializationError {
                    transport_type: "http-stream".to_string(),
                    reason: format!("Failed to serialize notification: {e}"),
                })
            })?;

        let mut request_builder = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .body(json_body);

        if let Some(auth) = &self.auth_header {
            request_builder = request_builder.header("Authorization", auth);
        }

        if let Some(session_id) = &self.session_id {
            request_builder = request_builder.header("mcp-session-id", session_id);
        }

        let response = request_builder.send().await.map_err(|e| {
            McpError::Transport(TransportError::NetworkError {
                transport_type: "http-stream".to_string(),
                reason: format!("Notification request failed: {e}"),
            })
        })?;

        if !response.status().is_success() {
            return Err(McpError::Transport(TransportError::HttpError {
                status_code: response.status().as_u16(),
                reason: "Notification failed".to_string(),
            }));
        }

        self.info.increment_notifications_sent();
        Ok(())
    }

    async fn receive_message(
        &mut self,
        _timeout_duration: Option<Duration>,
    ) -> McpResult<JsonRpcMessage> {
        // For Modern Streamable HTTP, unsolicited messages would come via SSE
        // This is not implemented yet - would require persistent SSE connection
        Err(McpError::Transport(TransportError::InvalidConfig {
            transport_type: "http-stream".to_string(),
            reason: "Unsolicited message reception not implemented for Modern Streamable HTTP"
                .to_string(),
        }))
    }

    async fn disconnect(&mut self) -> McpResult<()> {
        info!("Disconnecting MCP Streamable HTTP transport");

        self.session_id = None;
        self.connected = false;

        // Clear pending requests
        {
            let mut pending = self.pending_requests.lock().await;
            pending.clear();
        }

        self.info.mark_disconnected();

        info!("MCP Streamable HTTP transport disconnected");
        Ok(())
    }

    fn get_info(&self) -> TransportInfo {
        let mut info = self.info.clone();

        // Add MCP-specific metadata
        info.add_metadata("base_url", serde_json::json!(self.base_url));
        info.add_metadata("mcp_endpoint", serde_json::json!(self.get_mcp_url()));
        info.add_metadata("has_auth", serde_json::json!(self.auth_header.is_some()));
        info.add_metadata("has_session", serde_json::json!(self.session_id.is_some()));
        info.add_metadata(
            "protocol",
            serde_json::json!("Modern Streamable HTTP (2025-03-26)"),
        );

        if let Some(session_id) = &self.session_id {
            info.add_metadata("session_id", serde_json::json!(session_id));
        }

        // Add pending requests count
        if let Ok(pending) = self.pending_requests.try_lock() {
            info.add_metadata("pending_requests", serde_json::json!(pending.len()));
        }

        info
    }

    fn get_config(&self) -> &TransportConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_stream_transport_creation() {
        let transport = HttpStreamTransport::new("http://localhost:3001".to_string(), None);
        assert_eq!(transport.base_url, "http://localhost:3001");
        assert!(!transport.is_connected());
        assert_eq!(transport.get_mcp_url(), "http://localhost:3001/mcp");
    }

    #[test]
    fn test_mcp_url_formatting() {
        let transport1 = HttpStreamTransport::new("http://localhost:3001".to_string(), None);
        assert_eq!(transport1.get_mcp_url(), "http://localhost:3001/mcp");

        let transport2 = HttpStreamTransport::new("http://localhost:3001/".to_string(), None);
        assert_eq!(transport2.get_mcp_url(), "http://localhost:3001/mcp");
    }

    #[test]
    fn test_transport_info_metadata() {
        let transport = HttpStreamTransport::new(
            "http://localhost:3001".to_string(),
            Some("Bearer token".to_string()),
        );
        let info = transport.get_info();

        assert_eq!(info.transport_type, "http-stream");
        assert!(info.metadata.contains_key("has_auth"));
        assert!(info.metadata.contains_key("protocol"));
    }

    #[test]
    fn test_auth_header_handling() {
        let transport_with_auth = HttpStreamTransport::new(
            "http://localhost:3001".to_string(),
            Some("Bearer token123".to_string()),
        );
        assert!(transport_with_auth.auth_header.is_some());

        let transport_no_auth = HttpStreamTransport::new("http://localhost:3001".to_string(), None);
        assert!(transport_no_auth.auth_header.is_none());
    }
}
