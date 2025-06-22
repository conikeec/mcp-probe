//! MCP Client Implementation
//!
//! This module provides a high-level interface for MCP client operations,
//! handling protocol initialization, capability negotiation, and message
//! exchanges with MCP servers.
//!
//! The [`McpClient`] is the primary interface for interacting with MCP servers,
//! abstracting away transport details and providing a clean async API.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio::time::{sleep, Instant};

use crate::error::{McpError, McpResult, ProtocolError};
use crate::messages::{
    Capabilities, Implementation, InitializeRequest, InitializeResponse, InitializedNotification,
    JsonRpcId, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
    ProgressNotification, PromptListChangedNotification, ProtocolVersion,
    ResourceListChangedNotification, ResourceUpdatedNotification, ToolListChangedNotification,
};
use crate::transport::{factory::TransportFactory, Transport, TransportConfig};

use tracing::{debug, info, warn};

/// Configuration options for MCP client behavior.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Timeout for individual requests (default: 30 seconds)
    pub request_timeout: Duration,

    /// Timeout for the initialization process (default: 10 seconds)  
    pub init_timeout: Duration,

    /// Maximum number of retry attempts for failed operations
    pub max_retries: u32,

    /// Base delay for exponential backoff retries
    pub retry_base_delay: Duration,

    /// Whether to automatically handle server notifications
    pub auto_handle_notifications: bool,

    /// Buffer size for incoming messages
    pub message_buffer_size: usize,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(30),
            init_timeout: Duration::from_secs(10),
            max_retries: 3,
            retry_base_delay: Duration::from_secs(1),
            auto_handle_notifications: true,
            message_buffer_size: 1000,
        }
    }
}

/// State of the MCP client connection and protocol negotiation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientState {
    /// Client is disconnected
    Disconnected,
    /// Client is connecting to server
    Connecting,
    /// Client is performing protocol initialization
    Initializing,
    /// Client is ready for operations
    Ready,
    /// Client encountered an error
    Error(String),
}

/// Information about the connected MCP server.
#[derive(Debug, Clone)]
pub struct ServerInfo {
    /// Server implementation details
    pub implementation: Implementation,
    /// Server protocol version
    pub protocol_version: ProtocolVersion,
    /// Server capabilities
    pub capabilities: Capabilities,
    /// Connection timestamp
    pub connected_at: Instant,
}

/// Statistics about client operations.
#[derive(Debug, Clone, Default)]
pub struct ClientStats {
    /// Number of requests sent
    pub requests_sent: u64,
    /// Number of responses received
    pub responses_received: u64,
    /// Number of notifications sent
    pub notifications_sent: u64,
    /// Number of notifications received
    pub notifications_received: u64,
    /// Number of errors encountered
    pub errors: u64,
    /// Number of retries performed
    pub retries: u64,
    /// Number of connection attempts
    pub connection_attempts: u64,
    /// Last activity timestamp
    pub last_activity: Option<Instant>,
}

/// Handler for MCP notifications from the server
#[async_trait]
pub trait NotificationHandler: Send + Sync {
    /// Handle progress notifications
    async fn handle_progress(&self, notification: ProgressNotification) -> McpResult<()> {
        debug!("Received progress notification: {:?}", notification);
        Ok(())
    }

    /// Handle resource updated notifications
    async fn handle_resource_updated(
        &self,
        notification: ResourceUpdatedNotification,
    ) -> McpResult<()> {
        debug!("Resource updated: {:?}", notification);
        Ok(())
    }

    /// Handle resource list changed notifications
    async fn handle_resource_list_changed(
        &self,
        notification: ResourceListChangedNotification,
    ) -> McpResult<()> {
        debug!("Resource list changed: {:?}", notification);
        Ok(())
    }

    /// Handle tool list changed notifications
    async fn handle_tool_list_changed(
        &self,
        notification: ToolListChangedNotification,
    ) -> McpResult<()> {
        debug!("Tool list changed: {:?}", notification);
        Ok(())
    }

    /// Handle prompt list changed notifications  
    async fn handle_prompt_list_changed(
        &self,
        notification: PromptListChangedNotification,
    ) -> McpResult<()> {
        debug!("Prompt list changed: {:?}", notification);
        Ok(())
    }
}

/// Default notification handler that logs all notifications.
#[derive(Debug, Default)]
pub struct DefaultNotificationHandler;

#[async_trait]
impl NotificationHandler for DefaultNotificationHandler {}

