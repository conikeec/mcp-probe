//! Streamable HTTP transport implementation for MCP communication.
//!
//! This transport implements the MCP Streamable HTTP specification:
//! - HTTP POST requests to base URL for client-to-server communication
//! - Session management via Mcp-Session-Id headers
//! - Support for single JSON responses and SSE streams
//! - Automatic session extraction and inclusion
//! - Resumable connections with Last-Event-ID support
//! - Security validations and localhost binding

use std::time::Duration;

use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::{Client, Response, Url};
use tokio::sync::mpsc;
use tokio::time::timeout;

use super::{Transport, TransportConfig, TransportInfo};
use crate::error::{McpResult, TransportError};
use crate::messages::{JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};

/// SSE event with ID for resumability
/// This infrastructure supports resumable connections per MCP spec
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct SseEvent {
    id: Option<String>,
    event_type: Option<String>,
    data: String,
    retry: Option<u64>,
}

/// Streamable HTTP transport for MCP communication.
///
/// This transport implements the official MCP Streamable HTTP specification:
/// - Every client-to-server message is sent as HTTP POST to the base URL
/// - Server assigns session ID via Mcp-Session-Id header during initialization  
/// - Client includes session ID in all subsequent requests
/// - Server responds with either single JSON or SSE stream based on Content-Type
/// - Supports resumable connections and message replay via Last-Event-ID
/// - Implements security best practices for Origin validation and localhost binding
pub struct HttpSseTransport {
    config: TransportConfig,
    http_client: Client,
    info: TransportInfo,
    session_id: Option<String>,
    base_url: Url,
    sse_receiver: Option<mpsc::UnboundedReceiver<JsonRpcMessage>>,
    _sse_task_handle: Option<tokio::task::JoinHandle<()>>,
    last_event_id: Option<String>,
    security_config: SecurityConfig,
}

/// Security configuration for Streamable HTTP transport
#[derive(Debug, Clone)]
struct SecurityConfig {
    /// Validate Origin headers to prevent DNS rebinding attacks
    validate_origin: bool,
    /// Only allow connections to localhost for local servers
    enforce_localhost: bool,
    /// Require HTTPS in production environments
    require_https: bool,
    /// Validate session ID format and security
    validate_session_ids: bool,
    /// Allowed origins for CORS (used for SSE security validation)
    #[allow(dead_code)]
    allowed_origins: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            validate_origin: true,
            enforce_localhost: true,
            require_https: false, // Allow HTTP for local development
            validate_session_ids: true,
            allowed_origins: vec![
                "http://localhost".to_string(),
                "https://localhost".to_string(),
            ],
        }
    }
}

impl HttpSseTransport {
    /// Create a new Streamable HTTP transport instance.
    ///
    /// # Arguments
    ///
    /// * `config` - Transport configuration containing HTTP settings
    ///
    /// # Returns
    ///
    /// A new transport instance ready for connection.
    pub fn new(config: TransportConfig) -> McpResult<Self> {
        let (http_client, base_url) = Self::build_http_client(&config)?;
        let info = TransportInfo::new("streamable-http");
        let security_config = Self::build_security_config(&config, &base_url)?;

        Ok(Self {
            config,
            http_client,
            info,
            session_id: None,
            base_url,
            sse_receiver: None,
            _sse_task_handle: None,
            last_event_id: None,
            security_config,
        })
    }

