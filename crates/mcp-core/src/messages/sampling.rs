//! Sampling-related message types for MCP LLM completion requests.
//!
//! This module provides types for:
//! - Server-to-client LLM completion requests
//! - Completion parameters (temperature, max tokens, etc.)
//! - Completion responses with generated content
//! - Model selection and configuration

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Request from server to client for LLM completion.
///
/// This allows MCP servers to request LLM completions from the client,
/// enabling servers to leverage the client's LLM capabilities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompleteRequest {
    /// The completion argument
    pub argument: CompletionArgument,
}

/// Arguments for a completion request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionArgument {
    /// Messages for the completion
    pub messages: Vec<SamplingMessage>,

    /// Optional model selection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_preferences: Option<ModelPreferences>,

    /// System prompt for the completion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// Include context about tools available to the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_context: Option<String>,

    /// Temperature for sampling (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Maximum number of tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,

    /// Stop sequences for completion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,

    /// Additional metadata for the request
    #[serde(flatten)]
    pub metadata: HashMap<String, Value>,
}

impl CompletionArgument {
    /// Create a new completion argument with messages.
    pub fn new(messages: Vec<SamplingMessage>) -> Self {
        Self {
            messages,
            model_preferences: None,
            system_prompt: None,
            include_context: None,
            temperature: None,
            max_tokens: None,
            stop_sequences: None,
            metadata: HashMap::new(),
        }
    }

    /// Set model preferences.
    pub fn with_model_preferences(mut self, preferences: ModelPreferences) -> Self {
        self.model_preferences = Some(preferences);
        self
    }

    /// Set system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set temperature.
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set maximum tokens.
    pub fn with_max_tokens(mut self, max_tokens: i32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Add stop sequences.
    pub fn with_stop_sequences(mut self, sequences: Vec<String>) -> Self {
        self.stop_sequences = Some(sequences);
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Model preferences for completion requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelPreferences {
    /// Preferred model names in order of preference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<String>>,

    /// Minimum cost tier acceptable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_priority: Option<CostPriority>,

    /// Minimum speed tier acceptable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_priority: Option<SpeedPriority>,

    /// Minimum intelligence tier acceptable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence_priority: Option<IntelligencePriority>,
}

impl ModelPreferences {
    /// Create new model preferences.
    pub fn new() -> Self {
        Self {
            models: None,
            cost_priority: None,
            speed_priority: None,
            intelligence_priority: None,
        }
    }

    /// Set preferred models.
    pub fn with_models(mut self, models: Vec<String>) -> Self {
        self.models = Some(models);
        self
    }

    /// Set cost priority.
    pub fn with_cost_priority(mut self, priority: CostPriority) -> Self {
        self.cost_priority = Some(priority);
        self
    }

    /// Set speed priority.
    pub fn with_speed_priority(mut self, priority: SpeedPriority) -> Self {
        self.speed_priority = Some(priority);
        self
    }

    /// Set intelligence priority.
    pub fn with_intelligence_priority(mut self, priority: IntelligencePriority) -> Self {
        self.intelligence_priority = Some(priority);
        self
    }
}

impl Default for ModelPreferences {
    fn default() -> Self {
        Self::new()
    }
}

/// Cost priority levels for model selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CostPriority {
    /// Lowest cost models
    Low,
    /// Medium cost models
    Medium,
    /// High cost models acceptable
    High,
}

/// Speed priority levels for model selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpeedPriority {
    /// Slowest acceptable speed
    Low,
    /// Medium speed required
    Medium,
    /// High speed required
    High,
}

/// Intelligence priority levels for model selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IntelligencePriority {
    /// Basic intelligence level
    Low,
    /// Medium intelligence level
    Medium,
    /// High intelligence level required
    High,
}

/// A message in a sampling request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SamplingMessage {
    /// Role of the message
    pub role: MessageRole,

    /// Content of the message
    pub content: SamplingContent,
}

impl SamplingMessage {
    /// Create a new sampling message.
    pub fn new(role: MessageRole, content: SamplingContent) -> Self {
        Self { role, content }
    }

    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(MessageRole::System, SamplingContent::text(content))
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(MessageRole::User, SamplingContent::text(content))
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Assistant, SamplingContent::text(content))
    }
}

/// Role of a message in sampling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// System message
    System,
    /// User message
    User,
    /// Assistant message
    Assistant,
}

