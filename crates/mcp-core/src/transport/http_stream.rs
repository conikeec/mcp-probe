//! HTTP streaming transport implementation for MCP communication.
//!
//! This transport uses HTTP streaming for bidirectional communication
//! with MCP servers over persistent HTTP connections.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::{Client, Response};
use tokio::sync::{mpsc, Mutex, oneshot};
use tokio::time::{timeout, sleep};
use tracing::{debug, error, info, warn};

use super::{Transport, TransportConfig, TransportInfo};
use crate::error::{McpError, McpResult, TransportError};
use crate::messages::{JsonRpcMessage, JsonRpcRequest, JsonRpcNotification, JsonRpcResponse};

/// A streaming connection wrapper that maintains thread safety
struct StreamingConnection {
    /// Background task handle for the connection
    task_handle: tokio::task::JoinHandle<()>,
    /// Sender for outbound messages
    outbound_sender: mpsc::UnboundedSender<JsonRpcMessage>,
    /// Receiver for inbound messages
    inbound_receiver: Arc<Mutex<mpsc::UnboundedReceiver<JsonRpcMessage>>>,
}

impl StreamingConnection {
    /// Create a new streaming connection
    async fn new(base_url: String, auth_header: Option<String>) -> McpResult<Self> {
        let client = Client::new();
        let (outbound_sender, mut outbound_receiver) = mpsc::unbounded_channel();
        let (inbound_sender, inbound_receiver) = mpsc::unbounded_channel();
        
        let auth_header_clone = auth_header.clone();
        let base_url_clone = base_url.clone();
        
        // Start the connection management task
        let task_handle = tokio::spawn(async move {
            let mut retry_count = 0;
            const MAX_RETRIES: u32 = 5;
            const RETRY_DELAY: Duration = Duration::from_secs(2);
            
            while retry_count < MAX_RETRIES {
                match Self::establish_stream(&client, &base_url_clone, &auth_header_clone).await {
                    Ok(response) => {
                        info!("HTTP stream connection established");
                        retry_count = 0; // Reset retry count on success
                        
                        // Handle the stream
                        if let Err(e) = Self::handle_stream(
                            response,
                            &mut outbound_receiver,
                            &inbound_sender,
                            &client,
                            &base_url_clone,
                            &auth_header_clone,
                        ).await {
                            error!("Stream handling error: {}", e);
                        }
                        
                        warn!("HTTP stream connection lost, attempting to reconnect...");
                    }
                    Err(e) => {
                        error!("Failed to establish HTTP stream: {}", e);
                        retry_count += 1;
                        
                        if retry_count < MAX_RETRIES {
                            sleep(RETRY_DELAY * retry_count).await;
                        }
                    }
                }
            }
            
            error!("HTTP stream connection failed after {} retries", MAX_RETRIES);
        });
        
        Ok(Self {
            task_handle,
            outbound_sender,
            inbound_receiver: Arc::new(Mutex::new(inbound_receiver)),
        })
    }
    
    /// Establish the HTTP streaming connection
    async fn establish_stream(
        client: &Client,
        base_url: &str,
        auth_header: &Option<String>,
    ) -> McpResult<Response> {
        let url = format!("{}/stream", base_url);
        let mut request_builder = client
            .get(&url)
            .header("Accept", "application/x-ndjson")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive");
        
        if let Some(auth) = auth_header {
            request_builder = request_builder.header("Authorization", auth);
        }
        
        let response = request_builder.send().await.map_err(|e| {
            McpError::Transport(TransportError::ConnectionError {
                transport_type: "http-stream".to_string(),
                reason: format!("Failed to establish HTTP stream: {}", e),
            })
        })?;
        
        if !response.status().is_success() {
            return Err(McpError::Transport(TransportError::ConnectionError {
                transport_type: "http-stream".to_string(),
                reason: format!("HTTP stream failed with status: {}", response.status()),
            }));
        }
        
        Ok(response)
    }
    
