//! Parameter validation utilities for MCP tool execution.
//!
//! This module provides reusable parameter validation logic that can be used across
//! interactive TUI mode, non-interactive CLI mode, and validation engines.

use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

/// Parameter validation errors
#[derive(Error, Debug, Clone)]
pub enum ValidationError {
    /// Schema compilation failed due to invalid JSON Schema
    #[error("Schema compilation failed: {0}")]
    SchemaError(String),

    /// A required parameter is missing from the input
    #[error("Parameter '{field}' is required but missing")]
    MissingRequired {
        /// The name of the missing required field
        field: String,
    },

    /// Parameter validation failed against the schema
    #[error("Parameter '{field}' validation failed: {reason}")]
    ValidationFailed {
        /// The name of the field that failed validation
        field: String,
        /// The reason why validation failed
        reason: String,
    },

    /// Value transformation failed during auto-correction
    #[error("Value transformation failed for '{field}': {reason}")]
    TransformationFailed {
        /// The name of the field where transformation failed
        field: String,
        /// The reason why transformation failed
        reason: String,
    },

    /// The provided JSON Schema is malformed or invalid
    #[error("JSON Schema is invalid: {0}")]
    InvalidSchema(String),
}

/// Result of parameter validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed
    pub is_valid: bool,
    /// List of validation errors
    pub errors: Vec<ValidationError>,
    /// List of warnings (non-blocking issues)
    pub warnings: Vec<String>,
    /// Transformed/cleaned parameters ready for use
    pub validated_params: Value,
    /// Applied transformations (for logging/debugging)
    pub transformations: Vec<String>,
}

/// Parameter validation and transformation engine
pub struct ParameterValidator {
    /// Whether to apply automatic transformations
    pub auto_transform: bool,
    /// Whether to be strict about unknown properties
    pub strict_mode: bool,
}

impl Default for ParameterValidator {
    fn default() -> Self {
        Self {
            auto_transform: true,
            strict_mode: false,
        }
    }
}

impl ParameterValidator {
    /// Create a new parameter validator
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a strict validator (no auto-transforms, strict schema compliance)
    pub fn strict() -> Self {
        Self {
            auto_transform: false,
            strict_mode: true,
        }
    }

    /// Validate parameters against a JSON Schema
    pub fn validate(&self, schema: &Value, params: &Value) -> ValidationResult {
        let mut result = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            validated_params: params.clone(),
            transformations: Vec::new(),
        };

        // Validate schema syntax
        if let Err(e) = self.validate_schema_syntax(schema) {
            result.is_valid = false;
            result.errors.push(e);
            return result;
        }

        // Apply transformations if enabled
        if self.auto_transform {
            if let Err(e) = self.apply_transformations(schema, &mut result) {
                result.is_valid = false;
                result.errors.push(e);
                return result;
            }
        }

        // Perform basic validation against schema
        if let Err(e) = self.validate_against_schema(schema, &result.validated_params) {
            result.is_valid = false;
            result.errors.push(e);
        }

        // Check for required fields
        if let Err(e) = self.check_required_fields(schema, &result.validated_params) {
            result.is_valid = false;
            result.errors.push(e);
        }