/// Content of a sampling message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SamplingContent {
    /// Text content
    #[serde(rename = "text")]
    Text {
        /// The text content
        text: String,
    },

    /// Image content
    #[serde(rename = "image")]
    Image {
        /// Image data (base64 or URL)
        data: String,

        /// MIME type of the image
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
}

impl SamplingContent {
    /// Create text content.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create image content.
    pub fn image(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self::Image {
            data: data.into(),
            mime_type: mime_type.into(),
        }
    }
}

/// Response to a completion request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompleteResponse {
    /// Completion result
    pub completion: CompletionResult,

    /// Model used for the completion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Stop reason for the completion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<StopReason>,
}

/// Result of a completion request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CompletionResult {
    /// Text completion result
    #[serde(rename = "text")]
    Text {
        /// Generated text
        text: String,
    },
}

impl CompletionResult {
    /// Create a text completion result.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }
}

/// Reason why completion stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// Reached end of sequence naturally
    EndTurn,
    /// Hit maximum token limit
    MaxTokens,
    /// Encountered stop sequence
    StopSequence,
    /// Tool call was made
    ToolUse,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_completion_argument_creation() {
        let messages = vec![
            SamplingMessage::system("You are a helpful assistant"),
            SamplingMessage::user("Hello, how are you?"),
        ];

        let arg = CompletionArgument::new(messages)
            .with_temperature(0.7)
            .with_max_tokens(1000)
            .with_system_prompt("Be helpful")
            .with_metadata("priority", json!("high"));

        assert_eq!(arg.temperature, Some(0.7));
        assert_eq!(arg.max_tokens, Some(1000));
        assert_eq!(arg.system_prompt, Some("Be helpful".to_string()));
        assert_eq!(arg.metadata.get("priority"), Some(&json!("high")));
    }

    #[test]
    fn test_model_preferences() {
        let prefs = ModelPreferences::new()
            .with_models(vec!["gpt-4".to_string(), "claude-3".to_string()])
            .with_cost_priority(CostPriority::Medium)
            .with_speed_priority(SpeedPriority::High)
            .with_intelligence_priority(IntelligencePriority::High);

        assert_eq!(
            prefs.models,
            Some(vec!["gpt-4".to_string(), "claude-3".to_string()])
        );
        assert_eq!(prefs.cost_priority, Some(CostPriority::Medium));
        assert_eq!(prefs.speed_priority, Some(SpeedPriority::High));
        assert_eq!(
            prefs.intelligence_priority,
            Some(IntelligencePriority::High)
        );
    }

    #[test]
    fn test_sampling_message_creation() {
        let system_msg = SamplingMessage::system("You are helpful");
        let user_msg = SamplingMessage::user("Hello");
        let assistant_msg = SamplingMessage::assistant("Hi there!");

        assert_eq!(system_msg.role, MessageRole::System);
        assert_eq!(user_msg.role, MessageRole::User);
        assert_eq!(assistant_msg.role, MessageRole::Assistant);
    }

    #[test]
    fn test_sampling_content_text() {
        let content = SamplingContent::text("Hello world");
        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(json["type"], "text");
        assert_eq!(json["text"], "Hello world");
    }

    #[test]
    fn test_sampling_content_image() {
        let content = SamplingContent::image("base64data", "image/png");
        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(json["type"], "image");
        assert_eq!(json["data"], "base64data");
        assert_eq!(json["mimeType"], "image/png");
    }

    #[test]
    fn test_completion_result() {
        let result = CompletionResult::text("Generated response");
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["type"], "text");
        assert_eq!(json["text"], "Generated response");
    }

    #[test]
    fn test_priority_serialization() {
        let cost = CostPriority::Low;
        let speed = SpeedPriority::Medium;
        let intel = IntelligencePriority::High;

        assert_eq!(serde_json::to_string(&cost).unwrap(), "\"low\"");
        assert_eq!(serde_json::to_string(&speed).unwrap(), "\"medium\"");
        assert_eq!(serde_json::to_string(&intel).unwrap(), "\"high\"");
    }

    #[test]
    fn test_stop_reason_serialization() {
        let reasons = [
            StopReason::EndTurn,
            StopReason::MaxTokens,
            StopReason::StopSequence,
            StopReason::ToolUse,
        ];

        let expected = [
            "\"end_turn\"",
            "\"max_tokens\"",
            "\"stop_sequence\"",
            "\"tool_use\"",
        ];

        for (reason, expected) in reasons.iter().zip(expected.iter()) {
            assert_eq!(serde_json::to_string(reason).unwrap(), *expected);
        }
    }
}