    /// Handle the streaming connection
    async fn handle_stream(
        response: Response,
        outbound_receiver: &mut mpsc::UnboundedReceiver<JsonRpcMessage>,
        inbound_sender: &mpsc::UnboundedSender<JsonRpcMessage>,
        client: &Client,
        base_url: &str,
        auth_header: &Option<String>,
    ) -> McpResult<()> {
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        
        loop {
            tokio::select! {
                // Handle incoming stream data
                chunk_result = stream.next() => {
                    match chunk_result {
                        Some(Ok(chunk)) => {
                            if let Ok(chunk_str) = std::str::from_utf8(&chunk) {
                                buffer.push_str(chunk_str);
                                
                                // Process complete JSON lines
                                while let Some(newline_pos) = buffer.find('\n') {
                                    let line = buffer[..newline_pos].trim().to_string();
                                    buffer.drain(..=newline_pos);
                                    
                                    if !line.is_empty() {
                                        if let Ok(message) = serde_json::from_str::<JsonRpcMessage>(&line) {
                                            debug!("Received message: {:?}", message);
                                            if inbound_sender.send(message).is_err() {
                                                warn!("Failed to send inbound message");
                                                return Ok(());
                                            }
                                        } else {
                                            warn!("Failed to parse JSON message: {}", line);
                                        }
                                    }
                                }
                            }
                        }
                        Some(Err(e)) => {
                            error!("Stream read error: {}", e);
                            return Err(McpError::Transport(TransportError::NetworkError {
                                transport_type: "http-stream".to_string(),
                                reason: format!("Stream read error: {}", e),
                            }));
                        }
                        None => {
                            warn!("Stream ended");
                            return Ok(());
                        }
                    }
                }
                
                // Handle outbound messages
                message = outbound_receiver.recv() => {
                    match message {
                        Some(msg) => {
                            if let Err(e) = Self::send_message(client, base_url, auth_header, &msg).await {
                                error!("Failed to send outbound message: {}", e);
                                return Err(e);
                            }
                        }
                        None => {
                            debug!("Outbound channel closed");
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
    
    /// Send a message via HTTP POST
    async fn send_message(
        client: &Client,
        base_url: &str,
        auth_header: &Option<String>,
        message: &JsonRpcMessage,
    ) -> McpResult<()> {
        let url = format!("{}/message", base_url);
        let json_body = serde_json::to_string(message).map_err(|e| {
            McpError::Transport(TransportError::SerializationError {
                transport_type: "http-stream".to_string(),
                reason: format!("Failed to serialize message: {}", e),
            })
        })?;
        
        let mut request_builder = client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(json_body);
        
        if let Some(auth) = auth_header {
            request_builder = request_builder.header("Authorization", auth);
        }
        
        let response = request_builder.send().await.map_err(|e| {
            McpError::Transport(TransportError::NetworkError {
                transport_type: "http-stream".to_string(),
                reason: format!("HTTP request failed: {}", e),
            })
        })?;
        
        if !response.status().is_success() {
            return Err(McpError::Transport(TransportError::NetworkError {
                transport_type: "http-stream".to_string(),
                reason: format!("HTTP request failed with status: {}", response.status()),
            }));
        }
        
        debug!("Message sent successfully");
        Ok(())
    }
    
    /// Send a message through the connection
    async fn send(&self, message: JsonRpcMessage) -> McpResult<()> {
        self.outbound_sender.send(message).map_err(|_| {
            McpError::Transport(TransportError::ConnectionError {
                transport_type: "http-stream".to_string(),
                reason: "Connection closed".to_string(),
            })
        })
    }
    
    /// Receive a message from the connection
    async fn receive(&self, timeout_duration: Option<Duration>) -> McpResult<JsonRpcMessage> {
        let mut receiver = self.inbound_receiver.lock().await;
        
        if let Some(timeout_duration) = timeout_duration {
            timeout(timeout_duration, receiver.recv())
                .await
                .map_err(|_| McpError::Transport(TransportError::TimeoutError {
                    transport_type: "http-stream".to_string(),
                    reason: format!("Receive timeout after {:?}", timeout_duration),
                }))?
                .ok_or_else(|| McpError::Transport(TransportError::DisconnectedError {
                    transport_type: "http-stream".to_string(),
                    reason: "Connection closed".to_string(),
                }))
        } else {
            receiver.recv().await.ok_or_else(|| McpError::Transport(TransportError::DisconnectedError {
                transport_type: "http-stream".to_string(),
                reason: "Connection closed".to_string(),
            }))
        }
    }
}

/// HTTP streaming transport implementation
pub struct HttpStreamTransport {
    /// HTTP client for making requests
    client: Client,
    /// Base URL for the MCP server
    base_url: String,
    /// Optional authentication header
    auth_header: Option<String>,
    /// Transport configuration
    config: TransportConfig,
    /// Current streaming connection
    connection: Option<StreamingConnection>,
    /// Transport information
    info: TransportInfo,
    /// Pending requests awaiting responses
    pending_requests: Arc<Mutex<HashMap<String, oneshot::Sender<JsonRpcResponse>>>>,
}

impl HttpStreamTransport {
    /// Create a new HTTP streaming transport.
    pub fn new(base_url: String, auth_header: Option<String>) -> Self {
        let client = Client::new();
        
        Self {
            client,
            base_url: base_url.clone(),
            auth_header: auth_header.clone(),
            config: TransportConfig::HttpStream(crate::transport::config::HttpStreamConfig {
                base_url: base_url.parse().unwrap_or_else(|_| "http://localhost".parse().unwrap()),
                timeout: Duration::from_secs(300),
                headers: std::collections::HashMap::new(),
                auth: auth_header.map(|token| crate::transport::config::AuthConfig::bearer(token)),
                compression: true,
                flow_control_window: 65536,
            }),
            connection: None,
            info: TransportInfo::new("http-stream"),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Transport for HttpStreamTransport {
    fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    async fn connect(&mut self) -> McpResult<()> {
        info!("Connecting HTTP streaming transport to {}", self.base_url);
        
        let connection = StreamingConnection::new(
            self.base_url.clone(),
            self.auth_header.clone(),
        ).await?;
        
        self.connection = Some(connection);
        self.info.mark_connected();
        
        info!("HTTP streaming transport connected successfully");
        Ok(())
    }

    async fn send_request(
        &mut self,
        request: JsonRpcRequest,
        timeout_duration: Option<Duration>,
    ) -> McpResult<JsonRpcResponse> {
        let connection = self.connection.as_ref().ok_or_else(|| {
            McpError::Transport(TransportError::NotConnected {
                transport_type: "http-stream".to_string(),
                reason: "Transport not connected".to_string(),
            })
        })?;

        let request_id = request.id.clone();
        let (response_sender, response_receiver) = oneshot::channel();
        
        // Store the response sender for correlation
        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(request_id.to_string(), response_sender);
        }

        // Send the request
        connection.send(JsonRpcMessage::Request(request)).await?;
        self.info.increment_requests_sent();

        // Wait for response with timeout
        let timeout_duration = timeout_duration.unwrap_or(Duration::from_secs(30));
        let response = timeout(timeout_duration, response_receiver)
            .await
            .map_err(|_| McpError::Transport(TransportError::TimeoutError {
                transport_type: "http-stream".to_string(),
                reason: format!("Request {} timed out after {:?}", request_id, timeout_duration),
            }))?
            .map_err(|_| McpError::Transport(TransportError::NetworkError {
                transport_type: "http-stream".to_string(),
                reason: "Response channel closed unexpectedly".to_string(),
            }))?;

        self.info.increment_responses_received();
        Ok(response)
    }

    async fn send_notification(&mut self, notification: JsonRpcNotification) -> McpResult<()> {
        let connection = self.connection.as_ref().ok_or_else(|| {
            McpError::Transport(TransportError::NotConnected {
                transport_type: "http-stream".to_string(),
                reason: "Transport not connected".to_string(),
            })
        })?;

        connection.send(JsonRpcMessage::Notification(notification)).await?;
        self.info.increment_notifications_sent();
        Ok(())
    }

    async fn receive_message(&mut self, timeout_duration: Option<Duration>) -> McpResult<JsonRpcMessage> {
        let connection = self.connection.as_ref().ok_or_else(|| {
            McpError::Transport(TransportError::NotConnected {
                transport_type: "http-stream".to_string(),
                reason: "Transport not connected".to_string(),
            })
        })?;

        let message = connection.receive(timeout_duration).await?;
        
        // Handle response correlation
        if let JsonRpcMessage::Response(ref response) = message {
            let response_sender = {
                let mut pending = self.pending_requests.lock().await;
                pending.remove(&response.id.to_string())
            };
            
            if let Some(sender) = response_sender {
                let _ = sender.send(response.clone());
                // Don't return the response here since it's handled via the oneshot channel
                return self.receive_message(timeout_duration).await;
            }
        }

        // Update statistics
        match &message {
            JsonRpcMessage::Request(_) => {
                // Server-to-client request - not typically expected in MCP
                warn!("Received unexpected server-to-client request");
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

    async fn disconnect(&mut self) -> McpResult<()> {
        info!("Disconnecting HTTP streaming transport");
        
        if let Some(connection) = self.connection.take() {
            // Abort the connection task
            connection.task_handle.abort();
            let _ = connection.task_handle.await;
        }
        
        // Clear pending requests
        {
            let mut pending = self.pending_requests.lock().await;
            pending.clear();
        }
        
        self.info.mark_disconnected();
        
        info!("HTTP streaming transport disconnected");
        Ok(())
    }

    fn get_info(&self) -> TransportInfo {
        let mut info = self.info.clone();
        
        // Add HTTP streaming specific metadata
        info.add_metadata("base_url", serde_json::json!(self.base_url));
        info.add_metadata("has_auth", serde_json::json!(self.auth_header.is_some()));
        
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
        let transport = HttpStreamTransport::new(
            "http://localhost:8080".to_string(),
            Some("Bearer token123".to_string()),
        );
        
        assert_eq!(transport.get_info().transport_type, "http-stream");
        assert!(!transport.is_connected());
        assert_eq!(transport.base_url, "http://localhost:8080");
        assert_eq!(transport.auth_header, Some("Bearer token123".to_string()));
    }

    #[test]
    fn test_transport_info_metadata() {
        let transport = HttpStreamTransport::new(
            "https://api.example.com".to_string(),
            None,
        );
        
        let info = transport.get_info();
        assert!(info.metadata.contains_key("base_url"));
        assert!(info.metadata.contains_key("has_auth"));
        assert_eq!(info.metadata.get("has_auth").unwrap(), &serde_json::json!(false));
    }

    #[test]
    fn test_auth_header_handling() {
        let transport = HttpStreamTransport::new(
            "http://localhost:8080".to_string(),
            Some("Basic dXNlcjpwYXNz".to_string()),
        );
        
        assert!(transport.auth_header.is_some());
        assert!(transport.auth_header.unwrap().starts_with("Basic "));
    }
} 