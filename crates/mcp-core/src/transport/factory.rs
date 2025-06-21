//! Transport factory for creating transport instances.

use super::{Transport, TransportConfig};
use crate::error::McpResult;

#[cfg(feature = "stdio")]
use super::stdio::StdioTransport;

#[cfg(feature = "http-sse")]
use super::http_sse::HttpSseTransport;

#[cfg(feature = "http-stream")]
use super::http_stream::HttpStreamTransport;

/// Factory for creating transport instances.
///
/// This factory provides a unified interface for creating different types of MCP transports
/// based on the provided configuration. It abstracts away the concrete transport types
/// and provides a consistent API for transport creation.
///
/// # Examples
///
/// ```rust
/// use mcp_probe_core::transport::{TransportFactory, TransportConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = TransportConfig::stdio("python", &["server.py"]);
///     let transport = TransportFactory::create(config).await?;
///     println!("Created transport: {}", transport.get_info().transport_type);
///     Ok(())
/// }
/// ```
pub struct TransportFactory;

impl TransportFactory {
    /// Create a transport instance from configuration.
    ///
    /// This method validates the configuration and creates the appropriate transport
    /// implementation based on the transport type specified in the config.
    ///
    /// # Arguments
    ///
    /// * `config` - The transport configuration specifying which transport to create
    ///
    /// # Returns
    ///
    /// A boxed transport instance ready for use, or an error if creation fails.
    ///
    /// # Errors
    ///
    /// * `ConfigError` - If the configuration is invalid
    /// * `TransportError` - If the transport cannot be created
    /// * `FeatureError` - If the requested transport type is not compiled in
    pub async fn create(config: TransportConfig) -> McpResult<Box<dyn Transport>> {
        // Validate configuration before attempting to create transport
        config.validate()?;

        match config {
            #[cfg(feature = "stdio")]
            TransportConfig::Stdio(_) => Ok(Box::new(StdioTransport::new(config))),

            #[cfg(not(feature = "stdio"))]
            TransportConfig::Stdio(_) => Err(crate::error::ConfigError::InvalidValue {
                parameter: "transport_type".to_string(),
                value: "stdio".to_string(),
                reason: "stdio transport support not compiled in (enable 'stdio' feature)"
                    .to_string(),
            }
            .into()),

            #[cfg(feature = "http-sse")]
            TransportConfig::HttpSse(_) => Ok(Box::new(HttpSseTransport::new(config)?)),

            #[cfg(not(feature = "http-sse"))]
            TransportConfig::HttpSse(_) => Err(crate::error::ConfigError::InvalidValue {
                parameter: "transport_type".to_string(),
                value: "http-sse".to_string(),
                reason: "http-sse transport support not compiled in (enable 'http-sse' feature)"
                    .to_string(),
            }
            .into()),

            #[cfg(feature = "http-stream")]
            TransportConfig::HttpStream(stream_config) => {
                let auth_header = stream_config.auth.as_ref().map(|auth| match auth {
                    crate::transport::config::AuthConfig::Bearer { token } => token.clone(),
                    crate::transport::config::AuthConfig::Basic { username, password } => {
                        // Proper base64 encoding for HTTP Basic Auth
                        let credentials = format!("{}:{}", username, password);
                        let encoded = base64_encode(credentials.as_bytes());
                        format!("Basic {}", encoded)
                    }
                    crate::transport::config::AuthConfig::Header { value, .. } => value.clone(),
                    crate::transport::config::AuthConfig::OAuth { .. } => {
                        // OAuth requires more complex handling - for now return the token
                        "Bearer oauth-token".to_string()
                    }
                });

                Ok(Box::new(HttpStreamTransport::new(
                    stream_config.base_url.to_string(),
                    auth_header,
                )))
            }

            #[cfg(not(feature = "http-stream"))]
            TransportConfig::HttpStream(_) => Err(crate::error::ConfigError::InvalidValue {
                parameter: "transport_type".to_string(),
                value: "http-stream".to_string(),
                reason:
                    "http-stream transport support not compiled in (enable 'http-stream' feature)"
                        .to_string(),
            }
            .into()),
        }
    }