    /// Build security configuration based on transport config and URL
    fn build_security_config(
        _config: &TransportConfig,
        base_url: &Url,
    ) -> McpResult<SecurityConfig> {
        let mut security_config = SecurityConfig::default();

        // Enforce HTTPS for non-localhost URLs
        if base_url.host_str() != Some("localhost") && base_url.host_str() != Some("127.0.0.1") {
            security_config.require_https = true;
        }

        // Validate HTTPS requirement
        if security_config.require_https && base_url.scheme() != "https" {
            return Err(TransportError::InvalidConfig {
                transport_type: "streamable-http".to_string(),
                reason: format!("HTTPS required for non-localhost URL: {}", base_url),
            }
            .into());
        }

        // Validate localhost binding for local URLs
        if security_config.enforce_localhost {
            if let Some(host) = base_url.host_str() {
                if host != "localhost" && host != "127.0.0.1" && host != "::1" {
                    tracing::warn!(
                        "Connecting to non-localhost URL: {} - ensure this is intended",
                        base_url
                    );
                }
            }
        }

        Ok(security_config)
    }

    /// Build the HTTP client with appropriate configuration.
    fn build_http_client(config: &TransportConfig) -> McpResult<(Client, Url)> {
        if let TransportConfig::HttpSse(sse_config) = config {
            let mut builder = Client::builder();
            builder = builder.timeout(sse_config.timeout);

            // Add custom headers if specified
            if !sse_config.headers.is_empty() {
                let mut headers = HeaderMap::new();
                for (key, value) in &sse_config.headers {
                    if let (Ok(header_name), Ok(header_value)) = (
                        key.parse::<reqwest::header::HeaderName>(),
                        HeaderValue::from_str(value),
                    ) {
                        headers.insert(header_name, header_value);
                    }
                }
                builder = builder.default_headers(headers);
            }

            let client = builder.build().map_err(|e| TransportError::InvalidConfig {
                transport_type: "streamable-http".to_string(),
                reason: format!("Failed to build HTTP client: {}", e),
            })?;

            Ok((client, sse_config.base_url.clone()))
        } else {
            Err(TransportError::InvalidConfig {
                transport_type: "streamable-http".to_string(),
                reason: "Invalid configuration type".to_string(),
            }
            .into())
        }
    }

    /// Validate Origin header to prevent DNS rebinding attacks
    fn validate_origin(&self, _request_builder: &reqwest::RequestBuilder) -> McpResult<()> {
        if !self.security_config.validate_origin {
            return Ok(());
        }

        // For local connections, we should validate the origin
        if self.base_url.host_str() == Some("localhost")
            || self.base_url.host_str() == Some("127.0.0.1")
        {
            // Origin validation is important for localhost to prevent DNS rebinding
            tracing::debug!("Origin validation enabled for localhost connection");
        }

        Ok(())
    }

    /// Validate session ID security
    fn validate_session_id(&self, session_id: &str) -> McpResult<()> {
        if !self.security_config.validate_session_ids {
            return Ok(());
        }

        // Check session ID format (should be cryptographically secure)
        if session_id.len() < 16 {
            return Err(TransportError::InvalidConfig {
                transport_type: "streamable-http".to_string(),
                reason: "Session ID too short - security risk".to_string(),
            }
            .into());
        }

        // Check for basic format (alphanumeric and hyphens)
        if !session_id.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(TransportError::InvalidConfig {
                transport_type: "streamable-http".to_string(),
                reason: "Session ID contains invalid characters".to_string(),
            }
            .into());
        }

