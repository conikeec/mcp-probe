//! Stdio transport implementation for local process MCP communication.
//!
//! This transport spawns a local process and communicates via stdin/stdout,
//! which is the most common MCP transport mechanism. It's ideal for
//! local development, testing, and integrating with language-specific
//! MCP server implementations.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, Mutex};
use tokio::time::timeout;

use super::{Transport, TransportConfig, TransportInfo};
use crate::error::{McpResult, TransportError};
use crate::messages::{JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};

/// Stdio transport for local process MCP communication.
///
/// This transport implementation provides:
/// - Local process spawning with configurable command and arguments
/// - JSON-RPC communication over stdin/stdout
/// - Process lifecycle management with graceful shutdown
/// - Environment variable and working directory configuration
/// - Request/response correlation and timeout handling
/// - Automatic cleanup of child processes
pub struct StdioTransport {
    config: TransportConfig,
    info: TransportInfo,
    child_process: Option<Child>,
    message_sender: Option<mpsc::UnboundedSender<JsonRpcMessage>>,
    message_receiver: Option<mpsc::UnboundedReceiver<JsonRpcMessage>>,
    outbound_sender: Option<mpsc::UnboundedSender<JsonRpcMessage>>,
    outbound_receiver: Option<mpsc::UnboundedReceiver<JsonRpcMessage>>,
    pending_requests: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<JsonRpcResponse>>>>,
}