/// High-level MCP client for communicating with MCP servers.
///
/// The `McpClient` handles the complete MCP protocol flow including:
/// - Transport connection management
/// - Protocol initialization and capability negotiation
/// - Request/response correlation and timeouts
/// - Server notification handling
/// - Automatic retries and error recovery
pub struct McpClient {
    transport: Box<dyn Transport>,
    config: ClientConfig,
    state: RwLock<ClientState>,
    server_info: RwLock<Option<ServerInfo>>,
    stats: Arc<RwLock<ClientStats>>,
    request_counter: AtomicU64,
    pending_requests: Arc<RwLock<HashMap<String, oneshot::Sender<JsonRpcResponse>>>>,
    notification_handler: Arc<dyn NotificationHandler>,
    _message_sender: Option<mpsc::UnboundedSender<JsonRpcMessage>>,
}

impl McpClient {
    /// Create a new MCP client with the specified transport configuration.
    ///
    /// # Arguments
    ///
    /// * `transport_config` - Configuration for the transport layer
    /// * `client_config` - Configuration for client behavior
    /// * `notification_handler` - Handler for server notifications
    ///
    /// # Returns
    ///
    /// A new MCP client ready for connection.
    ///
    /// # Example
    ///
    /// ```rust
    /// use mcp_probe_core::client::{McpClient, ClientConfig, DefaultNotificationHandler};
    /// use mcp_probe_core::transport::TransportConfig;
    ///
    /// # async fn example() -> mcp_probe_core::McpResult<()> {
    /// let transport_config = TransportConfig::stdio("python", &["server.py"]);
    /// let client_config = ClientConfig::default();
    /// let handler = Box::new(DefaultNotificationHandler);
    ///
    /// let client = McpClient::new(transport_config, client_config, handler).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(
        transport_config: TransportConfig,
        client_config: ClientConfig,
        notification_handler: Box<dyn NotificationHandler>,
    ) -> McpResult<Self> {
        let transport = TransportFactory::create(transport_config).await?;

        Ok(Self {
            transport,
            config: client_config,
            state: RwLock::new(ClientState::Disconnected),
            server_info: RwLock::new(None),
            stats: Arc::new(RwLock::new(ClientStats::default())),
            request_counter: AtomicU64::new(1),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            notification_handler: notification_handler.into(),
            _message_sender: None,
        })
    }

    /// Create a new MCP client with default configuration and notification handler.
    ///
    /// # Arguments
    ///
    /// * `transport_config` - Configuration for the transport layer
    ///
    /// # Returns
    ///
    /// A new MCP client with default settings.
    pub async fn with_defaults(transport_config: TransportConfig) -> McpResult<Self> {
        Self::new(
            transport_config,
            ClientConfig::default(),
            Box::new(DefaultNotificationHandler),
        )
        .await
    }

    /// Get the current client state.
    pub async fn state(&self) -> ClientState {
        self.state.read().await.clone()
    }

    /// Get information about the connected server.
    pub async fn server_info(&self) -> Option<ServerInfo> {
        self.server_info.read().await.clone()
    }

    /// Get client operation statistics.
    pub async fn stats(&self) -> ClientStats {
        self.stats.read().await.clone()
    }

    /// Check if the client is connected and ready for operations.
    pub async fn is_ready(&self) -> bool {
        matches!(self.state().await, ClientState::Ready)
    }

    /// Get transport information and metadata.
    pub fn transport_info(&self) -> crate::transport::TransportInfo {
        self.transport.get_info()
    }

    /// Connect to the MCP server and perform protocol initialization.
    ///
    /// This method:
    /// 1. Establishes the transport connection
    /// 2. Sends the initialize request
    /// 3. Waits for the initialize response
    /// 4. Sends the initialized notification
    /// 5. Starts message processing
    ///
    /// # Arguments
    ///
    /// * `client_info` - Information about this client implementation
    ///
    /// # Returns
    ///
    /// Server information upon successful connection.
    ///
    /// # Example
    ///
    /// ```rust
    /// use mcp_probe_core::messages::Implementation;
    ///
    /// # async fn example(mut client: mcp_probe_core::client::McpClient) -> mcp_probe_core::McpResult<()> {
    /// let client_info = Implementation {
    ///     name: "mcp-probe".to_string(),
    ///     version: "0.1.0".to_string(),
    ///     metadata: std::collections::HashMap::new(),
    /// };
    ///
    /// let server_info = client.connect(client_info).await?;
    /// println!("Connected to server: {}", server_info.implementation.name);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(&mut self, client_info: Implementation) -> McpResult<ServerInfo> {
        info!("Connecting MCP client to server");

        // Update state
        *self.state.write().await = ClientState::Connecting;

        // Connect transport
        self.transport.connect().await.map_err(|e| {
            let error = format!("Transport connection failed: {}", e);
            self.set_error_state(error.clone());
            McpError::Protocol(ProtocolError::InitializationFailed { reason: error })
        })?;

        // Start message processing
        self.start_message_processing().await?;

        // Perform protocol initialization
        let server_info = self.perform_initialization(client_info).await?;

        // Update state to ready
        *self.state.write().await = ClientState::Ready;
        *self.server_info.write().await = Some(server_info.clone());

        info!(
            "MCP client connected successfully to {}",
            server_info.implementation.name
        );
        Ok(server_info)
    }

    /// Disconnect from the MCP server.
    pub async fn disconnect(&mut self) -> McpResult<()> {
        info!("Disconnecting MCP client");

        // Update state
        *self.state.write().await = ClientState::Disconnected;

        // Clear server info
        *self.server_info.write().await = None;

        // Clear pending requests
        self.pending_requests.write().await.clear();

        // Disconnect transport
        self.transport.disconnect().await?;

        info!("MCP client disconnected");
        Ok(())
    }

    /// Send a notification to the server.
    pub async fn send_notification<T>(&mut self, method: &str, params: T) -> McpResult<()>
    where
        T: serde::Serialize,
    {
        if !self.is_ready().await {
            return Err(McpError::Protocol(ProtocolError::NotInitialized {
                reason: "Client not ready for notifications".to_string(),
            }));
        }

        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params: Some(serde_json::to_value(params)?),
        };

        self.transport.send_notification(notification).await?;
        self.stats.write().await.notifications_sent += 1;
        Ok(())
    }

    /// Send a request to the server and wait for a response.
    pub async fn send_request<T>(&mut self, method: &str, params: T) -> McpResult<JsonRpcResponse>
    where
        T: serde::Serialize,
    {
        if !self.is_ready().await {
            return Err(McpError::Protocol(ProtocolError::NotInitialized {
                reason: "Client not ready for requests".to_string(),
            }));
        }

        self.send_request_with_timeout(method, params, None).await
    }

    // Private helper methods

    fn set_error_state(&self, error: String) {
        if let Ok(mut state) = self.state.try_write() {
            *state = ClientState::Error(error);
        }
    }

    fn generate_request_id(&self) -> String {
        let counter = self.request_counter.fetch_add(1, Ordering::SeqCst);
        format!("req_{}", counter)
    }

    async fn start_message_processing(&mut self) -> McpResult<()> {
        tracing::info!("Starting message processing task");
        let (sender, mut receiver) = mpsc::unbounded_channel();
        self._message_sender = Some(sender);

        // Clone necessary data for the task
        let pending_requests = Arc::clone(&self.pending_requests);
        let stats = Arc::clone(&self.stats);
        let notification_handler = Arc::clone(&self.notification_handler);

        // Start message processing task
        tokio::spawn(async move {
            tracing::debug!("Message processing task started, waiting for messages");
            while let Some(message) = receiver.recv().await {
                tracing::debug!("Received message in processing task: {:?}", message);
                match message {
                    JsonRpcMessage::Response(response) => {
                        tracing::debug!("Processing response with ID: {}", response.id);
                        // Handle response correlation
                        if let Some(sender) = pending_requests
                            .write()
                            .await
                            .remove(&response.id.to_string())
                        {
                            tracing::debug!(
                                "Found pending request for ID {}, sending response",
                                response.id
                            );
                            let _ = sender.send(response);
                            stats.write().await.responses_received += 1;
                        } else {
                            tracing::warn!(
                                "Received response for unknown request ID: {}",
                                response.id
                            );
                        }
                    }
                    JsonRpcMessage::Notification(notification) => {
                        tracing::debug!("Processing notification: {}", notification.method);
                        // Handle server notifications
                        Self::handle_notification(&*notification_handler, notification).await;
                        stats.write().await.notifications_received += 1;
                    }
                    JsonRpcMessage::Request(_) => {
                        // Server-to-client requests are rare in MCP but possible
                        tracing::warn!("Received unexpected server-to-client request");
                    }
                }
            }
        });

        Ok(())
    }

    async fn handle_notification(
        handler: &dyn NotificationHandler,
        notification: JsonRpcNotification,
    ) {
        match notification.method.as_str() {
            "notifications/progress" => {
                if let Some(params) = notification.params {
                    if let Ok(progress) = serde_json::from_value::<ProgressNotification>(params) {
                        let _ = handler.handle_progress(progress).await;
                    }
                }
            }
            "notifications/resources/updated" => {
                if let Some(params) = notification.params {
                    if let Ok(resource_updated) =
                        serde_json::from_value::<ResourceUpdatedNotification>(params)
                    {
                        let _ = handler.handle_resource_updated(resource_updated).await;
                    }
                }
            }
            "notifications/resources/list_changed" => {
                if let Some(params) = notification.params {
                    if let Ok(list_changed) =
                        serde_json::from_value::<ResourceListChangedNotification>(params)
                    {
                        let _ = handler.handle_resource_list_changed(list_changed).await;
                    }
                }
            }
            "notifications/tools/list_changed" => {
                if let Some(params) = notification.params {
                    if let Ok(list_changed) =
                        serde_json::from_value::<ToolListChangedNotification>(params)
                    {
                        let _ = handler.handle_tool_list_changed(list_changed).await;
                    }
                }
            }
            "notifications/prompts/list_changed" => {
                if let Some(params) = notification.params {
                    if let Ok(list_changed) =
                        serde_json::from_value::<PromptListChangedNotification>(params)
                    {
                        let _ = handler.handle_prompt_list_changed(list_changed).await;
                    }
                }
            }
            _ => {
                warn!("Unknown notification method: {}", notification.method);
            }
        }
    }

    async fn perform_initialization(
        &mut self,
        client_info: Implementation,
    ) -> McpResult<ServerInfo> {
        *self.state.write().await = ClientState::Initializing;
        tracing::info!("Starting MCP protocol initialization");

        // Create initialize request with proper client capabilities
        let capabilities = Capabilities {
            standard: crate::messages::StandardCapabilities {
                tools: Some(crate::messages::ToolCapabilities {
                    list_changed: Some(true),
                }),
                resources: Some(crate::messages::ResourceCapabilities {
                    subscribe: Some(true),
                    list_changed: Some(true),
                }),
                prompts: Some(crate::messages::PromptCapabilities {
                    list_changed: Some(true),
                }),
                ..Default::default()
            },
            ..Default::default()
        };

        let request = InitializeRequest {
            protocol_version: ProtocolVersion::default(),
            capabilities,
            client_info,
        };

        tracing::debug!("Sending initialize request: {:?}", request);

        // Send initialize request bypassing ready check (we're initializing!)
        let response = self
            .send_initialization_request("initialize", request, Some(self.config.init_timeout))
            .await?;

        // Parse initialize response
        tracing::debug!("Received initialize response: {:?}", response);
        let init_response: InitializeResponse = match response.result {
            Some(result) => {
                tracing::debug!("Parsing initialize response result: {:?}", result);
                serde_json::from_value(result)?
            }
            None => {
                tracing::error!("Initialize response missing result field");
                return Err(McpError::Protocol(ProtocolError::InitializationFailed {
                    reason: "Missing result in initialize response".to_string(),
                }));
            }
        };

        tracing::info!(
            "Successfully parsed initialize response from server: {}",
            init_response.server_info.name
        );

        // Send initialized notification
        let initialized = InitializedNotification {
            metadata: HashMap::new(), // Empty metadata map
        };
        tracing::debug!("Sending initialized notification");
        self.send_initialized_notification("initialized", initialized)
            .await?;

        // Create server info
        let server_info = ServerInfo {
            implementation: init_response.server_info,
            protocol_version: init_response.protocol_version,
            capabilities: init_response.capabilities,
            connected_at: Instant::now(),
        };

        Ok(server_info)
    }

    /// Send initialization request without ready state check
    async fn send_initialization_request<T>(
        &mut self,
        method: &str,
        params: T,
        timeout_duration: Option<Duration>,
    ) -> McpResult<JsonRpcResponse>
    where
        T: serde::Serialize,
    {
        tracing::debug!("Sending initialization request: {}", method);
        let request_id = self.generate_request_id();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: JsonRpcId::String(request_id.clone()),
            method: method.to_string(),
            params: Some(serde_json::to_value(params)?),
        };

        let timeout_val = timeout_duration.unwrap_or(self.config.request_timeout);

        // Send request with retries (bypassing ready check)
        self.send_request_with_retries(request, timeout_val).await
    }

    /// Send initialization notification without ready state check
    async fn send_initialized_notification<T>(&mut self, method: &str, params: T) -> McpResult<()>
    where
        T: serde::Serialize,
    {
        tracing::debug!("Sending initialization notification: {}", method);

        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params: Some(serde_json::to_value(params)?),
        };

        self.transport.send_notification(notification).await?;
        self.stats.write().await.notifications_sent += 1;
        tracing::debug!("Initialization notification sent successfully");
        Ok(())
    }

    async fn send_request_with_timeout<T>(
        &mut self,
        method: &str,
        params: T,
        timeout_duration: Option<Duration>,
    ) -> McpResult<JsonRpcResponse>
    where
        T: serde::Serialize,
    {
        let request_id = self.generate_request_id();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: JsonRpcId::String(request_id.clone()),
            method: method.to_string(),
            params: Some(serde_json::to_value(params)?),
        };

        let timeout_val = timeout_duration.unwrap_or(self.config.request_timeout);

        // Send request with retries
        self.send_request_with_retries(request, timeout_val).await
    }

    async fn send_request_with_retries(
        &mut self,
        request: JsonRpcRequest,
        timeout_duration: Duration,
    ) -> McpResult<JsonRpcResponse> {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            match self
                .send_single_request(request.clone(), timeout_duration)
                .await
            {
                Ok(response) => {
                    if attempt > 0 {
                        self.stats.write().await.retries += attempt as u64;
                    }
                    return Ok(response);
                }
                Err(e) => {
                    last_error = Some(e);

                    if attempt < self.config.max_retries {
                        let delay = self.config.retry_base_delay * 2_u32.pow(attempt);
                        debug!(
                            "Request failed, retrying in {:?} (attempt {} of {})",
                            delay,
                            attempt + 1,
                            self.config.max_retries + 1
                        );
                        sleep(delay).await;
                    }
                }
            }
        }

        self.stats.write().await.errors += 1;
        Err(last_error.unwrap())
    }

    async fn send_single_request(
        &mut self,
        request: JsonRpcRequest,
        timeout_duration: Duration,
    ) -> McpResult<JsonRpcResponse> {
        let request_id = request.id.to_string();
        tracing::debug!("Sending single request with ID: {}", request_id);

        // Send request and get response from transport (handles SSE internally)
        let response = self
            .transport
            .send_request(request, Some(timeout_duration))
            .await?;
        self.stats.write().await.requests_sent += 1;

        tracing::debug!("Received response for request ID: {}", response.id);
        Ok(response)
    }
}

