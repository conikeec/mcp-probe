//! Debug Command - Interactive MCP Protocol Negotiation
//!
//! This command provides an interactive debugging experience for MCP protocol
//! negotiation using our beautiful fluid DSL. It demonstrates the elegance
//! of composable flow construction while providing rich debugging capabilities.

use anyhow::Result;
use clap::Parser;
use std::time::Duration;
use tracing::{info, debug, warn};

use mcp_core::{
    messages::Implementation,
    transport::{TransportConfig, StdioConfig},
};

use crate::flows::{
    Connect, Initialize, WaitForResponse, ProcessCapabilities,
    SendNotification, TransitionTo, FlowStep, NegotiationState,
    FlowDurationExt, demo::run_all_demos,
};
use crate::utils::{print_banner, print_success, print_error, print_info};

/// Debug MCP protocol negotiation with interactive flow visualization
#[derive(Parser, Debug)]
pub struct DebugCommand {
    /// Transport type to use for connection
    #[clap(short, long, value_enum, default_value = "stdio")]
    pub transport: TransportType,
    
    /// Target server command (for stdio transport)
    #[clap(long, default_value = "mcp-server")]
    pub command: String,
    
    /// Server URL (for HTTP transports)
    #[clap(long)]
    pub url: Option<String>,
    
    /// Connection timeout in seconds
    #[clap(long, default_value = "30")]
    pub timeout: u64,
    
    /// Enable verbose debug output
    #[clap(short, long)]
    pub verbose: bool,
    
    /// Show DSL demonstration instead of actual debugging
    #[clap(long)]
    pub demo: bool,
    
    /// Use strict validation
    #[clap(long)]
    pub strict: bool,
    
    /// Maximum retry attempts
    #[clap(long, default_value = "3")]
    pub max_retries: u32,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum TransportType {
    Stdio,
    HttpSse,
    HttpStream,
}

impl DebugCommand {
    /// Execute the debug command with beautiful flow visualization
    pub async fn execute(&self) -> Result<()> {
        if self.demo {
            return self.run_dsl_demonstration().await;
        }
        
        print_banner("🔍 MCP Protocol Debug Session");
        
        if self.verbose {
            print_info("Debug mode enabled - showing detailed negotiation flow");
        }
        
        // Create elegant client info
        let client_info = Implementation {
            name: "mcp-probe".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            metadata: std::collections::HashMap::new(),
        };
        
        print_info(&format!("Client: {} v{}", client_info.name, client_info.version));
        
        // Build transport configuration
        let transport_config = self.build_transport_config()?;
        print_info(&format!("Transport: {:?}", self.transport));
        
        // Create our beautiful negotiation flow
        let flow = self.create_elegant_flow(client_info.clone());
        print_success("✨ Negotiation flow created with elegant DSL");
        
        // Execute the flow with rich debugging
        self.execute_flow_with_debugging(flow, transport_config).await
    }
    
    /// Run the DSL demonstration showcasing the beauty of our syntax
    async fn run_dsl_demonstration(&self) -> Result<()> {
        print_banner("🎨 MCP Negotiation Flow DSL - Bricolage Demonstration");
        print_info("Showcasing the elegant, fluid syntax of our negotiation DSL");
        println!();
        
        // Run all the beautiful demonstrations
        run_all_demos().await?;
        
        println!();
        print_success("🌟 DSL demonstration complete!");
        print_info("The beauty of bricolage - composing complex flows from simple, elegant parts");
        
        Ok(())
    }
    
    /// Create an elegant negotiation flow using our beautiful DSL
    fn create_elegant_flow(&self, client_info: Implementation) -> crate::flows::NegotiationFlow {
        let timeout_duration = self.timeout.secs();
        
        if self.strict {
            // Strict validation flow with enhanced checking
            FlowStep::chain(Connect::with_timeout(timeout_duration))
                .then(Initialize::with_client_info(client_info.clone()))
                .then(WaitForResponse::with_validation())
                .then(ProcessCapabilities::with_strict_validation())
                .then(SendNotification::initialized())
                .then(TransitionTo::ready_state())
                .with_retry_policy(crate::flows::RetryPolicy {
                    max_attempts: self.max_retries,
                    initial_delay: 1000.millis(),
                    max_delay: 30.secs(),
                    backoff_multiplier: 2.0,
                })
                .build(client_info)
        } else {
            // Standard flow with graceful handling
            FlowStep::chain(Connect::with_timeout(timeout_duration))
                .then(Initialize::with_client_info(client_info.clone()))
                .then(WaitForResponse::with_validation())
                .then(ProcessCapabilities::extract_all())
                .then(SendNotification::initialized())
                .then(TransitionTo::ready_state())
                .build(client_info)
        }
    }
    
