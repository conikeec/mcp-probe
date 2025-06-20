//! MCP Negotiation Flow DSL - A Fluid Programming Interface
//!
//! This module implements a type-level DSL for composing MCP protocol negotiation
//! flows using elegant, pipe-like syntax. Inspired by shell scripting but designed
//! specifically for protocol state management and validation.

#![allow(dead_code)]

use anyhow::Result;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::time::timeout;

use mcp_core::{
    error::McpResult,
    messages::{
        Implementation, InitializeRequest, InitializeResponse, JsonRpcMessage, ProtocolVersion,
    },
    transport::{Transport, TransportConfig},
};

pub mod demo;
pub mod negotiation;
pub mod states;
pub mod transitions;

/// The main macro for defining MCP negotiation flows with elegant syntax
///
/// # Examples
///
/// ```rust,ignore
/// let flow = mcpflow! {
///     Connect::with_timeout(30.secs())
///         .then(Initialize::with_client_info(client_info))
///         .then(WaitForResponse::with_validation())
///         .then(ProcessCapabilities::extract_all())
///         .then(SendNotification::initialized())
///         .then(TransitionTo::ready_state())
/// };
/// ```
#[macro_export]
macro_rules! mcpflow {
    // Single step
    ($step:expr) => {
        $crate::flows::FlowStep::single($step)
    };

    // Multiple steps with then chaining
    ($first:expr $(, $rest:expr)*) => {
        $crate::flows::FlowStep::chain($first)$(.then($rest))*
    };

    // With conditional branching
    ($step:expr, $condition:expr, $($pattern:pat => $branch:expr),*) => {
        match $condition {
            $(
                $pattern => $branch,
            )*
        }
    };
}

/// Core negotiation flow builder with fluent interface
#[derive(Debug, Clone)]
pub struct NegotiationFlow {
    steps: Vec<FlowStepEnum>,
    context: FlowContext,
    config: FlowConfig,
}

/// Enum representing all possible flow steps
#[derive(Debug, Clone)]
pub enum FlowStepEnum {
    Connect(ConnectStep),
    Initialize(InitializeStep),
    WaitForResponse(WaitForResponseStep),
    ProcessCapabilities(ProcessCapabilitiesStep),
    SendNotification(SendNotificationStep),
    TransitionTo(TransitionToStep),
}

impl FlowStepEnum {
    /// Execute this flow step
    pub async fn execute(
        &self,
        context: &mut FlowContext,
        transport: &mut Box<dyn Transport>,
    ) -> McpResult<()> {
        match self {
            FlowStepEnum::Connect(step) => step.execute(context, transport).await,
            FlowStepEnum::Initialize(step) => step.execute(context, transport).await,
            FlowStepEnum::WaitForResponse(step) => step.execute(context, transport).await,
            FlowStepEnum::ProcessCapabilities(step) => step.execute(context, transport).await,
            FlowStepEnum::SendNotification(step) => step.execute(context, transport).await,
            FlowStepEnum::TransitionTo(step) => step.execute(context, transport).await,
        }
    }

    /// Get the name of this step for logging
    pub fn step_name(&self) -> &'static str {
        match self {
            FlowStepEnum::Connect(step) => step.step_name(),
            FlowStepEnum::Initialize(step) => step.step_name(),
            FlowStepEnum::WaitForResponse(step) => step.step_name(),
            FlowStepEnum::ProcessCapabilities(step) => step.step_name(),
            FlowStepEnum::SendNotification(step) => step.step_name(),
            FlowStepEnum::TransitionTo(step) => step.step_name(),
        }
    }

    /// Check if this step can be retried on failure
    pub fn is_retryable(&self) -> bool {
        match self {
            FlowStepEnum::Connect(step) => step.is_retryable(),
            FlowStepEnum::Initialize(step) => step.is_retryable(),
            FlowStepEnum::WaitForResponse(step) => step.is_retryable(),
            FlowStepEnum::ProcessCapabilities(step) => step.is_retryable(),
            FlowStepEnum::SendNotification(step) => step.is_retryable(),
            FlowStepEnum::TransitionTo(step) => step.is_retryable(),
        }
    }

    /// Get timeout for this specific step
    pub fn timeout(&self) -> Option<Duration> {
        match self {
            FlowStepEnum::Connect(step) => step.timeout(),
            FlowStepEnum::Initialize(step) => step.timeout(),
            FlowStepEnum::WaitForResponse(step) => step.timeout(),
            FlowStepEnum::ProcessCapabilities(step) => step.timeout(),
            FlowStepEnum::SendNotification(step) => step.timeout(),
            FlowStepEnum::TransitionTo(step) => step.timeout(),
        }
    }
}

