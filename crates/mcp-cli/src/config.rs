//! Configuration management for MCP Probe CLI
//!
//! This module handles loading and managing configuration settings from various
//! sources including files, environment variables, and command-line arguments.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// CLI configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Default transport configuration
    pub transport: Option<mcp_core::transport::TransportConfig>,

    /// Client information
    pub client: ClientConfig,

    /// Debug settings
    pub debug: DebugConfig,

    /// Logging configuration
    pub logging: LoggingConfig,

    /// TUI settings
    pub tui: TuiConfig,
}

/// Client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Client name
    pub name: String,

    /// Client version
    pub version: String,

    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

/// Debug configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugConfig {
    /// Show raw protocol messages
    pub show_raw_messages: bool,

    /// Automatically save sessions
    pub auto_save_sessions: bool,

    /// Session save directory
    pub session_directory: PathBuf,

    /// Maximum number of saved sessions
    pub max_saved_sessions: u32,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,

    /// Log format (pretty, json, compact)
    pub format: String,

    /// Optional log file path
    pub file: Option<PathBuf>,

    /// Whether to log to stderr
    pub stderr: bool,
}

/// TUI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    /// Color scheme
    pub color_scheme: String,

    /// Key bindings
    pub key_bindings: std::collections::HashMap<String, String>,

    /// UI refresh rate in milliseconds
    pub refresh_rate_ms: u64,

    /// Whether to show help panel by default
    pub show_help: bool,
}

// Default implementation is now derived

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            name: "mcp-probe".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            metadata: std::collections::HashMap::new(),
        }
    }
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            show_raw_messages: false,
            auto_save_sessions: true,
            session_directory: dirs::home_dir()
                .unwrap_or_else(|| ".".into())
                .join(".mcp-probe")
                .join("sessions"),
            max_saved_sessions: 100,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "pretty".to_string(),
            file: None,
            stderr: true,
        }
    }
}

impl Default for TuiConfig {
    fn default() -> Self {
        let mut key_bindings = std::collections::HashMap::new();
        key_bindings.insert("quit".to_string(), "q".to_string());
        key_bindings.insert("help".to_string(), "h".to_string());
        key_bindings.insert("refresh".to_string(), "r".to_string());

        Self {
            color_scheme: "default".to_string(),
            key_bindings,
            refresh_rate_ms: 100,
            show_help: true,
        }
    }
}

impl Config {
    /// Load configuration from file
    #[allow(dead_code)]
    pub fn load_from_file(path: &std::path::Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;

        Ok(config)
    }

    /// Save configuration to file
    #[allow(dead_code)]
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;

        Ok(())
    }

    /// Merge with another configuration (other takes precedence)
    #[allow(dead_code)]
    pub fn merge(&mut self, other: &Config) {
        if other.transport.is_some() {
            self.transport = other.transport.clone();
        }

        // Merge debug settings
        if other.debug.show_raw_messages {
            self.debug.show_raw_messages = true;
        }

        // Merge logging settings
        if other.logging.level != "info" {
            self.logging.level = other.logging.level.clone();
        }

        if other.logging.format != "pretty" {
            self.logging.format = other.logging.format.clone();
        }

        if other.logging.file.is_some() {
            self.logging.file = other.logging.file.clone();
        }
    }

    /// Get default configuration file path
    #[allow(dead_code)]
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| ".".into()))
            .join("mcp-probe")
            .join("config.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.client.name, "mcp-probe");
        assert_eq!(config.logging.level, "info");
        assert!(!config.debug.show_raw_messages);
    }

    #[test]
    fn test_config_serialization() -> Result<()> {
        let config = Config::default();
        let toml_str = toml::to_string(&config)?;
        let parsed: Config = toml::from_str(&toml_str)?;

        assert_eq!(config.client.name, parsed.client.name);
        assert_eq!(config.logging.level, parsed.logging.level);

        Ok(())
    }

    #[test]
    fn test_config_file_operations() -> Result<()> {
        let config = Config::default();
        let temp_file = NamedTempFile::new()?;

        // Save config
        config.save_to_file(temp_file.path())?;

        // Load config
        let loaded = Config::load_from_file(temp_file.path())?;
        assert_eq!(config.client.name, loaded.client.name);

        Ok(())
    }

    #[test]
    fn test_config_merge() {
        let mut base = Config::default();
        let mut other = Config::default();
        other.debug.show_raw_messages = true;
        other.logging.level = "debug".to_string();

        base.merge(&other);

        assert!(base.debug.show_raw_messages);
        assert_eq!(base.logging.level, "debug");
    }
}