    /// Execute the flow with rich debugging and visualization
    async fn execute_flow_with_debugging(
        &self, 
        mut flow: crate::flows::NegotiationFlow, 
        transport_config: TransportConfig
    ) -> Result<()> {
        print_info("🚀 Starting MCP negotiation flow execution");
        
        // Show initial state
        self.display_flow_state(flow.context());
        
        match flow.execute(transport_config).await {
            Ok(final_context) => {
                print_success("🎉 MCP negotiation completed successfully!");
                self.display_success_summary(final_context);
                self.display_timing_information(final_context);
                self.display_capabilities_summary(final_context);
                Ok(())
            }
            Err(error) => {
                print_error(&format!("❌ MCP negotiation failed: {}", error));
                self.display_error_context(flow.context());
                Err(error)
            }
        }
    }
    
    /// Display current flow state with beautiful formatting
    fn display_flow_state(&self, context: &crate::flows::FlowContext) {
        match &context.state {
            NegotiationState::Idle => {
                print_info("📍 State: Idle - Ready to begin negotiation");
            }
            NegotiationState::Connecting { transport_type } => {
                print_info(&format!("📍 State: Connecting via {}", transport_type));
            }
            NegotiationState::Initializing { protocol_version } => {
                print_info(&format!("📍 State: Initializing with protocol {}", protocol_version));
            }
            NegotiationState::Awaiting { timeout_remaining } => {
                print_info(&format!("📍 State: Awaiting response (timeout: {:?})", timeout_remaining));
            }
            NegotiationState::Processing { capability_count } => {
                print_info(&format!("📍 State: Processing {} capabilities", capability_count));
            }
            NegotiationState::Finalizing { notifications_pending } => {
                print_info(&format!("📍 State: Finalizing ({} notifications pending)", notifications_pending));
            }
            NegotiationState::Ready { session_id } => {
                print_success(&format!("📍 State: Ready! Session ID: {}", session_id));
            }
            NegotiationState::Retrying { attempt, reason } => {
                warn!("📍 State: Retrying (attempt {}) - {}", attempt, reason);
            }
            NegotiationState::Failed { reason } => {
                print_error(&format!("📍 State: Failed - {}", reason));
            }
        }
    }
    
    /// Display success summary with rich information
    fn display_success_summary(&self, context: &crate::flows::FlowContext) {
        println!("\n🎯 Negotiation Summary:");
        
        if let Some(server_info) = &context.server_info {
            print_info(&format!("   Server: {} v{}", server_info.name, server_info.version));
        }
        
        print_info(&format!("   Client: {} v{}", context.client_info.name, context.client_info.version));
        print_info(&format!("   Errors encountered: {}", context.errors.len()));
    }
    
    /// Display timing information for performance analysis
    fn display_timing_information(&self, context: &crate::flows::FlowContext) {
        println!("\n⏱️  Timing Analysis:");
        
        if let Some(connection_time) = context.timing.connection_time {
            print_info(&format!("   Connection: {:?}", connection_time));
        }
        
        if let Some(negotiation_time) = context.timing.negotiation_time {
            print_info(&format!("   Total negotiation: {:?}", negotiation_time));
        }
        
        if self.verbose {
            println!("   📊 Step-by-step timing:");
            for (step, duration) in &context.timing.step_timings {
                print_info(&format!("      {}: {:?}", step, duration));
            }
        }
    }
    
