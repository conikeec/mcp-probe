# 🛠️ MCP Probe SDK Documentation

**Build powerful MCP (Model Context Protocol) applications with the mcp-core SDK**

The MCP Probe project provides `mcp-core`, a robust Rust library for implementing MCP clients and servers. This guide shows you how to leverage the SDK to build your own MCP applications.

## 📦 Installation

Add `mcp-core` to your `Cargo.toml`:

```toml
[dependencies]
mcp-core = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
```

## 🚀 Quick Start

### Basic MCP Client

```rust
use mcp_core::{
    client::McpClient,
    transport::{TransportConfig, TransportFactory},
    messages::initialization::Implementation,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create transport configuration
    let transport_config = TransportConfig::http_sse(
        "http://localhost:3000/sse",
        None // No authentication
    )?;

    // 2. Create transport
    let transport = TransportFactory::create_transport(transport_config).await?;

    // 3. Create client info
    let client_info = Implementation {
        name: "my-mcp-app".to_string(),
        version: "1.0.0".to_string(),
        metadata: HashMap::new(),
    };

    // 4. Initialize MCP client
    let mut client = McpClient::new(transport, client_info);

    // 5. Connect and initialize
    client.initialize().await?;

    // 6. Discover available tools
    let tools = client.list_tools().await?;
    println!("Found {} tools", tools.tools.len());

    // 7. Call a tool
    let result = client.call_tool(
        "add_numbers",
        serde_json::json!({
            "a": 10,
            "b": 20
        })
    ).await?;

    println!("Result: {:?}", result);

    Ok(())
}
```

## 🔧 Core Components

### 1. Transport Layer

The SDK supports multiple transport mechanisms:

#### HTTP Server-Sent Events (Recommended)

```rust
use mcp_core::transport::{TransportConfig, http_sse::HttpSseConfig};

// Basic HTTP SSE
let config = TransportConfig::http_sse("http://localhost:3000/sse", None)?;

// With authentication
let config = TransportConfig::http_sse(
    "https://api.example.com/mcp",
    Some("Bearer your-token".to_string())
)?;

// Advanced configuration
let sse_config = HttpSseConfig {
    base_url: "http://localhost:3000".to_string(),
    endpoint: "/sse".to_string(),
    auth_header: Some("Bearer token".to_string()),
    timeout: Duration::from_secs(30),
    max_retries: 3,
    headers: vec![
        ("User-Agent".to_string(), "MyApp/1.0".to_string()),
    ],
};
let config = TransportConfig::HttpSse(sse_config);
```

#### STDIO Transport

```rust
use mcp_core::transport::{TransportConfig, stdio::StdioConfig};

// Basic STDIO
let config = TransportConfig::stdio("python", &["server.py"])?;

// Advanced STDIO with environment
let stdio_config = StdioConfig {
    command: "python".to_string(),
    args: vec!["-m".to_string(), "my_mcp_server".to_string()],
    working_dir: Some("/path/to/server".into()),
    env_vars: HashMap::from([
        ("API_KEY".to_string(), "secret".to_string()),
    ]),
};
let config = TransportConfig::Stdio(stdio_config);
```

#### HTTP Streaming

```rust
use mcp_core::transport::{TransportConfig, http_stream::HttpStreamConfig};

let stream_config = HttpStreamConfig {
    base_url: "http://localhost:3000".to_string(),
    endpoint: "/stream".to_string(),
    auth_header: None,
    timeout: Duration::from_secs(30),
    max_retries: 3,
};
let config = TransportConfig::HttpStream(stream_config);
```

### 2. MCP Client

#### Complete Client Example

