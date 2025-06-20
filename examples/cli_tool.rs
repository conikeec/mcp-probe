// CLI Tool Example using MCP Probe SDK
// This example shows how to build a command-line tool that interacts with MCP servers

use mcp_core::{
    client::McpClient,
    transport::{TransportConfig, TransportFactory},
    messages::initialization::Implementation,
    error::McpError,
};
use clap::{Parser, Subcommand, Args};
use serde_json::Value;
use std::collections::HashMap;
use tokio;

/// A CLI tool for interacting with MCP servers
#[derive(Parser)]
#[command(name = "mcp-cli-example")]
#[command(about = "A CLI tool demonstrating MCP SDK usage")]
#[command(version = "1.0.0")]
pub struct Cli {
    /// MCP server URL
    #[arg(short, long, env = "MCP_SERVER_URL", default_value = "http://localhost:3000/sse")]
    server: String,
    
    /// Authentication token (if required)
    #[arg(short, long, env = "MCP_AUTH_TOKEN")]
    auth_token: Option<String>,
    
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
    
    /// Request timeout in seconds
    #[arg(short, long, default_value = "30")]
    timeout: u64,
    
    /// Command to execute
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List server capabilities
    List {
        /// Type of capabilities to list
        #[command(subcommand)]
        capability_type: CapabilityType,
    },
    /// Call a tool
    Call(CallArgs),
    /// Read a resource
    Read {
        /// Resource URI to read
        uri: String,
        /// Output format
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,
    },
    /// Get a prompt
    Prompt {
        /// Prompt name
        name: String,
        /// Arguments as JSON string
        #[arg(short, long)]
        args: Option<String>,
    },
    /// Interactive mode
    Interactive,
}

#[derive(Subcommand)]
pub enum CapabilityType {
    /// List available tools
    Tools,
    /// List available resources
    Resources,
    /// List available prompts
    Prompts,
    /// List all capabilities
    All,
}

#[derive(Args)]
pub struct CallArgs {
    /// Tool name to call
    name: String,
    /// Parameters as JSON string
    #[arg(short, long)]
    params: Option<String>,
    /// Output format
    #[arg(short, long, default_value = "json")]
    format: OutputFormat,
    /// Save output to file
    #[arg(short, long)]
    output: Option<String>,
}

#[derive(clap::ValueEnum, Clone)]
pub enum OutputFormat {
    Json,
    Text,
    Pretty,
    Raw,
}

struct McpCliTool {
    client: McpClient,
    verbose: bool,
}

impl McpCliTool {
    async fn new(server_url: &str, auth_token: Option<String>, verbose: bool, timeout: u64) -> Result<Self, Box<dyn std::error::Error>> {
        if verbose {
            println!("🔗 Connecting to MCP server: {}", server_url);
        }
        
        // Create transport configuration
        let auth_header = auth_token.map(|token| {
            if token.starts_with("Bearer ") {
                token
            } else {
                format!("Bearer {}", token)
            }
        });
        
        let transport_config = TransportConfig::http_sse(server_url, auth_header)?;
        let transport = TransportFactory::create_transport(transport_config).await?;
        
        // Create client
        let client_info = Implementation {
            name: "mcp-cli-example".to_string(),
            version: "1.0.0".to_string(),
            metadata: HashMap::from([
                ("description".to_string(), Value::String("CLI tool using MCP SDK".to_string())),
                ("timeout".to_string(), Value::Number(timeout.into())),
            ]),
        };
        
        let mut client = McpClient::new(transport, client_info);
        
        // Initialize connection
        client.initialize().await?;
        
        if verbose {
            println!("✅ Successfully connected to MCP server");
        }
        
        Ok(Self { client, verbose })
    }
    
    async fn list_capabilities(&mut self, capability_type: CapabilityType) -> Result<(), Box<dyn std::error::Error>> {
        match capability_type {
            CapabilityType::Tools => self.list_tools().await?,
            CapabilityType::Resources => self.list_resources().await?,
            CapabilityType::Prompts => self.list_prompts().await?,
            CapabilityType::All => {
                self.list_tools().await?;
                println!();
                self.list_resources().await?;
                println!();
                self.list_prompts().await?;
            }
        }
        Ok(())
    }
    
    async fn list_tools(&mut self) -> Result<(), McpError> {
        if self.verbose {
            println!("🔧 Fetching available tools...");
        }
        
        let response = self.client.list_tools().await?;
        
        println!("Tools ({}):", response.tools.len());
        if response.tools.is_empty() {
            println!("  No tools available");
            return Ok(());
        }
        
        for tool in response.tools {
            println!("  📧 {}", tool.name);
            if let Some(description) = tool.description {
                println!("     {}", description);
            }
            
            // Show input schema if available
            if let Some(schema) = tool.input_schema {
                if self.verbose {
                    if let Some(properties) = schema.get("properties") {
                        println!("     Parameters:");
                        if let Some(props) = properties.as_object() {
                            for (param_name, param_schema) in props {
                                let param_type = param_schema.get("type")
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("unknown");
                                let required = schema.get("required")
                                    .and_then(|r| r.as_array())
                                    .map(|arr| arr.iter().any(|v| v.as_str() == Some(param_name)))
                                    .unwrap_or(false);
                                let marker = if required { "*" } else { " " };
                                println!("       {}{}: {}", marker, param_name, param_type);
                            }
                        }
                    }
                }
            }
            println!();
        }
        
        Ok(())
    }
    
