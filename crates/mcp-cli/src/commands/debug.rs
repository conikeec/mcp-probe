//! Debug Command - Interactive MCP Protocol Debugging with TUI
//!
//! This command provides a comprehensive interactive debugging experience for MCP servers
//! using a rich terminal user interface (TUI) built with ratatui.

use crate::{cli::DebugArgs, tui::DebuggerApp};
use anyhow::Result;
use clap::Parser;
use mcp_probe_core::{
    client::McpClient,
    messages::{
        prompts::{ListPromptsRequest, ListPromptsResponse, Prompt},
        resources::{ListResourcesRequest, ListResourcesResponse, Resource},
        tools::{ListToolsRequest, ListToolsResponse, Tool},
        Implementation,
    },
    transport::TransportConfig,
    McpResult,
};

/// Extension trait to add higher-level methods to McpClient
trait McpClientExt {
    async fn list_tools(&mut self) -> McpResult<Vec<Tool>>;
    async fn list_resources(&mut self) -> McpResult<Vec<Resource>>;
    async fn list_prompts(&mut self) -> McpResult<Vec<Prompt>>;
}

impl McpClientExt for McpClient {
    async fn list_tools(&mut self) -> McpResult<Vec<Tool>> {
        let request = ListToolsRequest { cursor: None };
        let response = self.send_request("tools/list", request).await?;

        if let Some(result) = response.result {
            let list_response: ListToolsResponse = serde_json::from_value(result)?;
            Ok(list_response.tools)
        } else {
            Ok(Vec::new())
        }
    }

    async fn list_resources(&mut self) -> McpResult<Vec<Resource>> {
        let request = ListResourcesRequest { cursor: None };
        let response = self.send_request("resources/list", request).await?;

        if let Some(result) = response.result {
            let list_response: ListResourcesResponse = serde_json::from_value(result)?;
            Ok(list_response.resources)
        } else {
            Ok(Vec::new())
        }
    }

    async fn list_prompts(&mut self) -> McpResult<Vec<Prompt>> {
        let request = ListPromptsRequest { cursor: None };
        let response = self.send_request("prompts/list", request).await?;

        if let Some(result) = response.result {
            let list_response: ListPromptsResponse = serde_json::from_value(result)?;
            Ok(list_response.prompts)
        } else {
            Ok(Vec::new())
        }
    }
}

/// Debug MCP server with interactive TUI
#[derive(Parser, Debug)]
pub struct DebugCommand {
    /// Transport configuration
    #[command(flatten)]
    pub transport: crate::cli::TransportArgs,

    /// Configuration file to load
    #[arg(short, long)]
    pub config: Option<std::path::PathBuf>,

    /// Start in non-interactive mode
    #[arg(long)]
    pub non_interactive: bool,

    /// Show raw MCP protocol messages
    #[arg(long)]
    pub show_raw: bool,

    /// Save session to file
    #[arg(long)]
    pub save_session: Option<std::path::PathBuf>,

    /// Load and replay a previous session
    #[arg(long)]
    pub replay_session: Option<std::path::PathBuf>,

    /// Connection timeout in seconds
    #[arg(long, default_value = "30")]
    pub timeout: u64,

    /// Maximum number of retry attempts
    #[arg(long, default_value = "3")]
    pub max_retries: u32,
}

impl DebugCommand {
    /// Execute the debug command
    pub async fn execute(&self) -> Result<()> {
        // Create client info
        let client_info = Implementation {
            name: "mcp-probe".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            metadata: std::collections::HashMap::new(),
        };

        // Build transport configuration
        let transport_config = self.transport.to_transport_config()?;

        // Convert to DebugArgs structure
        let debug_args = DebugArgs {
            transport: self.transport.clone(),
            config: self.config.clone(),
            non_interactive: self.non_interactive,
            show_raw: self.show_raw,
            save_session: self.save_session.clone(),
            replay_session: self.replay_session.clone(),
            timeout: self.timeout,
            max_retries: self.max_retries,
        };

        if self.non_interactive {
            // Run in simple non-interactive mode
            self.run_non_interactive(transport_config, client_info)
                .await
        } else {
            // Launch the rich TUI experience
            self.run_interactive_tui(transport_config, client_info, debug_args)
                .await
        }
    }