```rust
use mcp_core::{
    client::{McpClient, ClientConfig},
    transport::TransportFactory,
    messages::{
        initialization::Implementation,
        tools::{CallToolRequest, ToolResult},
        resources::ListResourcesRequest,
        prompts::ListPromptsRequest,
    },
    error::McpError,
};

pub struct MyMcpApp {
    client: McpClient,
}

impl MyMcpApp {
    pub async fn new(server_url: &str) -> Result<Self, McpError> {
        // Configure transport
        let transport_config = TransportConfig::http_sse(server_url, None)?;
        let transport = TransportFactory::create_transport(transport_config).await?;

        // Create client with custom configuration
        let client_info = Implementation {
            name: "my-mcp-app".to_string(),
            version: "1.0.0".to_string(),
            metadata: std::collections::HashMap::from([
                ("author".to_string(), serde_json::Value::String("Your Name".to_string())),
                ("description".to_string(), serde_json::Value::String("My MCP Application".to_string())),
            ]),
        };

        let config = ClientConfig {
            timeout: Duration::from_secs(30),
            max_retries: 3,
            enable_logging: true,
        };

        let mut client = McpClient::with_config(transport, client_info, config);

        // Initialize connection
        client.initialize().await?;

        Ok(Self { client })
    }

    /// Discover all server capabilities
    pub async fn discover_capabilities(&mut self) -> Result<(), McpError> {
        // List tools
        let tools_response = self.client.list_tools().await?;
        println!("📧 Tools available: {}", tools_response.tools.len());
        for tool in &tools_response.tools {
            println!("  • {} - {}", tool.name, tool.description.as_deref().unwrap_or("No description"));
        }

        // List resources
        let resources_response = self.client.list_resources().await?;
        println!("📁 Resources available: {}", resources_response.resources.len());
        for resource in &resources_response.resources {
            println!("  • {} - {}", resource.name, resource.description.as_deref().unwrap_or("No description"));
        }

        // List prompts
        let prompts_response = self.client.list_prompts().await?;
        println!("💬 Prompts available: {}", prompts_response.prompts.len());
        for prompt in &prompts_response.prompts {
            println!("  • {} - {}", prompt.name, prompt.description.as_deref().unwrap_or("No description"));
        }

        Ok(())
    }

    /// Execute a tool with parameters
    pub async fn execute_tool(&mut self, tool_name: &str, params: serde_json::Value) -> Result<Vec<ToolResult>, McpError> {
        let response = self.client.call_tool(tool_name, params).await?;
        Ok(response.content)
    }

    /// Read a resource
    pub async fn read_resource(&mut self, resource_uri: &str) -> Result<String, McpError> {
        let response = self.client.read_resource(resource_uri).await?;

        // Extract text content from the first result
        if let Some(content) = response.contents.first() {
            match content {
                mcp_core::messages::resources::ResourceContent::Text { text, .. } => {
                    Ok(text.clone())
                }
                mcp_core::messages::resources::ResourceContent::Blob { .. } => {
                    Err(McpError::InvalidResponse("Resource contains binary data".to_string()))
                }
            }
        } else {
            Err(McpError::InvalidResponse("No content in resource response".to_string()))
        }
    }

    /// Get a prompt with arguments
    pub async fn get_prompt(&mut self, prompt_name: &str, args: Option<serde_json::Value>) -> Result<String, McpError> {
        let response = self.client.get_prompt(prompt_name, args).await?;

        // Combine all messages into a single string
        let content = response.messages.iter()
            .map(|msg| {
                msg.content.iter()
                    .filter_map(|content| {
                        match content {
                            mcp_core::messages::prompts::PromptContent::Text { text } => Some(text.as_str()),
                            _ => None,
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        Ok(content)
    }
}

// Usage example
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = MyMcpApp::new("http://localhost:3000/sse").await?;

    // Discover what's available
    app.discover_capabilities().await?;

    // Execute a tool
    let result = app.execute_tool(
        "calculate",
        serde_json::json!({
            "operation": "add",
            "a": 15,
            "b": 25
        })
    ).await?;

    println!("Calculation result: {:?}", result);

    // Read a resource
    let readme = app.read_resource("file://README.md").await?;
    println!("README content: {}", readme);

    // Get a prompt
    let prompt_text = app.get_prompt(
        "code_review",
        Some(serde_json::json!({
            "language": "rust",
            "style": "detailed"
        }))
    ).await?;

    println!("Code review prompt: {}", prompt_text);

    Ok(())
}
```

## 🌟 Advanced Usage

### Custom Transport Implementation

Create your own transport for specific protocols:

```rust
use mcp_core::transport::{Transport, TransportInfo};
use async_trait::async_trait;

pub struct CustomTransport {
    // Your transport fields
}

#[async_trait]
impl Transport for CustomTransport {
    async fn send(&mut self, message: &str) -> Result<(), mcp_core::error::McpError> {
        // Implement message sending
        todo!()
    }

    async fn receive(&mut self) -> Result<String, mcp_core::error::McpError> {
        // Implement message receiving
        todo!()
    }

    fn info(&self) -> TransportInfo {
        TransportInfo {
            transport_type: "custom".to_string(),
            connection_string: "custom://connection".to_string(),
            // ... other fields
        }
    }
}
```

### Error Handling Patterns

```rust
use mcp_core::error::{McpError, TransportError};

async fn robust_mcp_client() -> Result<(), McpError> {
    let mut retries = 3;

    loop {
        match connect_and_use_client().await {
            Ok(result) => return Ok(result),
            Err(McpError::Transport(TransportError::ConnectionFailed(_))) if retries > 0 => {
                retries -= 1;
                println!("Connection failed, retrying... ({} attempts left)", retries);
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
            Err(McpError::Transport(TransportError::Timeout)) => {
                return Err(McpError::InvalidResponse("Server timeout".to_string()));
            }
            Err(e) => return Err(e),
        }
    }
}

async fn connect_and_use_client() -> Result<(), McpError> {
    // Your client logic here
    Ok(())
}
```

