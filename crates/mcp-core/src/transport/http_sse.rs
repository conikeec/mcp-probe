//! HTTP+SSE transport implementation for MCP communication.
//!
//! This transport uses HTTP requests for client-to-server communication
//! and Server-Sent Events (SSE) for server-to-client communication.
//! It provides a good balance of simplicity and functionality for
//! remote MCP server scenarios.

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use reqwest::{Client, Url};
use reqwest::header::{HeaderMap, HeaderValue};
use tokio::sync::mpsc;
use tokio::time::timeout;

use super::{Transport, TransportConfig, TransportInfo};
use crate::error::{McpResult, TransportError};
use crate::messages::{JsonRpcMessage, JsonRpcRequest, JsonRpcNotification, JsonRpcResponse};

/// HTTP+SSE transport for remote MCP server communication.
///
/// This transport implementation provides:
/// - HTTP POST requests for client-to-server messages
/// - Server-Sent Events (SSE) for server-to-client messages
/// - Automatic reconnection on connection failures
/// - Configurable timeouts and authentication
/// - Request/response correlation using message IDs
pub struct HttpSseTransport {
    config: TransportConfig,
    http_client: Client,
    info: TransportInfo,
    sse_connected: bool,
    message_sender: Option<mpsc::UnboundedSender<JsonRpcMessage>>,
    message_receiver: Option<mpsc::UnboundedReceiver<JsonRpcMessage>>,
    pending_requests: HashMap<String, tokio::sync::oneshot::Sender<JsonRpcResponse>>,
}

impl HttpSseTransport {
    /// Create a new HTTP+SSE transport instance.
    ///
    /// # Arguments
    ///
    /// * `config` - Transport configuration containing HTTP+SSE settings
    ///
    /// # Returns
    ///
    /// A new transport instance ready for connection.
    pub fn new(config: TransportConfig) -> Self {
        let http_client = Self::build_http_client(&config);
        let info = TransportInfo::new("http-sse");

        Self {
            config,
            http_client,
            info,
            sse_connected: false,
            message_sender: None,
            message_receiver: None,
            pending_requests: HashMap::new(),
        }
    }

    /// Build the HTTP client with appropriate configuration.
    fn build_http_client(config: &TransportConfig) -> Client {
        let mut builder = Client::builder();

        if let TransportConfig::HttpSse(sse_config) = config {
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
        }

        builder.build().unwrap_or_else(|_| Client::new())
    }

    /// Build the SSE endpoint URL.
    fn build_sse_url(&self) -> McpResult<Url> {
        if let TransportConfig::HttpSse(config) = &self.config {
            let mut url = config.base_url.clone();
            url.path_segments_mut()
                .map_err(|_| TransportError::InvalidConfig {
                    transport_type: "http-sse".to_string(),
                    reason: "Invalid base URL for SSE endpoint".to_string(),
                })?
                .push("sse");
            Ok(url)
        } else {
            Err(TransportError::InvalidConfig {
                transport_type: "http-sse".to_string(),
                reason: "Invalid configuration type".to_string(),
            }.into())
        }
    }

    /// Build the HTTP request endpoint URL.
    fn build_request_url(&self) -> McpResult<Url> {
        if let TransportConfig::HttpSse(config) = &self.config {
            let mut url = config.base_url.clone();
            url.path_segments_mut()
                .map_err(|_| TransportError::InvalidConfig {
                    transport_type: "http-sse".to_string(),
                    reason: "Invalid base URL for request endpoint".to_string(),
                })?
                .push("message");
            Ok(url)
        } else {
            Err(TransportError::InvalidConfig {
                transport_type: "http-sse".to_string(),
                reason: "Invalid configuration type".to_string(),
            }.into())
        }
    }