/// Flow execution context tracking state and metadata
#[derive(Debug, Clone)]
pub struct FlowContext {
    pub state: NegotiationState,
    pub client_info: Implementation,
    pub server_info: Option<Implementation>,
    pub capabilities: CapabilitySet,
    pub timing: TimingInfo,
    pub errors: Vec<FlowError>,
    pub metadata: std::collections::HashMap<String, Value>,
}

/// Negotiation states with rich metadata
#[derive(Debug, Clone, PartialEq)]
pub enum NegotiationState {
    /// Initial state before connection
    Idle,
    /// Establishing transport connection
    Connecting { transport_type: String },
    /// Sending initialization request
    Initializing { protocol_version: String },
    /// Waiting for server response
    Awaiting { timeout_remaining: Duration },
    /// Processing server capabilities
    Processing { capability_count: usize },
    /// Sending final notifications
    Finalizing { notifications_pending: usize },
    /// Successfully ready for operations
    Ready { session_id: String },
    /// Recoverable error state
    Retrying { attempt: u32, reason: String },
    /// Terminal error state
    Failed { reason: String },
}

/// Capability tracking with granular information
#[derive(Debug, Clone, Default)]
pub struct CapabilitySet {
    pub tools: ToolCapabilities,
    pub resources: ResourceCapabilities,
    pub prompts: PromptCapabilities,
    pub logging: LoggingCapabilities,
    pub extensions: std::collections::HashMap<String, Value>,
}

#[derive(Debug, Clone, Default)]
pub struct ToolCapabilities {
    pub list_allowed: bool,
    pub execute_allowed: bool,
    pub available_tools: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ResourceCapabilities {
    pub list_allowed: bool,
    pub read_allowed: bool,
    pub available_resources: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PromptCapabilities {
    pub list_allowed: bool,
    pub execute_allowed: bool,
    pub available_prompts: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct LoggingCapabilities {
    pub enabled: bool,
    pub levels: Vec<String>,
}

/// Timing information for performance analysis
#[derive(Debug, Clone)]
pub struct TimingInfo {
    pub start_time: Instant,
    pub connection_time: Option<Duration>,
    pub negotiation_time: Option<Duration>,
    pub total_time: Duration,
    pub step_timings: std::collections::HashMap<String, Duration>,
}

/// Flow configuration with rich options
#[derive(Debug, Clone)]
pub struct FlowConfig {
    pub timeouts: TimeoutConfig,
    pub retry_policy: RetryPolicy,
    pub validation: ValidationConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    pub connection: Duration,
    pub initialization: Duration,
    pub response: Duration,
    pub total: Duration,
}

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
}

#[derive(Debug, Clone)]
pub struct ValidationConfig {
    pub strict_protocol_version: bool,
    pub require_capabilities: Vec<String>,
    pub allow_unknown_capabilities: bool,
}

#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub log_raw_messages: bool,
    pub log_timing: bool,
    pub log_state_transitions: bool,
}

/// Flow execution error with rich context
#[derive(Debug, Clone, thiserror::Error)]
pub enum FlowError {
    #[error("Transport error: {message}")]
    Transport { message: String },

    #[error("Protocol error: {message}")]
    Protocol { message: String },

    #[error("Timeout in step '{step}' after {duration:?}")]
    Timeout { step: String, duration: Duration },

    #[error("Validation failed: {reason}")]
    Validation { reason: String },

    #[error("Configuration error: {parameter} = {value}")]
    Configuration { parameter: String, value: String },
}

/// Trait for flow step handlers
pub trait FlowHandler: std::fmt::Debug + Send + Sync {
    /// Execute this flow step
    async fn execute(
        &self,
        context: &mut FlowContext,
        transport: &mut Box<dyn Transport>,
    ) -> McpResult<()>;

