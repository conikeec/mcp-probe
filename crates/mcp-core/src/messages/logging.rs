//! Logging-related message types for MCP server-to-client logging and progress.
//!
//! This module provides types for:
//! - Server logging messages to client
//! - Log level configuration
//! - Progress notifications for long-running operations
//! - Resource change notifications

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Log level enumeration for MCP logging.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Debug level logging (most verbose)
    Debug,
    /// Info level logging
    Info,
    /// Notice level logging
    Notice,
    /// Warning level logging
    Warning,
    /// Error level logging
    Error,
    /// Critical level logging (least verbose)
    Critical,
}

impl LogLevel {
    /// Get all log levels in order from most to least verbose.
    pub fn all() -> Vec<Self> {
        vec![
            Self::Debug,
            Self::Info,
            Self::Notice,
            Self::Warning,
            Self::Error,
            Self::Critical,
        ]
    }

    /// Check if this log level is more verbose than another.
    pub fn is_more_verbose_than(&self, other: &Self) -> bool {
        self < other
    }

    /// Check if this log level is less verbose than another.
    pub fn is_less_verbose_than(&self, other: &Self) -> bool {
        self > other
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Notice => "notice",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Critical => "critical",
        };
        write!(f, "{}", s)
    }
}

/// Request to set the logging level for the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetLevelRequest {
    /// The logging level to set
    pub level: LogLevel,
}

impl SetLevelRequest {
    /// Create a new set level request.
    pub fn new(level: LogLevel) -> Self {
        Self { level }
    }
}

/// Notification containing a log message from the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoggingNotification {
    /// The log level
    pub level: LogLevel,

    /// The log message
    pub data: Value,

    /// Optional logger name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logger: Option<String>,
}

impl LoggingNotification {
    /// Create a new logging notification.
    pub fn new(level: LogLevel, data: Value) -> Self {
        Self {
            level,
            data,
            logger: None,
        }
    }

    /// Create a logging notification with a logger name.
    pub fn with_logger(level: LogLevel, data: Value, logger: impl Into<String>) -> Self {
        Self {
            level,
            data,
            logger: Some(logger.into()),
        }
    }

    /// Create a debug log notification.
    pub fn debug(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Debug, Value::String(message.into()))
    }

    /// Create an info log notification.
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Info, Value::String(message.into()))
    }

    /// Create a notice log notification.
    pub fn notice(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Notice, Value::String(message.into()))
    }

    /// Create a warning log notification.
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Warning, Value::String(message.into()))
    }

    /// Create an error log notification.
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Error, Value::String(message.into()))
    }

    /// Create a critical log notification.
    pub fn critical(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Critical, Value::String(message.into()))
    }
}

/// Progress notification for long-running operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressNotification {
    /// Progress token for this operation
    pub progress_token: ProgressToken,

    /// Current progress (0.0 to 1.0)
    pub progress: f64,

    /// Total number of items (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
}

impl ProgressNotification {
    /// Create a new progress notification.
    pub fn new(progress_token: impl Into<ProgressToken>, progress: f64) -> Self {
        Self {
            progress_token: progress_token.into(),
            progress,
            total: None,
        }
    }

    /// Create a progress notification with a total count.
    pub fn with_total(progress_token: impl Into<ProgressToken>, progress: f64, total: u64) -> Self {
        Self {
            progress_token: progress_token.into(),
            progress,
            total: Some(total),
        }
    }
}

/// Progress token for tracking long-running operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProgressToken {
    /// String-based progress token
    String(String),
    /// Numeric progress token
    Number(i64),
}

impl From<String> for ProgressToken {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for ProgressToken {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<i64> for ProgressToken {
    fn from(n: i64) -> Self {
        Self::Number(n)
    }
}

impl From<i32> for ProgressToken {
    fn from(n: i32) -> Self {
        Self::Number(n as i64)
    }
}

impl std::fmt::Display for ProgressToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => write!(f, "{}", s),
            Self::Number(n) => write!(f, "{}", n),
        }
    }
}

/// Notification that a resource has been updated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceUpdatedNotification {
    /// URI of the updated resource
    pub uri: String,

    /// Additional metadata about the update
    #[serde(flatten)]
    pub metadata: HashMap<String, Value>,
}