    /// Display capabilities summary
    fn display_capabilities_summary(&self, context: &crate::flows::FlowContext) {
        println!("\n🛠️  Server Capabilities:");
        
        let caps = &context.capabilities;
        
        if caps.tools.list_allowed {
            print_success("   ✅ Tools: List allowed");
            if caps.tools.execute_allowed {
                print_success("   ✅ Tools: Execute allowed");
            }
        }
        
        if caps.resources.list_allowed {
            print_success("   ✅ Resources: List allowed");
            if caps.resources.read_allowed {
                print_success("   ✅ Resources: Read allowed");
            }
        }
        
        if caps.prompts.list_allowed {
            print_success("   ✅ Prompts: List allowed");
            if caps.prompts.execute_allowed {
                print_success("   ✅ Prompts: Execute allowed");
            }
        }
        
        if caps.logging.enabled {
            print_success(&format!("   ✅ Logging: Enabled (levels: {:?})", caps.logging.levels));
        }
        
        if !caps.extensions.is_empty() {
            print_info(&format!("   🔧 Extensions: {} available", caps.extensions.len()));
        }
    }
    
    /// Display error context for debugging
    fn display_error_context(&self, context: &crate::flows::FlowContext) {
        println!("\n🔍 Error Context:");
        
        self.display_flow_state(context);
        
        if !context.errors.is_empty() {
            println!("   🚨 Errors encountered:");
            for error in &context.errors {
                print_error(&format!("      - {}", error));
            }
        }
        
        if self.verbose {
            println!("   📊 Debug information:");
            print_info(&format!("      Total execution time: {:?}", context.timing.total_time));
            print_info(&format!("      Steps completed: {}", context.timing.step_timings.len()));
        }
    }
    
    /// Build transport configuration based on command arguments
    fn build_transport_config(&self) -> Result<TransportConfig> {
        match self.transport {
            TransportType::Stdio => {
                Ok(TransportConfig::Stdio(mcp_core::transport::StdioConfig {
                    command: self.command.clone(),
                    args: vec![],
                    environment: std::collections::HashMap::new(),
                    working_dir: None,
                    timeout: Duration::from_secs(self.timeout),
                }))
            }
            TransportType::HttpSse => {
                let url = self.url.as_ref()
                    .ok_or_else(|| anyhow::anyhow!("URL required for HTTP+SSE transport"))?;
                Ok(TransportConfig::HttpSse(mcp_core::transport::HttpSseConfig {
                    base_url: url.parse()?,
                    timeout: Duration::from_secs(self.timeout),
                    auth: None,
                    headers: std::collections::HashMap::new(),
                }))
            }
            TransportType::HttpStream => {
                let url = self.url.as_ref()
                    .ok_or_else(|| anyhow::anyhow!("URL required for HTTP streaming transport"))?;
                Ok(TransportConfig::HttpStream(mcp_core::transport::HttpStreamConfig {
                    base_url: url.parse()?,
                    timeout: Duration::from_secs(self.timeout),
                    auth: None,
                    compression: false,
                    flow_control_window: 65536,
                    headers: std::collections::HashMap::new(),
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_debug_command_creation() {
        let cmd = DebugCommand {
            transport: TransportType::Stdio,
            command: "test-server".to_string(),
            url: None,
            timeout: 30,
            verbose: true,
            demo: false,
            strict: false,
            max_retries: 3,
        };
        
        let client_info = Implementation {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            metadata: std::collections::HashMap::new(),
        };
        
        let flow = cmd.create_elegant_flow(client_info);
        // Just check that flow is created successfully without panicking
        assert_eq!(flow.context().state, NegotiationState::Idle);
    }
    
    #[tokio::test]
    async fn test_demo_mode() {
        let cmd = DebugCommand {
            transport: TransportType::Stdio,
            command: "test".to_string(),
            url: None,
            timeout: 30,
            verbose: false,
            demo: true,
            strict: false,
            max_retries: 3,
        };
        
        // Demo should run without errors
        assert!(cmd.run_dsl_demonstration().await.is_ok());
    }
    
    #[test]
    fn test_transport_config_building() {
        let cmd = DebugCommand {
            transport: TransportType::Stdio,
            command: "test-server".to_string(),
            url: None,
            timeout: 30,
            verbose: false,
            demo: false,
            strict: false,
            max_retries: 3,
        };
        
        let config = cmd.build_transport_config().unwrap();
        match config {
            TransportConfig::Stdio(StdioConfig { command, .. }) => {
                assert_eq!(command, "test-server");
            }
            _ => panic!("Expected Stdio transport config"),
        }
    }
} 