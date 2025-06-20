//! Terminal User Interface for MCP Probe
//!
//! This module provides a rich interactive TUI for debugging MCP servers,
//! allowing real-time inspection of the negotiation process and server capabilities.

use anyhow::Result;
use ratatui::crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use mcp_core::{
    client::McpClient,
    messages::{
        prompts::{
            GetPromptRequest, GetPromptResponse, ListPromptsRequest, ListPromptsResponse, Prompt,
        },
        resources::{
            ListResourcesRequest, ListResourcesResponse, ReadResourceRequest, ReadResourceResponse,
            Resource,
        },
        tools::{CallToolRequest, CallToolResponse, ListToolsRequest, ListToolsResponse},
        Implementation, JsonRpcRequest, JsonRpcResponse,
    },
    transport::TransportConfig,
    McpResult,
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame, Terminal,
};
use serde_json::Value;
use std::{
    collections::HashMap,
    io,
    time::{Duration, Instant},
};
use tui_textarea::{Input, TextArea};

use crate::search::{SearchCategory, SearchEngine, SearchResult};

/// Extension trait to add higher-level methods to McpClient
trait McpClientExt {
    async fn list_resources(&mut self) -> McpResult<Vec<Resource>>;
    async fn list_prompts(&mut self) -> McpResult<Vec<Prompt>>;
    async fn call_tool(&mut self, name: &str, arguments: Value) -> McpResult<CallToolResponse>;
    async fn read_resource(&mut self, uri: &str) -> McpResult<ReadResourceResponse>;
    async fn get_prompt(
        &mut self,
        name: &str,
        arguments: Option<Value>,
    ) -> McpResult<GetPromptResponse>;
}

impl McpClientExt for McpClient {
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

    async fn call_tool(&mut self, name: &str, arguments: Value) -> McpResult<CallToolResponse> {
        let request = CallToolRequest {
            name: name.to_string(),
            arguments: Some(arguments),
        };
        let response = self.send_request("tools/call", request).await?;

        // Log the raw response for debugging
        tracing::info!("=== RAW TOOL CALL RESPONSE ===");
        tracing::info!(
            "Full response: {}",
            serde_json::to_string_pretty(&response)
                .unwrap_or_else(|_| "Failed to serialize response".to_string())
        );

        if let Some(result) = response.result {
            tracing::info!("=== RESULT FIELD ANALYSIS ===");
            tracing::info!(
                "Result field: {}",
                serde_json::to_string_pretty(&result)
                    .unwrap_or_else(|_| "Failed to serialize result".to_string())
            );

            // Try to parse as standard MCP CallToolResponse first
            if let Ok(call_response) = serde_json::from_value::<CallToolResponse>(result.clone()) {
                tracing::info!("✅ Successfully parsed as standard MCP CallToolResponse");
                tracing::info!("Content items: {}", call_response.content.len());

                // Check if we got a "successful" parse but with empty content
                // This happens when the tool returns non-MCP format like {"a": 30, "b": 40}
                if call_response.content.is_empty() && result.is_object() {
                    tracing::warn!("⚠️  Parsed as CallToolResponse but content is empty - tool likely returned non-MCP format");
                    tracing::warn!("🔄 Falling back to custom parsing...");

                    // Force custom parsing
                    let result_text = serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|_| result.to_string());
                    tracing::info!("📝 Custom parsed result text: {}", result_text);

                    Ok(CallToolResponse {
                        content: vec![mcp_core::messages::tools::ToolResult::Text {
                            text: result_text,
                        }],
                        is_error: Some(false),
                    })
                } else {
                    Ok(call_response)
                }
            } else {
                tracing::info!(
                    "❌ Failed to parse as standard MCP CallToolResponse, using custom parsing"
                );
                // Handle non-standard tool responses (like add_numbers)
                // Convert the raw result into a text-based ToolResult
                let result_text = if result.is_object() || result.is_array() {
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
                } else {
                    result.to_string()
                };

                tracing::info!("📝 Custom parsed result text: {}", result_text);

                Ok(CallToolResponse {
                    content: vec![mcp_core::messages::tools::ToolResult::Text {
                        text: result_text,
                    }],
                    is_error: Some(false),
                })
            }
        } else {
            Err(mcp_core::McpError::Protocol(
                mcp_core::error::ProtocolError::RequestFailed {
                    reason: "No result in tool call response".to_string(),
                },
            ))
        }
    }

    async fn read_resource(&mut self, uri: &str) -> McpResult<ReadResourceResponse> {
        let request = ReadResourceRequest {
            uri: uri.to_string(),
        };
        let response = self.send_request("resources/read", request).await?;

        if let Some(result) = response.result {
            let read_response: ReadResourceResponse = serde_json::from_value(result)?;
            Ok(read_response)
        } else {
            Err(mcp_core::McpError::Protocol(
                mcp_core::error::ProtocolError::RequestFailed {
                    reason: "No result in resource read response".to_string(),
                },
            ))
        }
    }

    async fn get_prompt(
        &mut self,
        name: &str,
        arguments: Option<Value>,
    ) -> McpResult<GetPromptResponse> {
        let request = GetPromptRequest {
            name: name.to_string(),
            arguments,
        };
        let response = self.send_request("prompts/get", request).await?;

        if let Some(result) = response.result {
            let prompt_response: GetPromptResponse = serde_json::from_value(result)?;
            Ok(prompt_response)
        } else {
            Err(mcp_core::McpError::Protocol(
                mcp_core::error::ProtocolError::RequestFailed {
                    reason: "No result in prompt get response".to_string(),
                },
            ))
        }
    }
}

/// Main TUI application for interactive debugging
pub struct DebuggerApp {
    /// Transport configuration
    transport_config: TransportConfig,

    /// Client implementation info
    client_info: Implementation,

    /// Current application state
    state: AppState,

    /// UI state manager
    ui_state: UiState,

    /// Message history
    message_history: Vec<MessageEntry>,

    /// Current server capabilities
    capabilities: ServerCapabilities,

    /// Environment variables for capability invocation
    env_variables: HashMap<String, String>,

    /// MCP client instance
    client: Option<McpClient>,

    /// Session ID from server
    session_id: Option<String>,

    /// Session start time
    session_start: Instant,

    /// Message counter
    message_count: usize,

    /// Error counter
    error_count: usize,

    /// Discovery progress
    discovery_step: String,

    /// Search engine for capabilities
    search_engine: SearchEngine,
}

/// Application state
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    /// Initial state
    Initializing,
    /// Connecting to server
    Connecting,
    /// Discovering capabilities
    Discovering,
    /// Ready for interaction
    Ready,
    /// Error state
    Error(String),
    /// Shutting down
    ShuttingDown,
}

/// UI state for managing focus and interactions
#[derive(Debug)]
pub struct UiState {
    /// Currently focused panel
    current_focus: FocusedPanel,

    /// List states for different panels
    protocol_flow_state: ListState,
    capabilities_state: ListState,
    message_history_state: ListState,

    /// Input state
    input_area: TextArea<'static>,

    /// Environment variables input area
    env_input_area: TextArea<'static>,

    /// Whether environment variables dialog is open
    env_dialog_open: bool,

    /// Scroll states
    message_scroll: ScrollbarState,
    capability_details_scroll: ScrollbarState,
    search_results_scroll: ScrollbarState,
    env_vars_scroll: ScrollbarState,
    response_viewer_scroll: ScrollbarState,
    response_viewer_horizontal_scroll: ScrollbarState,

    /// Response viewer scroll positions
    response_viewer_vertical_pos: usize,
    response_viewer_horizontal_pos: usize,

    /// Ordered list of parameter field names for consistent access
    param_field_names: Vec<String>,

    /// Whether help dialog is open
    help_dialog_open: bool,

    /// Whether to show raw JSON
    show_raw_json: bool,

    /// Current selected message index for inspection
    selected_message_index: Option<usize>,

    /// Capability selection tracking
    capability_indices: Vec<Option<CapabilityRef>>,

    /// Current capability view state
    capability_view: CapabilityView,
    capability_detail_state: ListState,
    capability_page: usize,
    capability_page_size: usize,

    /// Parameter form dialog state
    parameter_dialog_open: bool,
    selected_capability: Option<CapabilityRef>,

    /// New enhanced parameter form state
    param_fields: HashMap<String, ParamField>,
    param_selected_field: usize,
    param_edit_mode: bool,

    /// Search functionality
    search_input: TextArea<'static>,
    search_active: bool,
    search_results: Vec<SearchResult>,
    search_results_state: ListState,

    /// Response viewer
    response_viewer_open: bool,
    response_viewer_mode: ResponseViewMode,
    selected_response: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct ParamField {
    pub value: String,
    pub required: bool,
    pub description: Option<String>,
    pub param_type: Option<String>,
}

/// Which panel is currently focused
#[derive(Debug, Clone, PartialEq)]
pub enum FocusedPanel {
    ProtocolFlow,
    MessageInspector,
    Capabilities,
    Input,
    EnvVariables,
}

/// Reference to a specific capability
#[derive(Debug, Clone)]
pub enum CapabilityRef {
    Tool(usize),
    Resource(usize),
    Prompt(usize),
}

/// Capability category for navigation
#[derive(Debug, Clone, PartialEq)]
pub enum CapabilityCategory {
    Tools,
    Resources,
    Prompts,
}

/// Current view state for capabilities
#[derive(Debug, Clone)]
pub enum CapabilityView {
    Categories,
    DetailedList(CapabilityCategory),
}

/// Response viewer display modes
#[derive(Debug, Clone, PartialEq)]
pub enum ResponseViewMode {
    Formatted, // Nicely formatted with syntax highlighting
    RawJson,   // Raw JSON view
    TreeView,  // Tree-like structure view
    Summary,   // Summary with key information
}

/// Message entry for history tracking
#[derive(Debug, Clone)]
pub struct MessageEntry {
    /// Timestamp
    timestamp: Instant,

    /// Message type
    message_type: MessageType,

    /// Request data
    request: Option<JsonRpcRequest>,

    /// Response data
    response: Option<JsonRpcResponse>,

    /// Raw response data for better viewing
    raw_response: Option<Value>,

    /// Error information
    error: Option<String>,

    /// Success information
    success: Option<String>,
}

/// Message type classification
#[derive(Debug, Clone)]
pub enum MessageType {
    Initialize,
    ListTools,
    ListResources,
    ListPrompts,
    CallTool,
    GetResource,
    GetPrompt,
    Other(String),
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageType::Initialize => write!(f, "Initialize"),
            MessageType::ListTools => write!(f, "List Tools"),
            MessageType::ListResources => write!(f, "List Resources"),
            MessageType::ListPrompts => write!(f, "List Prompts"),
            MessageType::CallTool => write!(f, "Call Tool"),
            MessageType::GetResource => write!(f, "Get Resource"),
            MessageType::GetPrompt => write!(f, "Get Prompt"),
            MessageType::Other(s) => write!(f, "{}", s),
        }
    }
}

/// Server capabilities
#[derive(Debug, Clone, Default)]
pub struct ServerCapabilities {
    /// Available tools
    tools: Vec<ToolInfo>,

    /// Available resources
    resources: Vec<ResourceInfo>,

    /// Available prompts
    prompts: Vec<PromptInfo>,
}

/// Tool information
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,      // Clean display name (without prefix)
    pub full_name: String, // Original full name with prefix (for API calls)
    pub description: Option<String>,
    pub parameters: Option<Value>,
}

/// Resource information
#[derive(Debug, Clone)]
pub struct ResourceInfo {
    pub uri: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

/// Prompt information
#[derive(Debug, Clone)]
pub struct PromptInfo {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Option<Value>,
}

impl DebuggerApp {
    /// Create a new debugger application
    pub fn new(transport_config: TransportConfig, client_info: Implementation) -> Result<Self> {
        eprintln!(
            "Creating DebuggerApp with transport: {:?}",
            transport_config
        );
        tracing::info!("Creating DebuggerApp");
        tracing::debug!("Transport config: {:?}", transport_config);
        let mut input_area = TextArea::default();
        input_area.set_placeholder_text(
            "Enter MCP command (e.g., tools.calculator {\"a\": 5, \"b\": 3})",
        );

        let mut env_input_area = TextArea::default();
        env_input_area
            .set_placeholder_text("Enter environment variables (KEY=value,KEY2=value2...)");

        let mut search_input = TextArea::default();
        search_input.set_placeholder_text(
            "Type to search by name, description, or keywords (supports fuzzy matching)",
        );

        let ui_state = UiState {
            current_focus: FocusedPanel::Capabilities,
            protocol_flow_state: ListState::default().with_selected(Some(0)),
            capabilities_state: ListState::default().with_selected(Some(0)),
            message_history_state: ListState::default(),
            input_area,
            env_input_area,
            env_dialog_open: false,
            message_scroll: ScrollbarState::new(0),
            capability_details_scroll: ScrollbarState::new(0),
            search_results_scroll: ScrollbarState::new(0),
            env_vars_scroll: ScrollbarState::new(0),
            response_viewer_scroll: ScrollbarState::new(0),
            response_viewer_horizontal_scroll: ScrollbarState::new(0),
            response_viewer_vertical_pos: 0,
            response_viewer_horizontal_pos: 0,
            param_field_names: Vec::new(),
            help_dialog_open: false,
            show_raw_json: false,
            selected_message_index: None,
            capability_indices: Vec::new(),
            capability_view: CapabilityView::Categories,
            capability_detail_state: ListState::default(),
            capability_page: 0,
            capability_page_size: 10,
            parameter_dialog_open: false,
            selected_capability: None,
            param_fields: HashMap::new(),
            param_selected_field: 0,
            param_edit_mode: false,
            search_input,
            search_active: false,
            search_results: Vec::new(),
            search_results_state: ListState::default(),
            response_viewer_open: false,
            response_viewer_mode: ResponseViewMode::Formatted,
            selected_response: None,
        };

        Ok(Self {
            transport_config,
            client_info,
            state: AppState::Initializing,
            ui_state,
            message_history: Vec::new(),
            capabilities: ServerCapabilities::default(),
            env_variables: HashMap::new(),
            client: None,
            session_id: None,
            session_start: Instant::now(),
            message_count: 0,
            error_count: 0,
            discovery_step: String::new(),
            search_engine: SearchEngine::new(),
        })
    }

    /// Run the TUI application
    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Start the main event loop
        let result = self.run_app(&mut terminal).await;

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    /// Run the main application loop
    async fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        // Start client initialization in background
        let mut client_initialized = false;
        let mut initialization_task: Option<tokio::task::JoinHandle<Result<McpClient>>> = None;

