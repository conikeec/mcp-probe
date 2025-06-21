//! Command-line interface definitions for MCP Probe
//!
//! This module defines the CLI structure using clap for parsing command-line
//! arguments and providing a clean interface for various MCP debugging operations.

use clap::{Parser, Subcommand, ValueEnum};
use mcp_probe_core::transport::TransportConfig;
use std::path::PathBuf;
use url::Url;

/// MCP Probe - Interactive Model Context Protocol debugger and client
#[derive(Parser)]
#[command(
    name = "mcp-probe",
    version,
    about = "A production-grade MCP client and debugger built in Rust",
    long_about = "MCP Probe provides both a powerful SDK for building MCP clients and an intuitive debugging tool for validating MCP servers before deploying them to LLM hosts."
)]
pub struct Cli {
    /// Enable verbose logging (can be used multiple times)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Disable colored output
    #[arg(long)]
    pub no_color: bool,

    /// Output format for structured data
    #[arg(long, value_enum, default_value = "pretty")]
    pub output: OutputFormat,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Interactive debugging session with an MCP server
    Debug(crate::commands::debug::DebugCommand),

    /// Run automated tests against an MCP server
    Test(TestArgs),

    /// Manage configuration files and settings
    Config(ConfigArgs),

    /// Validate MCP server protocol compliance
    Validate(ValidateArgs),

    /// Export session data and generate reports
    Export(ExportArgs),
}

/// Arguments for the debug command
#[derive(Parser)]
pub struct DebugArgs {
    /// Transport type to use for connection
    #[command(flatten)]
    pub transport: TransportArgs,

    /// Configuration file to load
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Start in non-interactive mode
    #[arg(long)]
    pub non_interactive: bool,

    /// Show raw MCP protocol messages
    #[arg(long)]
    pub show_raw: bool,

    /// Save session to file
    #[arg(long)]
    pub save_session: Option<PathBuf>,

    /// Load and replay a previous session
    #[arg(long)]
    pub replay_session: Option<PathBuf>,

    /// Connection timeout in seconds
    #[arg(long, default_value = "30")]
    pub timeout: u64,

    /// Maximum number of retry attempts
    #[arg(long, default_value = "3")]
    pub max_retries: u32,
}

/// Arguments for the test command
#[derive(Parser, Debug)]
pub struct TestArgs {
    /// Test suite to run (default: all)
    #[arg(short, long)]
    pub suite: Option<String>,

    /// Transport configuration
    #[command(flatten)]
    pub transport: TransportArgs,

    /// Configuration file with test definitions
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Generate detailed test report
    #[arg(long)]
    pub report: bool,

    /// Output directory for test reports
    #[arg(long)]
    pub output_dir: Option<PathBuf>,

    /// Fail fast on first test failure
    #[arg(long)]
    pub fail_fast: bool,

    /// Test timeout in seconds
    #[arg(long, default_value = "60")]
    pub timeout: u64,
}

/// Arguments for the config command
#[derive(Parser, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Generate a new configuration file
    Init {
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Configuration template to use
        #[arg(short, long, value_enum, default_value = "full")]
        template: ConfigTemplate,
    },

    /// Validate an existing configuration file
    Validate {
        /// Configuration file to validate
        config: PathBuf,
    },

    /// Show current configuration
    Show {
        /// Configuration file to display
        config: Option<PathBuf>,
    },
}

/// Arguments for the validate command
#[derive(Parser, Debug)]
pub struct ValidateArgs {
    /// Transport configuration
    #[command(flatten)]
    pub transport: TransportArgs,

    /// Configuration file to load
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Validation rules to apply
    #[arg(long, value_delimiter = ',')]
    pub rules: Vec<String>,

    /// Output validation report
    #[arg(long)]
    pub report: Option<PathBuf>,

    /// Severity level for validation failures
    #[arg(long, value_enum, default_value = "error")]
    pub severity: Severity,
}

/// Arguments for the export command
#[derive(Parser, Debug)]
pub struct ExportArgs {
    /// Session file to export
    pub session: PathBuf,

    /// Export format
    #[arg(short, long, value_enum, default_value = "json")]
    pub format: ExportFormat,

    /// Output file (default: stdout)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Include raw protocol messages
    #[arg(long)]
    pub include_raw: bool,

    /// Include timing information
    #[arg(long)]
    pub include_timing: bool,
}

/// Transport configuration arguments
#[derive(Parser, Clone, Debug)]
pub struct TransportArgs {
    /// Use stdio transport with command
    #[arg(long, value_name = "COMMAND")]
    pub stdio: Option<String>,

    /// Command arguments for stdio transport
    #[arg(long, requires = "stdio")]
    pub args: Vec<String>,

    /// Working directory for stdio command
    #[arg(long, requires = "stdio")]
    pub working_dir: Option<PathBuf>,

    /// Use HTTP+SSE transport with URL
    #[arg(long, value_name = "URL")]
    pub http_sse: Option<Url>,

    /// Use HTTP streaming transport with URL
    #[arg(long, value_name = "URL")]
    pub http_stream: Option<Url>,

    /// Authentication header for HTTP transports
    #[arg(long, requires = "http_sse")]
    pub auth_header: Option<String>,

    /// Custom headers for HTTP transports (key=value format)
    #[arg(long, requires = "http_sse")]
    pub headers: Vec<String>,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    /// Human-readable pretty output
    Pretty,
    /// JSON format
    Json,
    /// YAML format
    Yaml,
    /// Plain text
    Text,
}

/// Configuration template types for quick setup
#[derive(Clone, Debug, ValueEnum)]
pub enum ConfigTemplate {
    /// Minimal configuration
    Minimal,
    /// Full configuration with all options
    Full,
    /// Development-focused configuration
    Dev,
    /// Production-ready configuration
    Prod,
}

/// Validation severity levels
#[derive(ValueEnum, Clone, Debug)]
pub enum Severity {
    /// Information level
    Info,
    /// Warning level
    Warning,
    /// Error level
    Error,
    /// Critical level
    Critical,
}

/// Export format options
#[derive(ValueEnum, Clone, Debug)]
pub enum ExportFormat {
    /// JSON format
    Json,
    /// YAML format
    Yaml,
    /// Markdown report
    Markdown,
    /// HTML report
    Html,
    /// CSV data
    Csv,
}

impl TransportArgs {
    /// Convert transport arguments to TransportConfig
    pub fn to_transport_config(&self) -> anyhow::Result<TransportConfig> {
        match (&self.stdio, &self.http_sse, &self.http_stream) {
            (Some(command), None, None) => {
                // Parse command and arguments
                let args: Vec<String> = self.args.to_vec();
                Ok(TransportConfig::stdio(command, &args))
            }
            (None, Some(url), None) => Ok(TransportConfig::http_sse(url.as_str())?),
            (None, None, Some(url)) => Ok(TransportConfig::http_stream(url.clone())?),
            (None, None, None) => {
                anyhow::bail!("No transport specified. Use --stdio, --http-sse, or --http-stream")
            }
            _ => {
                anyhow::bail!("Only one transport type can be specified at a time")
            }
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Pretty => write!(f, "pretty"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Yaml => write!(f, "yaml"),
            OutputFormat::Text => write!(f, "text"),
        }
    }
}