/// Builder for creating MCP clients with custom configuration.
pub struct McpClientBuilder {
    transport_config: Option<TransportConfig>,
    client_config: ClientConfig,
    notification_handler: Option<Box<dyn NotificationHandler>>,
}

impl McpClientBuilder {
    /// Create a new client builder.
    pub fn new() -> Self {
        Self {
            transport_config: None,
            client_config: ClientConfig::default(),
            notification_handler: None,
        }
    }

    /// Set the transport configuration.
    pub fn transport(mut self, config: TransportConfig) -> Self {
        self.transport_config = Some(config);
        self
    }

    /// Set the client configuration.
    pub fn config(mut self, config: ClientConfig) -> Self {
        self.client_config = config;
        self
    }

    /// Set a custom notification handler.
    pub fn notification_handler(mut self, handler: Box<dyn NotificationHandler>) -> Self {
        self.notification_handler = Some(handler);
        self
    }

    /// Set request timeout.
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.client_config.request_timeout = timeout;
        self
    }

    /// Set initialization timeout.
    pub fn init_timeout(mut self, timeout: Duration) -> Self {
        self.client_config.init_timeout = timeout;
        self
    }

    /// Set maximum retry attempts.
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.client_config.max_retries = retries;
        self
    }

    /// Build the MCP client.
    pub async fn build(self) -> McpResult<McpClient> {
        let transport_config = self.transport_config.ok_or_else(|| {
            McpError::Protocol(ProtocolError::InvalidConfig {
                reason: "Transport configuration is required".to_string(),
            })
        })?;

        let notification_handler = self
            .notification_handler
            .unwrap_or_else(|| Box::new(DefaultNotificationHandler));

        McpClient::new(transport_config, self.client_config, notification_handler).await
    }
}

impl Default for McpClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::TransportConfig;

    #[tokio::test]
    async fn test_client_creation() {
        let config = TransportConfig::stdio("echo", &[] as &[String]);
        let client_config = ClientConfig::default();
        let handler = Box::new(DefaultNotificationHandler);
        let client = McpClient::new(config, client_config, handler)
            .await
            .unwrap();

        assert_eq!(client.state().await, ClientState::Disconnected);
    }

    #[tokio::test]
    async fn test_client_with_defaults() {
        let config = TransportConfig::stdio("echo", &[] as &[String]);
        let client = McpClient::with_defaults(config).await.unwrap();

        assert_eq!(client.state().await, ClientState::Disconnected);
        assert!(!client.is_ready().await);
    }

    #[test]
    fn test_client_config_defaults() {
        let config = ClientConfig::default();
        assert_eq!(config.request_timeout, Duration::from_secs(30));
        assert_eq!(config.init_timeout, Duration::from_secs(10));
        assert_eq!(config.max_retries, 3);
    }
}
