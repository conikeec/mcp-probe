//! Tool-related message types for MCP tool discovery and execution.
//!
//! This module provides types for:
//! - Tool discovery (listing available tools)
//! - Tool execution (calling tools with parameters)
//! - Tool schema definitions (parameter validation)
//! - Tool result handling (success/error responses)

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Request to list available tools from the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListToolsRequest {
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Response containing the list of available tools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListToolsResponse {
    /// List of available tools
    pub tools: Vec<Tool>,

    /// Optional cursor for next page of results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Tool definition including schema and metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Tool {
    /// Unique name of the tool
    pub name: String,

    /// Human-readable description of what the tool does
    pub description: String,

    /// JSON Schema for the tool's input parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<Value>,

    /// Additional tool extensions and metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Value>,

    /// Whether the tool is read-only
    #[serde(rename = "readOnly", skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,

    /// Return type schema for the tool
    #[serde(rename = "returnType", skip_serializing_if = "Option::is_none")]
    pub return_type: Option<Value>,
}

// Custom deserializer for Tool to handle multiple schema field names
impl<'de> Deserialize<'de> for Tool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "camelCase")]
        enum Field {
            Name,
            Description,
            #[serde(alias = "input_schema")]
            InputSchema,
            #[serde(alias = "parameters_schema")]
            ParametersSchema,
            Extensions,
            #[serde(alias = "read_only")]
            ReadOnly,
            ReturnType,
            #[serde(other)]
            Unknown,
        }

        struct ToolVisitor;

        impl<'de> Visitor<'de> for ToolVisitor {
            type Value = Tool;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Tool")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Tool, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut name = None;
                let mut description = None;
                let mut input_schema = None;
                let mut extensions = None;
                let mut read_only = None;
                let mut return_type = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Name => {
                            if name.is_some() {
                                return Err(de::Error::duplicate_field("name"));
                            }
                            name = Some(map.next_value()?);
                        }
                        Field::Description => {
                            if description.is_some() {
                                return Err(de::Error::duplicate_field("description"));
                            }
                            description = Some(map.next_value()?);
                        }
                        Field::InputSchema => {
                            if input_schema.is_none() {
                                input_schema = Some(map.next_value()?);
                            } else {
                                // Skip if we already have a schema
                                let _: Value = map.next_value()?;
                            }
                        }
                        Field::ParametersSchema => {
                            if input_schema.is_none() {
                                input_schema = Some(map.next_value()?);
                            } else {
                                // Skip if we already have a schema
                                let _: Value = map.next_value()?;
                            }
                        }
                        Field::Extensions => {
                            if extensions.is_some() {
                                return Err(de::Error::duplicate_field("extensions"));
                            }
                            extensions = Some(map.next_value()?);
                        }
                        Field::ReadOnly => {
                            if read_only.is_some() {
                                return Err(de::Error::duplicate_field("readOnly"));
                            }
                            read_only = Some(map.next_value()?);
                        }
                        Field::ReturnType => {
                            if return_type.is_some() {
                                return Err(de::Error::duplicate_field("returnType"));
                            }
                            return_type = Some(map.next_value()?);
                        }
                        Field::Unknown => {
                            // Skip unknown fields
                            let _: Value = map.next_value()?;
                        }
                    }
                }

                let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
                let description =
                    description.ok_or_else(|| de::Error::missing_field("description"))?;

                Ok(Tool {
                    name,
                    description,
                    input_schema,
                    extensions,
                    read_only,
                    return_type,
                })
            }
        }

        const FIELDS: &[&str] = &[
            "name",
            "description",
            "inputSchema",
            "parametersSchema",
            "extensions",
            "readOnly",
            "returnType",
        ];
        deserializer.deserialize_struct("Tool", FIELDS, ToolVisitor)
    }
}