    /// Get the name of this step for logging
    fn step_name(&self) -> &'static str;

    /// Check if this step can be retried on failure
    fn is_retryable(&self) -> bool {
        true
    }

    /// Get timeout for this specific step
    fn timeout(&self) -> Option<Duration> {
        None
    }
}

/// Fluent builder for flow steps
pub struct FlowStep;

impl FlowStep {
    /// Create a single-step flow
    pub fn single(step: FlowStepEnum) -> FlowBuilder {
        FlowBuilder::new().add_step(step)
    }

    /// Create a multi-step flow with chaining
    pub fn chain(step: FlowStepEnum) -> FlowBuilder {
        Self::single(step)
    }
}

/// Builder for constructing complex flows
#[derive(Debug, Clone)]
pub struct FlowBuilder {
    steps: Vec<FlowStepEnum>,
    config: FlowConfig,
}

impl FlowBuilder {
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            config: FlowConfig::default(),
        }
    }

    /// Add a step to the flow
    pub fn add_step(mut self, step: FlowStepEnum) -> Self {
        self.steps.push(step);
        self
    }

    /// Chain another step with fluent syntax
    pub fn then(self, step: FlowStepEnum) -> Self {
        self.add_step(step)
    }

    /// Configure timeouts
    pub fn with_timeouts(mut self, timeouts: TimeoutConfig) -> Self {
        self.config.timeouts = timeouts;
        self
    }

    /// Configure retry policy
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.config.retry_policy = policy;
        self
    }

    /// Build the final negotiation flow
    pub fn build(self, client_info: Implementation) -> NegotiationFlow {
        let context = FlowContext {
            state: NegotiationState::Idle,
            client_info,
            server_info: None,
            capabilities: CapabilitySet::default(),
            timing: TimingInfo {
                start_time: Instant::now(),
                connection_time: None,
                negotiation_time: None,
                total_time: Duration::ZERO,
                step_timings: std::collections::HashMap::new(),
            },
            errors: Vec::new(),
            metadata: std::collections::HashMap::new(),
        };

        NegotiationFlow {
            steps: self.steps,
            context,
            config: self.config,
        }
    }
}

/// Convenient flow step constructors with fluent interfaces
pub struct Connect;

impl Connect {
    pub fn with_timeout(timeout: Duration) -> FlowStepEnum {
        FlowStepEnum::Connect(ConnectStep { timeout })
    }

    pub fn with_default_timeout() -> FlowStepEnum {
        FlowStepEnum::Connect(ConnectStep {
            timeout: Duration::from_secs(30),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ConnectStep {
    timeout: Duration,
}

impl FlowHandler for ConnectStep {
    async fn execute(
        &self,
        context: &mut FlowContext,
        transport: &mut Box<dyn Transport>,
    ) -> McpResult<()> {
        let step_start = Instant::now();
        context.state = NegotiationState::Connecting {
            transport_type: "unknown".to_string(), // TODO: Get from transport
        };

        timeout(self.timeout, transport.connect())
            .await
            .map_err(|_| {
                mcp_core::error::McpError::Transport(
                    mcp_core::error::TransportError::ConnectionFailed {
                        transport_type: "generic".to_string(),
                        reason: format!("Timeout after {:?}", self.timeout),
                    },
                )
            })??;

        context.timing.connection_time = Some(step_start.elapsed());
        context
            .timing
            .step_timings
            .insert("connect".to_string(), step_start.elapsed());

        tracing::info!("Transport connected in {:?}", step_start.elapsed());
        Ok(())
    }

    fn step_name(&self) -> &'static str {
        "connect"
    }
    fn timeout(&self) -> Option<Duration> {
        Some(self.timeout)
    }
}

pub struct Initialize;

impl Initialize {
    pub fn with_client_info(client_info: Implementation) -> FlowStepEnum {
        FlowStepEnum::Initialize(InitializeStep {
            client_info,
            protocol_version: ProtocolVersion::Custom("2024-11-05".to_string()),
        })
    }
}

#[derive(Debug, Clone)]
pub struct InitializeStep {
    client_info: Implementation,
    protocol_version: ProtocolVersion,
}

impl FlowHandler for InitializeStep {
    async fn execute(
        &self,
        context: &mut FlowContext,
        transport: &mut Box<dyn Transport>,
    ) -> McpResult<()> {
        let step_start = Instant::now();
        context.state = NegotiationState::Initializing {
            protocol_version: self.protocol_version.to_string(),
        };

        let request = InitializeRequest {
            protocol_version: self.protocol_version.clone(),
            capabilities: mcp_core::messages::Capabilities::default(),
            client_info: self.client_info.clone(),
        };

        let json_request = mcp_core::messages::JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: mcp_core::messages::JsonRpcId::String("init_1".to_string()),
            method: "initialize".to_string(),
            params: Some(serde_json::to_value(request)?),
        };

        transport
            .send_request(json_request, Some(Duration::from_secs(30)))
            .await?;

        context
            .timing
            .step_timings
            .insert("initialize".to_string(), step_start.elapsed());
        tracing::info!("Initialize request sent in {:?}", step_start.elapsed());
        Ok(())
    }

    fn step_name(&self) -> &'static str {
        "initialize"
    }
}

pub struct WaitForResponse;

impl WaitForResponse {
    pub fn with_timeout(timeout: Duration) -> FlowStepEnum {
        FlowStepEnum::WaitForResponse(WaitForResponseStep {
            timeout,
            validate_response: true,
        })
    }