### Session Management

```rust
use mcp_core::client::McpClient;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct SessionData {
    pub server_url: String,
    pub session_id: Option<String>,
    pub capabilities: Option<serde_json::Value>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

pub struct SessionManager {
    session_file: std::path::PathBuf,
}

impl SessionManager {
    pub fn new(session_file: impl Into<std::path::PathBuf>) -> Self {
        Self {
            session_file: session_file.into(),
        }
    }

    pub async fn save_session(&self, client: &McpClient) -> Result<(), Box<dyn std::error::Error>> {
        let session_data = SessionData {
            server_url: client.transport_info().connection_string.clone(),
            session_id: client.session_id().map(|s| s.to_string()),
            capabilities: None, // Store discovered capabilities if needed
            last_activity: chrono::Utc::now(),
        };

        let json = serde_json::to_string_pretty(&session_data)?;
        tokio::fs::write(&self.session_file, json).await?;

        Ok(())
    }

    pub async fn load_session(&self) -> Result<Option<SessionData>, Box<dyn std::error::Error>> {
        if !self.session_file.exists() {
            return Ok(None);
        }

        let content = tokio::fs::read_to_string(&self.session_file).await?;
        let session_data: SessionData = serde_json::from_str(&content)?;

        Ok(Some(session_data))
    }
}
```

## 🔌 Integration Patterns

### Web Application Integration

```rust
// Using with Axum web framework
use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    mcp_client: Arc<Mutex<McpClient>>,
}

#[derive(Deserialize)]
pub struct ToolCallRequest {
    tool_name: String,
    parameters: serde_json::Value,
}

#[derive(Serialize)]
pub struct ToolCallResponse {
    success: bool,
    result: Option<serde_json::Value>,
    error: Option<String>,
}

async fn call_tool_endpoint(
    State(state): State<AppState>,
    Json(request): Json<ToolCallRequest>,
) -> Result<Json<ToolCallResponse>, StatusCode> {
    let mut client = state.mcp_client.lock().await;

    match client.call_tool(&request.tool_name, request.parameters).await {
        Ok(result) => Ok(Json(ToolCallResponse {
            success: true,
            result: Some(serde_json::to_value(result).unwrap()),
            error: None,
        })),
        Err(e) => Ok(Json(ToolCallResponse {
            success: false,
            result: None,
            error: Some(e.to_string()),
        })),
    }
}

pub fn create_app(mcp_client: McpClient) -> Router {
    let state = AppState {
        mcp_client: Arc::new(Mutex::new(mcp_client)),
    };

    Router::new()
        .route("/api/tools/call", post(call_tool_endpoint))
        .with_state(state)
}
```

### CLI Tool Integration

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mcp-tool")]
#[command(about = "A CLI tool using MCP SDK")]
pub struct Cli {
    /// MCP server URL
    #[arg(short, long, default_value = "http://localhost:3000/sse")]
    server: String,