    async fn list_resources(&mut self) -> Result<(), McpError> {
        if self.verbose {
            println!("📁 Fetching available resources...");
        }
        
        let response = self.client.list_resources().await?;
        
        println!("Resources ({}):", response.resources.len());
        if response.resources.is_empty() {
            println!("  No resources available");
            return Ok(());
        }
        
        for resource in response.resources {
            println!("  📄 {} ({})", resource.name, resource.uri);
            if let Some(description) = resource.description {
                println!("     {}", description);
            }
            if let Some(mime_type) = resource.mime_type {
                println!("     Type: {}", mime_type);
            }
            println!();
        }
        
        Ok(())
    }
    
    async fn list_prompts(&mut self) -> Result<(), McpError> {
        if self.verbose {
            println!("💬 Fetching available prompts...");
        }
        
        let response = self.client.list_prompts().await?;
        
        println!("Prompts ({}):", response.prompts.len());
        if response.prompts.is_empty() {
            println!("  No prompts available");
            return Ok(());
        }
        
        for prompt in response.prompts {
            println!("  💭 {}", prompt.name);
            if let Some(description) = prompt.description {
                println!("     {}", description);
            }
            
            if self.verbose && !prompt.arguments.is_empty() {
                println!("     Arguments:");
                for arg in prompt.arguments {
                    let required_marker = if arg.required { "*" } else { " " };
                    println!("       {}{}: {}", required_marker, arg.name, arg.description.unwrap_or_default());
                }
            }
            println!();
        }
        
        Ok(())
    }
    
    async fn call_tool(&mut self, args: CallArgs) -> Result<(), Box<dyn std::error::Error>> {
        if self.verbose {
            println!("🎯 Calling tool: {}", args.name);
        }
        
        // Parse parameters
        let params = if let Some(params_str) = args.params {
            serde_json::from_str(&params_str)?
        } else {
            Value::Null
        };
        
        if self.verbose && params != Value::Null {
            println!("📋 Parameters: {}", serde_json::to_string_pretty(&params)?);
        }
        
        // Call the tool
        let result = self.client.call_tool(&args.name, params).await?;
        
        // Format and display output
        let output = self.format_tool_result(&result.content, &args.format)?;
        
        if let Some(output_file) = args.output {
            tokio::fs::write(&output_file, &output).await?;
            println!("💾 Output saved to: {}", output_file);
        } else {
            println!("{}", output);
        }
        
        Ok(())
    }
    