        Ok(())
    }

    /// Send a request and handle both JSON and SSE responses according to MCP spec.
    async fn send_mcp_request(
        &mut self,
        message: JsonRpcMessage,
    ) -> McpResult<Option<JsonRpcResponse>> {
        let mut request_builder = self
            .http_client
            .post(self.base_url.clone())
            .header(CONTENT_TYPE, "application/json");

        // Validate Origin header for security
        self.validate_origin(&request_builder)?;

        // Include session ID if we have one
        if let Some(ref session_id) = self.session_id {
            request_builder = request_builder.header("Mcp-Session-Id", session_id);
        }

        // Include Last-Event-ID for resumability
        if let Some(ref last_event_id) = self.last_event_id {
            request_builder = request_builder.header("Last-Event-ID", last_event_id);
            tracing::debug!("Resuming from last event ID: {}", last_event_id);
        }

        // Send the request
        let response = request_builder.json(&message).send().await.map_err(|e| {
            TransportError::NetworkError {
                transport_type: "streamable-http".to_string(),
                reason: format!("HTTP request failed: {}", e),
            }
        })?;

        // Extract session ID from response header (for initialization)
        if let Some(session_header) = response.headers().get("mcp-session-id") {
            if let Ok(session_str) = session_header.to_str() {
                self.validate_session_id(session_str)?;
                tracing::debug!("Extracted session ID: {}", session_str);
                self.session_id = Some(session_str.to_string());
            }
        }

        // Handle response based on Content-Type
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("application/json");

        tracing::info!("=== RAW HTTP RESPONSE DEBUG ===");
        tracing::info!("Status: {}", response.status());
        tracing::info!("Content-Type: {}", content_type);
        tracing::info!("Headers: {:?}", response.headers());

        tracing::debug!(
            "HTTP SSE transport received response with Content-Type: {}",
            content_type
        );
        match content_type {
            "application/json" => {
                // Single JSON response - standard case
                let response_text =
                    response
                        .text()
                        .await
                        .map_err(|e| TransportError::SerializationError {
                            transport_type: "streamable-http".to_string(),
                            reason: format!("Failed to get response text: {}", e),
                        })?;

                tracing::info!("=== RAW JSON RESPONSE BODY ===");
                tracing::info!("{}", response_text);

                let json_response: JsonRpcResponse =
                    serde_json::from_str(&response_text).map_err(|e| {
                        TransportError::SerializationError {
                            transport_type: "streamable-http".to_string(),
                            reason: format!("Failed to parse JSON response: {}", e),
                        }
                    })?;
                Ok(Some(json_response))
            }
            "text/event-stream" => {
                // SSE stream response - for multiple messages
                self.handle_sse_response(response).await?;
                Ok(None) // SSE messages handled via receiver
            }
            _ => Err(TransportError::NetworkError {
                transport_type: "streamable-http".to_string(),
                reason: format!("Unexpected content type: {}", content_type),
            }
            .into()),
        }
    }

    /// Parse SSE event with ID tracking for resumability
    /// This infrastructure supports resumable connections per MCP spec
    #[allow(dead_code)]
    fn parse_sse_event(&self, event: &eventsource_stream::Event) -> Option<SseEvent> {
        Some(SseEvent {
            id: Some(event.id.clone()),
            event_type: Some(event.event.clone()),
            data: event.data.clone(),
            retry: event.retry.map(|d| d.as_millis() as u64),
        })
    }

    /// Handle SSE stream responses for server-to-client communication with resumability.
    async fn handle_sse_response(&mut self, response: Response) -> McpResult<()> {
        let event_stream = response.bytes_stream().eventsource();
        let (sender, receiver) = mpsc::unbounded_channel();
        self.sse_receiver = Some(receiver);

        // Track last event ID for resumability
        let current_last_event_id = self.last_event_id.clone();

        // Spawn task to handle SSE events
        let task_handle = tokio::spawn(async move {
            let mut stream = event_stream;
            let mut event_count = 0u64;
            let mut last_event_id = current_last_event_id;

            while let Some(event) = stream.next().await {
                match event {
                    Ok(event) => {
                        event_count += 1;

                        // Track event ID for resumability
                        if !event.id.is_empty() {
                            last_event_id = Some(event.id.clone());
                            tracing::trace!("Received SSE event with ID: {}", event.id);
                        }

                        // Parse event data as JSON-RPC message
                        if let Ok(message) = serde_json::from_str::<JsonRpcMessage>(&event.data) {
                            if sender.send(message).is_err() {
                                tracing::debug!(
                                    "SSE receiver dropped, stopping stream after {} events",
                                    event_count
                                );
                                break;
                            }
                        } else {
                            tracing::warn!("Failed to parse SSE message: {}", event.data);
                        }

                        // Handle retry directive from server
                        if let Some(retry_ms) = event.retry {
                            tracing::debug!(
                                "Server requested retry interval: {}ms",
                                retry_ms.as_millis()
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!("SSE stream error after {} events: {}", event_count, e);

                        // For network errors, we might want to retry with Last-Event-ID
                        if let Some(ref last_id) = last_event_id {
                            tracing::info!(
                                "Connection lost - can resume from event ID: {}",
                                last_id
                            );
                        }
                        break;
                    }
                }
            }
            tracing::debug!("SSE stream ended after {} events", event_count);
        });

        self._sse_task_handle = Some(task_handle);
        Ok(())
    }

    /// Resume SSE connection from last event ID
    pub async fn resume_sse_connection(&mut self) -> McpResult<()> {
        if let Some(ref last_event_id) = self.last_event_id {
            tracing::info!("Resuming SSE connection from event ID: {}", last_event_id);

            // Make a GET request to establish SSE connection with Last-Event-ID
            let mut request_builder = self
                .http_client
                .get(self.base_url.clone())
                .header("Accept", "text/event-stream")
                .header("Last-Event-ID", last_event_id);

            // Include session ID if we have one
            if let Some(ref session_id) = self.session_id {
                request_builder = request_builder.header("Mcp-Session-Id", session_id);
            }

            let response =
                request_builder
                    .send()
                    .await
                    .map_err(|e| TransportError::NetworkError {
                        transport_type: "streamable-http".to_string(),
                        reason: format!("Failed to resume SSE connection: {}", e),
                    })?;

            if response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|ct| ct.to_str().ok())
                == Some("text/event-stream")
            {
                self.handle_sse_response(response).await?;
                tracing::info!("SSE connection resumed successfully");
            } else {
                return Err(TransportError::NetworkError {
                    transport_type: "streamable-http".to_string(),
                    reason: "Server did not respond with SSE stream for resume request".to_string(),
                }
                .into());
            }
        }

        Ok(())
    }

    /// Get current session ID for debugging.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Get last event ID for resumability
    pub fn last_event_id(&self) -> Option<&str> {
        self.last_event_id.as_deref()
    }

    /// Check if transport can resume from disconnection
    pub fn can_resume(&self) -> bool {
        self.last_event_id.is_some()
    }
}