        loop {
            // Start client initialization if not already started
            if !client_initialized && initialization_task.is_none() {
                let transport_config = self.transport_config.clone();
                let client_info = self.client_info.clone();

                tracing::info!("Starting MCP client initialization");
                tracing::debug!("Transport config: {:?}", transport_config);
                tracing::debug!("Client info: {:?}", client_info);

                initialization_task = Some(tokio::spawn(async move {
                    tracing::debug!("Creating MCP client with defaults");
                    let mut client =
                        McpClient::with_defaults(transport_config)
                            .await
                            .map_err(|e| {
                                tracing::error!("Failed to create MCP client: {}", e);
                                anyhow::anyhow!("Failed to create MCP client: {}", e)
                            })?;

                    tracing::debug!("Attempting to connect to MCP server");
                    let _server_info = client.connect(client_info).await.map_err(|e| {
                        tracing::error!("Failed to connect to MCP server: {}", e);
                        anyhow::anyhow!("Failed to connect to MCP server: {}", e)
                    })?;

                    tracing::info!("MCP client connected successfully");
                    Ok(client)
                }));

                self.state = AppState::Connecting;
                tracing::info!("State changed to Connecting");
            }

            // Check if client initialization completed
            if let Some(ref mut task) = initialization_task {
                if task.is_finished() {
                    match task.await {
                        Ok(Ok(mut client)) => {
                            // Client connected successfully - extract session ID
                            tracing::info!("Client initialization completed successfully");
                            self.state = AppState::Discovering;
                            self.discovery_step = "Extracting session...".to_string();
                            tracing::debug!(
                                "State changed to Discovering, step: Extracting session..."
                            );

                            // Add initialization message to history
                            self.add_message(MessageEntry {
                                timestamp: std::time::Instant::now(),
                                message_type: MessageType::Initialize,
                                request: None,
                                response: None,
                                raw_response: None,
                                error: None,
                                success: None,
                            });

                            // Extract session ID from the transport
                            let transport_info = client.transport_info();
                            tracing::debug!("Transport info: {:?}", transport_info);

                            if let Some(session_value) = transport_info.metadata.get("session_id") {
                                tracing::debug!(
                                    "Found session_id in metadata: {:?}",
                                    session_value
                                );
                                if let Some(session_str) = session_value.as_str() {
                                    if !session_str.is_empty() && session_str != "null" {
                                        self.session_id = Some(session_str.to_string());
                                        tracing::info!("Extracted session ID: {}", session_str);
                                    } else {
                                        tracing::warn!(
                                            "Session ID is empty or null: {}",
                                            session_str
                                        );
                                    }
                                } else {
                                    tracing::warn!(
                                        "Session ID metadata is not a string: {:?}",
                                        session_value
                                    );
                                }
                            } else {
                                tracing::warn!("No session_id found in transport metadata");
                            }

                            // Debug: Show what we extracted
                            let session_debug = self.session_id.as_deref().unwrap_or("none");
                            self.add_message(MessageEntry {
                                timestamp: std::time::Instant::now(),
                                message_type: MessageType::Other("Debug".to_string()),
                                request: None,
                                response: None,
                                raw_response: None,
                                error: None,
                                success: Some(format!("Session ID extracted: {}", session_debug)),
                            });

                            // Discover capabilities step by step
                            self.discovery_step = "Listing tools...".to_string();
                            tracing::debug!("Starting tools discovery");

                            // Let's get the raw JSON response first
                            let request = ListToolsRequest { cursor: None };
                            match client.send_request("tools/list", request).await {
                                Ok(raw_response) => {
                                    tracing::debug!("Raw tools/list response: {:?}", raw_response);

                                    if let Some(result) = raw_response.result {
                                        tracing::debug!("Raw result field: {:?}", result);

                                        match serde_json::from_value::<ListToolsResponse>(result) {
                                            Ok(list_response) => {
                                                let tools = list_response.tools;
                                                tracing::info!(
                                                    "Successfully listed {} tools",
                                                    tools.len()
                                                );

                                                // Let's examine the first few tools in detail
                                                for (i, tool) in tools.iter().take(3).enumerate() {
                                                    tracing::debug!(
                                                        "Tool #{}: name={}, desc={}",
                                                        i,
                                                        tool.name,
                                                        tool.description
                                                    );
                                                    tracing::debug!(
                                                        "Tool #{} input_schema: {:?}",
                                                        i,
                                                        tool.input_schema
                                                    );
                                                    if let Some(ref schema) = tool.input_schema {
                                                        tracing::debug!(
                                                            "Tool #{} schema JSON: {}",
                                                            i,
                                                            serde_json::to_string_pretty(schema)
                                                                .unwrap_or_else(
                                                                    |_| "Invalid JSON".to_string()
                                                                )
                                                        );
                                                    }
                                                }

                                                self.capabilities.tools = tools
                                                    .into_iter()
                                                    .map(|tool| {
                                                        ToolInfo {
                                                            name: Self::strip_tool_prefix(
                                                                &tool.name,
                                                            )
                                                            .to_string(),
                                                            full_name: tool.name, // Use the exact name from server
                                                            description: Some(tool.description),
                                                            parameters: tool.input_schema,
                                                        }
                                                    })
                                                    .collect();

                                                // Add success message to history
                                                self.add_message(MessageEntry {
                                                    timestamp: std::time::Instant::now(),
                                                    message_type: MessageType::ListTools,
                                                    request: None,
                                                    response: None,
                                                    raw_response: None,
                                                    error: None,
                                                    success: Some(format!(
                                                        "Found {} tools",
                                                        self.capabilities.tools.len()
                                                    )),
                                                });
                                            }
                                            Err(e) => {
                                                tracing::error!(
                                                    "Failed to deserialize tools list response: {}",
                                                    e
                                                );
                                                self.add_error(format!(
                                                    "Failed to parse tools: {}",
                                                    e
                                                ));
                                                self.add_message(MessageEntry {
                                                    timestamp: std::time::Instant::now(),
                                                    message_type: MessageType::ListTools,
                                                    request: None,
                                                    response: None,
                                                    raw_response: None,
                                                    error: Some(format!(
                                                        "Tools parse error: {}",
                                                        e
                                                    )),
                                                    success: None,
                                                });
                                            }
                                        }
                                    } else {
                                        tracing::error!("No result field in tools/list response");
                                        self.add_error("No result in tools response".to_string());
                                        self.add_message(MessageEntry {
                                            timestamp: std::time::Instant::now(),
                                            message_type: MessageType::ListTools,
                                            request: None,
                                            response: None,
                                            raw_response: None,
                                            error: Some("No result in tools response".to_string()),
                                            success: None,
                                        });
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Failed to get raw tools response: {}", e);
                                    self.add_error(format!("Failed to list tools: {}", e));
                                    self.add_message(MessageEntry {
                                        timestamp: std::time::Instant::now(),
                                        message_type: MessageType::ListTools,
                                        request: None,
                                        response: None,
                                        raw_response: None,
                                        error: Some(format!("Tools error: {}", e)),
                                        success: None,
                                    });
                                }
                            }

                            self.discovery_step = "Listing resources...".to_string();
                            match client.list_resources().await {
                                Ok(resources) => {
                                    self.capabilities.resources = resources
                                        .into_iter()
                                        .map(|resource| ResourceInfo {
                                            uri: resource.uri,
                                            name: Some(resource.name),
                                            description: resource.description,
                                            mime_type: resource.mime_type,
                                        })
                                        .collect();

                                    // Add success message to history
                                    self.add_message(MessageEntry {
                                        timestamp: std::time::Instant::now(),
                                        message_type: MessageType::ListResources,
                                        request: None,
                                        response: None,
                                        raw_response: None,
                                        error: None,
                                        success: Some(format!(
                                            "Found {} resources",
                                            self.capabilities.resources.len()
                                        )),
                                    });
                                }
                                Err(e) => {
                                    self.add_error(format!("Failed to list resources: {}", e));
                                    self.add_message(MessageEntry {
                                        timestamp: std::time::Instant::now(),
                                        message_type: MessageType::ListResources,
                                        request: None,
                                        response: None,
                                        raw_response: None,
                                        error: Some(format!("Resources error: {}", e)),
                                        success: None,
                                    });
                                }
                            }

                            self.discovery_step = "Listing prompts...".to_string();
                            match client.list_prompts().await {
                                Ok(prompts) => {
                                    self.capabilities.prompts = prompts
                                        .into_iter()
                                        .map(|prompt| PromptInfo {
                                            name: prompt.name,
                                            description: Some(prompt.description),
                                            arguments: prompt.arguments,
                                        })
                                        .collect();

                                    // Add success message to history
                                    self.add_message(MessageEntry {
                                        timestamp: std::time::Instant::now(),
                                        message_type: MessageType::ListPrompts,
                                        request: None,
                                        response: None,
                                        raw_response: None,
                                        error: None,
                                        success: Some(format!(
                                            "Found {} prompts",
                                            self.capabilities.prompts.len()
                                        )),
                                    });
                                }
                                Err(e) => {
                                    self.add_error(format!("Failed to list prompts: {}", e));
                                    self.add_message(MessageEntry {
                                        timestamp: std::time::Instant::now(),
                                        message_type: MessageType::ListPrompts,
                                        request: None,
                                        response: None,
                                        raw_response: None,
                                        error: Some(format!("Prompts error: {}", e)),
                                        success: None,
                                    });
                                }
                            }

                            self.discovery_step = "Discovery complete".to_string();
                            tracing::info!("All discovery completed successfully, client ready");

                            // Index capabilities for search
                            self.search_engine.index_tools(&self.capabilities.tools);
                            self.search_engine
                                .index_resources(&self.capabilities.resources);
                            self.search_engine.index_prompts(&self.capabilities.prompts);
                            tracing::info!(
                                "Indexed {} items for search",
                                self.search_engine.total_items()
                            );

                            self.client = Some(client);
                            self.state = AppState::Ready;
                            client_initialized = true;
                            tracing::info!(
                                "State changed to Ready, client initialization complete"
                            );
                        }
                        Ok(Err(e)) => {
                            // Connection failed
                            tracing::error!("Client initialization failed: {}", e);
                            let error_msg = format!("Connection failed: {}", e);
                            self.state = AppState::Error(error_msg.clone());
                            self.add_error(format!("Failed to connect to MCP server: {}", e));

                            // Add detailed error to message history
                            self.add_message(MessageEntry {
                                timestamp: std::time::Instant::now(),
                                message_type: MessageType::Initialize,
                                request: None,
                                response: None,
                                raw_response: None,
                                error: Some(format!("Connection error: {}", e)),
                                success: None,
                            });

                            client_initialized = true; // Don't retry automatically
                        }
                        Err(e) => {
                            // Task panicked
                            tracing::error!("Client initialization task panicked: {}", e);
                            let error_msg = format!("Client task failed: {}", e);
                            self.state = AppState::Error(error_msg.clone());
                            self.add_error(format!("Client initialization task failed: {}", e));

                            // Add detailed error to message history
                            self.add_message(MessageEntry {
                                timestamp: std::time::Instant::now(),
                                message_type: MessageType::Initialize,
                                request: None,
                                response: None,
                                raw_response: None,
                                error: Some(format!("Task panic: {}", e)),
                                success: None,
                            });

                            client_initialized = true;
                        }
                    }
                    initialization_task = None;
                }
            }

            // Draw the UI
            terminal.draw(|f| self.draw_ui(f))?;

            // Handle events
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press && self.handle_key_event(key.code).await? {
                        break; // Exit requested
                    }
                }
            }

            // Check for shutdown
            if self.state == AppState::ShuttingDown {
                break;
            }
        }