impl StdioTransport {
    /// Create a new stdio transport instance.
    ///
    /// # Arguments
    ///
    /// * `config` - Transport configuration containing stdio settings
    ///
    /// # Returns
    ///
    /// A new transport instance ready for connection.
    pub fn new(config: TransportConfig) -> Self {
        let info = TransportInfo::new("stdio");

        Self {
            config,
            info,
            child_process: None,
            message_sender: None,
            message_receiver: None,
            outbound_sender: None,
            outbound_receiver: None,
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Spawn the child process and set up communication channels.
    async fn spawn_process(&mut self) -> McpResult<()> {
        if let TransportConfig::Stdio(stdio_config) = &self.config {
            tracing::debug!(
                "Spawning process: {} {:?}",
                stdio_config.command,
                stdio_config.args
            );

            let mut command = Command::new(&stdio_config.command);
            command
                .args(&stdio_config.args)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

            // Set working directory if specified
            if let Some(ref working_dir) = stdio_config.working_dir {
                command.current_dir(working_dir);
            }

            // Set environment variables
            for (key, value) in &stdio_config.environment {
                command.env(key, value);
            }

            // Spawn the child process
            let mut child = command
                .spawn()
                .map_err(|e| TransportError::ConnectionError {
                    transport_type: "stdio".to_string(),
                    reason: format!("Failed to spawn process: {}", e),
                })?;

            // Extract streams from child process
            let stdin = child
                .stdin
                .take()
                .ok_or_else(|| TransportError::ConnectionError {
                    transport_type: "stdio".to_string(),
                    reason: "Failed to get stdin".to_string(),
                })?;

            let stdout = child
                .stdout
                .take()
                .ok_or_else(|| TransportError::ConnectionError {
                    transport_type: "stdio".to_string(),
                    reason: "Failed to get stdout".to_string(),
                })?;

            let stderr = child
                .stderr
                .take()
                .ok_or_else(|| TransportError::ConnectionError {
                    transport_type: "stdio".to_string(),
                    reason: "Failed to get stderr".to_string(),
                })?;

            // Create channels for bidirectional communication
            let (inbound_sender, inbound_receiver) = mpsc::unbounded_channel();
            let (outbound_sender, outbound_receiver) = mpsc::unbounded_channel();

            // Store channels
            self.message_sender = Some(inbound_sender.clone());
            self.message_receiver = Some(inbound_receiver);
            self.outbound_sender = Some(outbound_sender);

            // Start I/O processing tasks
            let pending_requests = self.pending_requests.clone();
            self.start_io_tasks(
                stdin,
                stdout,
                stderr,
                inbound_sender,
                outbound_receiver,
                pending_requests,
            )
            .await;

            // Store the child process
            self.child_process = Some(child);

            Ok(())
        } else {
            Err(TransportError::InvalidConfig {
                transport_type: "stdio".to_string(),
                reason: "Invalid configuration type".to_string(),
            }
            .into())
        }
    }

    /// Start the I/O processing tasks for reading from and writing to the child process.
    async fn start_io_tasks(
        &mut self,
        mut stdin: tokio::process::ChildStdin,
        stdout: tokio::process::ChildStdout,
        stderr: tokio::process::ChildStderr,
        inbound_sender: mpsc::UnboundedSender<JsonRpcMessage>,
        mut outbound_receiver: mpsc::UnboundedReceiver<JsonRpcMessage>,
        pending_requests: Arc<
            Mutex<HashMap<String, tokio::sync::oneshot::Sender<JsonRpcResponse>>>,
        >,
    ) {
        // Start stdout reader task
        let stdout_sender = inbound_sender.clone();
        let pending_requests_clone = pending_requests.clone();
        tokio::spawn(async move {
            let mut stdout_reader = BufReader::new(stdout);
            let mut line = String::new();

            loop {
                line.clear();
                match stdout_reader.read_line(&mut line).await {
                    Ok(0) => {
                        tracing::debug!("Child process stdout closed (EOF)");
                        break;
                    }
                    Ok(_) => {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            tracing::debug!("Received from stdout: {}", trimmed);
                            match serde_json::from_str::<JsonRpcMessage>(trimmed) {
                                Ok(message) => {
                                    // Handle response correlation for request/response messages
                                    if let JsonRpcMessage::Response(ref response) = message {
                                        let maybe_response_sender = pending_requests_clone
                                            .lock()
                                            .await
                                            .remove(&response.id.to_string());

                                        if let Some(response_sender) = maybe_response_sender {
                                            // Send response directly to the waiting request
                                            let _ = response_sender.send(response.clone());
                                            continue; // Don't send to inbound_sender for responses
                                        }
                                    }

                                    // Send other messages (notifications, server requests) to inbound_sender
                                    if stdout_sender.send(message).is_err() {
                                        tracing::warn!("Failed to send stdout message to handler");
                                        break;
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Failed to parse JSON message from stdout: {} ({})",
                                        e,
                                        trimmed
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error reading from stdout: {}", e);
                        break;
                    }
                }
            }
            tracing::debug!("Stdout reader task finished");
        });

        // Start stderr reader task
        tokio::spawn(async move {
            let mut stderr_reader = BufReader::new(stderr);
            let mut line = String::new();

            loop {
                line.clear();
                match stderr_reader.read_line(&mut line).await {
                    Ok(0) => {
                        tracing::debug!("Child process stderr closed (EOF)");
                        break;
                    }
                    Ok(_) => {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            tracing::warn!("MCP process stderr: {}", trimmed);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error reading from stderr: {}", e);
                        break;
                    }
                }
            }
            tracing::debug!("Stderr reader task finished");
        });

        // Start stdin writer task
        use tokio::io::AsyncWriteExt;
        tokio::spawn(async move {
            while let Some(message) = outbound_receiver.recv().await {
                match serde_json::to_string(&message) {
                    Ok(json_line) => {
                        let message_with_newline = format!("{}\n", json_line);
                        tracing::debug!("Sending to stdin: {}", json_line);

                        if let Err(e) = stdin.write_all(message_with_newline.as_bytes()).await {
                            tracing::error!("Failed to write to stdin: {}", e);
                            break;
                        }

                        if let Err(e) = stdin.flush().await {
                            tracing::error!("Failed to flush stdin: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to serialize outbound message: {}", e);
                    }
                }
            }
            tracing::debug!("Stdin writer task finished");
        });
    }

    /// Kill the child process if it exists.
    async fn kill_process(&mut self) -> McpResult<()> {
        if let Some(mut child) = self.child_process.take() {
            tracing::debug!("Terminating child process (PID: {:?})", child.id());

            // Try graceful shutdown first
            if let Err(e) = child.kill().await {
                tracing::warn!("Failed to kill child process: {}", e);
            }

            // Wait for the process to exit with a timeout
            let exit_timeout = Duration::from_secs(5);
            match timeout(exit_timeout, child.wait()).await {
                Ok(Ok(exit_status)) => {
                    tracing::debug!("Child process exited with status: {}", exit_status);
                }
                Ok(Err(e)) => {
                    tracing::warn!("Error waiting for child process to exit: {}", e);
                }
                Err(_) => {
                    tracing::warn!(
                        "Child process did not exit within timeout, may still be running"
                    );
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn connect(&mut self) -> McpResult<()> {
        tracing::info!("Connecting stdio transport");

        // Spawn the child process and set up communication
        self.spawn_process().await?;

        // Update transport info
        self.info.mark_connected();

        tracing::info!("Stdio transport connected successfully");
        Ok(())
    }

    async fn disconnect(&mut self) -> McpResult<()> {
        tracing::info!("Disconnecting stdio transport");

        // Close message channels
        self.message_sender = None;
        self.message_receiver = None;
        self.outbound_sender = None;
        self.outbound_receiver = None;

        // Kill the child process
        self.kill_process().await?;

        // Clear pending requests
        self.pending_requests.lock().await.clear();

        // Update transport info
        self.info.mark_disconnected();

        tracing::info!("Stdio transport disconnected");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.info.connected
            && self.child_process.is_some()
            && self.message_sender.is_some()
            && self.outbound_sender.is_some()
    }

    async fn send_request(
        &mut self,
        request: JsonRpcRequest,
        timeout_duration: Option<Duration>,
    ) -> McpResult<JsonRpcResponse> {
        if !self.is_connected() {
            return Err(TransportError::NotConnected {
                transport_type: "stdio".to_string(),
                reason: "Transport not connected".to_string(),
            }
            .into());
        }

        let request_id = request.id.clone();
        let (response_sender, response_receiver) = tokio::sync::oneshot::channel();

        // Store the response sender for correlation
        self.pending_requests
            .lock()
            .await
            .insert(request_id.to_string(), response_sender);

        // Send the request
        if let Some(sender) = &self.outbound_sender {
            sender.send(JsonRpcMessage::Request(request)).map_err(|_| {
                TransportError::ProcessError {
                    reason: "Failed to send request to child process".to_string(),
                }
            })?;
        }

        self.info.increment_requests_sent();

        // Wait for response with timeout
        let timeout_duration = timeout_duration.unwrap_or(Duration::from_secs(30));
        let response = timeout(timeout_duration, response_receiver)
            .await
            .map_err(|_| TransportError::TimeoutError {
                transport_type: "stdio".to_string(),
                reason: format!(
                    "Request {} timed out after {:?}",
                    request_id, timeout_duration
                ),
            })?
            .map_err(|_| TransportError::ProcessError {
                reason: "Response channel closed unexpectedly".to_string(),
            })?;

        self.info.increment_responses_received();
        Ok(response)
    }

    async fn send_notification(&mut self, notification: JsonRpcNotification) -> McpResult<()> {
        if !self.is_connected() {
            return Err(TransportError::NotConnected {
                transport_type: "stdio".to_string(),
                reason: "Transport not connected".to_string(),
            }
            .into());
        }

        if let Some(sender) = &self.outbound_sender {
            sender
                .send(JsonRpcMessage::Notification(notification))
                .map_err(|_| TransportError::ProcessError {
                    reason: "Failed to send notification to child process".to_string(),
                })?;
        }

        self.info.increment_notifications_sent();
        Ok(())
    }

    async fn receive_message(
        &mut self,
        timeout_duration: Option<Duration>,
    ) -> McpResult<JsonRpcMessage> {
        if !self.is_connected() {
            return Err(TransportError::NotConnected {
                transport_type: "stdio".to_string(),
                reason: "Transport not connected".to_string(),
            }
            .into());
        }

        let receiver =
            self.message_receiver
                .as_mut()
                .ok_or_else(|| TransportError::NotConnected {
                    transport_type: "stdio".to_string(),
                    reason: "Message receiver not available".to_string(),
                })?;

        let message = if let Some(timeout_duration) = timeout_duration {
            timeout(timeout_duration, receiver.recv())
                .await
                .map_err(|_| TransportError::TimeoutError {
                    transport_type: "stdio".to_string(),
                    reason: format!("Message receive timed out after {:?}", timeout_duration),
                })?
                .ok_or_else(|| TransportError::ProcessError {
                    reason: "Child process stdout closed".to_string(),
                })?
        } else {
            receiver
                .recv()
                .await
                .ok_or_else(|| TransportError::ProcessError {
                    reason: "Child process stdout closed".to_string(),
                })?
        };

        // Response correlation is now handled in the stdout reader task
        // This method now only handles notifications and server-to-client requests

        // Update statistics
        match &message {
            JsonRpcMessage::Request(_) => {
                // Server-to-client request - handled normally in stdio
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

        // Add stdio-specific metadata
        if let TransportConfig::Stdio(config) = &self.config {
            info.add_metadata("command", serde_json::json!(config.command));
            info.add_metadata("args", serde_json::json!(config.args));
            info.add_metadata("working_dir", serde_json::json!(config.working_dir));
            info.add_metadata("timeout", serde_json::json!(config.timeout.as_secs()));
            info.add_metadata(
                "environment_vars",
                serde_json::json!(config.environment.len()),
            );
        }

        // TODO: Figure out how to handle async here
        // info.add_metadata(
        //     "pending_requests",
        //     serde_json::json!(self.pending_requests.lock().await.len()),
        // );
        info.add_metadata(
            "has_process",
            serde_json::json!(self.child_process.is_some()),
        );

        if let Some(ref child) = self.child_process {
            info.add_metadata("process_id", serde_json::json!(child.id()));
        }

        info
    }

    fn get_config(&self) -> &TransportConfig {
        &self.config
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        // Ensure child process is cleaned up when transport is dropped
        if let Some(mut child) = self.child_process.take() {
            let _ = child.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::TransportConfig;

    #[test]
    fn test_stdio_transport_creation() {
        let config = TransportConfig::stdio("echo", &["hello".to_string()]);
        let transport = StdioTransport::new(config);

        assert_eq!(transport.get_info().transport_type, "stdio");
        assert!(!transport.is_connected());
    }

    #[test]
    fn test_transport_info_metadata() {
        let config = TransportConfig::stdio("python", &["-m".to_string(), "server".to_string()]);
        let transport = StdioTransport::new(config);

        let info = transport.get_info();
        assert!(info.metadata.contains_key("command"));
        assert!(info.metadata.contains_key("args"));
        assert!(info.metadata.contains_key("timeout"));
    }

    #[tokio::test]
    async fn test_process_spawn_failure() {
        let config = TransportConfig::stdio("nonexistent_command_12345", &[] as &[String]);
        let mut transport = StdioTransport::new(config);

        let result = transport.connect().await;
        assert!(result.is_err());
        assert!(!transport.is_connected());
    }

    #[test]
    fn test_drop_cleanup() {
        let config = TransportConfig::stdio("sleep", &["1".to_string()]);
        let transport = StdioTransport::new(config);

        // Test that Drop implementation doesn't panic
        drop(transport);
    }

    #[test]
    fn test_environment_variables() {
        let mut config = TransportConfig::stdio("echo", &["test".to_string()]);

        if let TransportConfig::Stdio(ref mut stdio_config) = config {
            stdio_config
                .environment
                .insert("TEST_VAR".to_string(), "test_value".to_string());
        }

        let transport = StdioTransport::new(config);
        let info = transport.get_info();

        assert_eq!(
            info.metadata.get("environment_vars").unwrap(),
            &serde_json::json!(1)
        );
    }
}