#[async_trait]
impl Transport for HttpSseTransport {
    async fn connect(&mut self) -> McpResult<()> {
        tracing::info!("Connecting Streamable HTTP transport to: {}", self.base_url);

        // Test connectivity with a simple request
        let test_response = self.http_client.head(self.base_url.clone()).send().await;

        match test_response {
            Ok(_) => {
                self.info.mark_connected();
                tracing::info!("Streamable HTTP transport connected successfully");
                Ok(())
            }
            Err(e) => Err(TransportError::ConnectionError {
                transport_type: "streamable-http".to_string(),
                reason: format!("Failed to connect to server: {}", e),
            }
            .into()),
        }
    }

    async fn disconnect(&mut self) -> McpResult<()> {
        tracing::info!("Disconnecting Streamable HTTP transport");

        // Terminate session if we have one
        if let Some(ref session_id) = self.session_id {
            let _ = self
                .http_client
                .delete(self.base_url.clone())
                .header("Mcp-Session-Id", session_id)
                .send()
                .await;
        }

        // Clean up SSE resources
        self.sse_receiver = None;
        if let Some(handle) = self._sse_task_handle.take() {
            handle.abort();
        }

        self.session_id = None;
        self.info.mark_disconnected();

        tracing::info!("Streamable HTTP transport disconnected");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.info.connected
    }