impl Tool {
    /// Create a new tool definition.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema: None,
            extensions: None,
            read_only: None,
            return_type: None,
        }
    }

    /// Set the input schema for this tool.
    pub fn with_input_schema(mut self, schema: Value) -> Self {
        self.input_schema = Some(schema);
        self
    }

    /// Set the extensions for this tool.
    pub fn with_extensions(mut self, extensions: Value) -> Self {
        self.extensions = Some(extensions);
        self
    }

    /// Set the read-only flag for this tool.
    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.read_only = Some(read_only);
        self
    }

    /// Set the return type schema for this tool.
    pub fn with_return_type(mut self, return_type: Value) -> Self {
        self.return_type = Some(return_type);
        self
    }
}

/// Request to call a tool with specific arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallToolRequest {
    /// Name of the tool to call
    pub name: String,

    /// Arguments to pass to the tool
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

/// Response from a tool call operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallToolResponse {
    /// Results from the tool execution
    #[serde(default)]
    pub content: Vec<ToolResult>,

    /// Whether the tool is making a progress notification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Result content from a tool execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolResult {
    /// Text content result
    #[serde(rename = "text")]
    Text {
        /// The text content
        text: String,
    },

    /// Image content result
    #[serde(rename = "image")]
    Image {
        /// Image data (base64 encoded)
        data: String,

        /// MIME type of the image
        #[serde(rename = "mimeType")]
        mime_type: String,
    },

    /// Resource reference result
    #[serde(rename = "resource")]
    Resource {
        /// URI of the resource
        resource: ResourceReference,
    },
}

/// Reference to a resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceReference {
    /// URI of the resource
    pub uri: String,

    /// Optional description of the resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_creation() {
        let tool = Tool::new("calculator", "A simple calculator tool").with_input_schema(json!({
            "type": "object",
            "properties": {
                "expression": {"type": "string"}
            },
            "required": ["expression"]
        }));

        assert_eq!(tool.name, "calculator");
        assert_eq!(tool.description, "A simple calculator tool");
        assert!(tool.input_schema.is_some());
        assert_eq!(tool.extensions, None);
        assert_eq!(tool.read_only, None);
        assert_eq!(tool.return_type, None);
    }

    #[test]
    fn test_list_tools_request() {
        let request = ListToolsRequest { cursor: None };
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: ListToolsRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request, deserialized);
    }

    #[test]
    fn test_call_tool_request() {
        let request = CallToolRequest {
            name: "calculator".to_string(),
            arguments: Some(json!({"expression": "2 + 2"})),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: CallToolRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request, deserialized);
    }

    #[test]
    fn test_tool_result_text() {
        let result = ToolResult::Text {
            text: "The answer is 4".to_string(),
        };

        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["type"], "text");
        assert_eq!(json["text"], "The answer is 4");
    }

    #[test]
    fn test_tool_result_image() {
        let result = ToolResult::Image {
            data: "base64data".to_string(),
            mime_type: "image/png".to_string(),
        };

        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["type"], "image");
        assert_eq!(json["mimeType"], "image/png");
    }

    #[test]
    fn test_tool_deserialization_with_camel_case() {
        // Test that we can deserialize tools with camelCase field names
        let json_str = r#"{
            "name": "test-tool",
            "description": "A test tool",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "param1": {"type": "string"}
                }
            },
            "readOnly": true
        }"#;

        let tool: Tool = serde_json::from_str(json_str).unwrap();
        assert_eq!(tool.name, "test-tool");
        assert_eq!(tool.description, "A test tool");
        assert!(tool.input_schema.is_some());
        assert_eq!(tool.read_only, Some(true));
        assert_eq!(tool.return_type, None);
    }

    #[test]
    fn test_tool_deserialization_with_parameters_schema() {
        // Test that we can deserialize tools with parametersSchema field
        let json_str = r#"{
            "name": "test-tool",
            "description": "A test tool",
            "parametersSchema": {
                "type": "object",
                "properties": {
                    "param1": {"type": "string"}
                }
            }
        }"#;

        let tool: Tool = serde_json::from_str(json_str).unwrap();
        assert_eq!(tool.name, "test-tool");
        assert_eq!(tool.description, "A test tool");
        assert!(tool.input_schema.is_some());
    }
}