        Ok(())
    }

    /// Handle keyboard events
    async fn handle_key_event(&mut self, key: KeyCode) -> Result<bool> {
        // Handle search mode first
        if self.ui_state.search_active {
            match key {
                KeyCode::Esc => {
                    self.ui_state.search_active = false;
                    self.ui_state.search_results.clear();
                    self.ui_state.current_focus = FocusedPanel::Capabilities;
                    return Ok(false);
                }
                KeyCode::Enter => {
                    // Select from search results
                    if let Some(selected) = self.ui_state.search_results_state.selected() {
                        if let Some(result) = self.ui_state.search_results.get(selected) {
                            self.select_search_result(result.clone());
                        }
                    }
                    return Ok(false);
                }
                KeyCode::Up => {
                    if !self.ui_state.search_results.is_empty() {
                        let current = self.ui_state.search_results_state.selected().unwrap_or(0);
                        if current > 0 {
                            self.ui_state.search_results_state.select(Some(current - 1));
                        } else {
                            self.ui_state
                                .search_results_state
                                .select(Some(self.ui_state.search_results.len() - 1));
                        }
                    }
                    return Ok(false);
                }
                KeyCode::Down => {
                    if !self.ui_state.search_results.is_empty() {
                        let current = self.ui_state.search_results_state.selected().unwrap_or(0);
                        if current + 1 < self.ui_state.search_results.len() {
                            self.ui_state.search_results_state.select(Some(current + 1));
                        } else {
                            self.ui_state.search_results_state.select(Some(0));
                        }
                    }
                    return Ok(false);
                }
                _ => {
                    // Handle search input
                    let key_event = crossterm::event::KeyEvent::new(
                        key,
                        crossterm::event::KeyModifiers::empty(),
                    );
                    self.ui_state.search_input.input(Input::from(key_event));
                    self.perform_search();
                    return Ok(false);
                }
            }
        }

        match key {
            KeyCode::Char('q') if !self.ui_state.env_dialog_open => {
                self.state = AppState::ShuttingDown;
                return Ok(true);
            }
            KeyCode::Char('/') => {
                // Activate search mode (available from any panel)
                self.ui_state.search_active = true;
                self.ui_state.current_focus = FocusedPanel::Capabilities;
                // Clear any previous search
                self.ui_state.search_input.select_all();
                self.ui_state.search_input.cut();
                return Ok(false);
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                // Open response viewer for selected message
                if let Some(selected_idx) = self.ui_state.message_history_state.selected() {
                    if let Some(message) = self.message_history.get(selected_idx) {
                        if let Some(ref response) = message.raw_response {
                            self.ui_state.selected_response = Some(response.clone());
                            self.ui_state.response_viewer_open = true;
                            // Reset scroll positions to start from top
                            self.ui_state.response_viewer_vertical_pos = 0;
                            self.ui_state.response_viewer_horizontal_pos = 0;
                            return Ok(false);
                        }
                    }
                }
                return Ok(false);
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                // Cycle response view modes
                if self.ui_state.response_viewer_open {
                    self.ui_state.response_viewer_mode = match self.ui_state.response_viewer_mode {
                        ResponseViewMode::Formatted => ResponseViewMode::RawJson,
                        ResponseViewMode::RawJson => ResponseViewMode::TreeView,
                        ResponseViewMode::TreeView => ResponseViewMode::Summary,
                        ResponseViewMode::Summary => ResponseViewMode::Formatted,
                    };
                }
                return Ok(false);
            }
            KeyCode::F(1) => {
                self.ui_state.help_dialog_open = !self.ui_state.help_dialog_open;
            }
            KeyCode::F(2) => {
                // Save session
                self.save_session().await?;
            }
            KeyCode::F(3) => {
                // Toggle raw JSON view
                self.ui_state.show_raw_json = !self.ui_state.show_raw_json;
            }
            KeyCode::F(4) => {
                // Clear messages
                self.message_history.clear();
                self.message_count = 0;
                self.error_count = 0;
            }
            KeyCode::F(5) => {
                // Open environment variables dialog
                self.ui_state.env_dialog_open = !self.ui_state.env_dialog_open;
                if self.ui_state.env_dialog_open {
                    self.ui_state.current_focus = FocusedPanel::EnvVariables;
                } else {
                    self.ui_state.current_focus = FocusedPanel::Input;
                }
            }
            KeyCode::Enter => {
                if self.ui_state.parameter_dialog_open {
                    if self.ui_state.param_field_names.is_empty() {
                        // No parameters - execute directly
                        self.execute_selected_capability().await?;
                    } else if self.ui_state.param_edit_mode {
                        // Exit edit mode and move to next field
                        self.ui_state.param_edit_mode = false;
                        self.navigate_parameter_form_down();
                    } else {
                        // Enter edit mode for current field (fallback case)
                        self.ui_state.param_edit_mode = true;
                    }
                } else if self.ui_state.env_dialog_open
                    && self.ui_state.current_focus == FocusedPanel::EnvVariables
                {
                    self.parse_env_variables();
                    self.ui_state.env_dialog_open = false;
                    self.ui_state.current_focus = FocusedPanel::Input;
                } else if self.ui_state.current_focus == FocusedPanel::Capabilities {
                    self.select_capability();
                } else if self.ui_state.current_focus == FocusedPanel::Input {
                    self.execute_command().await?;
                }
            }
            KeyCode::Esc => {
                if self.ui_state.response_viewer_open {
                    self.ui_state.response_viewer_open = false;
                    self.ui_state.selected_response = None;
                } else if self.ui_state.parameter_dialog_open {
                    if self.ui_state.param_edit_mode {
                        // Exit edit mode
                        self.ui_state.param_edit_mode = false;
                    } else {
                        // Close parameter dialog
                        self.ui_state.parameter_dialog_open = false;
                        self.ui_state.selected_capability = None;
                        self.ui_state.current_focus = FocusedPanel::Capabilities;
                    }
                } else if self.ui_state.env_dialog_open {
                    self.ui_state.env_dialog_open = false;
                    self.ui_state.current_focus = FocusedPanel::Input;
                } else if self.ui_state.help_dialog_open {
                    self.ui_state.help_dialog_open = false;
                } else if matches!(
                    self.ui_state.capability_view,
                    CapabilityView::DetailedList(_)
                ) {
                    // Go back from detailed list to categories
                    self.ui_state.capability_view = CapabilityView::Categories;
                    self.ui_state.current_focus = FocusedPanel::Capabilities;
                }
            }
            KeyCode::Tab => {
                if self.ui_state.parameter_dialog_open
                    && !self.ui_state.param_field_names.is_empty()
                {
                    // Execute with current parameter values
                    self.execute_selected_capability().await?;
                } else {
                    // Auto-open response viewer for latest message with results
                    self.open_latest_response_viewer();
                }
            }
            KeyCode::Up => {
                if self.ui_state.response_viewer_open {
                    self.navigate_response_viewer_up();
                } else if self.ui_state.parameter_dialog_open && !self.ui_state.param_edit_mode {
                    self.navigate_parameter_form_up();
                } else if self.ui_state.current_focus == FocusedPanel::Capabilities {
                    self.navigate_capabilities_up();
                }
            }
            KeyCode::Down => {
                if self.ui_state.response_viewer_open {
                    self.navigate_response_viewer_down();
                } else if self.ui_state.parameter_dialog_open && !self.ui_state.param_edit_mode {
                    self.navigate_parameter_form_down();
                } else if self.ui_state.current_focus == FocusedPanel::Capabilities {
                    self.navigate_capabilities_down();
                }
            }
            KeyCode::Left => {
                if self.ui_state.response_viewer_open {
                    self.navigate_response_viewer_left();
                } else if matches!(
                    self.ui_state.capability_view,
                    CapabilityView::DetailedList(_)
                ) {
                    self.navigate_page_left();
                }
            }
            KeyCode::Right => {
                if self.ui_state.response_viewer_open {
                    self.navigate_response_viewer_right();
                } else if matches!(
                    self.ui_state.capability_view,
                    CapabilityView::DetailedList(_)
                ) {
                    self.navigate_page_right();
                }
            }
            KeyCode::PageUp => {
                if self.ui_state.response_viewer_open {
                    self.navigate_response_viewer_page_up();
                }
            }
            KeyCode::PageDown => {
                if self.ui_state.response_viewer_open {
                    self.navigate_response_viewer_page_down();
                }
            }
            KeyCode::Home => {
                if self.ui_state.response_viewer_open {
                    self.navigate_response_viewer_home();
                }
            }
            KeyCode::End => {
                if self.ui_state.response_viewer_open {
                    self.navigate_response_viewer_end();
                }
            }
            _ => {
                // Handle input for focused text areas
                match self.ui_state.current_focus {
                    FocusedPanel::Input => {
                        self.ui_state.input_area.input(Input::from(
                            crossterm::event::KeyEvent::new(key, event::KeyModifiers::empty()),
                        ));
                    }
                    FocusedPanel::EnvVariables if self.ui_state.env_dialog_open => {
                        self.ui_state.env_input_area.input(Input::from(
                            crossterm::event::KeyEvent::new(key, event::KeyModifiers::empty()),
                        ));
                    }
                    _ => {
                        // Handle parameter form text input when in edit mode
                        if self.ui_state.parameter_dialog_open && self.ui_state.param_edit_mode {
                            // Safety check: ensure param_selected_field is within bounds
                            if self.ui_state.param_selected_field
                                < self.ui_state.param_field_names.len()
                            {
                                if let Some(field_name) = self
                                    .ui_state
                                    .param_field_names
                                    .get(self.ui_state.param_selected_field)
                                {
                                    let field_name = field_name.clone();
                                    if let Some(field) =
                                        self.ui_state.param_fields.get_mut(&field_name)
                                    {
                                        // Handle text input for the current field
                                        match key {
                                            KeyCode::Char(c) => {
                                                field.value.push(c);
                                                tracing::debug!("Added char '{}' to field '{}', value now: '{}'", c, field_name, field.value);
                                            }
                                            KeyCode::Backspace => {
                                                field.value.pop();
                                                tracing::debug!(
                                                    "Removed char from field '{}', value now: '{}'",
                                                    field_name,
                                                    field.value
                                                );
                                            }
                                            _ => {}
                                        }
                                    } else {
                                        tracing::error!(
                                            "Field '{}' not found in param_fields HashMap",
                                            field_name
                                        );
                                    }
                                } else {
                                    tracing::error!(
                                        "No field name at index {} in param_field_names",
                                        self.ui_state.param_selected_field
                                    );
                                }
                            } else {
                                tracing::error!(
                                    "param_selected_field {} is out of bounds (len: {})",
                                    self.ui_state.param_selected_field,
                                    self.ui_state.param_field_names.len()
                                );
                            }
                        }
                    }
                }
            }
        }
        Ok(false)
    }

    /// Cycle through focusable panels
    fn cycle_focus(&mut self) {
        if self.ui_state.env_dialog_open || self.ui_state.parameter_dialog_open {
            return; // Don't cycle when dialogs are open
        }

        self.ui_state.current_focus = match self.ui_state.current_focus {
            FocusedPanel::ProtocolFlow => FocusedPanel::MessageInspector,
            FocusedPanel::MessageInspector => FocusedPanel::Capabilities,
            FocusedPanel::Capabilities => FocusedPanel::Input,
            FocusedPanel::Input => FocusedPanel::ProtocolFlow,
            FocusedPanel::EnvVariables => FocusedPanel::Input,
        };
    }

    /// Parse environment variables from input
    fn parse_env_variables(&mut self) {
        let input = self.ui_state.env_input_area.lines().join("");
        self.env_variables.clear();

        for pair in input.split(',') {
            let pair = pair.trim();
            if let Some((key, value)) = pair.split_once('=') {
                self.env_variables
                    .insert(key.trim().to_string(), value.trim().to_string());
            }
        }

        // Clear the input area
        self.ui_state.env_input_area = TextArea::default();
        self.ui_state
            .env_input_area
            .set_placeholder_text("Enter environment variables (KEY=value,KEY2=value2...)");
    }

    /// Navigate capabilities up
    fn navigate_capabilities_up(&mut self) {
        match self.ui_state.capability_view {
            CapabilityView::Categories => {
                let current = self.ui_state.capabilities_state.selected().unwrap_or(0);
                if current > 0 {
                    self.ui_state.capabilities_state.select(Some(current - 1));
                }
            }
            CapabilityView::DetailedList(_) => {
                let current = self
                    .ui_state
                    .capability_detail_state
                    .selected()
                    .unwrap_or(0);
                if current > 0 {
                    self.ui_state
                        .capability_detail_state
                        .select(Some(current - 1));
                }
            }
        }
    }

    /// Navigate capabilities down
    fn navigate_capabilities_down(&mut self) {
        match self.ui_state.capability_view {
            CapabilityView::Categories => {
                let current = self.ui_state.capabilities_state.selected().unwrap_or(0);
                if current < 2 {
                    // We have 3 categories (0, 1, 2)
                    self.ui_state.capabilities_state.select(Some(current + 1));
                }
            }
            CapabilityView::DetailedList(_) => {
                let current = self
                    .ui_state
                    .capability_detail_state
                    .selected()
                    .unwrap_or(0);
                let max_items = self.ui_state.capability_indices.len();
                if current + 1 < max_items {
                    self.ui_state
                        .capability_detail_state
                        .select(Some(current + 1));
                }
            }
        }
    }

    /// Navigate parameter form up
    fn navigate_parameter_form_up(&mut self) {
        if self.ui_state.param_selected_field > 0 {
            self.ui_state.param_selected_field -= 1;
            // Automatically enter edit mode for better UX
            self.ui_state.param_edit_mode = true;
        }
    }

    /// Navigate parameter form down
    fn navigate_parameter_form_down(&mut self) {
        if self.ui_state.param_selected_field + 1 < self.ui_state.param_field_names.len() {
            self.ui_state.param_selected_field += 1;
            // Automatically enter edit mode for better UX
            self.ui_state.param_edit_mode = true;
        }
    }

    /// Navigate response viewer up (vertical scrolling)
    fn navigate_response_viewer_up(&mut self) {
        if self.ui_state.response_viewer_vertical_pos > 0 {
            self.ui_state.response_viewer_vertical_pos -= 1;
        }
    }

    /// Navigate response viewer down (vertical scrolling)
    fn navigate_response_viewer_down(&mut self) {
        self.ui_state.response_viewer_vertical_pos += 1;
        // Max bounds will be checked in the draw function
    }

    /// Navigate response viewer left (horizontal scrolling)
    fn navigate_response_viewer_left(&mut self) {
        if self.ui_state.response_viewer_horizontal_pos > 0 {
            self.ui_state.response_viewer_horizontal_pos -= 1;
        }
    }

    /// Navigate response viewer right (horizontal scrolling)
    fn navigate_response_viewer_right(&mut self) {
        self.ui_state.response_viewer_horizontal_pos += 1;
        // Max bounds will be checked in the draw function
    }

    /// Navigate response viewer page up (fast vertical scrolling)
    fn navigate_response_viewer_page_up(&mut self) {
        self.ui_state.response_viewer_vertical_pos = self
            .ui_state
            .response_viewer_vertical_pos
            .saturating_sub(10);
    }

    /// Navigate response viewer page down (fast vertical scrolling)
    fn navigate_response_viewer_page_down(&mut self) {
        self.ui_state.response_viewer_vertical_pos += 10;
        // Max bounds will be checked in the draw function
    }

    /// Navigate response viewer to home (top)
    fn navigate_response_viewer_home(&mut self) {
        self.ui_state.response_viewer_vertical_pos = 0;
        self.ui_state.response_viewer_horizontal_pos = 0;
    }

    /// Navigate response viewer to end (bottom)
    fn navigate_response_viewer_end(&mut self) {
        // Set to a large value, will be clamped in draw function
        self.ui_state.response_viewer_vertical_pos = usize::MAX;
    }

    /// Perform search based on current search input
    fn perform_search(&mut self) {
        let query = self
            .ui_state
            .search_input
            .lines()
            .join(" ")
            .trim()
            .to_string();

        if query.is_empty() {
            self.ui_state.search_results.clear();
            return;
        }

        // Perform search with limit of 20 results
        self.ui_state.search_results = self.search_engine.search(&query, 20);

        // Reset selection to first result
        if !self.ui_state.search_results.is_empty() {
            self.ui_state.search_results_state.select(Some(0));
        } else {
            self.ui_state.search_results_state.select(None);
        }
    }

    /// Select a result from search results
    fn select_search_result(&mut self, result: SearchResult) {
        if let Some(item) = self.search_engine.get_item(result.index) {
            // Determine the capability reference based on category and index
            match item.category {
                SearchCategory::Tool => {
                    // Find the tool index in our capabilities
                    if let Some(tool_index) = self
                        .capabilities
                        .tools
                        .iter()
                        .position(|t| t.name == item.name)
                    {
                        self.ui_state.selected_capability = Some(CapabilityRef::Tool(tool_index));
                        self.ui_state.capability_view =
                            CapabilityView::DetailedList(CapabilityCategory::Tools);
                        self.open_parameter_form();
                    }
                }
                SearchCategory::Resource => {
                    // Find the resource index in our capabilities
                    if let Some(resource_index) = self
                        .capabilities
                        .resources
                        .iter()
                        .position(|r| r.uri == item.name)
                    {
                        self.ui_state.selected_capability =
                            Some(CapabilityRef::Resource(resource_index));
                        self.ui_state.capability_view =
                            CapabilityView::DetailedList(CapabilityCategory::Resources);
                        // Resources don't typically need parameter forms, so just select them
                    }
                }
                SearchCategory::Prompt => {
                    // Find the prompt index in our capabilities
                    if let Some(prompt_index) = self
                        .capabilities
                        .prompts
                        .iter()
                        .position(|p| p.name == item.name)
                    {
                        self.ui_state.selected_capability =
                            Some(CapabilityRef::Prompt(prompt_index));
                        self.ui_state.capability_view =
                            CapabilityView::DetailedList(CapabilityCategory::Prompts);
                        self.open_parameter_form();
                    }
                }
            }
        }

        // Deactivate search mode
        self.ui_state.search_active = false;
        self.ui_state.search_results.clear();
    }

    /// Navigate page left (previous page)
    fn navigate_page_left(&mut self) {
        if self.ui_state.capability_page > 0 {
            self.ui_state.capability_page -= 1;
            self.ui_state.capability_detail_state.select(Some(0));
        }
    }

    /// Navigate page right (next page)
    fn navigate_page_right(&mut self) {
        if let CapabilityView::DetailedList(ref category) = self.ui_state.capability_view {
            let total_count = match category {
                CapabilityCategory::Tools => self.capabilities.tools.len(),
                CapabilityCategory::Resources => self.capabilities.resources.len(),
                CapabilityCategory::Prompts => self.capabilities.prompts.len(),
            };

            let max_pages = total_count.div_ceil(self.ui_state.capability_page_size);
            if self.ui_state.capability_page + 1 < max_pages {
                self.ui_state.capability_page += 1;
                self.ui_state.capability_detail_state.select(Some(0));
            }
        }
    }

    /// Select a capability category or individual capability
    fn select_capability(&mut self) {
        match self.ui_state.capability_view {
            CapabilityView::Categories => {
                if let Some(selected) = self.ui_state.capabilities_state.selected() {
                    let category = match selected {
                        0 => CapabilityCategory::Tools,
                        1 => CapabilityCategory::Resources,
                        2 => CapabilityCategory::Prompts,
                        _ => return,
                    };

                    self.ui_state.capability_view = CapabilityView::DetailedList(category);
                    self.ui_state.capability_page = 0;
                    self.ui_state.capability_detail_state.select(Some(0));
                }
            }
            CapabilityView::DetailedList(_) => {
                if let Some(selected) = self.ui_state.capability_detail_state.selected() {
                    if let Some(Some(capability_ref)) =
                        self.ui_state.capability_indices.get(selected)
                    {
                        self.ui_state.selected_capability = Some(capability_ref.clone());
                        self.open_parameter_form();
                    }
                }
            }
        }
    }

    /// Open parameter form for selected capability
    fn open_parameter_form(&mut self) {
        if let Some(ref capability_ref) = self.ui_state.selected_capability {
            // Clear existing form data
            self.ui_state.param_fields.clear();
            self.ui_state.param_field_names.clear();
            self.ui_state.param_selected_field = 0;
            self.ui_state.param_edit_mode = false;

            match capability_ref {
                CapabilityRef::Tool(index) => {
                    if let Some(tool) = self.capabilities.tools.get(*index) {
                        tracing::debug!(
                            "Opening parameter form for tool: {} - {}",
                            tool.name,
                            tool.description.as_deref().unwrap_or("No description")
                        );
                        tracing::debug!("Tool parameters field: {:?}", tool.parameters);

                        if let Some(ref params) = tool.parameters {
                            tracing::debug!("Tool has parameters, building form from schema");
                            let params_clone = params.clone();
                            self.build_parameter_form_from_schema(&params_clone);
                        } else {
                            tracing::debug!("Tool has no parameters field");
                        }
                    } else {
                        tracing::debug!("Tool not found at index {}", index);
                    }
                }
                CapabilityRef::Resource(_index) => {
                    // Resources typically don't have parameters in the same way
                    tracing::debug!("Opening parameter form for resource (not implemented)");
                }
                CapabilityRef::Prompt(_index) => {
                    // Prompts may have arguments
                    tracing::debug!("Opening parameter form for prompt (not implemented)");
                }
            }

            tracing::debug!(
                "Final param_fields count: {}",
                self.ui_state.param_fields.len()
            );

            // Auto-enter edit mode if we have parameters for better UX
            if !self.ui_state.param_field_names.is_empty() {
                self.ui_state.param_edit_mode = true;
            }

            self.ui_state.parameter_dialog_open = true;
        }
    }

    /// Build parameter form from JSON schema
    fn build_parameter_form_from_schema(&mut self, schema: &Value) {
        // Debug: Log the schema to see its structure
        tracing::debug!(
            "Building parameter form from schema: {}",
            serde_json::to_string_pretty(schema).unwrap_or_else(|_| "Invalid JSON".to_string())
        );

        // Handle different schema formats
        let properties = if let Some(props) = schema.get("properties") {
            props
        } else if schema.is_object() && !schema.as_object().unwrap().is_empty() {
            // Schema might be the properties object directly
            schema
        } else {
            tracing::debug!("No properties found in schema");
            return;
        };

        if let Some(props_obj) = properties.as_object() {
            let required_fields: Vec<String> = schema
                .get("required")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            tracing::debug!(
                "Found {} properties, {} required: {:?}",
                props_obj.len(),
                required_fields.len(),
                required_fields
            );

            // First, collect and sort parameter names for consistent ordering
            let mut param_names: Vec<String> = props_obj.keys().cloned().collect();
            // Sort so required fields come first, then alphabetical
            param_names.sort_by(|a, b| {
                let a_required = required_fields.contains(a);
                let b_required = required_fields.contains(b);
                match (a_required, b_required) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.cmp(b),
                }
            });

            for param_name in param_names {
                let param_schema = &props_obj[&param_name];
                let description = param_schema
                    .get("description")
                    .and_then(|d| d.as_str())
                    .map(String::from)
                    .or_else(|| {
                        // Try alternative description paths
                        param_schema
                            .get("title")
                            .and_then(|t| t.as_str())
                            .map(String::from)
                    });

                let param_type = param_schema
                    .get("type")
                    .and_then(|t| t.as_str())
                    .map(String::from)
                    .or_else(|| {
                        // Infer type from other properties
                        if param_schema.get("enum").is_some() {
                            Some("enum".to_string())
                        } else if param_schema.get("properties").is_some() {
                            Some("object".to_string())
                        } else if param_schema.get("items").is_some() {
                            Some("array".to_string())
                        } else {
                            Some("string".to_string()) // Default fallback
                        }
                    });

                let is_required = required_fields.contains(&param_name);

                tracing::debug!(
                    "Adding parameter: {} (type: {:?}, required: {}, desc: {:?})",
                    param_name,
                    param_type,
                    is_required,
                    description
                );

                // Add to ordered list for consistent access
                self.ui_state.param_field_names.push(param_name.clone());

                // Add to HashMap for quick lookup
                self.ui_state.param_fields.insert(
                    param_name.clone(),
                    ParamField {
                        value: String::new(),
                        required: is_required,
                        description,
                        param_type,
                    },
                );
            }
        } else {
            tracing::debug!("Properties is not an object: {:?}", properties);
        }

        tracing::debug!(
            "Final param_fields count: {}",
            self.ui_state.param_fields.len()
        );
    }

    /// Execute selected capability with form parameters
    async fn execute_selected_capability(&mut self) -> Result<()> {
        if let Some(ref capability_ref) = self.ui_state.selected_capability.clone() {
            // Build parameters object from form fields
            let mut params = serde_json::Map::new();

            for (param_name, field) in &self.ui_state.param_fields {
                if !field.value.is_empty() {
                    // Try to parse as JSON first, then fall back to string
                    let param_value =
                        if let Ok(json_value) = serde_json::from_str::<Value>(&field.value) {
                            json_value
                        } else {
                            Value::String(field.value.clone())
                        };
                    params.insert(param_name.clone(), param_value);
                }
            }

            let params_value = Value::Object(params);

            match capability_ref {
                CapabilityRef::Tool(index) => {
                    if let Some(tool) = self.capabilities.tools.get(*index) {
                        let tool_name = tool.full_name.clone(); // Use full name with prefix for API call
                        let params_str = params_value.to_string();
                        self.execute_tool(&tool_name, &params_str).await?;
                    }
                }
                CapabilityRef::Resource(index) => {
                    if let Some(resource) = self.capabilities.resources.get(*index) {
                        let resource_uri = resource.uri.clone();
                        self.get_resource(&resource_uri).await?;
                    }
                }
                CapabilityRef::Prompt(index) => {
                    if let Some(prompt) = self.capabilities.prompts.get(*index) {
                        let prompt_name = prompt.name.clone();
                        let params_str = params_value.to_string();
                        self.get_prompt(&prompt_name, &params_str).await?;
                    }
                }
            }

            // Close the parameter dialog
            self.ui_state.parameter_dialog_open = false;
            self.ui_state.selected_capability = None;
        }

        Ok(())
    }

    /// Execute a command from the input area
    async fn execute_command(&mut self) -> Result<()> {
        let input = self.ui_state.input_area.lines().join(" ");
        if input.trim().is_empty() {
            return Ok(());
        }

        // Parse command (simplified parser for demo)
        // Format: tools.toolname {"param": "value"}
        // Or: resources.resourcename
        // Or: prompts.promptname {"param": "value"}

        let parts: Vec<&str> = input.trim().splitn(2, ' ').collect();
        if parts.is_empty() {
            return Ok(());
        }

        let command_parts: Vec<&str> = parts[0].split('.').collect();
        if command_parts.len() != 2 {
            self.add_error("Invalid command format. Use: tools.toolname {params}".to_string());
            return Ok(());
        }

        let (category, name) = (command_parts[0], command_parts[1]);
        let params = if parts.len() > 1 { parts[1] } else { "{}" };

        match category {
            "tools" => {
                // Find the tool by clean name and use full name for execution
                if let Some(tool) = self.capabilities.tools.iter().find(|t| t.name == name) {
                    let full_name = tool.full_name.clone();
                    self.execute_tool(&full_name, params).await?;
                } else {
                    self.add_error(format!("Tool '{}' not found", name));
                }
            }
            "resources" => {
                self.get_resource(name).await?;
            }
            "prompts" => {
                self.get_prompt(name, params).await?;
            }
            _ => {
                self.add_error(format!("Unknown category: {}", category));
            }
        }

        // Clear input
        self.ui_state.input_area = TextArea::default();
        self.ui_state.input_area.set_placeholder_text(
            "Enter MCP command (e.g., tools.calculator {\"a\": 5, \"b\": 3})",
        );
        self.ui_state.input_area.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("Interactive Terminal"),
        );

        Ok(())
    }

    /// Execute a tool with parameters
    async fn execute_tool(&mut self, tool_name: &str, params_str: &str) -> Result<()> {
        if let Some(client) = &mut self.client {
            // Parse parameters
            let mut params: Value = serde_json::from_str(params_str)
                .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));

            // Inject environment variables
            if let Value::Object(ref mut map) = params {
                for (key, value) in &self.env_variables {
                    if !map.contains_key(key) {
                        map.insert(key.clone(), Value::String(value.clone()));
                    }
                }
            }

            tracing::info!("=== EXECUTING TOOL ===");
            tracing::info!("Tool name: '{}'", tool_name);
            tracing::info!(
                "Parameters: {}",
                serde_json::to_string_pretty(&params)
                    .unwrap_or_else(|_| "Failed to serialize params".to_string())
            );

            match client.call_tool(tool_name, params.clone()).await {
                Ok(result) => {
                    tracing::info!("=== TOOL EXECUTION RESULT DEBUG ===");
                    tracing::info!("Tool: '{}'", tool_name);
                    tracing::info!("Content items: {}", result.content.len());
                    tracing::info!("is_error: {:?}", result.is_error);

                    // Log the raw result structure for debugging
                    tracing::debug!("Raw result structure: {:#?}", result);

                    // Log the actual content for debugging
                    if result.content.is_empty() {
                        tracing::warn!(
                            "⚠️  Tool returned EMPTY content array - this might indicate:"
                        );
                        tracing::warn!("   - Tool executed but produced no output");
                        tracing::warn!("   - Response parsing issue");
                        tracing::warn!("   - Server-side tool implementation issue");
                    } else {
                        tracing::info!("📄 Tool returned {} content items:", result.content.len());
                        for (i, content_item) in result.content.iter().enumerate() {
                            match content_item {
                                mcp_core::messages::tools::ToolResult::Text { text } => {
                                    tracing::info!(
                                        "Content[{}]: Text with {} chars: '{}'",
                                        i,
                                        text.len(),
                                        if text.len() > 200 { &text[..200] } else { text }
                                    );
                                }
                                mcp_core::messages::tools::ToolResult::Image {
                                    mime_type,
                                    data,
                                } => {
                                    tracing::info!(
                                        "Content[{}]: Image {} with {} bytes",
                                        i,
                                        mime_type,
                                        data.len()
                                    );
                                }
                                mcp_core::messages::tools::ToolResult::Resource { resource } => {
                                    tracing::info!("Content[{}]: Resource {}", i, resource.uri);
                                }
                            }
                        }
                    }

                    // Convert result to JSON value for better handling
                    let result_json = match serde_json::to_value(&result) {
                        Ok(json) => {
                            tracing::info!("=== SERIALIZED RESULT JSON ===");
                            tracing::info!(
                                "{}",
                                serde_json::to_string_pretty(&json)
                                    .unwrap_or_else(|_| "Failed to pretty print".to_string())
                            );

                            // Check if there are any unexpected fields
                            if let Some(obj) = json.as_object() {
                                tracing::debug!(
                                    "Response contains {} top-level fields:",
                                    obj.len()
                                );
                                for (key, value) in obj {
                                    tracing::debug!(
                                        "  {}: {}",
                                        key,
                                        if value.is_string()
                                            || value.is_number()
                                            || value.is_boolean()
                                        {
                                            value.to_string()
                                        } else {
                                            format!(
                                                "{} (type: {})",
                                                if value.is_array() {
                                                    "array"
                                                } else if value.is_object() {
                                                    "object"
                                                } else {
                                                    "other"
                                                },
                                                if value.is_array() {
                                                    format!(
                                                        "length {}",
                                                        value.as_array().unwrap().len()
                                                    )
                                                } else {
                                                    "".to_string()
                                                }
                                            )
                                        }
                                    );
                                }
                            }

                            json
                        }
                        Err(e) => {
                            tracing::error!("Failed to serialize tool result: {}", e);
                            self.add_error(format!("Failed to serialize tool result: {}", e));
                            return Ok(());
                        }
                    };

                    let success_summary = self.format_tool_response_summary(&result);
                    tracing::info!("=== SUCCESS SUMMARY ===");
                    tracing::info!("{}", success_summary);

                    // Additional analysis for empty responses
                    if result.content.is_empty() {
                        tracing::warn!("🔍 INVESTIGATING EMPTY RESPONSE:");
                        tracing::warn!("  - Tool name used: '{}'", tool_name);
                        tracing::warn!(
                            "  - Parameters sent: {}",
                            serde_json::to_string(&params)
                                .unwrap_or_else(|_| "Failed to serialize".to_string())
                        );
                        tracing::warn!("  - is_error flag: {:?}", result.is_error);
                        tracing::warn!("  - This suggests the tool executed successfully but returned no content");
                        tracing::warn!("  - Check if the tool implementation on the server actually returns content");
                    }

                    self.add_message(MessageEntry {
                        timestamp: Instant::now(),
                        message_type: MessageType::CallTool,
                        request: None,
                        response: None,
                        raw_response: Some(result_json.clone()),
                        error: None,
                        success: Some(success_summary),
                    });

                    // Auto-open response viewer for successful tool execution
                    self.ui_state.selected_response = Some(result_json);
                    self.ui_state.response_viewer_open = true;
                    // Reset scroll positions to start from top
                    self.ui_state.response_viewer_vertical_pos = 0;
                    self.ui_state.response_viewer_horizontal_pos = 0;
                    // Select the latest message in history
                    if !self.message_history.is_empty() {
                        self.ui_state
                            .message_history_state
                            .select(Some(self.message_history.len() - 1));
                    }

                    tracing::info!(
                        "Tool '{}' executed successfully - response viewer opened",
                        tool_name
                    );
                }
                Err(e) => {
                    tracing::error!("=== TOOL EXECUTION FAILED ===");
                    tracing::error!("Tool: '{}'", tool_name);
                    tracing::error!("Error: {}", e);
                    tracing::error!("Error debug: {:?}", e);

                    // Try to get more specific error information
                    let error_msg = if e.to_string().contains("Serialization error") {
                        format!("🔧 Tool '{}' execution failed with SERIALIZATION ERROR: {}\n💡 This usually means:\n  - Parameter format is incorrect\n  - Tool name is malformed\n  - Server rejected the request format", tool_name, e)
                    } else {
                        format!("Tool '{}' execution failed: {}", tool_name, e)
                    };

                    self.add_error(error_msg);
                }
            }
        }
        Ok(())
    }

    /// Format a tool response summary for display
    fn format_tool_response_summary(
        &self,
        result: &mcp_core::messages::tools::CallToolResponse,
    ) -> String {
        use mcp_core::messages::tools::ToolResult;

        let content_summary = if result.content.is_empty() {
            "No content returned (empty response)".to_string()
        } else {
            let mut summaries = Vec::new();
            for (i, content_item) in result.content.iter().enumerate() {
                let item_summary = match content_item {
                    ToolResult::Text { text } => {
                        if text.is_empty() {
                            format!("[{}] Empty text", i)
                        } else if text.len() > 100 {
                            format!("[{}] Text ({} chars): {}...", i, text.len(), &text[..97])
                        } else {
                            format!("[{}] Text: {}", i, text)
                        }
                    }
                    ToolResult::Image { data, mime_type } => {
                        format!("[{}] Image {} ({} bytes)", i, mime_type, data.len())
                    }
                    ToolResult::Resource { resource } => {
                        format!("[{}] Resource: {}", i, resource.uri)
                    }
                };
                summaries.push(item_summary);
            }
            summaries.join(", ")
        };

        let meta_info = if result.is_error == Some(true) {
            " [ERROR]".to_string()
        } else {
            "".to_string()
        };

        format!(
            "✅ Tool executed successfully{} - {}",
            meta_info, content_summary
        )
    }

    /// Format a resource response summary for display
    fn format_resource_response_summary(
        &self,
        result: &mcp_core::messages::resources::ReadResourceResponse,
    ) -> String {
        let content_count = result.contents.len();
        if content_count == 0 {
            "✅ Resource retrieved (no content)".to_string()
        } else if content_count == 1 {
            let content = &result.contents[0];
            let mime_info = content
                .mime_type()
                .map(|m| format!(" ({})", m))
                .unwrap_or_default();

            // Check content type and format accordingly
            let content_desc = match content {
                mcp_core::messages::resources::ResourceContent::Text { text, .. } => {
                    if text.len() > 100 {
                        format!("{} chars", text.len())
                    } else {
                        text.clone()
                    }
                }
                mcp_core::messages::resources::ResourceContent::Blob { blob, .. } => {
                    format!("{} bytes (binary)", blob.len())
                }
            };

            format!("✅ Resource retrieved: {}{}", content_desc, mime_info)
        } else {
            format!("✅ Resource retrieved: {} content items", content_count)
        }
    }

    /// Format a prompt response summary for display
    fn format_prompt_response_summary(
        &self,
        result: &mcp_core::messages::prompts::GetPromptResponse,
    ) -> String {
        let msg_count = result.messages.len();
        if msg_count == 0 {
            "✅ Prompt retrieved (no messages)".to_string()
        } else if msg_count == 1 {
            let message = &result.messages[0];
            // Get the content to describe the message
            match &message.content {
                mcp_core::messages::prompts::PromptContent::Text { text } => {
                    if text.len() > 100 {
                        format!("✅ Prompt retrieved: {} chars", text.len())
                    } else {
                        format!("✅ Prompt retrieved: {}", text)
                    }
                }
                mcp_core::messages::prompts::PromptContent::Image { .. } => {
                    "✅ Prompt retrieved: Image message".to_string()
                }
                mcp_core::messages::prompts::PromptContent::Resource { .. } => {
                    "✅ Prompt retrieved: Resource message".to_string()
                }
            }
        } else {
            format!("✅ Prompt retrieved: {} messages", msg_count)
        }
    }

    /// Get a resource
    async fn get_resource(&mut self, resource_uri: &str) -> Result<()> {
        if let Some(client) = &mut self.client {
            match client.read_resource(resource_uri).await {
                Ok(result) => {
                    let result_json = match serde_json::to_value(&result) {
                        Ok(json) => json,
                        Err(e) => {
                            self.add_error(format!("Failed to serialize resource result: {}", e));
                            return Ok(());
                        }
                    };

                    let summary = self.format_resource_response_summary(&result);

                    self.add_message(MessageEntry {
                        timestamp: Instant::now(),
                        message_type: MessageType::GetResource,
                        request: None,
                        response: None,
                        raw_response: Some(result_json.clone()),
                        error: None,
                        success: Some(summary),
                    });

                    // Auto-open response viewer for successful resource retrieval
                    self.ui_state.selected_response = Some(result_json);
                    self.ui_state.response_viewer_open = true;
                    // Reset scroll positions to start from top
                    self.ui_state.response_viewer_vertical_pos = 0;
                    self.ui_state.response_viewer_horizontal_pos = 0;
                    // Select the latest message in history
                    if !self.message_history.is_empty() {
                        self.ui_state
                            .message_history_state
                            .select(Some(self.message_history.len() - 1));
                    }

                    tracing::info!(
                        "Resource '{}' retrieved successfully - response viewer opened",
                        resource_uri
                    );
                }
                Err(e) => {
                    self.add_error(format!(
                        "Resource '{}' retrieval failed: {}",
                        resource_uri, e
                    ));
                }
            }
        }
        Ok(())
    }

    /// Get a prompt
    async fn get_prompt(&mut self, prompt_name: &str, params_str: &str) -> Result<()> {
        if let Some(client) = &mut self.client {
            let params: Value = serde_json::from_str(params_str)
                .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));

            match client.get_prompt(prompt_name, Some(params)).await {
                Ok(result) => {
                    let result_json = match serde_json::to_value(&result) {
                        Ok(json) => json,
                        Err(e) => {
                            self.add_error(format!("Failed to serialize prompt result: {}", e));
                            return Ok(());
                        }
                    };

                    let summary = self.format_prompt_response_summary(&result);

                    self.add_message(MessageEntry {
                        timestamp: Instant::now(),
                        message_type: MessageType::GetPrompt,
                        request: None,
                        response: None,
                        raw_response: Some(result_json.clone()),
                        error: None,
                        success: Some(summary),
                    });

                    // Auto-open response viewer for successful prompt retrieval
                    self.ui_state.selected_response = Some(result_json);
                    self.ui_state.response_viewer_open = true;
                    // Reset scroll positions to start from top
                    self.ui_state.response_viewer_vertical_pos = 0;
                    self.ui_state.response_viewer_horizontal_pos = 0;
                    // Select the latest message in history
                    if !self.message_history.is_empty() {
                        self.ui_state
                            .message_history_state
                            .select(Some(self.message_history.len() - 1));
                    }

                    tracing::info!(
                        "Prompt '{}' retrieved successfully - response viewer opened",
                        prompt_name
                    );
                }
                Err(e) => {
                    self.add_error(format!("Prompt '{}' retrieval failed: {}", prompt_name, e));
                }
            }
        }
        Ok(())
    }

    /// Add a message to history
    fn add_message(&mut self, message: MessageEntry) {
        self.message_history.push(message);
        self.message_count += 1;

        // Limit history size
        if self.message_history.len() > 1000 {
            self.message_history.remove(0);
        }
    }

    /// Add an error message
    fn add_error(&mut self, error: String) {
        self.error_count += 1;
        self.add_message(MessageEntry {
            timestamp: Instant::now(),
            message_type: MessageType::Other("Error".to_string()),
            request: None,
            response: None,
            raw_response: None,
            error: Some(error),
            success: None,
        });
    }

    /// Save current session
    async fn save_session(&self) -> Result<()> {
        // TODO: Implement session saving
        tracing::info!("Session saved (not implemented yet)");
        Ok(())
    }

    /// Draw the main UI
    fn draw_ui(&mut self, f: &mut Frame) {
        let size = f.area();

        // Main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Status bar
            ])
            .split(size);

        // Draw header
        self.draw_header(f, chunks[0]);

        // Draw main content
        self.draw_main_content(f, chunks[1]);

        // Draw status bar
        self.draw_status_bar(f, chunks[2]);

        // Draw dialogs
        if self.ui_state.help_dialog_open {
            self.draw_help_dialog(f, size);
        }

        if self.ui_state.env_dialog_open {
            self.draw_env_dialog(f, size);
        }

        if self.ui_state.parameter_dialog_open {
            self.draw_parameter_form_dialog(f, size);
        }

        if self.ui_state.search_active {
            self.draw_search_popup(f, size);
        }

        if self.ui_state.response_viewer_open {
            self.draw_response_viewer_dialog(f, size);
        }
    }

    /// Draw the header
    fn draw_header(&self, f: &mut Frame, area: Rect) {
        let server_info = match &self.transport_config {
            TransportConfig::Stdio(config) => format!("stdio:{}", config.command),
            TransportConfig::HttpSse(config) => format!("http+sse:{}", config.base_url),
            TransportConfig::HttpStream(config) => format!("http-stream:{}", config.base_url),
        };

        let status = match &self.state {
            AppState::Initializing => "Initializing".to_string(),
            AppState::Connecting => "Connecting".to_string(),
            AppState::Discovering => {
                if self.discovery_step.is_empty() {
                    "Discovering".to_string()
                } else {
                    format!("Discovering: {}", self.discovery_step)
                }
            }
            AppState::Ready => "Connected".to_string(),
            AppState::Error(_) => "Error".to_string(),
            AppState::ShuttingDown => "Shutting Down".to_string(),
        };

        let session_info = if let Some(ref session_id) = self.session_id {
            format!(
                " | Session: {}",
                if session_id.len() > 16 {
                    &session_id[..16]
                } else {
                    session_id
                }
            )
        } else {
            String::new()
        };

        let header_text = format!(
            "Server: {} | Transport: {} | Status: {} | Protocol: MCP-2024-11-05{}",
            server_info,
            self.transport_config.transport_type(),
            status,
            session_info
        );

        let header = Paragraph::new(header_text)
            .style(Style::default().fg(Color::White).bg(Color::Blue))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("MCP Client Debugger"),
            );

        f.render_widget(header, area);
    }

    /// Draw the main content area
    fn draw_main_content(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50), // Top row
                Constraint::Percentage(50), // Bottom row
            ])
            .split(area);

        // Top row layout
        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25), // Protocol Flow
                Constraint::Percentage(50), // Message Inspector
                Constraint::Percentage(25), // Controls
            ])
            .split(chunks[0]);

        // Bottom row layout
        let bottom_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25), // Capabilities
                Constraint::Percentage(75), // Interactive Terminal
            ])
            .split(chunks[1]);

        // Draw panels
        self.draw_protocol_flow(f, top_chunks[0]);
        self.draw_message_inspector(f, top_chunks[1]);
        self.draw_controls(f, top_chunks[2]);
        self.draw_capabilities(f, bottom_chunks[0]);
        self.draw_interactive_terminal(f, bottom_chunks[1]);
    }

    /// Draw protocol flow panel
    fn draw_protocol_flow(&mut self, f: &mut Frame, area: Rect) {
        let steps = [
            "1. ✓ Initialize",
            "2. ✓ Negotiate",
            "3. ✓ Discover",
            "4. → Test Tools",
            "5.   Validate",
        ];

        let items: Vec<ListItem> = steps.iter().map(|step| ListItem::new(*step)).collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Protocol Flow"),
            )
            .highlight_style(Style::default().fg(Color::Yellow));

        f.render_stateful_widget(list, area, &mut self.ui_state.protocol_flow_state);
    }

    /// Draw message inspector panel - shows detailed capability lists or message inspection
    fn draw_message_inspector(&mut self, f: &mut Frame, area: Rect) {
        match &self.ui_state.capability_view {
            CapabilityView::DetailedList(category) => {
                self.draw_capability_details(f, area, category.clone());
            }
            CapabilityView::Categories => {
                self.draw_message_details(f, area);
            }
        }
    }

    /// Draw detailed capability list for selected category
    fn draw_capability_details(&mut self, f: &mut Frame, area: Rect, category: CapabilityCategory) {
        let mut items = Vec::new();
        let mut capability_refs = Vec::new();

        let selected_detail_index = self
            .ui_state
            .capability_detail_state
            .selected()
            .unwrap_or(0);

        let (title, total_count) = match category {
            CapabilityCategory::Tools => {
                let start = self.ui_state.capability_page * self.ui_state.capability_page_size;

                for (display_idx, (i, tool)) in self
                    .capabilities
                    .tools
                    .iter()
                    .enumerate()
                    .skip(start)
                    .take(self.ui_state.capability_page_size)
                    .enumerate()
                {
                    // Format description with truncation
                    let desc = tool
                        .description
                        .as_ref()
                        .map(|d| {
                            if d.len() > 80 {
                                format!("{}...", &d[..77])
                            } else {
                                d.clone()
                            }
                        })
                        .unwrap_or_else(|| "No description available".to_string());

                    // Extract parameter information
                    let params_info = if let Some(ref params) = tool.parameters {
                        if let Some(properties) = params.get("properties") {
                            if let Some(obj) = properties.as_object() {
                                let required_params: Vec<String> = params
                                    .get("required")
                                    .and_then(|r| r.as_array())
                                    .map(|arr| {
                                        arr.iter()
                                            .filter_map(|v| v.as_str().map(String::from))
                                            .collect()
                                    })
                                    .unwrap_or_default();

                                let param_count = obj.len();
                                let required_count = required_params.len();

                                if param_count > 0 {
                                    format!(
                                        "📋 {} params ({} required)",
                                        param_count, required_count
                                    )
                                } else {
                                    "📋 No parameters".to_string()
                                }
                            } else {
                                "📋 Parameters available".to_string()
                            }
                        } else {
                            "📋 No parameters".to_string()
                        }
                    } else {
                        "📋 No parameters".to_string()
                    };

                    // Create multi-line item with selection indicator
                    let selection_indicator = if display_idx == selected_detail_index {
                        "→ "
                    } else {
                        "  "
                    };
                    let tool_name_style = if display_idx == selected_detail_index {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(ratatui::style::Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(ratatui::style::Modifier::BOLD)
                    };

                    let item_text = vec![
                        Line::from(vec![
                            Span::raw(selection_indicator),
                            Span::styled(format!("🔧 {}", tool.name), tool_name_style),
                        ]),
                        Line::from(vec![
                            Span::raw("   "),
                            Span::styled(params_info, Style::default().fg(Color::Green)),
                        ]),
                        Line::from(vec![
                            Span::raw("   "),
                            Span::styled(desc, Style::default().fg(Color::White)),
                        ]),
                        Line::from(""), // Empty line for spacing
                    ];

                    items.push(ListItem::new(item_text));
                    capability_refs.push(CapabilityRef::Tool(i));
                }

                (format!("🔧 Tools - Page {}/{} [↑/↓ Navigate, Enter Select & Configure, ←/→ Page, Esc Back]", 
                        self.ui_state.capability_page + 1,
                        self.capabilities.tools.len().div_ceil(self.ui_state.capability_page_size)),
                 self.capabilities.tools.len())
            }
            CapabilityCategory::Resources => {
                let start = self.ui_state.capability_page * self.ui_state.capability_page_size;

                for (display_idx, (i, resource)) in self
                    .capabilities
                    .resources
                    .iter()
                    .enumerate()
                    .skip(start)
                    .take(self.ui_state.capability_page_size)
                    .enumerate()
                {
                    let name = resource.name.as_ref().unwrap_or(&resource.uri);
                    let desc = resource
                        .description
                        .as_ref()
                        .map(|d| {
                            if d.len() > 80 {
                                format!("{}...", &d[..77])
                            } else {
                                d.clone()
                            }
                        })
                        .unwrap_or_else(|| "No description available".to_string());

                    let mime_info = resource
                        .mime_type
                        .as_ref()
                        .map(|m| format!("📄 Type: {}", m))
                        .unwrap_or_else(|| "📄 Type: Unknown".to_string());

                    let selection_indicator = if display_idx == selected_detail_index {
                        "→ "
                    } else {
                        "  "
                    };
                    let resource_name_style = if display_idx == selected_detail_index {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(ratatui::style::Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(ratatui::style::Modifier::BOLD)
                    };

                    let item_text = vec![
                        Line::from(vec![
                            Span::raw(selection_indicator),
                            Span::styled(format!("📁 {}", name), resource_name_style),
                        ]),
                        Line::from(vec![
                            Span::raw("   "),
                            Span::styled(
                                format!("🔗 URI: {}", resource.uri),
                                Style::default().fg(Color::Blue),
                            ),
                        ]),
                        Line::from(vec![
                            Span::raw("   "),
                            Span::styled(mime_info, Style::default().fg(Color::Green)),
                        ]),
                        Line::from(vec![
                            Span::raw("   "),
                            Span::styled(desc, Style::default().fg(Color::White)),
                        ]),
                        Line::from(""), // Empty line for spacing
                    ];

                    items.push(ListItem::new(item_text));
                    capability_refs.push(CapabilityRef::Resource(i));
                }

                (
                    format!(
                        "📁 Resources - Page {}/{} [↑/↓ Navigate, Enter Read, ←/→ Page, Esc Back]",
                        self.ui_state.capability_page + 1,
                        self.capabilities
                            .resources
                            .len()
                            .div_ceil(self.ui_state.capability_page_size)
                    ),
                    self.capabilities.resources.len(),
                )
            }
            CapabilityCategory::Prompts => {
                let start = self.ui_state.capability_page * self.ui_state.capability_page_size;

                for (display_idx, (i, prompt)) in self
                    .capabilities
                    .prompts
                    .iter()
                    .enumerate()
                    .skip(start)
                    .take(self.ui_state.capability_page_size)
                    .enumerate()
                {
                    let desc = prompt
                        .description
                        .as_ref()
                        .map(|d| {
                            if d.len() > 80 {
                                format!("{}...", &d[..77])
                            } else {
                                d.clone()
                            }
                        })
                        .unwrap_or_else(|| "No description available".to_string());

                    let args_info = if let Some(ref args) = prompt.arguments {
                        if let Some(properties) = args.get("properties") {
                            if let Some(obj) = properties.as_object() {
                                let required_args: Vec<String> = args
                                    .get("required")
                                    .and_then(|r| r.as_array())
                                    .map(|arr| {
                                        arr.iter()
                                            .filter_map(|v| v.as_str().map(String::from))
                                            .collect()
                                    })
                                    .unwrap_or_default();

                                let arg_count = obj.len();
                                let required_count = required_args.len();

                                if arg_count > 0 {
                                    format!(
                                        "📝 {} arguments ({} required)",
                                        arg_count, required_count
                                    )
                                } else {
                                    "📝 No arguments".to_string()
                                }
                            } else {
                                "📝 Arguments available".to_string()
                            }
                        } else {
                            "📝 No arguments".to_string()
                        }
                    } else {
                        "📝 No arguments".to_string()
                    };

                    let selection_indicator = if display_idx == selected_detail_index {
                        "→ "
                    } else {
                        "  "
                    };
                    let prompt_name_style = if display_idx == selected_detail_index {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(ratatui::style::Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(ratatui::style::Modifier::BOLD)
                    };

                    let item_text = vec![
                        Line::from(vec![
                            Span::raw(selection_indicator),
                            Span::styled(format!("💬 {}", prompt.name), prompt_name_style),
                        ]),
                        Line::from(vec![
                            Span::raw("   "),
                            Span::styled(args_info, Style::default().fg(Color::Green)),
                        ]),
                        Line::from(vec![
                            Span::raw("   "),
                            Span::styled(desc, Style::default().fg(Color::White)),
                        ]),
                        Line::from(""), // Empty line for spacing
                    ];

                    items.push(ListItem::new(item_text));
                    capability_refs.push(CapabilityRef::Prompt(i));
                }

                (format!("💬 Prompts - Page {}/{} [↑/↓ Navigate, Enter Configure & Run, ←/→ Page, Esc Back]", 
                        self.ui_state.capability_page + 1,
                        self.capabilities.prompts.len().div_ceil(self.ui_state.capability_page_size)),
                 self.capabilities.prompts.len())
            }
        };

        // Store capability references for selection
        self.ui_state.capability_indices = capability_refs.into_iter().map(Some).collect();

        let border_style = if self.ui_state.current_focus == FocusedPanel::Capabilities {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let list = List::new(items.clone())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(border_style),
            )
            .highlight_style(Style::default().fg(Color::Black).bg(Color::Green));

        // Update scrollbar state for capability details
        let item_count = items.len().saturating_sub(1);
        self.ui_state.capability_details_scroll = self
            .ui_state
            .capability_details_scroll
            .content_length(item_count);
        let scroll_pos = self
            .ui_state
            .capability_detail_state
            .selected()
            .unwrap_or(0);
        self.ui_state.capability_details_scroll =
            self.ui_state.capability_details_scroll.position(scroll_pos);

        f.render_stateful_widget(list, area, &mut self.ui_state.capability_detail_state);

        // Render the scrollbar for capability details
        f.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            area,
            &mut self.ui_state.capability_details_scroll,
        );

        // Show additional info at bottom
        if total_count > self.ui_state.capability_page_size {
            let info_area = Rect {
                x: area.x + 2,
                y: area.y + area.height - 2,
                width: area.width - 4,
                height: 1,
            };

            let page_info = format!(
                "📊 Showing {} of {} capabilities | Use ← → to navigate pages",
                std::cmp::min(
                    self.ui_state.capability_page_size,
                    total_count
                        - (self.ui_state.capability_page * self.ui_state.capability_page_size)
                ),
                total_count
            );

            let paragraph = Paragraph::new(page_info).style(Style::default().fg(Color::Cyan));
            f.render_widget(paragraph, info_area);
        }
    }

    /// Draw message details (original message inspector functionality)
    fn draw_message_details(&mut self, f: &mut Frame, area: Rect) {
        let _inner = area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });

        let content = if let Some(index) = self.ui_state.selected_message_index {
            if let Some(message) = self.message_history.get(index) {
                let mut text = Vec::new();

                if let Some(request) = &message.request {
                    text.push(Line::from(vec![Span::styled(
                        "Request:",
                        Style::default().fg(Color::Green),
                    )]));
                    text.push(Line::from(
                        serde_json::to_string_pretty(request).unwrap_or_default(),
                    ));
                }

                if let Some(response) = &message.response {
                    text.push(Line::from(""));
                    text.push(Line::from(vec![Span::styled(
                        "Response:",
                        Style::default().fg(Color::Blue),
                    )]));
                    text.push(Line::from(
                        serde_json::to_string_pretty(response).unwrap_or_default(),
                    ));
                }

                if let Some(error) = &message.error {
                    text.push(Line::from(""));
                    text.push(Line::from(vec![Span::styled(
                        "Error:",
                        Style::default().fg(Color::Red),
                    )]));
                    text.push(Line::from(error.clone()));
                }

                if let Some(success) = &message.success {
                    text.push(Line::from(""));
                    text.push(Line::from(vec![Span::styled(
                        "Success:",
                        Style::default().fg(Color::Green),
                    )]));
                    text.push(Line::from(success.clone()));
                }

                Text::from(text)
            } else {
                Text::from("No message selected")
            }
        } else {
            Text::from("Select a message to inspect or capability category to explore")
        };

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Message Inspector"),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    }

    /// Draw controls panel
    fn draw_controls(&self, f: &mut Frame, area: Rect) {
        let controls = [
            "[F1] Help",
            "[F2] Save",
            "[F3] Raw JSON",
            "[F4] Clear",
            "[F5] Env Vars",
            "[Q] Quit",
        ];

        let items: Vec<ListItem> = controls
            .iter()
            .map(|control| ListItem::new(*control))
            .collect();

        let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Controls"));

        f.render_widget(list, area);
    }

    /// Draw capabilities panel - shows categories
    fn draw_capabilities(&mut self, f: &mut Frame, area: Rect) {
        let mut items = Vec::new();

        // Show capability categories with better visual cues
        let tools_count = self.capabilities.tools.len();
        let resources_count = self.capabilities.resources.len();
        let prompts_count = self.capabilities.prompts.len();

        // Add visual indicators for selected item
        let selected_index = self.ui_state.capabilities_state.selected().unwrap_or(0);

        items.push(ListItem::new(Line::from(vec![
            if selected_index == 0 { "→ " } else { "  " }.into(),
            Span::styled(
                "🔧 Tools",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled(
                format!(" ({}) ", tools_count),
                Style::default().fg(Color::White),
            ),
            if tools_count > 0 {
                Span::styled(
                    "- GitHub API, calculations, etc.",
                    Style::default().fg(Color::DarkGray),
                )
            } else {
                Span::styled("- No tools available", Style::default().fg(Color::Red))
            },
        ])));

        items.push(ListItem::new(Line::from(vec![
            if selected_index == 1 { "→ " } else { "  " }.into(),
            Span::styled(
                "📁 Resources",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled(
                format!(" ({}) ", resources_count),
                Style::default().fg(Color::White),
            ),
            if resources_count > 0 {
                Span::styled(
                    "- Files, documents, data sources",
                    Style::default().fg(Color::DarkGray),
                )
            } else {
                Span::styled("- No resources available", Style::default().fg(Color::Red))
            },
        ])));

        items.push(ListItem::new(Line::from(vec![
            if selected_index == 2 { "→ " } else { "  " }.into(),
            Span::styled(
                "💬 Prompts",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled(
                format!(" ({}) ", prompts_count),
                Style::default().fg(Color::White),
            ),
            if prompts_count > 0 {
                Span::styled(
                    "- Templates, conversations, queries",
                    Style::default().fg(Color::DarkGray),
                )
            } else {
                Span::styled("- No prompts available", Style::default().fg(Color::Red))
            },
        ])));

        // Add a search prompt
        items.push(ListItem::new(Line::from(""))); // Empty line
        items.push(ListItem::new(Line::from(vec![
            Span::styled("🔍 Press ", Style::default().fg(Color::Yellow)),
            Span::styled(
                "/",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
            Span::styled(
                " to search all capabilities instantly!",
                Style::default().fg(Color::Yellow),
            ),
        ])));

        let title = if self.ui_state.current_focus == FocusedPanel::Capabilities {
            match self.ui_state.capability_view {
                CapabilityView::Categories => {
                    "🎯 Select Category [↑/↓ Navigate, Enter Browse, / Search, Tab Switch Panel]"
                }
                CapabilityView::DetailedList(_) => {
                    "📋 Capability Browser [Esc Back to Categories, / Search]"
                }
            }
        } else {
            "Categories [Tab to Focus, / Search]"
        };

        let border_style = if self.ui_state.current_focus == FocusedPanel::Capabilities {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(border_style),
            )
            .highlight_style(Style::default().fg(Color::Black).bg(Color::Yellow));

        f.render_stateful_widget(list, area, &mut self.ui_state.capabilities_state);
    }

    /// Draw search popup
    fn draw_search_popup(&mut self, f: &mut Frame, area: Rect) {
        let popup_area = centered_rect(90, 80, area);

        f.render_widget(Clear, popup_area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search input
                Constraint::Min(0),    // Search results
                Constraint::Length(3), // Instructions
            ])
            .split(popup_area);

        // Draw search input
        let search_block = Block::default()
            .borders(Borders::ALL)
            .title("🔍 Search MCP Capabilities - Instant Fuzzy Search")
            .border_style(Style::default().fg(Color::Yellow));

        self.ui_state.search_input.set_block(search_block);
        f.render_widget(&self.ui_state.search_input, chunks[0]);

        // Draw search results
        if self.ui_state.search_results.is_empty() {
            let no_results_text = if self
                .ui_state
                .search_input
                .lines()
                .join("")
                .trim()
                .is_empty()
            {
                format!("💡 Start typing to search across {} capabilities...\n\n🎯 Search Features:\n• Exact name matches (highest priority)\n• Prefix matches (e.g., 'git' finds 'github_*')\n• Description keywords\n• Fuzzy matching (handles typos)\n• Token-based search\n\n📊 Available:\n• 🔧 {} Tools\n• 📁 {} Resources\n• 💬 {} Prompts", 
                    self.search_engine.total_items(),
                    self.capabilities.tools.len(),
                    self.capabilities.resources.len(),
                    self.capabilities.prompts.len())
            } else {
                "❌ No results found.\n\n💡 Try:\n• Different keywords\n• Checking spelling\n• Using partial names\n• Searching by description terms".to_string()
            };

            let no_results = Paragraph::new(no_results_text)
                .style(Style::default().fg(Color::Cyan))
                .alignment(ratatui::layout::Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Search Results")
                        .border_style(Style::default().fg(Color::Yellow)),
                )
                .wrap(ratatui::widgets::Wrap { trim: true });

            f.render_widget(no_results, chunks[1]);
        } else {
            // Show search results
            let results_items: Vec<ListItem> = self
                .ui_state
                .search_results
                .iter()
                .map(|result| {
                    if let Some(item) = self.search_engine.get_item(result.index) {
                        let category_icon = match item.category {
                            SearchCategory::Tool => "🔧",
                            SearchCategory::Resource => "📁",
                            SearchCategory::Prompt => "💬",
                        };

                        let category_color = match item.category {
                            SearchCategory::Tool => Color::Cyan,
                            SearchCategory::Resource => Color::Green,
                            SearchCategory::Prompt => Color::Magenta,
                        };

                        let score_display = format!("{:.0}%", result.score);
                        let description_preview = if item.description.len() > 120 {
                            format!("{}...", &item.description[..117])
                        } else {
                            item.description.clone()
                        };

                        // Create multi-line item for better readability
                        let content = vec![
                            Line::from(vec![
                                Span::styled(category_icon, Style::default().fg(category_color)),
                                Span::styled(" ", Style::default()),
                                Span::styled(
                                    &item.name,
                                    Style::default()
                                        .fg(category_color)
                                        .add_modifier(ratatui::style::Modifier::BOLD),
                                ),
                                Span::styled(" ", Style::default()),
                                Span::styled(
                                    format!("(Match: {})", score_display),
                                    Style::default().fg(Color::Yellow),
                                ),
                                Span::styled(
                                    format!(" - {}", result.match_reason),
                                    Style::default().fg(Color::Cyan),
                                ),
                            ]),
                            Line::from(vec![
                                Span::styled("   ", Style::default()),
                                Span::styled(
                                    description_preview,
                                    Style::default().fg(Color::White),
                                ),
                            ]),
                            Line::from(""), // Empty line for spacing
                        ];

                        ListItem::new(content)
                    } else {
                        ListItem::new("❌ Invalid result")
                    }
                })
                .collect();

            let results_list = List::new(results_items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(
                            "🎯 Search Results ({} matches found, best matches first)",
                            self.ui_state.search_results.len()
                        ))
                        .border_style(Style::default().fg(Color::Yellow)),
                )
                .highlight_style(Style::default().fg(Color::Black).bg(Color::Yellow));

            // Update scrollbar state for search results
            let results_count = self.ui_state.search_results.len().saturating_sub(1);
            self.ui_state.search_results_scroll = self
                .ui_state
                .search_results_scroll
                .content_length(results_count);
            let scroll_pos = self.ui_state.search_results_state.selected().unwrap_or(0);
            self.ui_state.search_results_scroll =
                self.ui_state.search_results_scroll.position(scroll_pos);

            f.render_stateful_widget(
                results_list,
                chunks[1],
                &mut self.ui_state.search_results_state,
            );

            // Render the scrollbar for search results
            f.render_stateful_widget(
                Scrollbar::default()
                    .orientation(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("↑"))
                    .end_symbol(Some("↓")),
                chunks[1],
                &mut self.ui_state.search_results_scroll,
            );
        }

        // Draw instructions
        let instructions = "🔍 Advanced Search: Exact names, partial matches, keywords, and fuzzy matching | [↑/↓] Navigate | [Enter] Select & Configure | [Esc] Close";
        let instructions_paragraph = Paragraph::new(instructions)
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            )
            .alignment(ratatui::layout::Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Instructions"));

        f.render_widget(instructions_paragraph, chunks[2]);
    }

    /// Draw interactive terminal panel
    fn draw_interactive_terminal(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // History
                Constraint::Length(3), // Input
            ])
            .split(area);

        // Draw message history with scrollbar
        let history_items: Vec<ListItem> = self
            .message_history
            .iter()
            .map(|msg| {
                let timestamp = msg.timestamp.duration_since(self.session_start);
                let time_str = format!(
                    "{:02}:{:02}",
                    timestamp.as_secs() / 60,
                    timestamp.as_secs() % 60
                );

                // Add response indicator
                let response_indicator = if msg.raw_response.is_some() {
                    " [R]" // R indicates response available for viewing
                } else {
                    ""
                };

                let (content, style) = if let Some(error) = &msg.error {
                    (
                        format!("[{}] ERROR: {}{}", time_str, error, response_indicator),
                        Style::default().fg(Color::Red),
                    )
                } else if let Some(success) = &msg.success {
                    (
                        format!("[{}] SUCCESS: {}{}", time_str, success, response_indicator),
                        Style::default().fg(Color::Green),
                    )
                } else {
                    (
                        format!("[{}] {}{}", time_str, msg.message_type, response_indicator),
                        Style::default(),
                    )
                };

                ListItem::new(content).style(style)
            })
            .collect();

        let history_list = List::new(history_items)
            .block(Block::default().borders(Borders::ALL).title("Message History [↑/↓ Navigate, ←/→ Scroll, R Response Viewer, Tab Latest Response]"))
            .highlight_style(Style::default().fg(Color::Yellow));

        // Update scrollbar state based on content length and area
        self.ui_state.message_scroll = self
            .ui_state
            .message_scroll
            .content_length(self.message_history.len().saturating_sub(1));
        let scroll_pos = self.ui_state.message_history_state.selected().unwrap_or(0);
        self.ui_state.message_scroll = self.ui_state.message_scroll.position(scroll_pos);

        f.render_stateful_widget(
            history_list,
            chunks[0],
            &mut self.ui_state.message_history_state,
        );

        // Render the scrollbar
        f.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            chunks[0],
            &mut self.ui_state.message_scroll,
        );

        // Draw input area
        let input_style = if self.ui_state.current_focus == FocusedPanel::Input {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let input_block = Block::default()
            .borders(Borders::ALL)
            .title("Interactive Terminal")
            .style(input_style);

        self.ui_state.input_area.set_block(input_block);
        f.render_widget(&self.ui_state.input_area, chunks[1]);
    }

    /// Draw status bar
    fn draw_status_bar(&self, f: &mut Frame, area: Rect) {
        let uptime = self.session_start.elapsed();

        let status_text = match &self.state {
            AppState::Ready => "Ready".to_string(),
            AppState::Connecting => "Connecting".to_string(),
            AppState::Discovering => {
                if self.discovery_step.is_empty() {
                    "Discovering".to_string()
                } else {
                    format!("Discovering: {}", self.discovery_step)
                }
            }
            AppState::Error(e) => format!("Error: {}", e),
            AppState::Initializing => "Initializing".to_string(),
            AppState::ShuttingDown => "Shutting down".to_string(),
        };

        let session_info = if let Some(ref session_id) = self.session_id {
            format!(
                " | Session: {}",
                if session_id.len() > 12 {
                    &session_id[..12]
                } else {
                    session_id
                }
            )
        } else {
            String::new()
        };

        let search_status = if self.ui_state.search_active {
            " | 🔍 SEARCH ACTIVE"
        } else {
            " | Press / to search"
        };

        let response_status = if self.ui_state.response_viewer_open {
            " | 📊 RESPONSE VIEWER (Press V to cycle views, Esc to close)"
        } else {
            " | Press Tab for latest results"
        };

        let full_status = format!(
            "Status: {} | Messages: {} | Errors: {} | Uptime: {:02}:{:02}:{:02} | Env Vars: {}{}{}{}",
            status_text,
            self.message_count,
            self.error_count,
            uptime.as_secs() / 3600,
            (uptime.as_secs() % 3600) / 60,
            uptime.as_secs() % 60,
            self.env_variables.len(),
            session_info,
            search_status,
            response_status
        );

        let status_bar = Paragraph::new(full_status)
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Left)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(status_bar, area);
    }

    /// Draw help dialog
    fn draw_help_dialog(&self, f: &mut Frame, area: Rect) {
        let popup_area = centered_rect(80, 60, area);

        let help_text = vec![
            Line::from("MCP Probe Help"),
            Line::from(""),
            Line::from("🔍 SEARCH CAPABILITIES:"),
            Line::from("  /         - Activate smart search (fuzzy matching)"),
            Line::from("  ↑/↓       - Navigate search results"),
            Line::from("  Enter     - Select and configure capability"),
            Line::from("  Esc       - Exit search mode"),
            Line::from(""),
            Line::from("📊 RESPONSE VIEWER:"),
            Line::from("  Tab       - Open latest response automatically"),
            Line::from("  R         - Open response viewer for selected message [R]"),
            Line::from("  V         - Cycle view modes (Formatted/Raw/Tree/Summary)"),
            Line::from("  ↑/↓       - Scroll vertically"),
            Line::from("  ←/→       - Scroll horizontally"),
            Line::from("  PgUp/PgDn - Fast vertical scrolling"),
            Line::from("  Home/End  - Jump to top/bottom"),
            Line::from("  Esc       - Close response viewer"),
            Line::from(""),
            Line::from("📝 PARAMETER FORMS:"),
            Line::from("  ↑/↓       - Navigate fields (auto-edit mode)"),
            Line::from("  Type      - Edit field values directly"),
            Line::from("  Enter     - Save field and move to next"),
            Line::from("  Tab       - Execute with current values"),
            Line::from("  Esc       - Cancel and go back"),
            Line::from(""),
            Line::from("Navigation:"),
            Line::from("  Tab       - Auto-open latest results / Cycle panels"),
            Line::from("  Enter     - Execute command / Confirm"),
            Line::from("  Esc       - Close dialogs"),
            Line::from(""),
            Line::from("Function Keys:"),
            Line::from("  F1        - Toggle this help"),
            Line::from("  F2        - Save session"),
            Line::from("  F3        - Toggle raw JSON view"),
            Line::from("  F4        - Clear message history"),
            Line::from("  F5        - Environment variables"),
            Line::from("  Q         - Quit application"),
            Line::from(""),
            Line::from("Commands:"),
            Line::from("  tools.name {\"param\": \"value\"}"),
            Line::from("  resources.uri"),
            Line::from("  prompts.name {\"param\": \"value\"}"),
            Line::from(""),
            Line::from("Environment Variables:"),
            Line::from("  Set KEY=value,KEY2=value2 format"),
            Line::from("  Variables are auto-injected into tool calls"),
        ];

        let help_paragraph = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title("Help"))
            .wrap(Wrap { trim: true });

        f.render_widget(Clear, popup_area);
        f.render_widget(help_paragraph, popup_area);
    }

    /// Draw environment variables dialog
    fn draw_env_dialog(&mut self, f: &mut Frame, area: Rect) {
        let popup_area = centered_rect(70, 50, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(3), // Input
                Constraint::Min(0),    // Current variables
                Constraint::Length(3), // Instructions
            ])
            .split(popup_area);

        // Title
        let title = Paragraph::new("Environment Variables Configuration")
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center);
        f.render_widget(Clear, popup_area);
        f.render_widget(title, chunks[0]);

        // Input area
        let input_style = if self.ui_state.current_focus == FocusedPanel::EnvVariables {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let input_block = Block::default()
            .borders(Borders::ALL)
            .title("Environment Variables")
            .style(input_style);

        self.ui_state.env_input_area.set_block(input_block);
        f.render_widget(&self.ui_state.env_input_area, chunks[1]);

        // Current variables with scrollbar
        let current_vars: Vec<ListItem> = self
            .env_variables
            .iter()
            .map(|(key, value)| ListItem::new(format!("{}={}", key, value)))
            .collect();

        let vars_list = List::new(current_vars).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Current Variables"),
        );

        // Update scrollbar state for environment variables
        let env_count = self.env_variables.len().saturating_sub(1);
        self.ui_state.env_vars_scroll = self.ui_state.env_vars_scroll.content_length(env_count);
        // No selection state for env vars, so we'll keep position at 0
        self.ui_state.env_vars_scroll = self.ui_state.env_vars_scroll.position(0);

        f.render_widget(vars_list, chunks[2]);

        // Render the scrollbar for environment variables
        if !self.env_variables.is_empty() {
            f.render_stateful_widget(
                Scrollbar::default()
                    .orientation(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("↑"))
                    .end_symbol(Some("↓")),
                chunks[2],
                &mut self.ui_state.env_vars_scroll,
            );
        }

        // Instructions
        let instructions = Paragraph::new(
            "Enter variables as KEY=value,KEY2=value2... | Press Enter to apply | Esc to cancel",
        )
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
        f.render_widget(instructions, chunks[3]);
    }

    /// Draw parameter form dialog
    fn draw_parameter_form_dialog(&mut self, f: &mut Frame, area: Rect) {
        let popup_area = centered_rect(90, 80, area);

        f.render_widget(Clear, popup_area);

        if let Some(capability_ref) = self.ui_state.selected_capability.clone() {
            // Use the new enhanced parameter form
            self.draw_parameter_form(f, popup_area, &capability_ref);
        } else {
            // Fallback for unknown capability
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Title
                    Constraint::Min(0),    // Content
                    Constraint::Length(3), // Instructions
                ])
                .split(popup_area);

            let title = Paragraph::new("❌ Error: Unknown Capability")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Red)),
                )
                .alignment(Alignment::Center)
                .style(
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                );
            f.render_widget(title, chunks[0]);

            let content = Paragraph::new("No capability selected for parameter configuration.\nThis shouldn't happen - please report this bug.")
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(Color::White));
            f.render_widget(content, chunks[1]);

            let instructions = Paragraph::new("🔙 [Esc] Go Back")
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(instructions, chunks[2]);
        }
    }

    /// Draw parameter input form
    fn draw_parameter_form(&mut self, f: &mut Frame, area: Rect, capability_ref: &CapabilityRef) {
        // Split area for form header and fields
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header with capability info
                Constraint::Min(1),    // Parameter fields
                Constraint::Length(3), // Instructions/controls
            ])
            .split(area);

        // Draw header with capability information
        let (header_text, header_style) = match capability_ref {
            CapabilityRef::Tool(i) => {
                if let Some(tool) = self.capabilities.tools.get(*i) {
                    (
                        format!("🔧 Configure Tool: {}", tool.name),
                        Style::default().fg(Color::Cyan),
                    )
                } else {
                    (
                        "🔧 Tool Configuration".to_string(),
                        Style::default().fg(Color::Cyan),
                    )
                }
            }
            CapabilityRef::Resource(i) => {
                if let Some(resource) = self.capabilities.resources.get(*i) {
                    let name = resource.name.as_ref().unwrap_or(&resource.uri);
                    (
                        format!("📁 Read Resource: {}", name),
                        Style::default().fg(Color::Green),
                    )
                } else {
                    (
                        "📁 Resource Access".to_string(),
                        Style::default().fg(Color::Green),
                    )
                }
            }
            CapabilityRef::Prompt(i) => {
                if let Some(prompt) = self.capabilities.prompts.get(*i) {
                    (
                        format!("💬 Configure Prompt: {}", prompt.name),
                        Style::default().fg(Color::Magenta),
                    )
                } else {
                    (
                        "💬 Prompt Configuration".to_string(),
                        Style::default().fg(Color::Magenta),
                    )
                }
            }
        };

        let header_block = Block::default()
            .borders(Borders::ALL)
            .title("Parameter Configuration")
            .border_style(Style::default().fg(Color::Yellow));

        let header_paragraph = Paragraph::new(header_text)
            .style(header_style.add_modifier(ratatui::style::Modifier::BOLD))
            .alignment(ratatui::layout::Alignment::Center)
            .block(header_block);

        f.render_widget(header_paragraph, chunks[0]);

        // Draw parameter fields
        if self.ui_state.param_field_names.is_empty() {
            // No parameters needed
            let no_params_text = match capability_ref {
                CapabilityRef::Tool(_) => "✅ This tool requires no parameters.\n\nPress Enter to execute immediately, or Esc to cancel.",
                CapabilityRef::Resource(_) => "✅ This resource requires no parameters.\n\nPress Enter to read the resource, or Esc to cancel.",
                CapabilityRef::Prompt(_) => "✅ This prompt requires no arguments.\n\nPress Enter to run the prompt, or Esc to cancel.",
            };

            let no_params_paragraph = Paragraph::new(no_params_text)
                .style(Style::default().fg(Color::Green))
                .alignment(ratatui::layout::Alignment::Center)
                .block(Block::default().borders(Borders::ALL))
                .wrap(ratatui::widgets::Wrap { trim: true });

            f.render_widget(no_params_paragraph, chunks[1]);
        } else {
            // Show parameter input fields
            let field_height = 3; // Each field takes 3 lines
            let available_height = chunks[1].height as usize;
            let max_visible_fields = available_height / field_height;

            let selected_field = self.ui_state.param_selected_field;
            let scroll_start = if selected_field >= max_visible_fields {
                selected_field - max_visible_fields + 1
            } else {
                0
            };

            let mut y_offset = 0;
            for (field_idx, param_name) in self
                .ui_state
                .param_field_names
                .iter()
                .enumerate()
                .skip(scroll_start)
                .take(max_visible_fields)
            {
                let field = match self.ui_state.param_fields.get(param_name) {
                    Some(f) => f,
                    None => {
                        tracing::error!("Field '{}' not found in param_fields", param_name);
                        continue;
                    }
                };

                if y_offset + field_height > available_height {
                    break;
                }

                let field_area = Rect {
                    x: chunks[1].x,
                    y: chunks[1].y + y_offset as u16,
                    width: chunks[1].width,
                    height: field_height as u16,
                };

                // Determine field styling based on selection and requirement status
                let is_selected = field_idx == selected_field;
                let is_required = field.required;
                let has_value = !field.value.is_empty();

                let border_style = if is_selected {
                    Style::default().fg(Color::Yellow)
                } else if is_required && !has_value {
                    Style::default().fg(Color::Red)
                } else if has_value {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                // Create title with requirement indicator
                let field_title = if is_required {
                    format!("📋 {} (REQUIRED)", param_name)
                } else {
                    format!("📝 {} (optional)", param_name)
                };

                // Add type information if available
                let type_info = if let Some(ref param_type) = field.param_type {
                    format!(" [{}]", param_type)
                } else {
                    String::new()
                };

                let full_title = format!("{}{}", field_title, type_info);

                let field_block = Block::default()
                    .borders(Borders::ALL)
                    .title(full_title)
                    .border_style(border_style);

                // Show field content with placeholder or value
                let field_content = if field.value.is_empty() {
                    if let Some(ref desc) = field.description {
                        if is_selected {
                            format!("💡 {}\n\n{}", desc, "Type your input here...")
                        } else {
                            format!("💡 {}", desc)
                        }
                    } else if is_selected {
                        "Type your input here...".to_string()
                    } else {
                        "(empty)".to_string()
                    }
                } else {
                    field.value.clone()
                };

                let content_style = if field.value.is_empty() {
                    if is_selected {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }
                } else {
                    Style::default().fg(Color::White)
                };

                let field_paragraph = Paragraph::new(field_content)
                    .style(content_style)
                    .block(field_block)
                    .wrap(ratatui::widgets::Wrap { trim: true });

                f.render_widget(field_paragraph, field_area);

                // Show cursor if this field is selected and in edit mode
                if is_selected && self.ui_state.param_edit_mode {
                    let cursor_x = field_area.x + 1 + field.value.len() as u16;
                    let cursor_y = field_area.y + 1;

                    if cursor_x < field_area.x + field_area.width - 1 {
                        f.set_cursor_position((cursor_x, cursor_y));
                    }
                }

                y_offset += field_height;
            }

            // Show scroll indicator if needed
            if self.ui_state.param_field_names.len() > max_visible_fields {
                let scroll_info = format!(
                    "Showing fields {}-{} of {} | ↑/↓ to scroll",
                    scroll_start + 1,
                    std::cmp::min(
                        scroll_start + max_visible_fields,
                        self.ui_state.param_field_names.len()
                    ),
                    self.ui_state.param_field_names.len()
                );

                let scroll_area = Rect {
                    x: chunks[1].x + 2,
                    y: chunks[1].y + chunks[1].height - 1,
                    width: chunks[1].width - 4,
                    height: 1,
                };

                let scroll_paragraph =
                    Paragraph::new(scroll_info).style(Style::default().fg(Color::Cyan));
                f.render_widget(scroll_paragraph, scroll_area);
            }
        }

        // Draw instructions at bottom
        let instructions = if self.ui_state.param_field_names.is_empty() {
            "🚀 [Enter] Execute | [Esc] Cancel & Go Back"
        } else if self.ui_state.param_edit_mode {
            "✏️  EDITING MODE | [Enter] Save & Move to Next | [Esc] Stop Editing | [Tab] Execute with Current Values"
        } else {
            "📝 [↑/↓] Navigate & Edit Fields | [Tab] Execute | [Esc] Cancel & Go Back"
        };

        let instructions_block = Block::default()
            .borders(Borders::ALL)
            .title("🎮 Controls")
            .border_style(Style::default().fg(Color::Cyan));

        let instructions_paragraph = Paragraph::new(instructions)
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            )
            .alignment(ratatui::layout::Alignment::Center)
            .block(instructions_block);

        f.render_widget(instructions_paragraph, chunks[2]);
    }

    /// Draw response viewer dialog
    fn draw_response_viewer_dialog(&mut self, f: &mut Frame, area: Rect) {
        if let Some(ref response) = self.ui_state.selected_response {
            let popup_area = centered_rect(90, 80, area);

            // Mode selection display
            let mode_display = match self.ui_state.response_viewer_mode {
                ResponseViewMode::Formatted => "📖 Formatted",
                ResponseViewMode::RawJson => "📄 Raw JSON",
                ResponseViewMode::TreeView => "🌳 Tree View",
                ResponseViewMode::Summary => "📋 Summary",
            };

            let mode_help =
                " | Press 'V' to cycle view modes | ↑/↓ ←/→ PgUp/PgDn to scroll | ESC to close";
            let title = format!("Response Viewer - {}{}", mode_display, mode_help);

            // Generate content based on view mode
            let content = self.format_response_content(response);

            // Split content into lines and calculate dimensions for scrolling
            let lines: Vec<&str> = content.lines().collect();
            let total_lines = lines.len();
            let visible_area = Rect {
                x: popup_area.x + 1,
                y: popup_area.y + 1,
                width: popup_area.width.saturating_sub(2),
                height: popup_area.height.saturating_sub(2),
            };
            let visible_lines = visible_area.height as usize;

            // Calculate maximum line width for horizontal scrolling
            let max_line_width = lines.iter().map(|line| line.len()).max().unwrap_or(0);
            let visible_chars = visible_area.width as usize;

            // Clamp scroll positions to valid ranges
            let max_vertical_scroll = total_lines.saturating_sub(visible_lines);
            let max_horizontal_scroll = max_line_width.saturating_sub(visible_chars);

            if self.ui_state.response_viewer_vertical_pos > max_vertical_scroll {
                self.ui_state.response_viewer_vertical_pos = max_vertical_scroll;
            }
            if self.ui_state.response_viewer_horizontal_pos > max_horizontal_scroll {
                self.ui_state.response_viewer_horizontal_pos = max_horizontal_scroll;
            }

            // Update scroll states based on content dimensions and current positions
            self.ui_state.response_viewer_scroll = self
                .ui_state
                .response_viewer_scroll
                .content_length(max_vertical_scroll)
                .position(self.ui_state.response_viewer_vertical_pos);

            self.ui_state.response_viewer_horizontal_scroll = self
                .ui_state
                .response_viewer_horizontal_scroll
                .content_length(max_horizontal_scroll)
                .position(self.ui_state.response_viewer_horizontal_pos);

            // Get current scroll positions
            let vertical_scroll = self.ui_state.response_viewer_vertical_pos;
            let horizontal_scroll = self.ui_state.response_viewer_horizontal_pos;

            let block = Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Cyan));

            let text = Text::from(content);
            let paragraph = Paragraph::new(text)
                .block(block)
                .wrap(Wrap { trim: false }) // Disable wrapping for proper horizontal scroll
                .scroll((vertical_scroll as u16, horizontal_scroll as u16));

            f.render_widget(Clear, popup_area);
            f.render_widget(paragraph, popup_area);

            // Render vertical scrollbar
            if total_lines > visible_lines {
                let scrollbar_area = Rect {
                    x: popup_area.x + popup_area.width - 1,
                    y: popup_area.y + 1,
                    width: 1,
                    height: popup_area.height.saturating_sub(2),
                };

                let vertical_scrollbar = Scrollbar::default()
                    .orientation(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("↑"))
                    .end_symbol(Some("↓"));

                f.render_stateful_widget(
                    vertical_scrollbar,
                    scrollbar_area,
                    &mut self.ui_state.response_viewer_scroll,
                );
            }

            // Render horizontal scrollbar
            if max_line_width > visible_chars {
                let h_scrollbar_area = Rect {
                    x: popup_area.x + 1,
                    y: popup_area.y + popup_area.height - 1,
                    width: popup_area.width.saturating_sub(2),
                    height: 1,
                };

                let horizontal_scrollbar = Scrollbar::default()
                    .orientation(ScrollbarOrientation::HorizontalBottom)
                    .begin_symbol(Some("←"))
                    .end_symbol(Some("→"));

                f.render_stateful_widget(
                    horizontal_scrollbar,
                    h_scrollbar_area,
                    &mut self.ui_state.response_viewer_horizontal_scroll,
                );
            }
        }
    }

    /// Format response content based on current view mode
    fn format_response_content(&self, response: &Value) -> String {
        match self.ui_state.response_viewer_mode {
            ResponseViewMode::Formatted => self.format_response_formatted(response),
            ResponseViewMode::RawJson => serde_json::to_string_pretty(response)
                .unwrap_or_else(|_| "Invalid JSON".to_string()),
            ResponseViewMode::TreeView => Self::format_response_tree(response, 0),
            ResponseViewMode::Summary => self.format_response_summary(response),
        }
    }

    /// Format response in a structured, readable way
    fn format_response_formatted(&self, response: &Value) -> String {
        match response {
            Value::Object(obj) => {
                let mut lines = Vec::new();

                lines.push("📋 RESPONSE ANALYSIS:".to_string());
                lines.push(format!("  Total fields: {}", obj.len()));
                lines.push("".to_string());

                // Handle common MCP response fields
                if let Some(content) = obj.get("content") {
                    lines.push("📝 CONTENT:".to_string());
                    match content {
                        Value::Array(arr) => {
                            if arr.is_empty() {
                                lines.push("  ⚠️  Content array is EMPTY".to_string());
                            } else {
                                lines.push(format!("  {} items in content array:", arr.len()));
                                lines.push(self.format_content_array(content));
                            }
                        }
                        _ => {
                            lines.push("  Content is not an array:".to_string());
                            lines.push(format!(
                                "  {}",
                                serde_json::to_string_pretty(content).unwrap_or_default()
                            ));
                        }
                    }
                    lines.push("".to_string());
                }

                if let Some(is_error) = obj.get("isError").or_else(|| obj.get("is_error")) {
                    if is_error.as_bool() == Some(true) {
                        lines.push("⚠️  ERROR FLAG: true".to_string());
                        lines.push("".to_string());
                    }
                }

                if let Some(meta) = obj.get("_meta") {
                    lines.push("🔍 METADATA:".to_string());
                    lines.push(format!(
                        "  {}",
                        serde_json::to_string_pretty(meta).unwrap_or_default()
                    ));
                    lines.push("".to_string());
                }

                // Handle other fields
                for (key, value) in obj {
                    if !matches!(key.as_str(), "content" | "isError" | "is_error" | "_meta") {
                        lines.push(format!("📊 {}:", key.to_uppercase()));
                        lines.push(format!("  {}", Self::format_value_indented(value, 1)));
                        lines.push("".to_string());
                    }
                }

                if lines.len() == 3 {
                    // Only header + empty lines
                    lines.push(
                        "⚠️  This response appears to be empty or contain no useful data."
                            .to_string(),
                    );
                    lines.push("".to_string());
                    lines.push("🔍 RAW OBJECT FIELDS:".to_string());
                    for (key, value) in obj {
                        lines.push(format!(
                            "  {}: {}",
                            key,
                            serde_json::to_string(value).unwrap_or_default()
                        ));
                    }
                }

                lines.join("\n")
            }
            _ => {
                format!(
                    "📊 Non-object response:\n{}",
                    serde_json::to_string_pretty(response).unwrap_or_default()
                )
            }
        }
    }

    /// Format content array in a readable way
    fn format_content_array(&self, content: &Value) -> String {
        match content {
            Value::Array(arr) => {
                let mut lines = Vec::new();
                for (i, item) in arr.iter().enumerate() {
                    lines.push(format!("  [{}] {}", i + 1, self.format_content_item(item)));
                }
                lines.join("\n")
            }
            _ => format!(
                "  {}",
                serde_json::to_string_pretty(content).unwrap_or_default()
            ),
        }
    }

    /// Format a single content item
    fn format_content_item(&self, item: &Value) -> String {
        if let Value::Object(obj) = item {
            if let Some(text) = obj.get("text") {
                if let Some(text_str) = text.as_str() {
                    return format!("Text: {}", text_str);
                }
            }
            if let Some(data) = obj.get("data") {
                if let Some(mime_type) = obj.get("mimeType").or_else(|| obj.get("mime_type")) {
                    return format!(
                        "Binary data ({}): {} bytes",
                        mime_type.as_str().unwrap_or("unknown"),
                        data.as_str().map(|s| s.len()).unwrap_or(0)
                    );
                }
            }
            if let Some(resource) = obj.get("resource") {
                return format!(
                    "Resource: {}",
                    serde_json::to_string_pretty(resource).unwrap_or_default()
                );
            }
        }
        serde_json::to_string_pretty(item).unwrap_or_default()
    }

    /// Format value with indentation
    fn format_value_indented(value: &Value, indent: usize) -> String {
        let indent_str = "  ".repeat(indent);
        match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(arr) => {
                let mut lines = vec!["[".to_string()];
                for item in arr {
                    lines.push(format!(
                        "{}{},",
                        indent_str,
                        Self::format_value_indented(item, indent + 1)
                    ));
                }
                lines.push("]".to_string());
                lines.join("\n")
            }
            Value::Object(obj) => {
                let mut lines = vec!["{".to_string()];
                for (key, val) in obj {
                    lines.push(format!(
                        "{}\"{}\": {},",
                        indent_str,
                        key,
                        Self::format_value_indented(val, indent + 1)
                    ));
                }
                lines.push("}".to_string());
                lines.join("\n")
            }
        }
    }

    /// Format response as a tree structure
    fn format_response_tree(value: &Value, depth: usize) -> String {
        let indent = "  ".repeat(depth);
        let connector = if depth == 0 { "" } else { "├─ " };

        match value {
            Value::Object(obj) => {
                let mut lines = Vec::new();
                if depth == 0 {
                    lines.push("📦 Response Object".to_string());
                }

                for (i, (key, val)) in obj.iter().enumerate() {
                    let is_last = i == obj.len() - 1;
                    let branch = if is_last { "└─ " } else { "├─ " };

                    match val {
                        Value::Object(_) => {
                            lines.push(format!("{}{}{} 📁", indent, branch, key));
                            lines.push(Self::format_response_tree(val, depth + 1));
                        }
                        Value::Array(arr) => {
                            lines.push(format!("{}{}{} 📚 [{}]", indent, branch, key, arr.len()));
                            if !arr.is_empty() {
                                lines.push(Self::format_response_tree(val, depth + 1));
                            }
                        }
                        Value::String(s) => {
                            let preview = if s.len() > 50 {
                                format!("{}...", &s[..47])
                            } else {
                                s.clone()
                            };
                            lines.push(format!("{}{}{} 📝 \"{}\"", indent, branch, key, preview));
                        }
                        Value::Number(n) => {
                            lines.push(format!("{}{}{} 🔢 {}", indent, branch, key, n));
                        }
                        Value::Bool(b) => {
                            lines.push(format!("{}{}{} ✓ {}", indent, branch, key, b));
                        }
                        Value::Null => {
                            lines.push(format!("{}{}{} ∅ null", indent, branch, key));
                        }
                    }
                }
                lines.join("\n")
            }
            Value::Array(arr) => {
                let mut lines = Vec::new();
                for (i, item) in arr.iter().enumerate() {
                    let is_last = i == arr.len() - 1;
                    let branch = if is_last { "└─ " } else { "├─ " };
                    lines.push(format!("{}{}[{}]", indent, branch, i));
                    lines.push(Self::format_response_tree(item, depth + 1));
                }
                lines.join("\n")
            }
            _ => format!("{}{}{:?}", indent, connector, value),
        }
    }

    /// Format response summary with key statistics
    fn format_response_summary(&self, response: &Value) -> String {
        let mut lines = Vec::new();
        lines.push("📊 RESPONSE SUMMARY".to_string());
        lines.push("─".repeat(50));
        lines.push("".to_string());

        match response {
            Value::Object(obj) => {
                lines.push(format!("🔍 Type: Object ({} fields)", obj.len()));
                lines.push("".to_string());

                // Analyze content
                if let Some(content) = obj.get("content") {
                    match content {
                        Value::Array(arr) => {
                            lines.push(format!("📝 Content Items: {}", arr.len()));
                            for (i, item) in arr.iter().enumerate() {
                                if let Value::Object(content_obj) = item {
                                    if let Some(text) = content_obj.get("text") {
                                        if let Some(text_str) = text.as_str() {
                                            lines.push(format!(
                                                "   [{}] Text: {} chars",
                                                i + 1,
                                                text_str.len()
                                            ));
                                        }
                                    } else if content_obj.get("data").is_some() {
                                        lines.push(format!("   [{}] Binary data", i + 1));
                                    }
                                }
                            }
                            lines.push("".to_string());
                        }
                        _ => {
                            lines.push("📝 Content: Single item".to_string());
                            lines.push("".to_string());
                        }
                    }
                }

                // Check for error status
                if let Some(is_error) = obj.get("isError").or_else(|| obj.get("is_error")) {
                    if is_error.as_bool() == Some(true) {
                        lines.push("⚠️  Status: Error".to_string());
                    } else {
                        lines.push("✅ Status: Success".to_string());
                    }
                    lines.push("".to_string());
                }

                // Field breakdown
                lines.push("🏗️  Fields:".to_string());
                for (key, value) in obj {
                    let value_type = match value {
                        Value::String(_) => "String",
                        Value::Number(_) => "Number",
                        Value::Bool(_) => "Boolean",
                        Value::Array(arr) => &format!("Array[{}]", arr.len()),
                        Value::Object(obj) => &format!("Object[{}]", obj.len()),
                        Value::Null => "Null",
                    };
                    lines.push(format!("   • {}: {}", key, value_type));
                }
            }
            Value::Array(arr) => {
                lines.push(format!("🔍 Type: Array ({} items)", arr.len()));
            }
            _ => {
                lines.push("🔍 Type: Primitive value".to_string());
            }
        }

        lines.push("".to_string());
        lines.push("─".repeat(50));

        let json_size = serde_json::to_string(response)
            .map(|s| s.len())
            .unwrap_or(0);
        lines.push(format!("📏 JSON Size: {} bytes", json_size));

        lines.join("\n")
    }

    /// Open response viewer for the latest message with results
    fn open_latest_response_viewer(&mut self) {
        // Find the latest message with raw_response data
        if let Some((index, message)) = self
            .message_history
            .iter()
            .enumerate()
            .rev()
            .find(|(_idx, msg)| msg.raw_response.is_some())
        {
            if let Some(ref response) = message.raw_response {
                self.ui_state.selected_response = Some(response.clone());
                self.ui_state.response_viewer_open = true;
                // Reset scroll positions to start from top
                self.ui_state.response_viewer_vertical_pos = 0;
                self.ui_state.response_viewer_horizontal_pos = 0;
                // Auto-select this message in the history for context
                self.ui_state.message_history_state.select(Some(index));
                tracing::info!("Opened response viewer for latest message with results");
            }
        } else {
            // No results found, just cycle focus as fallback
            self.cycle_focus();
        }
    }

    /// Strip the implementation prefix from tool names
    /// Handles formats like: "Z2l0aHViX2FwaTpnaXRodWJfYXBpOjAuMS4w.github.repos/list-for-user"
    /// Or: "prefix::actual.tool.name"
    fn strip_tool_prefix(name: &str) -> &str {
        // First try :: separator (old format)
        if let Some(pos) = name.find("::") {
            return &name[pos + 2..];
        }

        // Then try . separator with base64-looking prefix
        if let Some(pos) = name.find('.') {
            let prefix = &name[..pos];
            // Check if prefix looks like base64 (alphanumeric + / + =)
            if prefix.len() > 10
                && prefix
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '=')
            {
                return &name[pos + 1..];
            }
        }

        // If no recognizable prefix pattern, return as-is
        name
    }
}