        result
    }

    /// Validate JSON Schema syntax (simplified - no external deps for now)
    fn validate_schema_syntax(&self, schema: &Value) -> Result<(), ValidationError> {
        // Basic validation - ensure it's an object with proper structure
        if !schema.is_object() {
            return Err(ValidationError::InvalidSchema(
                "Schema must be a JSON object".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate parameters against schema (simplified validation)
    fn validate_against_schema(
        &self,
        schema: &Value,
        params: &Value,
    ) -> Result<(), ValidationError> {
        // Get the properties from the schema
        if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
            if let Some(params_obj) = params.as_object() {
                for (field_name, field_schema) in properties {
                    if let Some(param_value) = params_obj.get(field_name) {
                        // Check basic type validation
                        if let Some(expected_type) =
                            field_schema.get("type").and_then(|t| t.as_str())
                        {
                            let valid_type = match expected_type {
                                "string" => param_value.is_string(),
                                "number" => param_value.is_number(),
                                "integer" => {
                                    param_value.is_number()
                                        && param_value.as_f64().is_some_and(|n| n.fract() == 0.0)
                                }
                                "boolean" => param_value.is_boolean(),
                                "array" => param_value.is_array(),
                                "object" => param_value.is_object(),
                                _ => true, // Allow unknown types
                            };

                            if !valid_type {
                                return Err(ValidationError::ValidationFailed {
                                    field: field_name.clone(),
                                    reason: format!(
                                        "Expected type '{}' but got '{}'",
                                        expected_type,
                                        if param_value.is_string() {
                                            "string"
                                        } else if param_value.is_number() {
                                            "number"
                                        } else if param_value.is_boolean() {
                                            "boolean"
                                        } else if param_value.is_array() {
                                            "array"
                                        } else if param_value.is_object() {
                                            "object"
                                        } else {
                                            "null"
                                        }
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Apply automatic transformations to parameters
    fn apply_transformations(
        &self,
        schema: &Value,
        result: &mut ValidationResult,
    ) -> Result<(), ValidationError> {
        if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
            if let Value::Object(ref mut params_map) = result.validated_params {
                let mut transformations = Vec::new();

                for (field_name, field_schema) in properties {
                    if let Some(param_value) = params_map.get_mut(field_name) {
                        let field_transformations =
                            self.transform_field_value(field_name, field_schema, param_value)?;
                        transformations.extend(field_transformations);
                    }
                }

                result.transformations.extend(transformations);
            }
        }
        Ok(())
    }

    /// Transform a single field value based on its schema
    fn transform_field_value(
        &self,
        field_name: &str,
        field_schema: &Value,
        param_value: &mut Value,
    ) -> Result<Vec<String>, ValidationError> {
        let mut transformations = Vec::new();

        // URL auto-prefixing for string fields that look like URLs
        if let Some("string") = field_schema.get("type").and_then(|t| t.as_str()) {
            if let Some(description) = field_schema.get("description").and_then(|d| d.as_str()) {
                let desc_lower = description.to_lowercase();
                if desc_lower.contains("url")
                    || desc_lower.contains("uri")
                    || field_name.to_lowercase().contains("url")
                {
                    if let Value::String(url_str) = param_value {
                        let original_url = url_str.clone();
                        if let Some(fixed_url) = self.auto_fix_url(&original_url) {
                            *param_value = Value::String(fixed_url.clone());
                            transformations.push(format!(
                                "Auto-prefixed URL in '{field_name}': '{original_url}' → '{fixed_url}'"
                            ));
                        }
                    }
                }
            }
        }

        // Number type coercion
        if let Some("number") = field_schema.get("type").and_then(|t| t.as_str()) {
            if let Value::String(str_val) = param_value {
                let original_str = str_val.clone();
                if let Ok(num_val) = original_str.parse::<f64>() {
                    *param_value = Value::Number(serde_json::Number::from_f64(num_val).unwrap());
                    transformations.push(format!(
                        "Converted string to number in '{field_name}': '{original_str}' → {num_val}"
                    ));
                }
            }
        }

        // Integer type coercion
        if let Some("integer") = field_schema.get("type").and_then(|t| t.as_str()) {
            if let Value::String(str_val) = param_value {
                let original_str = str_val.clone();
                if let Ok(int_val) = original_str.parse::<i64>() {
                    *param_value = Value::Number(serde_json::Number::from(int_val));
                    transformations.push(format!(
                        "Converted string to integer in '{field_name}': '{original_str}' → {int_val}"
                    ));
                }
            }
        }

        // Boolean type coercion
        if let Some("boolean") = field_schema.get("type").and_then(|t| t.as_str()) {
            if let Value::String(str_val) = param_value {
                let original_str = str_val.clone();
                let bool_val = match original_str.to_lowercase().as_str() {
                    "true" | "yes" | "1" | "on" => Some(true),
                    "false" | "no" | "0" | "off" => Some(false),
                    _ => None,
                };
                if let Some(bool_val) = bool_val {
                    *param_value = Value::Bool(bool_val);
                    transformations.push(format!(
                        "Converted string to boolean in '{field_name}': '{original_str}' → {bool_val}"
                    ));
                }
            }
        }

        Ok(transformations)
    }

    /// Auto-fix URL format issues
    fn auto_fix_url(&self, url: &str) -> Option<String> {
        if url.is_empty() || url.starts_with("http://") || url.starts_with("https://") {
            return None; // Already valid or empty
        }

        // Auto-prefix based on common patterns
        if url.starts_with("localhost")
            || url.starts_with("127.0.0.1")
            || url.starts_with("0.0.0.0")
        {
            Some(format!("http://{url}"))
        } else if url.contains('.') && !url.contains(' ') {
            // Looks like a domain name
            Some(format!("https://{url}"))
        } else {
            None
        }
    }

    /// Check for required fields
    fn check_required_fields(&self, schema: &Value, params: &Value) -> Result<(), ValidationError> {
        if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
            if let Some(params_obj) = params.as_object() {
                for required_field in required {
                    if let Some(field_name) = required_field.as_str() {
                        if !params_obj.contains_key(field_name) {
                            return Err(ValidationError::MissingRequired {
                                field: field_name.to_string(),
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Quick validation check (returns only boolean)
    pub fn is_valid(&self, schema: &Value, params: &Value) -> bool {
        self.validate(schema, params).is_valid
    }

    /// Extract parameter hints from schema (for UI display)
    pub fn extract_parameter_hints(&self, schema: &Value) -> HashMap<String, ParameterHint> {
        let mut hints = HashMap::new();

        if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
            let required_fields: Vec<String> = schema
                .get("required")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            for (field_name, field_schema) in properties {
                let hint = ParameterHint {
                    name: field_name.clone(),
                    param_type: field_schema
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("string")
                        .to_string(),
                    description: field_schema
                        .get("description")
                        .and_then(|d| d.as_str())
                        .map(|s| s.to_string()),
                    required: required_fields.contains(field_name),
                    default_value: field_schema.get("default").cloned(),
                    enum_values: field_schema.get("enum").and_then(|e| e.as_array()).cloned(),
                    format: field_schema
                        .get("format")
                        .and_then(|f| f.as_str())
                        .map(|s| s.to_string()),
                    pattern: field_schema
                        .get("pattern")
                        .and_then(|p| p.as_str())
                        .map(|s| s.to_string()),
                    min_length: field_schema.get("minLength").and_then(|m| m.as_u64()),
                    max_length: field_schema.get("maxLength").and_then(|m| m.as_u64()),
                };
                hints.insert(field_name.clone(), hint);
            }
        }

        hints
    }
}

/// Parameter hint information extracted from JSON Schema
#[derive(Debug, Clone)]
pub struct ParameterHint {
    /// The parameter name
    pub name: String,
    /// The parameter type (string, number, boolean, etc.)
    pub param_type: String,
    /// Optional description of the parameter
    pub description: Option<String>,
    /// Whether this parameter is required
    pub required: bool,
    /// Default value for the parameter, if any
    pub default_value: Option<Value>,
    /// Allowed enum values, if the parameter is an enum
    pub enum_values: Option<Vec<Value>>,
    /// Format constraint (e.g., "uri", "email", "date-time")
    pub format: Option<String>,
    /// Regex pattern the value must match
    pub pattern: Option<String>,
    /// Minimum length for string values
    pub min_length: Option<u64>,
    /// Maximum length for string values
    pub max_length: Option<u64>,
}

/// Convenience function for quick parameter validation
pub fn validate_parameters(schema: &Value, params: &Value) -> ValidationResult {
    ParameterValidator::new().validate(schema, params)
}

/// Convenience function for strict parameter validation (no transformations)
pub fn validate_parameters_strict(schema: &Value, params: &Value) -> ValidationResult {
    ParameterValidator::strict().validate(schema, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_url_auto_prefixing() {
        let schema = json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to navigate to"
                }
            },
            "required": ["url"]
        });

        let params = json!({"url": "www.google.com"});
        let validator = ParameterValidator::new();
        let result = validator.validate(&schema, &params);

        assert!(result.is_valid);
        assert_eq!(result.validated_params["url"], "https://www.google.com");
        assert!(!result.transformations.is_empty());
    }

    #[test]
    fn test_localhost_url_prefixing() {
        let schema = json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to navigate to"
                }
            }
        });

        let params = json!({"url": "localhost:3000"});
        let validator = ParameterValidator::new();
        let result = validator.validate(&schema, &params);

        assert!(result.is_valid);
        assert_eq!(result.validated_params["url"], "http://localhost:3000");
    }

    #[test]
    fn test_type_coercion() {
        let schema = json!({
            "type": "object",
            "properties": {
                "width": {"type": "number"},
                "height": {"type": "integer"},
                "visible": {"type": "boolean"}
            }
        });

        let params = json!({
            "width": "800.5",
            "height": "600",
            "visible": "true"
        });

        let validator = ParameterValidator::new();
        let result = validator.validate(&schema, &params);

        assert!(result.is_valid);
        assert_eq!(result.validated_params["width"], 800.5);
        assert_eq!(result.validated_params["height"], 600);
        assert_eq!(result.validated_params["visible"], true);
        assert_eq!(result.transformations.len(), 3);
    }

    #[test]
    fn test_required_field_validation() {
        let schema = json!({
            "type": "object",
            "properties": {
                "url": {"type": "string"}
            },
            "required": ["url"]
        });

        let params = json!({});
        let validator = ParameterValidator::new();
        let result = validator.validate(&schema, &params);

        assert!(!result.is_valid);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e, ValidationError::MissingRequired { field } if field == "url")));
    }

    #[test]
    fn test_strict_mode_no_transforms() {
        let schema = json!({
            "type": "object",
            "properties": {
                "url": {"type": "string"}
            }
        });

        let params = json!({"url": "www.google.com"});
        let validator = ParameterValidator::strict();
        let result = validator.validate(&schema, &params);

        // In strict mode, no auto-transforms should occur
        assert_eq!(result.validated_params["url"], "www.google.com");
        assert!(result.transformations.is_empty());
    }
}