    /// Get a list of supported transport types for this build.
    ///
    /// This is useful for runtime checks and feature discovery.
    ///
    /// # Returns
    ///
    /// A vector of transport type names that are supported in this build.
    pub fn supported_transports() -> Vec<&'static str> {
        vec![
            #[cfg(feature = "stdio")]
            "stdio",
            #[cfg(feature = "http-sse")]
            "http-sse",
            #[cfg(feature = "http-stream")]
            "http-stream",
        ]
    }

    /// Create a transport with retry logic.
    ///
    /// This method attempts to create a transport and will retry on transient failures.
    /// This is useful for scenarios where network conditions or external dependencies
    /// might cause temporary failures during transport creation.
    ///
    /// # Arguments
    ///
    /// * `config` - The transport configuration
    /// * `max_retries` - Maximum number of retry attempts
    /// * `retry_delay` - Delay between retry attempts
    ///
    /// # Returns
    ///
    /// A boxed transport instance, or an error if all retries are exhausted.
    pub async fn create_with_retry(
        config: TransportConfig,
        max_retries: u32,
        retry_delay: std::time::Duration,
    ) -> McpResult<Box<dyn Transport>> {
        let mut last_error = None;

        for attempt in 0..=max_retries {
            match Self::create(config.clone()).await {
                Ok(transport) => return Ok(transport),
                Err(err) => {
                    last_error = Some(err);

                    // Don't retry on the last attempt
                    if attempt < max_retries {
                        tracing::warn!(
                            "Transport creation attempt {} failed, retrying in {:?}: {}",
                            attempt + 1,
                            retry_delay,
                            last_error.as_ref().unwrap()
                        );
                        tokio::time::sleep(retry_delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }
}

/// Simple base64 encoding implementation without external dependencies.
///
/// This is a basic implementation for HTTP Basic Auth. For production systems
/// requiring advanced base64 features, consider using a dedicated crate.
fn base64_encode(input: &[u8]) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::new();
    let mut i = 0;

    while i < input.len() {
        let byte1 = input[i];
        let byte2 = if i + 1 < input.len() { input[i + 1] } else { 0 };
        let byte3 = if i + 2 < input.len() { input[i + 2] } else { 0 };

        let combined = ((byte1 as u32) << 16) | ((byte2 as u32) << 8) | (byte3 as u32);

        result.push(CHARSET[((combined >> 18) & 0x3F) as usize] as char);
        result.push(CHARSET[((combined >> 12) & 0x3F) as usize] as char);

        if i + 1 < input.len() {
            result.push(CHARSET[((combined >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }

        if i + 2 < input.len() {
            result.push(CHARSET[(combined & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }

        i += 3;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::TransportConfig;

    #[test]
    fn test_supported_transports() {
        let transports = TransportFactory::supported_transports();
        assert!(!transports.is_empty());

        // All default features should be enabled
        #[cfg(feature = "stdio")]
        assert!(transports.contains(&"stdio"));

        #[cfg(feature = "http-sse")]
        assert!(transports.contains(&"http-sse"));

        #[cfg(feature = "http-stream")]
        assert!(transports.contains(&"http-stream"));
    }

    #[tokio::test]
    async fn test_create_stdio_transport() {
        let config = TransportConfig::stdio("echo", &["hello".to_string()]);
        let result = TransportFactory::create(config).await;

        #[cfg(feature = "stdio")]
        {
            assert!(result.is_ok());
            let transport = result.unwrap();
            assert_eq!(transport.get_info().transport_type, "stdio");
        }

        #[cfg(not(feature = "stdio"))]
        {
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_create_http_sse_transport() {
        let config = TransportConfig::http_sse("https://example.com/mcp").unwrap();
        let result = TransportFactory::create(config).await;

        #[cfg(feature = "http-sse")]
        {
            assert!(result.is_ok());
            let transport = result.unwrap();
            assert_eq!(transport.get_info().transport_type, "streamable-http");
        }

        #[cfg(not(feature = "http-sse"))]
        {
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    async fn test_invalid_config() {
        let config = TransportConfig::stdio("", &[] as &[String]);
        let result = TransportFactory::create(config).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode(b"hello"), "aGVsbG8=");
        assert_eq!(base64_encode(b"hello world"), "aGVsbG8gd29ybGQ=");
        assert_eq!(base64_encode(b"user:pass"), "dXNlcjpwYXNz");
        assert_eq!(base64_encode(b""), "");
    }

    #[tokio::test]
    async fn test_transport_creation() {
        let config = TransportConfig::stdio("echo", &[] as &[String]);
        let transport = TransportFactory::create(config).await;
        assert!(transport.is_ok());
    }
}
