//! Transport configuration system for MCP clients.
//!
//! This module provides a type-safe configuration system for all MCP transport types.
//! Configurations can be created programmatically or loaded from files.
//!
//! # Examples
//!
//! ```rust
//! use mcp_probe_core::transport::{TransportConfig, StdioConfig, HttpSseConfig};
//! use std::time::Duration;
//!
//! // Stdio transport configuration
//! let stdio_config = TransportConfig::Stdio(StdioConfig {
//!     command: "python".to_string(),
//!     args: vec!["server.py".to_string()],
//!     working_dir: Some("/path/to/server".to_string()),
//!     timeout: Duration::from_secs(30),
//!     environment: Default::default(),
//! });
//!
//! // HTTP+SSE transport configuration  
//! let http_config = TransportConfig::HttpSse(HttpSseConfig {
//!     base_url: "https://api.example.com/mcp".parse().unwrap(),
//!     timeout: Duration::from_secs(60),
//!     headers: Default::default(),
//!     auth: None,
//! });
//! ```

use crate::error::{ConfigError, McpResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use url::Url;

/// Transport configuration enum supporting all MCP transport types.
///
/// This enum provides type-safe configuration for different transport mechanisms,
/// ensuring that each transport gets the configuration parameters it needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TransportConfig {
    /// Local process communication via stdio
    Stdio(StdioConfig),

    /// Remote HTTP server with Server-Sent Events
    HttpSse(HttpSseConfig),

    /// Full-duplex HTTP streaming
    HttpStream(HttpStreamConfig),
}

impl TransportConfig {
    /// Create a new stdio transport configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::transport::TransportConfig;
    ///
    /// let config = TransportConfig::stdio("python", &["server.py"]);
    /// ```
    pub fn stdio(command: impl Into<String>, args: &[impl ToString]) -> Self {
        Self::Stdio(StdioConfig {
            command: command.into(),
            args: args.iter().map(|s| s.to_string()).collect(),
            working_dir: None,
            timeout: Duration::from_secs(30),
            environment: HashMap::new(),
        })
    }

    /// Create a new HTTP+SSE transport configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::transport::TransportConfig;
    ///
    /// let config = TransportConfig::http_sse("https://api.example.com/mcp").unwrap();
    /// ```
    pub fn http_sse(base_url: impl AsRef<str>) -> McpResult<Self> {
        let url = base_url
            .as_ref()
            .parse()
            .map_err(|e| ConfigError::InvalidValue {
                parameter: "base_url".to_string(),
                value: base_url.as_ref().to_string(),
                reason: format!("Invalid URL: {}", e),
            })?;

        Ok(Self::HttpSse(HttpSseConfig {
            base_url: url,
            timeout: Duration::from_secs(60),
            headers: HashMap::new(),
            auth: None,
        }))
    }

    /// Create a new HTTP streaming transport configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::transport::TransportConfig;
    ///
    /// let config = TransportConfig::http_stream("https://stream.example.com/mcp").unwrap();
    /// ```
    pub fn http_stream(base_url: impl AsRef<str>) -> McpResult<Self> {
        let url = base_url
            .as_ref()
            .parse()
            .map_err(|e| ConfigError::InvalidValue {
                parameter: "base_url".to_string(),
                value: base_url.as_ref().to_string(),
                reason: format!("Invalid URL: {}", e),
            })?;

        Ok(Self::HttpStream(HttpStreamConfig {
            base_url: url,
            timeout: Duration::from_secs(300),
            headers: HashMap::new(),
            auth: None,
            compression: true,
            flow_control_window: 65536,
        }))
    }