    pub fn with_validation() -> FlowStepEnum {
        FlowStepEnum::WaitForResponse(WaitForResponseStep {
            timeout: Duration::from_secs(30),
            validate_response: true,
        })
    }
}

#[derive(Debug, Clone)]
pub struct WaitForResponseStep {
    timeout: Duration,
    validate_response: bool,
}

impl FlowHandler for WaitForResponseStep {
    async fn execute(
        &self,
        context: &mut FlowContext,
        transport: &mut Box<dyn Transport>,
    ) -> McpResult<()> {
        let step_start = Instant::now();
        context.state = NegotiationState::Awaiting {
            timeout_remaining: self.timeout,
        };

        let message = timeout(self.timeout, transport.receive_message(Some(self.timeout)))
            .await
            .map_err(|_| {
                mcp_core::error::McpError::Transport(
                    mcp_core::error::TransportError::ConnectionFailed {
                        transport_type: "generic".to_string(),
                        reason: format!("Receive timeout after {:?}", self.timeout),
                    },
                )
            })??;

        match message {
            JsonRpcMessage::Response(response) => {
                if let Some(result) = response.result {
                    let init_response: InitializeResponse = serde_json::from_value(result)?;

                    // Store server info in context
                    context.server_info = Some(init_response.server_info.clone());

                    // Store capabilities for next step
                    context.metadata.insert(
                        "init_response".to_string(),
                        serde_json::to_value(init_response)?,
                    );

                    context
                        .timing
                        .step_timings
                        .insert("wait_response".to_string(), step_start.elapsed());
                    tracing::info!("Received initialize response in {:?}", step_start.elapsed());
                    Ok(())
                } else {
                    Err(mcp_core::error::McpError::Protocol(
                        mcp_core::error::ProtocolError::InvalidResponse {
                            reason: "Empty response from server".to_string(),
                        },
                    ))
                }
            }
            _ => Err(mcp_core::error::McpError::Protocol(
                mcp_core::error::ProtocolError::UnexpectedMessageType {
                    expected: "response".to_string(),
                    actual: "other".to_string(),
                },
            )),
        }
    }

    fn step_name(&self) -> &'static str {
        "wait_response"
    }
    fn timeout(&self) -> Option<Duration> {
        Some(self.timeout)
    }
}

pub struct ProcessCapabilities;

impl ProcessCapabilities {
    pub fn extract_all() -> FlowStepEnum {
        FlowStepEnum::ProcessCapabilities(ProcessCapabilitiesStep {
            strict_validation: false,
        })
    }

