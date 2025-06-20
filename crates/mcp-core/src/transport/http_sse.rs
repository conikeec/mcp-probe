//! Streamable HTTP transport implementation for MCP communication.
//!
//! This transport implements the MCP Streamable HTTP specification:
//! - HTTP POST requests to base URL for client-to-server communication
//! - Session management via Mcp-Session-Id headers
//! - Support for single JSON responses and SSE streams
//! - Automatic session extraction and inclusion

use std::time::Duration;

use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use reqwest::{Client, Response, Url};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use tokio::sync::mpsc;
use tokio::time::timeout;

use super::{Transport, TransportConfig, TransportInfo};
use crate::error::{McpResult, TransportError};
use crate::messages::{JsonRpcMessage, JsonRpcRequest, JsonRpcNotification, JsonRpcResponse};

/// Streamable HTTP transport for MCP communication.
///
/// This transport implements the official MCP Streamable HTTP specification:
/// - Every client-to-server message is sent as HTTP POST to the base URL
/// - Server assigns session ID via Mcp-Session-Id header during initialization  
/// - Client includes session ID in all subsequent requests
/// - Server responds with either single JSON or SSE stream based on Content-Type
/// - Supports resumable connections and message replay
pub struct HttpSseTransport {
    config: TransportConfig,
    http_client: Client,
    info: TransportInfo,
    session_id: Option<String>,
    base_url: Url,
    sse_receiver: Option<mpsc::UnboundedReceiver<JsonRpcMessage>>,
    _sse_task_handle: Option<tokio::task::JoinHandle<()>>,
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

        Ok(Self {
            config,
            http_client,
            info,
            session_id: None,
            base_url,
            sse_receiver: None,
            _sse_task_handle: None,
        })
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
                        HeaderValue::from_str(value)
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
            }.into())
        }
    }

    /// Send a request and handle both JSON and SSE responses according to MCP spec.
    async fn send_mcp_request(&mut self, message: JsonRpcMessage) -> McpResult<Option<JsonRpcResponse>> {
        let mut request_builder = self.http_client
            .post(self.base_url.clone())
            .header(CONTENT_TYPE, "application/json");

        // Include session ID if we have one
        if let Some(ref session_id) = self.session_id {
            request_builder = request_builder.header("Mcp-Session-Id", session_id);
        }

        // Send the request
        let response = request_builder
            .json(&message)
            .send()
            .await
            .map_err(|e| TransportError::NetworkError {
                transport_type: "streamable-http".to_string(),
                reason: format!("HTTP request failed: {}", e),
            })?;

        // Extract session ID from response header (for initialization)
        if let Some(session_header) = response.headers().get("mcp-session-id") {
            if let Ok(session_str) = session_header.to_str() {
                tracing::debug!("Extracted session ID: {}", session_str);
                self.session_id = Some(session_str.to_string());
            }
        }

        // Handle response based on Content-Type
        let content_type = response.headers()
            .get(CONTENT_TYPE)
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("application/json");

        match content_type {
            "application/json" => {
                // Single JSON response - standard case
                let json_response: JsonRpcResponse = response.json().await
                    .map_err(|e| TransportError::SerializationError {
                        transport_type: "streamable-http".to_string(),
                        reason: format!("Failed to parse JSON response: {}", e),
                    })?;
                Ok(Some(json_response))
            }
            "text/event-stream" => {
                // SSE stream response - for multiple messages
                self.handle_sse_response(response).await?;
                Ok(None) // SSE messages handled via receiver
            }
            _ => {
                Err(TransportError::NetworkError {
                    transport_type: "streamable-http".to_string(),
                    reason: format!("Unexpected content type: {}", content_type),
                }.into())
            }
        }
    }

    /// Handle SSE stream responses for server-to-client communication.
    async fn handle_sse_response(&mut self, response: Response) -> McpResult<()> {
        let event_stream = response.bytes_stream().eventsource();
        let (sender, receiver) = mpsc::unbounded_channel();
        self.sse_receiver = Some(receiver);

        // Spawn task to handle SSE events
        let task_handle = tokio::spawn(async move {
            let mut stream = event_stream;
            while let Some(event) = stream.next().await {
                match event {
                    Ok(event) => {
                        // Parse event data as JSON-RPC message
                        if let Ok(message) = serde_json::from_str::<JsonRpcMessage>(&event.data) {
                            if sender.send(message).is_err() {
                                tracing::debug!("SSE receiver dropped, stopping stream");
                                break;
                            }
                        } else {
                            tracing::warn!("Failed to parse SSE message: {}", event.data);
                        }
                    }
                    Err(e) => {
                        tracing::error!("SSE stream error: {}", e);
                        break;
                    }
                }
            }
            tracing::debug!("SSE stream ended");
        });

        self._sse_task_handle = Some(task_handle);
        Ok(())
    }

    /// Get current session ID for debugging.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }
}