    /// Run in non-interactive mode with simple output
    async fn run_non_interactive(
        &self,
        transport_config: TransportConfig,
        client_info: Implementation,
    ) -> Result<()> {
        println!("🔍 MCP Probe - Non-Interactive Debug Mode");
        println!("🔌 Transport: {}", transport_config.transport_type());
        println!("📡 Client: {} v{}", client_info.name, client_info.version);
        println!();

        // Create and connect client
        let mut client = mcp_probe_core::client::McpClient::with_defaults(transport_config).await?;
        let _server_info = client.connect(client_info).await?;

        println!("✅ Connected to MCP server successfully!");

        // List capabilities
        println!("\n🛠️  Server Capabilities:");

        match client.list_tools().await {
            Ok(tools) => {
                println!("📋 Tools ({}):", tools.len());
                for tool in tools {
                    println!("  → {} - {}", tool.name, tool.description);
                }
            }
            Err(e) => {
                println!("❌ Failed to list tools: {}", e);
            }
        }

        match client.list_resources().await {
            Ok(resources) => {
                println!("📁 Resources ({}):", resources.len());
                for resource in resources {
                    println!(
                        "  → {} - {}",
                        resource.uri,
                        resource.description.unwrap_or_default()
                    );
                }
            }
            Err(e) => {
                if e.to_string().contains("Method not found") {
                    println!("📁 Resources (0):");
                } else {
                    println!("❌ Failed to list resources: {}", e);
                }
            }
        }

        match client.list_prompts().await {
            Ok(prompts) => {
                println!("💬 Prompts ({}):", prompts.len());
                for prompt in prompts {
                    println!("  → {} - {}", prompt.name, prompt.description);
                }
            }
            Err(e) => {
                if e.to_string().contains("Method not found") {
                    println!("💬 Prompts (0):");
                } else {
                    println!("❌ Failed to list prompts: {}", e);
                }
            }
        }

        println!("\n✅ Debug session completed successfully!");
        Ok(())
    }

    /// Run the interactive TUI experience
    async fn run_interactive_tui(
        &self,
        transport_config: TransportConfig,
        client_info: Implementation,
        _debug_args: DebugArgs,
    ) -> Result<()> {
        // Create and run the TUI application
        let mut app = DebuggerApp::new(transport_config, client_info)?;
        app.run().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use url::Url;

    #[tokio::test]
    async fn test_debug_command_creation() {
        let cmd = DebugCommand {
            transport: crate::cli::TransportArgs {
                stdio: Some("test-server".to_string()),
                args: vec![],
                working_dir: None,
                http_sse: None,
                http_stream: None,
                auth_header: None,
                headers: vec![],
            },
            config: None,
            non_interactive: true,
            show_raw: false,
            save_session: None,
            replay_session: None,
            timeout: 30,
            max_retries: 3,
        };

        // Just verify the command structure is valid
        assert_eq!(cmd.timeout, 30);
        assert_eq!(cmd.max_retries, 3);
        assert!(cmd.non_interactive);
    }

    #[test]
    fn test_transport_config_conversion() {
        let transport_args = crate::cli::TransportArgs {
            stdio: Some("test-server".to_string()),
            args: vec!["--arg1".to_string(), "--arg2".to_string()],
            working_dir: Some(PathBuf::from("/tmp")),
            http_sse: None,
            http_stream: None,
            auth_header: None,
            headers: vec![],
        };

        let config = transport_args.to_transport_config().unwrap();
        match config {
            TransportConfig::Stdio(stdio_config) => {
                assert_eq!(stdio_config.command, "test-server");
                assert_eq!(stdio_config.args, vec!["--arg1", "--arg2"]);
            }
            _ => panic!("Expected Stdio transport config"),
        }
    }

    #[test]
    fn test_http_sse_transport_config() {
        let transport_args = crate::cli::TransportArgs {
            stdio: None,
            args: vec![],
            working_dir: None,
            http_sse: Some("http://localhost:3000".parse::<Url>().unwrap()),
            http_stream: None,
            auth_header: Some("Bearer token123".to_string()),
            headers: vec!["Content-Type=application/json".to_string()],
        };

        let config = transport_args.to_transport_config().unwrap();
        match config {
            TransportConfig::HttpSse(http_config) => {
                assert_eq!(http_config.base_url.to_string(), "http://localhost:3000/");
            }
            _ => panic!("Expected HttpSse transport config"),
        }
    }

    #[test]
    fn test_http_stream_transport_config() {
        let transport_args = crate::cli::TransportArgs {
            stdio: None,
            args: vec![],
            working_dir: None,
            http_sse: None,
            http_stream: Some("http://localhost:3000".parse::<Url>().unwrap()),
            auth_header: None,
            headers: vec![],
        };

        let config = transport_args.to_transport_config().unwrap();
        match config {
            TransportConfig::HttpStream(stream_config) => {
                assert_eq!(stream_config.base_url.to_string(), "http://localhost:3000/");
            }
            _ => panic!("Expected HttpStream transport config"),
        }
    }
}