/// Helper function to create centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcp_core::transport::TransportConfig;

    #[test]
    fn test_debugger_app_creation() -> Result<()> {
        let transport_config = TransportConfig::stdio("test", &["arg1"]);
        let client_info = Implementation {
            name: "test-client".to_string(),
            version: "0.1.0".to_string(),
            metadata: std::collections::HashMap::new(),
        };

        let app = DebuggerApp::new(transport_config, client_info)?;
        assert_eq!(app.state, AppState::Initializing);

        Ok(())
    }

    #[test]
    fn test_env_variables_parsing() {
        let mut app = create_test_app();

        // Simulate environment variable input
        app.ui_state
            .env_input_area
            .insert_str("API_KEY=test123,DEBUG=true,PORT=8080");
        app.parse_env_variables();

        assert_eq!(
            app.env_variables.get("API_KEY"),
            Some(&"test123".to_string())
        );
        assert_eq!(app.env_variables.get("DEBUG"), Some(&"true".to_string()));
        assert_eq!(app.env_variables.get("PORT"), Some(&"8080".to_string()));
    }

    fn create_test_app() -> DebuggerApp {
        let transport_config = TransportConfig::stdio("test", &["arg1"]);
        let client_info = Implementation {
            name: "test-client".to_string(),
            version: "0.1.0".to_string(),
            metadata: std::collections::HashMap::new(),
        };

        DebuggerApp::new(transport_config, client_info).unwrap()
    }
}