#[async_trait]
impl Transport for HttpSseTransport {
    async fn connect(&mut self) -> McpResult<()> {
        tracing::info!("Connecting Streamable HTTP transport to: {}", self.base_url);

        // Test connectivity with a simple request
        let test_response = self.http_client
            .head(self.base_url.clone())
            .send()
            .await;

        match test_response {
            Ok(_) => {
                self.info.mark_connected();
                tracing::info!("Streamable HTTP transport connected successfully");
                Ok(())
            }
            Err(e) => {
                Err(TransportError::ConnectionError {
                    transport_type: "streamable-http".to_string(),
                    reason: format!("Failed to connect to server: {}", e),
                }.into())
            }
        }
    }

    async fn disconnect(&mut self) -> McpResult<()> {
        tracing::info!("Disconnecting Streamable HTTP transport");

        // Terminate session if we have one
        if let Some(ref session_id) = self.session_id {
            let _ = self.http_client
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
            }.into());
        }

        let timeout_duration = timeout_duration.unwrap_or(Duration::from_secs(30));
        
        // Send request with timeout
        let response = timeout(
            timeout_duration,
            self.send_mcp_request(JsonRpcMessage::Request(request))
        ).await
        .map_err(|_| TransportError::TimeoutError {
            transport_type: "streamable-http".to_string(),
            reason: format!("Request timed out after {:?}", timeout_duration),
        })??;

        self.info.increment_requests_sent();

        match response {
            Some(json_response) => {
                self.info.increment_responses_received();
                Ok(json_response)
            }
            None => {
                // Response will come via SSE stream
                Err(TransportError::NetworkError {
                    transport_type: "streamable-http".to_string(),
                    reason: "Response expected via SSE stream - use receive_message()".to_string(),
                }.into())
            }
        }
    }

    async fn send_notification(&mut self, notification: JsonRpcNotification) -> McpResult<()> {
        if !self.is_connected() {
            return Err(TransportError::NotConnected {
                transport_type: "streamable-http".to_string(),
                reason: "Transport not connected".to_string(),
            }.into());
        }

        // Notifications don't expect responses
        self.send_mcp_request(JsonRpcMessage::Notification(notification)).await?;
        self.info.increment_notifications_sent();
        Ok(())
    }

    async fn receive_message(&mut self, timeout_duration: Option<Duration>) -> McpResult<JsonRpcMessage> {
        if !self.is_connected() {
            return Err(TransportError::NotConnected {
                transport_type: "streamable-http".to_string(),
                reason: "Transport not connected".to_string(),
            }.into());
        }

        let receiver = self.sse_receiver.as_mut().ok_or_else(|| {
            TransportError::NotConnected {
                transport_type: "streamable-http".to_string(),
                reason: "No SSE stream available - server uses single JSON responses".to_string(),
            }
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
            receiver.recv().await.ok_or_else(|| TransportError::DisconnectedError {
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
        info.add_metadata("has_sse_stream", serde_json::json!(self.sse_receiver.is_some()));
        
        if let TransportConfig::HttpSse(config) = &self.config {
            info.add_metadata("timeout", serde_json::json!(config.timeout.as_secs()));
            info.add_metadata("headers", serde_json::json!(config.headers));
            info.add_metadata("has_auth", serde_json::json!(config.auth.is_some()));
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
    }
} 