impl ResourceUpdatedNotification {
    /// Create a new resource updated notification.
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the notification.
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Notification that the list of resources has changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ResourceListChangedNotification {
    /// Additional metadata about the change
    #[serde(flatten)]
    pub metadata: HashMap<String, Value>,
}

impl ResourceListChangedNotification {
    /// Create a new resource list changed notification.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add metadata to the notification.
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Notification that the list of tools has changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ToolListChangedNotification {
    /// Additional metadata about the change
    #[serde(flatten)]
    pub metadata: HashMap<String, Value>,
}

impl ToolListChangedNotification {
    /// Create a new tool list changed notification.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add metadata to the notification.
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Notification that the list of prompts has changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PromptListChangedNotification {
    /// Additional metadata about the change
    #[serde(flatten)]
    pub metadata: HashMap<String, Value>,
}

impl PromptListChangedNotification {
    /// Create a new prompt list changed notification.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add metadata to the notification.
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Notice);
        assert!(LogLevel::Notice < LogLevel::Warning);
        assert!(LogLevel::Warning < LogLevel::Error);
        assert!(LogLevel::Error < LogLevel::Critical);

        assert!(LogLevel::Debug.is_more_verbose_than(&LogLevel::Error));
        assert!(LogLevel::Error.is_less_verbose_than(&LogLevel::Debug));
    }

    #[test]
    fn test_log_level_serialization() {
        let levels = LogLevel::all();
        let expected = ["debug", "info", "notice", "warning", "error", "critical"];

        for (level, expected) in levels.iter().zip(expected.iter()) {
            let json = serde_json::to_string(level).unwrap();
            assert_eq!(json, format!("\"{expected}\""));
            assert_eq!(level.to_string(), *expected);
        }
    }

    #[test]
    fn test_set_level_request() {
        let request = SetLevelRequest::new(LogLevel::Warning);
        assert_eq!(request.level, LogLevel::Warning);

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: SetLevelRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request, deserialized);
    }

    #[test]
    fn test_logging_notification() {
        let notification = LoggingNotification::with_logger(
            LogLevel::Info,
            json!("This is a test message"),
            "test_logger",
        );

        assert_eq!(notification.level, LogLevel::Info);
        assert_eq!(notification.data, json!("This is a test message"));
        assert_eq!(notification.logger, Some("test_logger".to_string()));
    }

    #[test]
    fn test_logging_notification_helpers() {
        let debug = LoggingNotification::debug("Debug message");
        let info = LoggingNotification::info("Info message");
        let warning = LoggingNotification::warning("Warning message");
        let error = LoggingNotification::error("Error message");

        assert_eq!(debug.level, LogLevel::Debug);
        assert_eq!(info.level, LogLevel::Info);
        assert_eq!(warning.level, LogLevel::Warning);
        assert_eq!(error.level, LogLevel::Error);
    }

    #[test]
    fn test_progress_notification() {
        let progress = ProgressNotification::new("operation-1", 0.5);
        assert_eq!(
            progress.progress_token,
            ProgressToken::String("operation-1".to_string())
        );
        assert_eq!(progress.progress, 0.5);
        assert_eq!(progress.total, None);

        let progress_with_total = ProgressNotification::with_total("operation-2", 0.75, 100);
        assert_eq!(progress_with_total.total, Some(100));
    }

    #[test]
    fn test_progress_token() {
        let string_token = ProgressToken::from("test");
        let number_token = ProgressToken::from(42i64);

        assert_eq!(string_token.to_string(), "test");
        assert_eq!(number_token.to_string(), "42");

        // Test serialization
        let json_string = serde_json::to_string(&string_token).unwrap();
        let json_number = serde_json::to_string(&number_token).unwrap();

        assert_eq!(json_string, "\"test\"");
        assert_eq!(json_number, "42");
    }

    #[test]
    fn test_notification_with_metadata() {
        let notification = ResourceUpdatedNotification::new("file:///test.txt")
            .with_metadata("timestamp", json!("2024-01-01T00:00:00Z"))
            .with_metadata("size", json!(1024));

        assert_eq!(notification.uri, "file:///test.txt");
        assert_eq!(
            notification.metadata.get("timestamp"),
            Some(&json!("2024-01-01T00:00:00Z"))
        );
        assert_eq!(notification.metadata.get("size"), Some(&json!(1024)));
    }
}
