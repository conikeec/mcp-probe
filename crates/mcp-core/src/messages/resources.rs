//! Resource-related message types for MCP resource discovery and access.
//!
//! This module provides types for:
//! - Resource discovery (listing available resources)
//! - Resource access (reading resource content)
//! - Resource subscriptions (watching for changes)
//! - Resource content handling (text, binary, etc.)

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Request to list available resources from the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListResourcesRequest {
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Response containing the list of available resources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListResourcesResponse {
    /// List of available resources
    pub resources: Vec<Resource>,

    /// Optional cursor for next page of results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Resource definition including metadata and access information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resource {
    /// Unique URI identifying the resource
    pub uri: String,

    /// Human-readable name of the resource
    pub name: String,

    /// Description of what the resource contains
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// MIME type of the resource content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

impl Resource {
    /// Create a new resource definition.
    pub fn new(uri: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            name: name.into(),
            description: None,
            mime_type: None,
        }
    }

    /// Set the description for this resource.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the MIME type for this resource.
    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }
}

/// Request to read the content of a specific resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadResourceRequest {
    /// URI of the resource to read
    pub uri: String,
}

/// Response containing the content of a resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadResourceResponse {
    /// Content of the resource
    #[serde(default)]
    pub contents: Vec<ResourceContent>,
}

/// Content of a resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResourceContent {
    /// Text content
    #[serde(rename = "text")]
    Text {
        /// The text content
        text: String,

        /// URI of the resource
        uri: String,

        /// MIME type of the content
        #[serde(rename = "mimeType")]
        #[serde(skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },

    /// Binary content (base64 encoded)
    #[serde(rename = "blob")]
    Blob {
        /// Base64 encoded binary data
        blob: String,

        /// URI of the resource
        uri: String,

        /// MIME type of the content
        #[serde(rename = "mimeType")]
        #[serde(skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
}

impl ResourceContent {
    /// Create text content.
    pub fn text(uri: impl Into<String>, text: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            uri: uri.into(),
            mime_type: None,
        }
    }

    /// Create text content with MIME type.
    pub fn text_with_mime_type(
        uri: impl Into<String>,
        text: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        Self::Text {
            text: text.into(),
            uri: uri.into(),
            mime_type: Some(mime_type.into()),
        }
    }

    /// Create binary content.
    pub fn blob(uri: impl Into<String>, blob: impl Into<String>) -> Self {
        Self::Blob {
            blob: blob.into(),
            uri: uri.into(),
            mime_type: None,
        }
    }

    /// Create binary content with MIME type.
    pub fn blob_with_mime_type(
        uri: impl Into<String>,
        blob: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        Self::Blob {
            blob: blob.into(),
            uri: uri.into(),
            mime_type: Some(mime_type.into()),
        }
    }

    /// Get the URI of this content.
    pub fn uri(&self) -> &str {
        match self {
            Self::Text { uri, .. } => uri,
            Self::Blob { uri, .. } => uri,
        }
    }

    /// Get the MIME type of this content.
    pub fn mime_type(&self) -> Option<&str> {
        match self {
            Self::Text { mime_type, .. } => mime_type.as_deref(),
            Self::Blob { mime_type, .. } => mime_type.as_deref(),
        }
    }
}

/// Request to subscribe to changes in a resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscribeRequest {
    /// URI of the resource to subscribe to
    pub uri: String,
}

/// Request to unsubscribe from changes in a resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsubscribeRequest {
    /// URI of the resource to unsubscribe from
    pub uri: String,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_resource_creation() {
        let resource = Resource::new("file:///path/to/file.txt", "file.txt")
            .with_description("A text file")
            .with_mime_type("text/plain");

        assert_eq!(resource.uri, "file:///path/to/file.txt");
        assert_eq!(resource.name, "file.txt");
        assert_eq!(resource.description, Some("A text file".to_string()));
        assert_eq!(resource.mime_type, Some("text/plain".to_string()));
    }

    #[test]
    fn test_list_resources_request() {
        let request = ListResourcesRequest { cursor: None };
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: ListResourcesRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request, deserialized);
    }

    #[test]
    fn test_read_resource_request() {
        let request = ReadResourceRequest {
            uri: "file:///path/to/file.txt".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: ReadResourceRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request, deserialized);
    }

    #[test]
    fn test_resource_content_text() {
        let content =
            ResourceContent::text_with_mime_type("file:///test.txt", "Hello world", "text/plain");

        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(json["type"], "text");
        assert_eq!(json["text"], "Hello world");
        assert_eq!(json["mimeType"], "text/plain");
        assert_eq!(content.uri(), "file:///test.txt");
        assert_eq!(content.mime_type(), Some("text/plain"));
    }

    #[test]
    fn test_resource_content_blob() {
        let content =
            ResourceContent::blob_with_mime_type("file:///test.png", "base64data", "image/png");

        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(json["type"], "blob");
        assert_eq!(json["blob"], "base64data");
        assert_eq!(json["mimeType"], "image/png");
        assert_eq!(content.uri(), "file:///test.png");
        assert_eq!(content.mime_type(), Some("image/png"));
    }

    #[test]
    fn test_resource_updated_notification() {
        let notification = ResourceUpdatedNotification::new("file:///test.txt")
            .with_metadata("timestamp", json!("2024-01-01T00:00:00Z"));

        assert_eq!(notification.uri, "file:///test.txt");
        assert_eq!(
            notification.metadata.get("timestamp"),
            Some(&json!("2024-01-01T00:00:00Z"))
        );
    }
}