    /// Command to execute
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List available tools
    List,
    /// Call a tool
    Call {
        /// Tool name
        name: String,
        /// Parameters as JSON
        #[arg(short, long)]
        params: Option<String>,
    },
    /// Read a resource
    Read {
        /// Resource URI
        uri: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let mut app = MyMcpApp::new(&cli.server).await?;

    match cli.command {
        Commands::List => {
            app.discover_capabilities().await?;
        }
        Commands::Call { name, params } => {
            let params = params
                .map(|p| serde_json::from_str(&p))
                .transpose()?
                .unwrap_or(serde_json::Value::Null);

            let result = app.execute_tool(&name, params).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::Read { uri } => {
            let content = app.read_resource(&uri).await?;
            println!("{}", content);
        }
    }

    Ok(())
}
```

## 📊 Monitoring and Observability

### Logging Integration

```rust
use tracing::{info, error, debug, instrument};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn setup_logging() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

pub struct InstrumentedMcpClient {
    inner: McpClient,
}

impl InstrumentedMcpClient {
    #[instrument(skip(self))]
    pub async fn call_tool_with_logging(&mut self, tool_name: &str, params: serde_json::Value) -> Result<Vec<ToolResult>, McpError> {
        info!("Calling tool: {}", tool_name);
        debug!("Tool parameters: {}", params);

        let start = std::time::Instant::now();
        let result = self.inner.call_tool(tool_name, params).await;
        let duration = start.elapsed();

        match &result {
            Ok(results) => {
                info!("Tool call succeeded in {:?}, returned {} results", duration, results.len());
            }
            Err(e) => {
                error!("Tool call failed in {:?}: {}", duration, e);
            }
        }

        result
    }
}
```

### Metrics Collection

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct McpMetrics {
    pub tool_calls_total: Arc<AtomicU64>,
    pub tool_calls_success: Arc<AtomicU64>,
    pub tool_calls_error: Arc<AtomicU64>,
    pub connection_attempts: Arc<AtomicU64>,
    pub connection_failures: Arc<AtomicU64>,
}

impl McpMetrics {
    pub fn new() -> Self {
        Self {
            tool_calls_total: Arc::new(AtomicU64::new(0)),
            tool_calls_success: Arc::new(AtomicU64::new(0)),
            tool_calls_error: Arc::new(AtomicU64::new(0)),
            connection_attempts: Arc::new(AtomicU64::new(0)),
            connection_failures: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn record_tool_call_success(&self) {
        self.tool_calls_total.fetch_add(1, Ordering::Relaxed);
        self.tool_calls_success.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_tool_call_error(&self) {
        self.tool_calls_total.fetch_add(1, Ordering::Relaxed);
        self.tool_calls_error.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_success_rate(&self) -> f64 {
        let total = self.tool_calls_total.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }
        let success = self.tool_calls_success.load(Ordering::Relaxed);
        success as f64 / total as f64
    }
}
```

## 🧪 Testing

### Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mcp_core::transport::TransportConfig;

    #[tokio::test]
    async fn test_mcp_client_creation() {
        let config = TransportConfig::stdio("echo", &["hello"]).unwrap();
        let transport = TransportFactory::create_transport(config).await.unwrap();

        let client_info = Implementation {
            name: "test-client".to_string(),
            version: "1.0.0".to_string(),
            metadata: Default::default(),
        };

        let client = McpClient::new(transport, client_info);
        assert_eq!(client.client_info().name, "test-client");
    }

    #[tokio::test]
    async fn test_tool_execution() {
        // Mock transport for testing
        let mut app = create_test_app().await;

        let result = app.execute_tool(
            "add_numbers",
            serde_json::json!({
                "a": 5,
                "b": 10
            })
        ).await;

        assert!(result.is_ok());
    }

    async fn create_test_app() -> MyMcpApp {
        // Create a test app with mock transport
        // This would typically use a test server or mock
        todo!("Implement test app creation")
    }
}
```

### Integration Testing

```rust
// Integration test with real MCP server
#[tokio::test]
#[ignore] // Run with `cargo test -- --ignored`
async fn test_real_server_integration() {
    let server_url = std::env::var("TEST_MCP_SERVER")
        .unwrap_or_else(|_| "http://localhost:3000/sse".to_string());

    let mut app = MyMcpApp::new(&server_url).await.unwrap();

    // Test capability discovery
    app.discover_capabilities().await.unwrap();

    // Test tool execution if available
    let tools = app.client.list_tools().await.unwrap();
    if let Some(tool) = tools.tools.first() {
        let result = app.execute_tool(&tool.name, serde_json::Value::Null).await;
        // Result might be success or error, both are valid for testing connectivity
        println!("Tool execution result: {:?}", result);
    }
}
```

## 📖 API Reference

### Core Types

```rust
// Client configuration
pub struct ClientConfig {
    pub timeout: Duration,
    pub max_retries: u32,
    pub enable_logging: bool,
}

// Transport configurations
pub enum TransportConfig {
    Stdio(StdioConfig),
    HttpSse(HttpSseConfig),
    HttpStream(HttpStreamConfig),
}

// Error types
pub enum McpError {
    Transport(TransportError),
    Protocol(String),
    InvalidResponse(String),
    Timeout,
    // ... other variants
}
```

For complete API documentation, run:

```bash
cargo doc --open --package mcp-core
```

## 🎯 Best Practices

1. **Connection Management**: Always handle connection failures gracefully with retries
2. **Error Handling**: Use proper error handling patterns and don't ignore errors
3. **Resource Cleanup**: Ensure proper cleanup of resources and connections
4. **Async Patterns**: Use proper async/await patterns and avoid blocking operations
5. **Configuration**: Make transport and client configuration externally configurable
6. **Logging**: Add comprehensive logging for debugging and monitoring
7. **Testing**: Write unit and integration tests for your MCP applications

## 🔗 Resources

- **API Documentation**: [docs.rs/mcp-core](https://docs.rs/mcp-core)
- **Example Applications**: See `examples/` directory in the repository
- **MCP Specification**: [Model Context Protocol Spec](https://modelcontextprotocol.io)
- **Community**: [GitHub Discussions](https://github.com/conikeec/mcp-probe/discussions)

---

**🚀 Start building powerful MCP applications with the mcp-core SDK!**

Made with ❤️ in Rust
