//! Prompt-related message types for MCP prompt templates and completion.
//!
//! This module provides types for:
//! - Prompt discovery (listing available prompts)
//! - Prompt templates with parameter substitution
//! - Prompt generation with arguments
//! - Prompt content handling

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Request to list available prompts from the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListPromptsRequest {
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Response containing the list of available prompts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListPromptsResponse {
    /// List of available prompts
    pub prompts: Vec<Prompt>,

    /// Optional cursor for next page of results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Prompt definition including schema and metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prompt {
    /// Unique name of the prompt
    pub name: String,

    /// Human-readable description of the prompt
    pub description: String,

    /// JSON Schema for the prompt's arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

impl Prompt {
    /// Create a new prompt definition.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            arguments: None,
        }
    }

    /// Set the arguments schema for this prompt.
    pub fn with_arguments(mut self, arguments: Value) -> Self {
        self.arguments = Some(arguments);
        self
    }
}

/// Request to get a prompt with specific arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetPromptRequest {
    /// Name of the prompt to get
    pub name: String,

    /// Arguments to substitute in the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

/// Response containing the generated prompt content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetPromptResponse {
    /// Description of the prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Generated messages for the prompt
    #[serde(default)]
    pub messages: Vec<PromptMessage>,
}

/// A message in a prompt template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptMessage {
    /// Role of the message (system, user, assistant)
    pub role: MessageRole,

    /// Content of the message
    pub content: PromptContent,
}

impl PromptMessage {
    /// Create a new prompt message.
    pub fn new(role: MessageRole, content: PromptContent) -> Self {
        Self { role, content }
    }

    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(MessageRole::System, PromptContent::text(content))
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(MessageRole::User, PromptContent::text(content))
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(MessageRole::Assistant, PromptContent::text(content))
    }
}

/// Role of a message in a conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// System message (instructions)
    System,
    /// User message (human input)
    User,
    /// Assistant message (AI response)
    Assistant,
}

/// Content of a prompt message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PromptContent {
    /// Text content
    #[serde(rename = "text")]
    Text {
        /// The text content
        text: String,
    },

    /// Image content
    #[serde(rename = "image")]
    Image {
        /// Image data (base64 encoded or URL)
        data: String,

        /// MIME type of the image
        #[serde(rename = "mimeType")]
        mime_type: String,
    },

    /// Resource reference
    #[serde(rename = "resource")]
    Resource {
        /// Resource reference
        resource: ResourceReference,
    },
}

impl PromptContent {
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

    /// Create resource content.
    pub fn resource(uri: impl Into<String>) -> Self {
        Self::Resource {
            resource: ResourceReference {
                uri: uri.into(),
                text: None,
            },
        }
    }

    /// Create resource content with description.
    pub fn resource_with_text(uri: impl Into<String>, text: impl Into<String>) -> Self {
        Self::Resource {
            resource: ResourceReference {
                uri: uri.into(),
                text: Some(text.into()),
            },
        }
    }
}

/// Reference to a resource within prompt content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceReference {
    /// URI of the resource
    pub uri: String,

    /// Optional description of the resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
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
    fn test_prompt_creation() {
        let prompt =
            Prompt::new("code_review", "Review code for best practices").with_arguments(json!({
                "type": "object",
                "properties": {
                    "language": {"type": "string"},
                    "code": {"type": "string"}
                },
                "required": ["code"]
            }));

        assert_eq!(prompt.name, "code_review");
        assert_eq!(prompt.description, "Review code for best practices");
        assert!(prompt.arguments.is_some());
    }

    #[test]
    fn test_list_prompts_request() {
        let request = ListPromptsRequest { cursor: None };
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: ListPromptsRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request, deserialized);
    }

    #[test]
    fn test_get_prompt_request() {
        let request = GetPromptRequest {
            name: "code_review".to_string(),
            arguments: Some(json!({"language": "rust", "code": "fn main() {}"})),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: GetPromptRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request, deserialized);
    }

    #[test]
    fn test_prompt_message_creation() {
        let system_msg = PromptMessage::system("You are a helpful assistant");
        let user_msg = PromptMessage::user("Hello, how are you?");
        let assistant_msg = PromptMessage::assistant("I'm doing well, thank you!");

        assert_eq!(system_msg.role, MessageRole::System);
        assert_eq!(user_msg.role, MessageRole::User);
        assert_eq!(assistant_msg.role, MessageRole::Assistant);
    }

    #[test]
    fn test_prompt_content_text() {
        let content = PromptContent::text("Hello world");
        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(json["type"], "text");
        assert_eq!(json["text"], "Hello world");
    }

    #[test]
    fn test_prompt_content_image() {
        let content = PromptContent::image("base64data", "image/png");
        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(json["type"], "image");
        assert_eq!(json["data"], "base64data");
        assert_eq!(json["mimeType"], "image/png");
    }

    #[test]
    fn test_prompt_content_resource() {
        let content = PromptContent::resource_with_text("file:///test.txt", "A test file");
        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(json["type"], "resource");
        assert_eq!(json["resource"]["uri"], "file:///test.txt");
        assert_eq!(json["resource"]["text"], "A test file");
    }

    #[test]
    fn test_message_role_serialization() {
        let system_role = MessageRole::System;
        let json = serde_json::to_string(&system_role).unwrap();
        assert_eq!(json, "\"system\"");

        let user_role = MessageRole::User;
        let json = serde_json::to_string(&user_role).unwrap();
        assert_eq!(json, "\"user\"");

        let assistant_role = MessageRole::Assistant;
        let json = serde_json::to_string(&assistant_role).unwrap();
        assert_eq!(json, "\"assistant\"");
    }
}