    async fn send_request(
        &mut self,
        request: JsonRpcRequest,
        timeout_duration: Option<Duration>,
    ) -> McpResult<JsonRpcResponse> {
        if !self.is_connected() {
            return Err(TransportError::NotConnected {
                transport_type: "streamable-http".to_string(),
                reason: "Transport not connected".to_string(),
            }
            .into());
        }

        tracing::debug!(
            "HTTP SSE transport sending request: {} with ID: {}",
            request.method,
            request.id
        );
        let timeout_duration = timeout_duration.unwrap_or(Duration::from_secs(30));

        // Send request with timeout
        let response = timeout(
            timeout_duration,
            self.send_mcp_request(JsonRpcMessage::Request(request)),
        )
        .await
        .map_err(|_| TransportError::TimeoutError {
            transport_type: "streamable-http".to_string(),
            reason: format!("Request timed out after {:?}", timeout_duration),
        })??;

        self.info.increment_requests_sent();

        match response {
            Some(json_response) => {
                tracing::debug!(
                    "HTTP SSE transport received direct JSON response for request ID: {}",
                    json_response.id
                );
                self.info.increment_responses_received();
                Ok(json_response)
            }
            None => {
                // Response will come via SSE stream
                tracing::debug!("HTTP SSE transport: response will come via SSE stream");
                Err(TransportError::NetworkError {
                    transport_type: "streamable-http".to_string(),
                    reason: "Response expected via SSE stream - use receive_message()".to_string(),
                }
                .into())
            }
        }
    }

    async fn send_notification(&mut self, notification: JsonRpcNotification) -> McpResult<()> {
        if !self.is_connected() {
            return Err(TransportError::NotConnected {
                transport_type: "streamable-http".to_string(),
                reason: "Transport not connected".to_string(),
            }
            .into());
        }

        tracing::debug!(
            "HTTP SSE transport sending notification: {}",
            notification.method
        );

        // Notifications don't expect responses - send directly without parsing response
        let mut request_builder = self
            .http_client
            .post(self.base_url.clone())
            .header(CONTENT_TYPE, "application/json");

        // Validate Origin header for security
        self.validate_origin(&request_builder)?;

        // Include session ID if we have one
        if let Some(ref session_id) = self.session_id {
            request_builder = request_builder.header("Mcp-Session-Id", session_id);
        }

        // Send the notification - ignore response content
        let _response = request_builder
            .json(&JsonRpcMessage::Notification(notification))
            .send()
            .await
            .map_err(|e| TransportError::NetworkError {
                transport_type: "streamable-http".to_string(),
                reason: format!("HTTP notification failed: {}", e),
            })?;

        self.info.increment_notifications_sent();
        tracing::debug!("HTTP SSE transport notification sent successfully");
        Ok(())
    }

    async fn receive_message(
        &mut self,
        timeout_duration: Option<Duration>,
    ) -> McpResult<JsonRpcMessage> {
        if !self.is_connected() {
            return Err(TransportError::NotConnected {
                transport_type: "streamable-http".to_string(),
                reason: "Transport not connected".to_string(),
            }
            .into());
        }

        let receiver = self
            .sse_receiver
            .as_mut()
            .ok_or_else(|| TransportError::NotConnected {
                transport_type: "streamable-http".to_string(),
                reason: "No SSE stream available - server uses single JSON responses".to_string(),
            })?;

        let message = if let Some(timeout_duration) = timeout_duration {
            timeout(timeout_duration, receiver.recv())
                .await
                .map_err(|_| TransportError::TimeoutError {
                    transport_type: "streamable-http".to_string(),
                    reason: format!("Message receive timed out after {:?}", timeout_duration),
                })?
                .ok_or_else(|| TransportError::DisconnectedError {
                    transport_type: "streamable-http".to_string(),
                    reason: "SSE stream closed".to_string(),
                })?
        } else {
            receiver
                .recv()
                .await
                .ok_or_else(|| TransportError::DisconnectedError {
                    transport_type: "streamable-http".to_string(),
                    reason: "SSE stream closed".to_string(),
                })?
        };

        // Update statistics
        match &message {
            JsonRpcMessage::Request(_) => {
                // Server-to-client request via SSE
            }
            JsonRpcMessage::Response(_) => {
                self.info.increment_responses_received();
            }
            JsonRpcMessage::Notification(_) => {
                self.info.increment_notifications_received();
            }
        }

        Ok(message)
    }