    /// Get a human-readable name for this transport type.
    pub fn transport_type(&self) -> &'static str {
        match self {
            Self::Stdio(_) => "stdio",
            Self::HttpSse(_) => "http-sse",
            Self::HttpStream(_) => "http-stream",
        }
    }

    /// Validate the configuration and return any errors.
    pub fn validate(&self) -> McpResult<()> {
        match self {
            Self::Stdio(config) => config.validate(),
            Self::HttpSse(config) => config.validate(),
            Self::HttpStream(config) => config.validate(),
        }
    }

    /// Load configuration from a file.
    ///
    /// Supports JSON, YAML, and TOML formats based on file extension.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::transport::TransportConfig;
    ///
    /// let config = TransportConfig::from_file("config.json")?;
    /// # Ok::<(), mcp_probe_core::error::McpError>(())
    /// ```
    pub fn from_file(path: impl AsRef<std::path::Path>) -> McpResult<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|_e| ConfigError::FileNotFound {
            path: path.display().to_string(),
        })?;

        let config: Self = match path.extension().and_then(|ext| ext.to_str()) {
            Some("json") => {
                serde_json::from_str(&content).map_err(|e| ConfigError::InvalidFormat {
                    path: path.display().to_string(),
                    reason: e.to_string(),
                })?
            }
            Some("yaml") | Some("yml") => {
                serde_yaml::from_str(&content).map_err(|e| ConfigError::InvalidFormat {
                    path: path.display().to_string(),
                    reason: e.to_string(),
                })?
            }
            Some("toml") => toml::from_str(&content).map_err(|e| ConfigError::InvalidFormat {
                path: path.display().to_string(),
                reason: e.to_string(),
            })?,
            _ => {
                return Err(ConfigError::InvalidFormat {
                    path: path.display().to_string(),
                    reason: "Unsupported file format. Use .json, .yaml, or .toml".to_string(),
                }
                .into())
            }
        };

        config.validate()?;
        Ok(config)
    }

    /// Save configuration to a file.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_probe_core::transport::TransportConfig;
    ///
    /// let config = TransportConfig::stdio("python", &["server.py"]);
    /// config.to_file("config.json")?;
    /// # Ok::<(), mcp_probe_core::error::McpError>(())
    /// ```
    pub fn to_file(&self, path: impl AsRef<std::path::Path>) -> McpResult<()> {
        let path = path.as_ref();
        let content = match path.extension().and_then(|ext| ext.to_str()) {
            Some("json") => {
                serde_json::to_string_pretty(self).map_err(|e| ConfigError::InvalidFormat {
                    path: path.display().to_string(),
                    reason: e.to_string(),
                })?
            }
            Some("yaml") | Some("yml") => {
                serde_yaml::to_string(self).map_err(|e| ConfigError::InvalidFormat {
                    path: path.display().to_string(),
                    reason: e.to_string(),
                })?
            }
            Some("toml") => toml::to_string(self).map_err(|e| ConfigError::InvalidFormat {
                path: path.display().to_string(),
                reason: e.to_string(),
            })?,
            _ => {
                return Err(ConfigError::InvalidFormat {
                    path: path.display().to_string(),
                    reason: "Unsupported file format. Use .json, .yaml, or .toml".to_string(),
                }
                .into())
            }
        };

        std::fs::write(path, content)?;

        Ok(())
    }
}

/// Configuration for stdio (local process) transport.
///
/// This transport spawns a local process and communicates via stdin/stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StdioConfig {
    /// Command to execute (e.g., "python", "/usr/bin/node")
    pub command: String,

    /// Arguments to pass to the command
    pub args: Vec<String>,

    /// Working directory for the process (optional)
    pub working_dir: Option<String>,

    /// Timeout for process operations
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,

    /// Environment variables to set for the process
    pub environment: HashMap<String, String>,
}

impl StdioConfig {
    /// Create a new stdio configuration.
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            working_dir: None,
            timeout: Duration::from_secs(30),
            environment: HashMap::new(),
        }
    }

    /// Add an argument to the command.
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add multiple arguments to the command.
    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(args.into_iter().map(|s| s.into()));
        self
    }

    /// Set the working directory.
    pub fn working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Set the timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Add an environment variable.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment.insert(key.into(), value.into());
        self
    }

    /// Validate the stdio configuration.
    pub fn validate(&self) -> McpResult<()> {
        if self.command.is_empty() {
            return Err(ConfigError::MissingParameter {
                parameter: "command".to_string(),
            }
            .into());
        }

        if let Some(ref dir) = self.working_dir {
            if !PathBuf::from(dir).exists() {
                return Err(ConfigError::InvalidValue {
                    parameter: "working_dir".to_string(),
                    value: dir.clone(),
                    reason: "Directory does not exist".to_string(),
                }
                .into());
            }
        }

        Ok(())
    }
}

/// Configuration for HTTP+SSE (Server-Sent Events) transport.
///
/// This transport uses HTTP requests for client-to-server communication
/// and Server-Sent Events for server-to-client communication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpSseConfig {
    /// Base URL for the MCP server
    pub base_url: Url,

    /// Timeout for HTTP requests
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,

    /// Additional HTTP headers to include
    pub headers: HashMap<String, String>,

    /// Authentication configuration
    pub auth: Option<AuthConfig>,
}

impl HttpSseConfig {
    /// Create a new HTTP+SSE configuration.
    pub fn new(base_url: Url) -> Self {
        Self {
            base_url,
            timeout: Duration::from_secs(60),
            headers: HashMap::new(),
            auth: None,
        }
    }

    /// Set the timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Add an HTTP header.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set authentication configuration.
    pub fn auth(mut self, auth: AuthConfig) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Validate the HTTP+SSE configuration.
    pub fn validate(&self) -> McpResult<()> {
        if self.base_url.scheme() != "http" && self.base_url.scheme() != "https" {
            return Err(ConfigError::InvalidValue {
                parameter: "base_url".to_string(),
                value: self.base_url.to_string(),
                reason: "URL must use http or https scheme".to_string(),
            }
            .into());
        }

        if let Some(ref auth) = self.auth {
            auth.validate()?;
        }

        Ok(())
    }
}