    pub fn with_strict_validation() -> FlowStepEnum {
        FlowStepEnum::ProcessCapabilities(ProcessCapabilitiesStep {
            strict_validation: true,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ProcessCapabilitiesStep {
    strict_validation: bool,
}

impl FlowHandler for ProcessCapabilitiesStep {
    async fn execute(
        &self,
        context: &mut FlowContext,
        _transport: &mut Box<dyn Transport>,
    ) -> McpResult<()> {
        let step_start = Instant::now();

        if let Some(response_value) = context.metadata.get("init_response") {
            let response: InitializeResponse = serde_json::from_value(response_value.clone())?;

            // Extract and process capabilities
            let capabilities_value = serde_json::to_value(&response.capabilities)?;
            let capability_count = if let Some(obj) = capabilities_value.as_object() {
                obj.len()
            } else {
                0
            };

            context.state = NegotiationState::Processing { capability_count };

            // Process each capability type
            if let Some(tools) = capabilities_value.get("tools") {
                if let Some(tools_obj) = tools.as_object() {
                    context.capabilities.tools.list_allowed = tools_obj
                        .get("list_allowed")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                }
            }

            // Similar processing for resources, prompts, logging...

            context
                .timing
                .step_timings
                .insert("process_capabilities".to_string(), step_start.elapsed());
            tracing::info!(
                "Processed {} capabilities in {:?}",
                capability_count,
                step_start.elapsed()
            );
            Ok(())
        } else {
            Err(mcp_core::error::McpError::Protocol(
                mcp_core::error::ProtocolError::InvalidResponse {
                    reason: "No initialize response found in context".to_string(),
                },
            ))
        }
    }

    fn step_name(&self) -> &'static str {
        "process_capabilities"
    }
}

pub struct SendNotification;

impl SendNotification {
    pub fn initialized() -> FlowStepEnum {
        FlowStepEnum::SendNotification(SendNotificationStep {
            notification_type: "initialized".to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct SendNotificationStep {
    notification_type: String,
}

impl FlowHandler for SendNotificationStep {
    async fn execute(
        &self,
        context: &mut FlowContext,
        transport: &mut Box<dyn Transport>,
    ) -> McpResult<()> {
        let step_start = Instant::now();
        context.state = NegotiationState::Finalizing {
            notifications_pending: 1,
        };

        let notification = mcp_core::messages::JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: self.notification_type.clone(),
            params: Some(serde_json::json!({})),
        };

        transport.send_notification(notification).await?;

        context
            .timing
            .step_timings
            .insert("send_notification".to_string(), step_start.elapsed());
        tracing::info!(
            "Sent {} notification in {:?}",
            self.notification_type,
            step_start.elapsed()
        );
        Ok(())
    }

    fn step_name(&self) -> &'static str {
        "send_notification"
    }
}

pub struct TransitionTo;

impl TransitionTo {
    pub fn ready_state() -> FlowStepEnum {
        FlowStepEnum::TransitionTo(TransitionToStep {
            target_state: "ready".to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct TransitionToStep {
    target_state: String,
}

impl FlowHandler for TransitionToStep {
    async fn execute(
        &self,
        context: &mut FlowContext,
        _transport: &mut Box<dyn Transport>,
    ) -> McpResult<()> {
        let step_start = Instant::now();

        let session_id = uuid::Uuid::new_v4().to_string();
        context.state = NegotiationState::Ready { session_id };

        context.timing.total_time = context.timing.start_time.elapsed();
        context.timing.negotiation_time = Some(context.timing.total_time);

        context
            .timing
            .step_timings
            .insert("transition_ready".to_string(), step_start.elapsed());
        tracing::info!("Transitioned to ready state in {:?}", step_start.elapsed());
        tracing::info!(
            "Total negotiation completed in {:?}",
            context.timing.total_time
        );
        Ok(())
    }

    fn step_name(&self) -> &'static str {
        "transition_ready"
    }
}

/// Implementation of the main negotiation flow
impl NegotiationFlow {
    /// Execute the complete flow with error handling and retries
    pub async fn execute(&mut self, transport_config: TransportConfig) -> Result<&FlowContext> {
        let mut transport =
            mcp_core::transport::factory::TransportFactory::create(transport_config).await?;

        for (index, step) in self.steps.iter().enumerate() {
            let mut attempts = 0;
            let max_attempts = self.config.retry_policy.max_attempts;

            loop {
                attempts += 1;

                match step.execute(&mut self.context, &mut transport).await {
                    Ok(()) => {
                        tracing::debug!(
                            "Step {} '{}' completed successfully",
                            index,
                            step.step_name()
                        );
                        break;
                    }
                    Err(error) => {
                        tracing::warn!(
                            "Step {} '{}' failed (attempt {}): {}",
                            index,
                            step.step_name(),
                            attempts,
                            error
                        );

                        if attempts >= max_attempts || !step.is_retryable() {
                            self.context.state = NegotiationState::Failed {
                                reason: format!(
                                    "Step '{}' failed after {} attempts: {}",
                                    step.step_name(),
                                    attempts,
                                    error
                                ),
                            };
                            return Err(error.into());
                        }

                        // Calculate backoff delay
                        let delay = Duration::from_millis(
                            (self.config.retry_policy.initial_delay.as_millis() as f64
                                * self
                                    .config
                                    .retry_policy
                                    .backoff_multiplier
                                    .powi((attempts - 1) as i32))
                                as u64,
                        );
                        let delay = delay.min(self.config.retry_policy.max_delay);

                        tracing::info!("Retrying step '{}' in {:?}", step.step_name(), delay);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Ok(&self.context)
    }

    /// Get current flow context
    pub fn context(&self) -> &FlowContext {
        &self.context
    }
}

/// Default configurations
impl Default for FlowConfig {
    fn default() -> Self {
        Self {
            timeouts: TimeoutConfig {
                connection: Duration::from_secs(30),
                initialization: Duration::from_secs(60),
                response: Duration::from_secs(30),
                total: Duration::from_secs(300),
            },
            retry_policy: RetryPolicy {
                max_attempts: 3,
                initial_delay: Duration::from_millis(1000),
                max_delay: Duration::from_secs(30),
                backoff_multiplier: 2.0,
            },
            validation: ValidationConfig {
                strict_protocol_version: false,
                require_capabilities: vec![],
                allow_unknown_capabilities: true,
            },
            logging: LoggingConfig {
                log_raw_messages: false,
                log_timing: true,
                log_state_transitions: true,
            },
        }
    }
}

/// Fluent helper functions with method chaining
pub trait FlowDurationExt {
    fn secs(self) -> Duration;
    fn millis(self) -> Duration;
}

impl FlowDurationExt for u64 {
    fn secs(self) -> Duration {
        Duration::from_secs(self)
    }
    fn millis(self) -> Duration {
        Duration::from_millis(self)
    }
}

/// Helper macro for conditional flow execution
#[macro_export]
macro_rules! flow_if {
    ($condition:expr => $branch:expr) => {
        if $condition {
            $branch
        } else {
            Ok(())
        }
    };
}

/// Helper macro for branching flows based on condition
#[macro_export]
macro_rules! flow_branch {
    ($step:expr, $condition:expr, $true_branch:expr, $false_branch:expr) => {
        if $condition {
            $true_branch
        } else {
            $false_branch
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_flow_builder() {
        let client_info = Implementation {
            name: "test-client".to_string(),
            version: "1.0.0".to_string(),
            metadata: std::collections::HashMap::new(),
        };

        let flow = FlowStep::chain(Connect::with_default_timeout())
            .then(Initialize::with_client_info(client_info.clone()))
            .then(WaitForResponse::with_validation())
            .then(ProcessCapabilities::extract_all())
            .then(SendNotification::initialized())
            .then(TransitionTo::ready_state())
            .build(client_info);

        assert_eq!(flow.steps.len(), 6);
        assert_eq!(flow.context.state, NegotiationState::Idle);
    }

    #[test]
    fn test_duration_extensions() {
        assert_eq!(30.secs(), Duration::from_secs(30));
        assert_eq!(500.millis(), Duration::from_millis(500));
    }

    #[test]
    fn test_capability_processing() {
        let capabilities = CapabilitySet::default();
        assert!(!capabilities.tools.list_allowed);
        assert_eq!(capabilities.tools.available_tools.len(), 0);
    }
}