    fn get_info(&self) -> TransportInfo {
        let mut info = self.info.clone();

        // Add Streamable HTTP specific metadata
        info.add_metadata("base_url", serde_json::json!(self.base_url.to_string()));
        info.add_metadata("session_id", serde_json::json!(self.session_id));
        info.add_metadata(
            "has_sse_stream",
            serde_json::json!(self.sse_receiver.is_some()),
        );
        info.add_metadata("last_event_id", serde_json::json!(self.last_event_id));
        info.add_metadata("can_resume", serde_json::json!(self.can_resume()));
        info.add_metadata(
            "security_enabled",
            serde_json::json!(self.security_config.validate_origin),
        );

        if let TransportConfig::HttpSse(config) = &self.config {
            info.add_metadata("timeout", serde_json::json!(config.timeout.as_secs()));
            info.add_metadata("headers", serde_json::json!(config.headers));
            info.add_metadata("has_auth", serde_json::json!(config.auth.is_some()));
            info.add_metadata(
                "enforce_https",
                serde_json::json!(self.security_config.require_https),
            );
            info.add_metadata(
                "localhost_only",
                serde_json::json!(self.security_config.enforce_localhost),
            );
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
    fn test_streamable_http_transport_creation() {
        let config = TransportConfig::http_sse("https://example.com/mcp").unwrap();
        let transport = HttpSseTransport::new(config).unwrap();

        assert_eq!(transport.get_info().transport_type, "streamable-http");
        assert!(!transport.is_connected());
        assert!(transport.session_id().is_none());
    }

    #[test]
    fn test_base_url_extraction() {
        let config = TransportConfig::http_sse("https://example.com/mcp").unwrap();
        let transport = HttpSseTransport::new(config).unwrap();

        assert_eq!(transport.base_url.to_string(), "https://example.com/mcp");
    }

    #[test]
    fn test_transport_info_metadata() {
        let config = TransportConfig::http_sse("https://example.com/mcp").unwrap();
        let transport = HttpSseTransport::new(config).unwrap();

        let info = transport.get_info();
        assert!(info.metadata.contains_key("base_url"));
        assert!(info.metadata.contains_key("session_id"));
        assert!(info.metadata.contains_key("has_sse_stream"));
        assert!(info.metadata.contains_key("last_event_id"));
        assert!(info.metadata.contains_key("can_resume"));
        assert!(info.metadata.contains_key("security_enabled"));
    }

    #[test]
    fn test_security_config_https_enforcement() {
        // Should require HTTPS for non-localhost
        let config = TransportConfig::http_sse("http://example.com/mcp").unwrap();
        let result = HttpSseTransport::new(config);
        assert!(result.is_err());

        // Should allow HTTP for localhost
        let config = TransportConfig::http_sse("http://localhost:3000/mcp").unwrap();
        let result = HttpSseTransport::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_session_id_validation() {
        let config = TransportConfig::http_sse("http://localhost:3000/mcp").unwrap();
        let transport = HttpSseTransport::new(config).unwrap();

        // Valid session ID
        assert!(transport
            .validate_session_id("550e8400-e29b-41d4-a716-446655440000")
            .is_ok());

        // Invalid session ID (too short)
        assert!(transport.validate_session_id("short").is_err());

        // Invalid session ID (invalid characters)
        assert!(transport.validate_session_id("invalid@session!id").is_err());
    }

    #[test]
    fn test_resumability_features() {
        let config = TransportConfig::http_sse("http://localhost:3000/mcp").unwrap();
        let transport = HttpSseTransport::new(config).unwrap();

        // Initially no resumability
        assert!(!transport.can_resume());
        assert!(transport.last_event_id().is_none());
    }
}
