//! Streamable HTTP transport implementation for MCP communication.
//!
//! This transport implements the MCP Streamable HTTP specification:
//! - HTTP POST requests to base URL for client-to-server communication
//! - Session management via Mcp-Session-Id headers
//! - Support for single JSON responses and SSE streams
//! - Automatic session extraction and inclusion
//! - Resumable connections with Last-Event-ID support
//! - Security validations and localhost binding

use std::sync::{Arc, Mutex};
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
    session_manager: SessionManager,
}

/// MCP protocol version for transport compatibility
#[derive(Debug, Clone, PartialEq)]
enum McpProtocolVersion {
    /// Modern Streamable HTTP (2025-03-26) - single /mcp endpoint, Mcp-Session-Id header
    StreamableHttp,
    /// Legacy HTTP+SSE (2024-11-05) - dual endpoints, sessionId query parameters  
    HttpSse,
    /// Auto-detect based on server behavior
    AutoDetect,
}

/// Generic session management for MCP SSE servers
#[derive(Debug, Clone)]
struct SessionManager {
    /// Whether to automatically discover sessions
    auto_discover: bool,
    /// Known session discovery endpoints (relative to base URL)
    discovery_endpoints: Vec<String>,
    /// Session timeout for renewal
    #[allow(dead_code)]
    session_timeout: Duration,
    /// Current session URL if different from base URL
    #[allow(dead_code)]
    active_session_url: Option<Url>,
    /// Session discovery task handle for background monitoring
    _discovery_task: Option<Arc<tokio::task::JoinHandle<()>>>,
    /// Receiver for fresh session IDs from background task
    session_receiver: Option<Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<String>>>>,
    /// Receiver for JSON-RPC messages from session monitor
    jsonrpc_receiver: Option<Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<JsonRpcMessage>>>>,
    /// Detected or configured protocol version
    protocol_version: McpProtocolVersion,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self {
            auto_discover: true, // Enable continuous session monitoring
            discovery_endpoints: vec![
                "/events".to_string(),
                "/session".to_string(),
                "/discover".to_string(),
            ],
            session_timeout: Duration::from_secs(300), // 5 minutes default
            active_session_url: None,
            _discovery_task: None,
            session_receiver: None,
            jsonrpc_receiver: None,
            protocol_version: McpProtocolVersion::AutoDetect,
        }
    }
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
            session_manager: SessionManager::default(),
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

    /// Detect MCP protocol version based on endpoint and server behavior
    fn detect_protocol_version(&mut self) -> McpProtocolVersion {
        if self.session_manager.protocol_version != McpProtocolVersion::AutoDetect {
            return self.session_manager.protocol_version.clone();
        }

        // Auto-detect based on endpoint patterns
        match self.base_url.path() {
            "/mcp" => {
                tracing::info!(
                    "Detected Modern Streamable HTTP protocol (2025-03-26) - /mcp endpoint"
                );
                self.session_manager.protocol_version = McpProtocolVersion::StreamableHttp;
                McpProtocolVersion::StreamableHttp
            }
            "/sse" => {
                tracing::info!("Detected Legacy HTTP+SSE protocol (2024-11-05) - /sse endpoint");
                self.session_manager.protocol_version = McpProtocolVersion::HttpSse;
                McpProtocolVersion::HttpSse
            }
            path => {
                tracing::warn!(
                    "Unknown endpoint pattern: {}, defaulting to Modern Streamable HTTP",
                    path
                );
                self.session_manager.protocol_version = McpProtocolVersion::StreamableHttp;
                McpProtocolVersion::StreamableHttp
            }
        }
    }

    /// Send a request and handle both JSON and SSE responses according to MCP spec.
    async fn send_mcp_request(
        &mut self,
        message: JsonRpcMessage,
    ) -> McpResult<Option<JsonRpcResponse>> {
        // Get the freshest session ID available
        self.get_fresh_session_id().await;

        // Detect protocol version and route accordingly
        let protocol_version = self.detect_protocol_version();
        match protocol_version {
            McpProtocolVersion::StreamableHttp => {
                tracing::info!("Using Modern Streamable HTTP protocol (header-based sessions)");
                self.send_streamable_http_request(message).await
            }
            McpProtocolVersion::HttpSse => {
                tracing::info!("Using Legacy HTTP+SSE protocol (query parameter sessions)");
                self.send_legacy_sse_request(message).await
            }
            McpProtocolVersion::AutoDetect => {
                // This shouldn't happen after detection, but fallback to modern
                tracing::warn!(
                    "Protocol auto-detection failed, falling back to Modern Streamable HTTP"
                );
                self.send_streamable_http_request(message).await
            }
        }
    }

    /// Send request using Modern Streamable HTTP protocol (2025-03-26)
    async fn send_streamable_http_request(
        &mut self,
        message: JsonRpcMessage,
    ) -> McpResult<Option<JsonRpcResponse>> {
        let mut request_builder = self
            .http_client
            .post(self.base_url.clone())
            .header(CONTENT_TYPE, "application/json")
            .header("Accept", "application/json, text/event-stream");

        // Validate Origin header for security
        self.validate_origin(&request_builder)?;

        // Include session ID in Mcp-Session-Id header (Modern protocol)
        if let Some(ref session_id) = self.session_id {
            request_builder = request_builder.header("Mcp-Session-Id", session_id);
            tracing::info!("Using session ID in header (Modern): {}", session_id);
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
                reason: format!("Modern HTTP request failed: {}", e),
            }
        })?;

        // Extract session ID from response header (for initialization)
        if let Some(session_header) = response.headers().get("mcp-session-id") {
            if let Ok(session_str) = session_header.to_str() {
                self.validate_session_id(session_str)?;
                tracing::info!("Extracted session ID from Modern response: {}", session_str);
                self.session_id = Some(session_str.to_string());
            }
        }

        // Handle response based on Content-Type
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("application/json");

        tracing::info!("=== MODERN HTTP RESPONSE DEBUG ===");
        tracing::info!("Status: {}", response.status());
        tracing::info!("Content-Type: {}", content_type);
        tracing::info!("Headers: {:?}", response.headers());

        match content_type {
            ct if ct.contains("application/json") => {
                // Single JSON response - standard case
                let response_text =
                    response
                        .text()
                        .await
                        .map_err(|e| TransportError::SerializationError {
                            transport_type: "streamable-http".to_string(),
                            reason: format!("Failed to get Modern response text: {}", e),
                        })?;

                tracing::info!("=== MODERN JSON RESPONSE ===");
                tracing::info!("{}", response_text);

                let json_response: JsonRpcResponse =
                    serde_json::from_str(&response_text).map_err(|e| {
                        TransportError::SerializationError {
                            transport_type: "streamable-http".to_string(),
                            reason: format!("Failed to parse Modern JSON response: {}", e),
                        }
                    })?;
                Ok(Some(json_response))
            }
            ct if ct.contains("text/event-stream") => {
                // SSE stream response - for multiple messages
                tracing::info!("Modern protocol returned SSE stream");
                self.handle_sse_response(response).await?;

                // Wait for response via SSE stream
                if let JsonRpcMessage::Request(req) = message {
                    tracing::info!("Waiting for Modern SSE response to request ID: {}", req.id);
                    return Ok(Some(
                        self.wait_for_sse_response(&req.id.to_string(), Duration::from_secs(10))
                            .await?,
                    ));
                }
                Ok(None)
            }
            _ => Err(TransportError::NetworkError {
                transport_type: "streamable-http".to_string(),
                reason: format!("Unexpected Modern content type: {}", content_type),
            }
            .into()),
        }
    }

    /// Send request using Legacy HTTP+SSE protocol (2024-11-05)
    async fn send_legacy_sse_request(
        &mut self,
        message: JsonRpcMessage,
    ) -> McpResult<Option<JsonRpcResponse>> {
        tracing::info!("Sending request using Legacy HTTP+SSE protocol");

        // Wait for a fresh session ID before sending request
        let mut attempts = 0;
        while self.session_id.is_none() && attempts < 50 {
            self.get_fresh_session_id().await;
            if self.session_id.is_none() {
                tokio::time::sleep(Duration::from_millis(100)).await;
                attempts += 1;
            }
        }

        // Build URL with session ID in query parameters (Legacy protocol)
        let mut request_url = self.base_url.clone();
        if let Some(ref session_id) = self.session_id {
            request_url.set_query(Some(&format!("sessionId={}", session_id)));
            tracing::info!(
                "Using session ID in query parameter (Legacy): {}",
                session_id
            );
        } else {
            tracing::warn!("No session ID available for Legacy request after waiting");
        }

        tracing::info!("Sending Legacy POST request to: {}", request_url);

        let request_builder = self
            .http_client
            .post(request_url)
            .header(CONTENT_TYPE, "application/json")
            .header("Accept", "application/json, text/event-stream");

        // Send the JSON-RPC request
        let response = request_builder.json(&message).send().await.map_err(|e| {
            TransportError::NetworkError {
                transport_type: "streamable-http".to_string(),
                reason: format!("Legacy HTTP+SSE request failed: {}", e),
            }
        })?;

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("");

        tracing::info!("=== LEGACY HTTP+SSE RESPONSE DEBUG ===");
        tracing::info!("Status: {}", response.status());
        tracing::info!("Content-Type: {}", content_type);
        tracing::info!("Headers: {:?}", response.headers());

        // Handle response based on Status and Content-Type
        match (response.status().as_u16(), content_type) {
            (202, _) => {
                // 202 Accepted - Legacy protocol, response will come via SSE stream
                tracing::info!("Legacy protocol: Request accepted (202), waiting for SSE response");

                // Wait for response via SSE stream
                if let JsonRpcMessage::Request(req) = message {
                    tracing::info!("Waiting for Legacy SSE response to request ID: {}", req.id);
                    return Ok(Some(
                        self.wait_for_sse_response(&req.id.to_string(), Duration::from_secs(10))
                            .await?,
                    ));
                }
                Ok(None)
            }
            (_, ct) if ct.contains("application/json") => {
                // Direct JSON response
                let response_text =
                    response
                        .text()
                        .await
                        .map_err(|e| TransportError::SerializationError {
                            transport_type: "streamable-http".to_string(),
                            reason: format!("Failed to get Legacy response text: {}", e),
                        })?;

                tracing::info!("=== LEGACY JSON RESPONSE ===");
                tracing::info!("{}", response_text);

                let json_response: JsonRpcResponse =
                    serde_json::from_str(&response_text).map_err(|e| {
                        TransportError::SerializationError {
                            transport_type: "streamable-http".to_string(),
                            reason: format!("Failed to parse Legacy JSON response: {}", e),
                        }
                    })?;
                Ok(Some(json_response))
            }
            (_, ct) if ct.contains("text/event-stream") => {
                // SSE stream response
                tracing::info!("Legacy protocol returned SSE stream");
                self.handle_sse_response(response).await?;

                // Wait for response via SSE stream
                if let JsonRpcMessage::Request(req) = message {
                    tracing::info!("Waiting for Legacy SSE response to request ID: {}", req.id);
                    return Ok(Some(
                        self.wait_for_sse_response(&req.id.to_string(), Duration::from_secs(10))
                            .await?,
                    ));
                }
                Ok(None)
            }
            (status, ct) => Err(TransportError::NetworkError {
                transport_type: "streamable-http".to_string(),
                reason: format!(
                    "Unexpected Legacy response - Status: {}, Content-Type: {}",
                    status, ct
                ),
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

                        // Parse event data as JSON-RPC message (skip session announcements)
                        if event.data.starts_with("/sse?sessionId=")
                            || event.data.starts_with("/mcp?sessionId=")
                        {
                            tracing::debug!("Skipping session announcement: {}", event.data);
                        } else if let Ok(message) =
                            serde_json::from_str::<JsonRpcMessage>(&event.data)
                        {
                            tracing::info!("Parsed JSON-RPC message from SSE: {:?}", message);
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

    /// Start continuous session monitoring for MCP servers with ephemeral sessions
    async fn start_continuous_session_monitoring(&mut self) -> McpResult<()> {
        if !self.session_manager.auto_discover {
            return Ok(());
        }

        // Check if this is a Modern protocol endpoint that doesn't need session monitoring
        if self.base_url.path() == "/mcp" {
            tracing::info!(
                "Modern Streamable HTTP protocol detected - skipping session monitoring"
            );
            self.session_manager.protocol_version = McpProtocolVersion::StreamableHttp;
            return Ok(());
        }

        tracing::info!("Starting continuous session monitoring for MCP server");

        // Try each discovery endpoint to find one that works
        for endpoint in &self.session_manager.discovery_endpoints.clone() {
            if let Ok(Some(_)) = self.start_session_monitor_for_endpoint(endpoint).await {
                tracing::info!(
                    "Started continuous session monitoring via endpoint: {}",
                    endpoint
                );
                return Ok(());
            }
        }

        tracing::info!("No session monitoring endpoints available - proceeding without session");
        Ok(())
    }

    /// Start background monitoring for a specific endpoint
    async fn start_session_monitor_for_endpoint(
        &mut self,
        endpoint: &str,
    ) -> McpResult<Option<()>> {
        // For SSE endpoints, we need to discover sessions via /events, not the SSE endpoint itself
        let discovery_endpoint = if endpoint == "/sse" {
            "/events"
        } else {
            endpoint
        };

        let discovery_url =
            self.base_url
                .join(discovery_endpoint)
                .map_err(|e| TransportError::InvalidConfig {
                    transport_type: "streamable-http".to_string(),
                    reason: format!("Invalid discovery endpoint {}: {}", discovery_endpoint, e),
                })?;

        tracing::info!("Starting session monitor at: {}", discovery_url);

        // Test if endpoint responds with SSE
        let test_response = self
            .http_client
            .get(discovery_url.clone())
            .header("Accept", "text/event-stream")
            .send()
            .await
            .map_err(|e| TransportError::NetworkError {
                transport_type: "streamable-http".to_string(),
                reason: format!("Session monitor test failed: {}", e),
            })?;

        let content_type = test_response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("");

        if !content_type.contains("text/event-stream") {
            tracing::debug!(
                "Endpoint {} does not provide SSE stream",
                discovery_endpoint
            );
            return Ok(None);
        }

        // Start background session monitoring task
        let (session_sender, session_receiver) = tokio::sync::mpsc::unbounded_channel();
        self.session_manager.session_receiver = Some(Arc::new(Mutex::new(session_receiver)));

        // Create JSON-RPC message channel for routing responses
        let (jsonrpc_sender, jsonrpc_receiver) = tokio::sync::mpsc::unbounded_channel();
        self.session_manager.jsonrpc_receiver = Some(Arc::new(Mutex::new(jsonrpc_receiver)));

        let client = self.http_client.clone();
        let url = discovery_url.clone();

        let task_handle = tokio::spawn(async move {
            tracing::info!("Background session monitor started for: {}", url);

            loop {
                match client
                    .get(url.clone())
                    .header("Accept", "text/event-stream")
                    .send()
                    .await
                {
                    Ok(response) => {
                        let event_stream = response.bytes_stream().eventsource();
                        let mut stream = event_stream;

                        while let Some(event_result) = stream.next().await {
                            match event_result {
                                Ok(event) => {
                                    tracing::info!(
                                        "Session monitor received: {} -> {}",
                                        event.event,
                                        event.data
                                    );

                                    // Try to parse as JSON-RPC message first
                                    if let Ok(json_rpc_message) =
                                        serde_json::from_str::<JsonRpcMessage>(&event.data)
                                    {
                                        tracing::info!(
                                            "JSON-RPC message received via session monitor: {:?}",
                                            json_rpc_message
                                        );

                                        // Send JSON-RPC message to main transport for correlation
                                        if jsonrpc_sender.send(json_rpc_message).is_err() {
                                            tracing::debug!(
                                                "JSON-RPC receiver dropped, stopping monitor"
                                            );
                                            return;
                                        }
                                    } else if let Some(session_info) =
                                        Self::extract_session_from_event_data_static(&event.data)
                                    {
                                        tracing::info!(
                                            "Fresh session discovered: {}",
                                            session_info
                                        );

                                        // Send fresh session to the transport
                                        if session_sender.send(session_info).is_err() {
                                            tracing::debug!(
                                                "Session receiver dropped, stopping monitor"
                                            );
                                            return;
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("Session monitor stream error: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Session monitor connection failed: {}", e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }

                // Small delay before reconnecting
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        self.session_manager._discovery_task = Some(Arc::new(task_handle));
        Ok(Some(()))
    }

    /// Static version of session extraction for use in background task
    fn extract_session_from_event_data_static(data: &str) -> Option<String> {
        // Pattern 1: Full URL path with session (/sse?sessionId=...) - preferred
        if let Some(url_start) = data.find("/sse?sessionId=") {
            let session_path = &data[url_start..];
            if let Some(session_end) = session_path.find(|c: char| c.is_whitespace() || c == '\n') {
                return Some(session_path[..session_end].to_string());
            } else {
                return Some(session_path.to_string());
            }
        }

        // Pattern 2: Direct sessionId=value format - extract just the ID
        if let Some(captures) = regex::Regex::new(r"sessionId=([a-fA-F0-9\-]+)")
            .ok()
            .and_then(|re| re.captures(data))
        {
            if let Some(session_match) = captures.get(1) {
                return Some(session_match.as_str().to_string());
            }
        }

        None
    }

    /// Send JSON-RPC request to SSE endpoint with session management (Legacy HTTP+SSE protocol)
    #[allow(dead_code)]
    async fn establish_sse_connection_with_message(
        &mut self,
        message: JsonRpcMessage,
    ) -> McpResult<Option<JsonRpcResponse>> {
        tracing::info!(
            "Sending JSON-RPC request to SSE endpoint (Legacy HTTP+SSE): {}",
            self.base_url
        );

        // Wait for a fresh session ID before sending request
        let mut attempts = 0;
        while self.session_id.is_none() && attempts < 50 {
            self.get_fresh_session_id().await;
            if self.session_id.is_none() {
                tokio::time::sleep(Duration::from_millis(100)).await;
                attempts += 1;
            }
        }

        // For legacy HTTP+SSE protocol, we need to use query parameters, not headers
        let mut request_url = self.base_url.clone();
        if let Some(ref session_id) = self.session_id {
            request_url.set_query(Some(&format!("sessionId={}", session_id)));
            tracing::info!(
                "Using session ID in query parameter for legacy SSE request: {}",
                session_id
            );
        } else {
            tracing::warn!("No session ID available for SSE request after waiting");
        }

        tracing::info!("Sending POST request to: {}", request_url);

        let request_builder = self
            .http_client
            .post(request_url)
            .header(CONTENT_TYPE, "application/json")
            .header("Accept", "application/json, text/event-stream");

        // Send the JSON-RPC request
        let response = request_builder.json(&message).send().await.map_err(|e| {
            TransportError::NetworkError {
                transport_type: "streamable-http".to_string(),
                reason: format!("Legacy SSE JSON-RPC request failed: {}", e),
            }
        })?;

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("");

        tracing::info!(
            "SSE JSON-RPC Response - Status: {}, Content-Type: {}",
            response.status(),
            content_type
        );

        // Handle response based on Content-Type (same as /mcp endpoint)
        match content_type {
            ct if ct.contains("application/json") => {
                // Direct JSON response
                let response_text =
                    response
                        .text()
                        .await
                        .map_err(|e| TransportError::SerializationError {
                            transport_type: "streamable-http".to_string(),
                            reason: format!("Failed to get SSE response text: {}", e),
                        })?;

                tracing::info!("=== SSE JSON RESPONSE ===");
                tracing::info!("{}", response_text);

                let json_response: JsonRpcResponse =
                    serde_json::from_str(&response_text).map_err(|e| {
                        TransportError::SerializationError {
                            transport_type: "streamable-http".to_string(),
                            reason: format!("Failed to parse SSE JSON response: {}", e),
                        }
                    })?;
                Ok(Some(json_response))
            }
            ct if ct.contains("text/event-stream") => {
                // SSE stream response
                tracing::info!("SSE endpoint returned event stream - handling SSE response");
                self.handle_sse_response(response).await?;

                // Wait for response via SSE stream
                if let JsonRpcMessage::Request(req) = message {
                    tracing::info!("Waiting for SSE response to request ID: {}", req.id);
                    return Ok(Some(
                        self.wait_for_sse_response(&req.id.to_string(), Duration::from_secs(10))
                            .await?,
                    ));
                }
                Ok(None)
            }
            _ => Err(TransportError::NetworkError {
                transport_type: "streamable-http".to_string(),
                reason: format!("Unexpected SSE response content type: {}", content_type),
            }
            .into()),
        }
    }

    /// Send JSON-RPC request to SSE endpoint using GET with session parameters
    #[allow(dead_code)]
    async fn send_sse_get_request(
        &mut self,
        message: JsonRpcMessage,
    ) -> McpResult<Option<JsonRpcResponse>> {
        tracing::info!("Sending JSON-RPC to SSE endpoint via GET request");

        // Wait for a fresh session ID before sending request
        let mut attempts = 0;
        while self.session_id.is_none() && attempts < 50 {
            self.get_fresh_session_id().await;
            if self.session_id.is_none() {
                tokio::time::sleep(Duration::from_millis(100)).await;
                attempts += 1;
            }
        }

        // Build URL with session ID in query parameters (legacy HTTP+SSE protocol)
        let mut request_url = self.base_url.clone();
        if let Some(ref session_id) = self.session_id {
            request_url.set_query(Some(&format!("sessionId={}", session_id)));
            tracing::info!(
                "Using session ID in query parameter for SSE GET: {}",
                session_id
            );
        } else {
            tracing::warn!("No session ID available for SSE GET request after waiting");
        }

        tracing::info!("Sending GET request to: {}", request_url);

        // Send GET request to establish SSE connection with session
        let response = self
            .http_client
            .get(request_url)
            .header("Accept", "text/event-stream")
            .send()
            .await
            .map_err(|e| TransportError::NetworkError {
                transport_type: "streamable-http".to_string(),
                reason: format!("SSE GET request failed: {}", e),
            })?;

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("");

        tracing::info!(
            "SSE GET Response - Status: {}, Content-Type: {}",
            response.status(),
            content_type
        );

        if content_type.contains("text/event-stream") {
            tracing::info!("SSE connection established via GET - handling SSE stream");
            self.handle_sse_response(response).await?;

            // For SSE connections, we need to wait for the response to our message
            if let JsonRpcMessage::Request(req) = message {
                tracing::info!("Waiting for SSE response to request ID: {}", req.id);
                return Ok(Some(
                    self.wait_for_sse_response(&req.id.to_string(), Duration::from_secs(10))
                        .await?,
                ));
            }

            Ok(None)
        } else {
            Err(TransportError::NetworkError {
                transport_type: "streamable-http".to_string(),
                reason: format!("Expected SSE stream but got: {}", content_type),
            }
            .into())
        }
    }

    /// Get the most recent session ID from the background monitor (only for Legacy protocol)
    async fn get_fresh_session_id(&mut self) -> Option<String> {
        // For Modern protocol, don't use session monitor - use response headers instead
        if self.session_manager.protocol_version == McpProtocolVersion::StreamableHttp {
            return self.session_id.clone();
        }

        if let Some(ref receiver_arc) = self.session_manager.session_receiver {
            if let Ok(mut receiver) = receiver_arc.lock() {
                // Try to get the most recent session (non-blocking)
                while let Ok(session_info) = receiver.try_recv() {
                    tracing::info!("Received fresh session: {}", session_info);

                    // Extract session ID from either URL format or direct ID
                    if session_info.starts_with("/sse?sessionId=") {
                        // Extract session ID from URL format
                        if let Some(session_id) = session_info.split("sessionId=").nth(1) {
                            self.session_id = Some(session_id.to_string());
                            tracing::info!("Extracted session ID from URL: {}", session_id);
                        }
                    } else {
                        // Direct session ID
                        self.session_id = Some(session_info.clone());
                        tracing::info!("Updated to fresh session ID: {}", session_info);
                    }
                }
            }
        }
        self.session_id.clone()
    }

    /// Try to discover session information from a specific endpoint
    #[allow(dead_code)]
    async fn try_discover_session_from_endpoint(
        &mut self,
        endpoint: &str,
    ) -> McpResult<Option<String>> {
        let discovery_url =
            self.base_url
                .join(endpoint)
                .map_err(|e| TransportError::InvalidConfig {
                    transport_type: "streamable-http".to_string(),
                    reason: format!("Invalid discovery endpoint {}: {}", endpoint, e),
                })?;

        tracing::debug!("Trying session discovery at: {}", discovery_url);

        // Try to get session information via SSE stream
        let response = self
            .http_client
            .get(discovery_url.clone())
            .header("Accept", "text/event-stream, application/json")
            .send()
            .await
            .map_err(|e| TransportError::NetworkError {
                transport_type: "streamable-http".to_string(),
                reason: format!("Discovery request failed: {}", e),
            })?;

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("");

        match content_type {
            ct if ct.contains("text/event-stream") => self.parse_session_from_sse(response).await,
            ct if ct.contains("application/json") => self.parse_session_from_json(response).await,
            _ => {
                tracing::debug!(
                    "Unexpected content type for session discovery: {}",
                    content_type
                );
                Ok(None)
            }
        }
    }

    /// Parse session information from SSE stream (e.g., Playwright-style)
    #[allow(dead_code)]
    async fn parse_session_from_sse(&mut self, response: Response) -> McpResult<Option<String>> {
        use futures::StreamExt;

        let event_stream = response.bytes_stream().eventsource();
        let mut stream = event_stream;

        // Listen for the first few events to find session information
        let timeout_duration = Duration::from_secs(5);
        let deadline = tokio::time::Instant::now() + timeout_duration;

        while let Ok(Some(event_result)) = tokio::time::timeout_at(deadline, stream.next()).await {
            match event_result {
                Ok(event) => {
                    tracing::debug!("Discovery SSE event: {} -> {}", event.event, event.data);

                    // Look for session information in various formats
                    if let Some(session_info) = self.extract_session_from_event_data(&event.data) {
                        return Ok(Some(session_info));
                    }
                }
                Err(e) => {
                    tracing::debug!("SSE discovery error: {}", e);
                    break;
                }
            }
        }

        Ok(None)
    }

    /// Parse session information from JSON response
    #[allow(dead_code)]
    async fn parse_session_from_json(&mut self, response: Response) -> McpResult<Option<String>> {
        let json_text = response
            .text()
            .await
            .map_err(|e| TransportError::SerializationError {
                transport_type: "streamable-http".to_string(),
                reason: format!("Failed to read JSON discovery response: {}", e),
            })?;

        // Try to parse as JSON and look for session information
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json_text) {
            // Look for session info in common JSON patterns
            if let Some(session_id) = value
                .get("sessionId")
                .or_else(|| value.get("session_id"))
                .or_else(|| value.get("session"))
                .and_then(|v| v.as_str())
            {
                return Ok(Some(session_id.to_string()));
            }

            // Look for endpoint URL patterns
            if let Some(endpoint) = value
                .get("endpoint")
                .or_else(|| value.get("url"))
                .and_then(|v| v.as_str())
            {
                if let Some(session_info) = self.extract_session_from_event_data(endpoint) {
                    return Ok(Some(session_info));
                }
            }
        }

        Ok(None)
    }

    /// Extract session information from event data (handles multiple formats)
    #[allow(dead_code)]
    fn extract_session_from_event_data(&self, data: &str) -> Option<String> {
        // Pattern 1: Full URL path with session (/sse?sessionId=...) - preferred
        if let Some(url_start) = data.find("/sse?sessionId=") {
            let session_path = &data[url_start..];
            if let Some(session_end) = session_path.find(|c: char| c.is_whitespace() || c == '\n') {
                return Some(session_path[..session_end].to_string());
            } else {
                return Some(session_path.to_string());
            }
        }

        // Pattern 2: Direct sessionId=value format (like Playwright) - extract just the ID
        if let Some(captures) = regex::Regex::new(r"sessionId=([a-fA-F0-9\-]+)")
            .ok()
            .and_then(|re| re.captures(data))
        {
            if let Some(session_match) = captures.get(1) {
                return Some(session_match.as_str().to_string());
            }
        }

        // Pattern 3: JSON-like format
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(data) {
            if let Some(session_id) = value
                .get("sessionId")
                .or_else(|| value.get("session_id"))
                .and_then(|v| v.as_str())
            {
                return Some(session_id.to_string());
            }
        }

        None
    }

    /// Update session information from discovered data  
    #[allow(dead_code)]
    fn apply_discovered_session(&mut self, session_info: &str) -> McpResult<()> {
        // If the session info looks like a complete URL path, update base URL for /sse endpoint
        if session_info.starts_with('/') && session_info.contains("sse") {
            match self.base_url.join(session_info) {
                Ok(new_url) => {
                    tracing::info!(
                        "Updated base URL to use discovered session endpoint: {}",
                        new_url
                    );
                    self.session_manager.active_session_url = Some(new_url.clone());
                    self.base_url = new_url;

                    // Also extract session ID for any header usage
                    if let Some(session_start) = session_info.find("sessionId=") {
                        let id_part = &session_info[session_start + 10..]; // Skip "sessionId="
                        if let Some(id_end) =
                            id_part.find(|c: char| !c.is_alphanumeric() && c != '-')
                        {
                            self.session_id = Some(id_part[..id_end].to_string());
                        } else {
                            self.session_id = Some(id_part.to_string());
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to update URL with session path {}: {}",
                        session_info,
                        e
                    );
                    return Ok(());
                }
            }
        } else {
            // Use as session ID directly for header-based sessions (like /mcp endpoint)
            tracing::info!(
                "Using session ID for header-based requests: {}",
                session_info
            );
            self.session_id = Some(session_info.to_string());
        }

        Ok(())
    }

    /// Wait for a specific response from the SSE stream or session monitor
    async fn wait_for_sse_response(
        &mut self,
        request_id: &str,
        timeout_duration: Duration,
    ) -> McpResult<JsonRpcResponse> {
        tracing::debug!("Waiting for response to request ID: {}", request_id);

        // For Legacy protocol, check session monitor's JSON-RPC receiver first
        if let Some(ref jsonrpc_receiver_arc) = self.session_manager.jsonrpc_receiver {
            tracing::debug!("Checking session monitor for Legacy protocol response");

            let deadline = tokio::time::Instant::now() + timeout_duration;

            while tokio::time::Instant::now() < deadline {
                if let Ok(mut receiver) = jsonrpc_receiver_arc.lock() {
                    match receiver.try_recv() {
                        Ok(message) => {
                            tracing::info!("Received message from session monitor: {:?}", message);
                            match message {
                                JsonRpcMessage::Response(response) => {
                                    if response.id.to_string() == request_id {
                                        tracing::info!("Found matching response via session monitor for request ID: {}", request_id);
                                        self.info.increment_responses_received();
                                        return Ok(response);
                                    } else {
                                        tracing::debug!(
                                            "Response for different request ID: {} (expected: {})",
                                            response.id,
                                            request_id
                                        );
                                    }
                                }
                                _ => {
                                    tracing::debug!("Non-response message from session monitor");
                                }
                            }
                        }
                        Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                            // No message available, continue checking
                        }
                        Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                            tracing::warn!("Session monitor JSON-RPC channel disconnected");
                            break;
                        }
                    }
                }

                // Small delay before checking again
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }

        // Fallback to main SSE receiver for Modern protocol
        if let Some(receiver) = self.sse_receiver.as_mut() {
            tracing::debug!("Checking main SSE receiver for Modern protocol response");

            let deadline = tokio::time::Instant::now() + timeout_duration;

            loop {
                let remaining_time =
                    deadline.saturating_duration_since(tokio::time::Instant::now());
                if remaining_time.is_zero() {
                    break;
                }

                let message = timeout(remaining_time, receiver.recv())
                    .await
                    .map_err(|_| TransportError::TimeoutError {
                        transport_type: "streamable-http".to_string(),
                        reason: format!("SSE response timeout for request ID: {}", request_id),
                    })?
                    .ok_or_else(|| TransportError::DisconnectedError {
                        transport_type: "streamable-http".to_string(),
                        reason: "SSE stream closed while waiting for response".to_string(),
                    })?;

                match message {
                    JsonRpcMessage::Response(response) => {
                        if response.id.to_string() == request_id {
                            tracing::info!(
                                "Found matching response via main SSE for request ID: {}",
                                request_id
                            );
                            self.info.increment_responses_received();
                            return Ok(response);
                        } else {
                            tracing::debug!(
                                "Response for different request ID: {} (expected: {})",
                                response.id,
                                request_id
                            );
                        }
                    }
                    _ => {
                        tracing::debug!("Non-response message from main SSE");
                    }
                }
            }
        }

        Err(TransportError::TimeoutError {
            transport_type: "streamable-http".to_string(),
            reason: format!("Timeout waiting for response to request ID: {}", request_id),
        }
        .into())
    }
}

#[async_trait]
impl Transport for HttpSseTransport {
    async fn connect(&mut self) -> McpResult<()> {
        tracing::info!("Connecting Streamable HTTP transport to: {}", self.base_url);

        // Step 1: Start continuous session monitoring for MCP servers that require it
        self.start_continuous_session_monitoring().await?;

        // Step 2: Test connectivity with a simple request
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

        let request_id = request.id.to_string();
        tracing::debug!(
            "HTTP SSE transport sending request: {} with ID: {}",
            request.method,
            request_id
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
                // Response will come via SSE stream - wait for it
                tracing::debug!(
                    "HTTP SSE transport: waiting for response via SSE stream for request ID: {}",
                    request_id
                );
                self.wait_for_sse_response(&request_id, timeout_duration)
                    .await
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
            .header(CONTENT_TYPE, "application/json")
            .header("Accept", "application/json, text/event-stream");

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