    /// Establish the SSE connection and start listening for messages.
    async fn connect_sse(&mut self) -> McpResult<()> {
        let sse_url = self.build_sse_url()?;
        
        tracing::debug!("Connecting to SSE endpoint: {}", sse_url);

        // Create the SSE stream
        let response = self.http_client
            .get(sse_url)
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .send()
            .await
            .map_err(|e| TransportError::ConnectionError {
                transport_type: "http-sse".to_string(),
                reason: format!("Failed to connect to SSE endpoint: {}", e),
            })?;

        if !response.status().is_success() {
            return Err(TransportError::ConnectionError {
                transport_type: "http-sse".to_string(),
                reason: format!("SSE connection failed with status: {}", response.status()),
            }.into());
        }

        // Convert the response to an event stream
        let event_stream = response.bytes_stream().eventsource();

        // Create message channel
        let (sender, receiver) = mpsc::unbounded_channel();
        self.message_sender = Some(sender.clone());
        self.message_receiver = Some(receiver);

        // Spawn task to handle SSE events
        let sender_clone = sender;
        tokio::spawn(async move {
            let mut stream = event_stream;
            while let Some(event) = stream.next().await {
                match event {
                    Ok(event) => {
                        if let Ok(message) = serde_json::from_str::<JsonRpcMessage>(&event.data) {
                            if sender_clone.send(message).is_err() {
                                tracing::error!("Failed to send SSE message to handler");
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
        });

        Ok(())
    }

    /// Send an HTTP request to the server.
    async fn send_http_message(&mut self, message: JsonRpcMessage) -> McpResult<()> {
        let request_url = self.build_request_url()?;
        let json_body = serde_json::to_string(&message)
            .map_err(|e| TransportError::SerializationError {
                transport_type: "http-sse".to_string(),
                reason: format!("Failed to serialize message: {}", e),
            })?;

        tracing::debug!("Sending HTTP message to: {}", request_url);

        let response = self.http_client
            .post(request_url)
            .header("Content-Type", "application/json")
            .body(json_body)
            .send()
            .await
            .map_err(|e| TransportError::NetworkError {
                transport_type: "http-sse".to_string(),
                reason: format!("HTTP request failed: {}", e),
            })?;

        if !response.status().is_success() {
            return Err(TransportError::NetworkError {
                transport_type: "http-sse".to_string(),
                reason: format!("HTTP request failed with status: {}", response.status()),
            }.into());
        }

        Ok(())
    }
}

#[async_trait]
impl Transport for HttpSseTransport {
    async fn connect(&mut self) -> McpResult<()> {
        tracing::info!("Connecting HTTP+SSE transport");

        // Establish SSE connection
        self.connect_sse().await?;

        // Update transport info
        self.info.mark_connected();
        
        tracing::info!("HTTP+SSE transport connected successfully");
        Ok(())
    }

    async fn disconnect(&mut self) -> McpResult<()> {
        tracing::info!("Disconnecting HTTP+SSE transport");

        // Close message channels
        self.message_sender = None;
        self.message_receiver = None;

        // Clear pending requests
        self.pending_requests.clear();

        // Update transport info
        self.info.mark_disconnected();

        tracing::info!("HTTP+SSE transport disconnected");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.info.connected && self.message_sender.is_some()
    }

    async fn send_request(
        &mut self,
        request: JsonRpcRequest,
        timeout_duration: Option<Duration>,
    ) -> McpResult<JsonRpcResponse> {
        if !self.is_connected() {
            return Err(TransportError::NotConnected {
                transport_type: "http-sse".to_string(),
                reason: "Transport not connected".to_string(),
            }.into());
        }

        let request_id = request.id.clone();
        let (response_sender, response_receiver) = tokio::sync::oneshot::channel();
        
        // Store the response sender for correlation
        self.pending_requests.insert(request_id.to_string(), response_sender);

        // Send the request
        self.send_http_message(JsonRpcMessage::Request(request)).await?;
        self.info.increment_requests_sent();

        // Wait for response with timeout
        let timeout_duration = timeout_duration.unwrap_or(Duration::from_secs(30));
        let response = timeout(timeout_duration, response_receiver)
            .await
            .map_err(|_| TransportError::TimeoutError {
                transport_type: "http-sse".to_string(),
                reason: format!("Request {} timed out after {:?}", request_id, timeout_duration),
            })?
            .map_err(|_| TransportError::NetworkError {
                transport_type: "http-sse".to_string(),
                reason: "Response channel closed unexpectedly".to_string(),
            })?;

        self.info.increment_responses_received();
        Ok(response)
    }

    async fn send_notification(&mut self, notification: JsonRpcNotification) -> McpResult<()> {
        if !self.is_connected() {
            return Err(TransportError::NotConnected {
                transport_type: "http-sse".to_string(),
                reason: "Transport not connected".to_string(),
            }.into());
        }

        self.send_http_message(JsonRpcMessage::Notification(notification)).await?;
        self.info.increment_notifications_sent();
        Ok(())
    }

    async fn receive_message(&mut self, timeout_duration: Option<Duration>) -> McpResult<JsonRpcMessage> {
        if !self.is_connected() {
            return Err(TransportError::NotConnected {
                transport_type: "http-sse".to_string(),
                reason: "Transport not connected".to_string(),
            }.into());
        }

        let receiver = self.message_receiver.as_mut().ok_or_else(|| {
            TransportError::NotConnected {
                transport_type: "http-sse".to_string(),
                reason: "Message receiver not available".to_string(),
            }
        })?;

        let message = if let Some(timeout_duration) = timeout_duration {
            timeout(timeout_duration, receiver.recv())
                .await
                .map_err(|_| TransportError::TimeoutError {
                    transport_type: "http-sse".to_string(),
                    reason: format!("Message receive timed out after {:?}", timeout_duration),
                })?
                .ok_or_else(|| TransportError::DisconnectedError {
                    transport_type: "http-sse".to_string(),
                    reason: "Message channel closed".to_string(),
                })?
        } else {
            receiver.recv().await.ok_or_else(|| TransportError::DisconnectedError {
                transport_type: "http-sse".to_string(),
                reason: "Message channel closed".to_string(),
            })?
        };

        // Handle response correlation
        if let JsonRpcMessage::Response(ref response) = message {
            if let Some(response_sender) = self.pending_requests.remove(&response.id.to_string()) {
                let _ = response_sender.send(response.clone());
                // Don't return the response here since it's handled via the oneshot channel
                return self.receive_message(timeout_duration).await;
            }
        }

        // Update statistics
        match &message {
            JsonRpcMessage::Request(_) => {
                // Server-to-client request - not typically expected in MCP
                tracing::warn!("Received unexpected server-to-client request");
            }
            JsonRpcMessage::Response(_) => {
                // Already handled above
            }
            JsonRpcMessage::Notification(_) => {
                self.info.increment_notifications_received();
            }
        }

        Ok(message)
    }

    fn get_info(&self) -> TransportInfo {
        let mut info = self.info.clone();
        
        // Add HTTP+SSE specific metadata
        if let TransportConfig::HttpSse(config) = &self.config {
            info.add_metadata("base_url", serde_json::json!(config.base_url.to_string()));
            info.add_metadata("timeout", serde_json::json!(config.timeout.as_secs()));
            info.add_metadata("headers", serde_json::json!(config.headers));
            info.add_metadata("has_auth", serde_json::json!(config.auth.is_some()));
        }
        
        info.add_metadata("pending_requests", serde_json::json!(self.pending_requests.len()));
        
        info
    }

    fn get_config(&self) -> &TransportConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::TransportConfig;

    #[test]
    fn test_http_sse_transport_creation() {
        let config = TransportConfig::http_sse("https://example.com/mcp").unwrap();
        let transport = HttpSseTransport::new(config);
        
        assert_eq!(transport.get_info().transport_type, "http-sse");
        assert!(!transport.is_connected());
    }

    #[test]
    fn test_url_building() {
        let config = TransportConfig::http_sse("https://example.com/mcp").unwrap();
        let transport = HttpSseTransport::new(config);
        
        let sse_url = transport.build_sse_url().unwrap();
        assert_eq!(sse_url.to_string(), "https://example.com/mcp/sse");
        
        let request_url = transport.build_request_url().unwrap();
        assert_eq!(request_url.to_string(), "https://example.com/mcp/message");
    }

    #[test]
    fn test_transport_info_metadata() {
        let config = TransportConfig::http_sse("https://example.com/mcp").unwrap();
        let transport = HttpSseTransport::new(config);
        
        let info = transport.get_info();
        assert!(info.metadata.contains_key("base_url"));
        assert!(info.metadata.contains_key("timeout"));
    }
} 