    async fn read_resource(&mut self, uri: &str, format: &OutputFormat) -> Result<(), Box<dyn std::error::Error>> {
        if self.verbose {
            println!("📖 Reading resource: {}", uri);
        }
        
        let response = self.client.read_resource(uri).await?;
        
        for content in response.contents {
            match content {
                mcp_core::messages::resources::ResourceContent::Text { text, .. } => {
                    match format {
                        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&text)?),
                        OutputFormat::Pretty => {
                            println!("📄 Text Content:");
                            println!("{}", text);
                        }
                        _ => println!("{}", text),
                    }
                }
                mcp_core::messages::resources::ResourceContent::Blob { data, mime_type } => {
                    match format {
                        OutputFormat::Json => {
                            let json_obj = serde_json::json!({
                                "type": "blob",
                                "mime_type": mime_type,
                                "size": data.len()
                            });
                            println!("{}", serde_json::to_string_pretty(&json_obj)?);
                        }
                        _ => {
                            println!("🗂️  Binary Content ({}): {} bytes", mime_type, data.len());
                            println!("   Use --format json to see metadata or save to file");
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn get_prompt(&mut self, name: &str, args: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
        if self.verbose {
            println!("💭 Getting prompt: {}", name);
        }
        
        let arguments = if let Some(args_str) = args {
            Some(serde_json::from_str(&args_str)?)
        } else {
            None
        };
        
        let response = self.client.get_prompt(name, arguments).await?;
        
        println!("💬 Prompt: {}", name);
        if let Some(description) = response.description {
            println!("📄 Description: {}", description);
        }
        
        println!("\n📝 Content:");
        for message in response.messages {
            println!("🗣️  Role: {}", message.role);
            for content in message.content {
                match content {
                    mcp_core::messages::prompts::PromptContent::Text { text } => {
                        println!("{}", text);
                    }
                    mcp_core::messages::prompts::PromptContent::Image { data, mime_type } => {
                        println!("🖼️  [Image: {} bytes, type: {}]", data.len(), mime_type);
                    }
                    mcp_core::messages::prompts::PromptContent::Resource { resource } => {
                        println!("📄 [Resource: {}]", resource.uri);
                    }
                }
            }
            println!();
        }
        
        Ok(())
    }
    
    async fn interactive_mode(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🎮 Entering interactive mode. Type 'help' for commands or 'quit' to exit.");
        
        loop {
            print!("mcp> ");
            use std::io::Write;
            std::io::stdout().flush()?;
            
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let input = input.trim();
            
            if input.is_empty() {
                continue;
            }
            
            match input {
                "quit" | "exit" => {
                    println!("👋 Goodbye!");
                    break;
                }
                "help" => {
                    self.show_interactive_help();
                }
                "tools" => {
                    if let Err(e) = self.list_tools().await {
                        eprintln!("❌ Error listing tools: {}", e);
                    }
                }
                "resources" => {
                    if let Err(e) = self.list_resources().await {
                        eprintln!("❌ Error listing resources: {}", e);
                    }
                }
                "prompts" => {
                    if let Err(e) = self.list_prompts().await {
                        eprintln!("❌ Error listing prompts: {}", e);
                    }
                }
                cmd if cmd.starts_with("call ") => {
                    let parts: Vec<&str> = cmd.splitn(3, ' ').collect();
                    if parts.len() >= 2 {
                        let tool_name = parts[1];
                        let params = parts.get(2).unwrap_or(&"{}");
                        
                        match serde_json::from_str(params) {
                            Ok(parsed_params) => {
                                match self.client.call_tool(tool_name, parsed_params).await {
                                    Ok(result) => {
                                        if let Ok(formatted) = self.format_tool_result(&result.content, &OutputFormat::Pretty) {
                                            println!("{}", formatted);
                                        }
                                    }
                                    Err(e) => eprintln!("❌ Tool call failed: {}", e),
                                }
                            }
                            Err(e) => eprintln!("❌ Invalid JSON parameters: {}", e),
                        }
                    } else {
                        eprintln!("Usage: call <tool_name> [json_params]");
                    }
                }
                _ => {
                    eprintln!("❓ Unknown command: {}. Type 'help' for available commands.", input);
                }
            }
        }
        
        Ok(())
    }
    
    fn show_interactive_help(&self) {
        println!("📚 Available commands:");
        println!("  tools     - List available tools");
        println!("  resources - List available resources");
        println!("  prompts   - List available prompts");
        println!("  call <tool_name> [json_params] - Call a tool");
        println!("  help      - Show this help");
        println!("  quit/exit - Exit interactive mode");
        println!();
        println!("💡 Examples:");
        println!("  call add_numbers {{\"a\": 10, \"b\": 20}}");
        println!("  call list_files");
    }
    
    fn format_tool_result(&self, results: &[mcp_core::messages::tools::ToolResult], format: &OutputFormat) -> Result<String, Box<dyn std::error::Error>> {
        match format {
            OutputFormat::Json => Ok(serde_json::to_string_pretty(results)?),
            OutputFormat::Raw => {
                let mut output = String::new();
                for result in results {
                    match result {
                        mcp_core::messages::tools::ToolResult::Text { text } => {
                            output.push_str(text);
                            output.push('\n');
                        }
                        mcp_core::messages::tools::ToolResult::Image { data, mime_type } => {
                            output.push_str(&format!("[Image: {} bytes, {}]\n", data.len(), mime_type));
                        }
                    }
                }
                Ok(output)
            }
            OutputFormat::Pretty => {
                let mut output = String::new();
                output.push_str("✅ Tool execution successful:\n");
                for (i, result) in results.iter().enumerate() {
                    match result {
                        mcp_core::messages::tools::ToolResult::Text { text } => {
                            output.push_str(&format!("📄 Result {}: {}\n", i + 1, text));
                        }
                        mcp_core::messages::tools::ToolResult::Image { data, mime_type } => {
                            output.push_str(&format!("🖼️  Result {}: Image ({} bytes, {})\n", i + 1, data.len(), mime_type));
                        }
                    }
                }
                Ok(output)
            }
            OutputFormat::Text => {
                let mut output = String::new();
                for result in results {
                    match result {
                        mcp_core::messages::tools::ToolResult::Text { text } => {
                            output.push_str(text);
                            output.push('\n');
                        }
                        mcp_core::messages::tools::ToolResult::Image { .. } => {
                            output.push_str("[Binary Image Content]\n");
                        }
                    }
                }
                Ok(output)
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // Initialize logging if verbose
    if cli.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }
    
    // Create MCP client
    let mut tool = McpCliTool::new(&cli.server, cli.auth_token, cli.verbose, cli.timeout).await?;
    
    // Execute command
    match cli.command {
        Commands::List { capability_type } => {
            tool.list_capabilities(capability_type).await?;
        }
        Commands::Call(args) => {
            tool.call_tool(args).await?;
        }
        Commands::Read { uri, format } => {
            tool.read_resource(&uri, &format).await?;
        }
        Commands::Prompt { name, args } => {
            tool.get_prompt(&name, args).await?;
        }
        Commands::Interactive => {
            tool.interactive_mode().await?;
        }
    }
    
    Ok(())
} 