/// Configuration for HTTP streaming transport.
///
/// This transport uses full-duplex HTTP streaming for bidirectional communication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpStreamConfig {
    /// Base URL for the MCP server
    pub base_url: Url,

    /// Timeout for streaming operations
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,

    /// Additional HTTP headers to include
    pub headers: HashMap<String, String>,

    /// Authentication configuration
    pub auth: Option<AuthConfig>,

    /// Enable compression for the stream
    pub compression: bool,

    /// Flow control window size
    pub flow_control_window: u32,
}

impl HttpStreamConfig {
    /// Create a new HTTP streaming configuration.
    pub fn new(base_url: Url) -> Self {
        Self {
            base_url,
            timeout: Duration::from_secs(300),
            headers: HashMap::new(),
            auth: None,
            compression: true,
            flow_control_window: 65536,
        }
    }

    /// Set the timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Add an HTTP header.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set authentication configuration.
    pub fn auth(mut self, auth: AuthConfig) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Enable or disable compression.
    pub fn compression(mut self, enabled: bool) -> Self {
        self.compression = enabled;
        self
    }

    /// Set the flow control window size.
    pub fn flow_control_window(mut self, size: u32) -> Self {
        self.flow_control_window = size;
        self
    }

    /// Validate the HTTP streaming configuration.
    pub fn validate(&self) -> McpResult<()> {
        if self.base_url.scheme() != "http" && self.base_url.scheme() != "https" {
            return Err(ConfigError::InvalidValue {
                parameter: "base_url".to_string(),
                value: self.base_url.to_string(),
                reason: "URL must use http or https scheme".to_string(),
            }
            .into());
        }

        if self.flow_control_window == 0 {
            return Err(ConfigError::InvalidValue {
                parameter: "flow_control_window".to_string(),
                value: self.flow_control_window.to_string(),
                reason: "Flow control window must be greater than 0".to_string(),
            }
            .into());
        }

        if let Some(ref auth) = self.auth {
            auth.validate()?;
        }

        Ok(())
    }
}

/// Authentication configuration for HTTP-based transports.
///
/// Supports various authentication schemes including basic auth,
/// bearer tokens, and OAuth 2.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum AuthConfig {
    /// HTTP Basic Authentication
    Basic { username: String, password: String },

    /// Bearer token authentication
    Bearer { token: String },

    /// OAuth 2.0 authentication
    OAuth {
        client_id: String,
        client_secret: String,
        token_url: Url,
        scope: Option<String>,
    },

    /// Custom header-based authentication
    Header { name: String, value: String },
}

impl AuthConfig {
    /// Create a new basic authentication configuration.
    pub fn basic(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self::Basic {
            username: username.into(),
            password: password.into(),
        }
    }

    /// Create a new bearer token authentication configuration.
    pub fn bearer(token: impl Into<String>) -> Self {
        Self::Bearer {
            token: token.into(),
        }
    }

    /// Create a new OAuth 2.0 authentication configuration.
    pub fn oauth(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        token_url: Url,
        scope: Option<String>,
    ) -> Self {
        Self::OAuth {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            token_url,
            scope,
        }
    }

    /// Create a new custom header authentication configuration.
    pub fn header(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self::Header {
            name: name.into(),
            value: value.into(),
        }
    }

    /// Validate the authentication configuration.
    pub fn validate(&self) -> McpResult<()> {
        match self {
            Self::Basic { username, password } => {
                if username.is_empty() || password.is_empty() {
                    return Err(ConfigError::InvalidValue {
                        parameter: "auth".to_string(),
                        value: "basic".to_string(),
                        reason: "Username and password cannot be empty".to_string(),
                    }
                    .into());
                }
            }
            Self::Bearer { token } => {
                if token.is_empty() {
                    return Err(ConfigError::InvalidValue {
                        parameter: "auth".to_string(),
                        value: "bearer".to_string(),
                        reason: "Token cannot be empty".to_string(),
                    }
                    .into());
                }
            }
            Self::OAuth {
                client_id,
                client_secret,
                token_url,
                ..
            } => {
                if client_id.is_empty() || client_secret.is_empty() {
                    return Err(ConfigError::InvalidValue {
                        parameter: "auth".to_string(),
                        value: "oauth".to_string(),
                        reason: "Client ID and secret cannot be empty".to_string(),
                    }
                    .into());
                }
                if token_url.scheme() != "https" {
                    return Err(ConfigError::InvalidValue {
                        parameter: "token_url".to_string(),
                        value: token_url.to_string(),
                        reason: "OAuth token URL must use HTTPS".to_string(),
                    }
                    .into());
                }
            }
            Self::Header { name, value } => {
                if name.is_empty() || value.is_empty() {
                    return Err(ConfigError::InvalidValue {
                        parameter: "auth".to_string(),
                        value: "header".to_string(),
                        reason: "Header name and value cannot be empty".to_string(),
                    }
                    .into());
                }
            }
        }
        Ok(())
    }
}
