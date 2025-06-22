//! Configuration management command implementation

use crate::cli::{ConfigAction, ConfigArgs, ConfigTemplate};
use anyhow::Result;

/// Execute the config command
pub async fn run(args: ConfigArgs) -> Result<()> {
    match args.action {
        ConfigAction::Init { output, template } => init_config(output, template).await,
        ConfigAction::Validate { config } => validate_config(config).await,
        ConfigAction::Show { config } => show_config(config).await,
    }
}

/// Initialize a new configuration file
async fn init_config(output: Option<std::path::PathBuf>, template: ConfigTemplate) -> Result<()> {
    use crate::paths::get_mcp_probe_paths;

    let config_path = if let Some(path) = output {
        path
    } else {
        // Use centralized path management for default config
        let paths = get_mcp_probe_paths()?;
        paths.default_config_file()
    };

    println!(
        "🔧 Initializing configuration file: {}",
        config_path.display()
    );
    println!("📝 Using template: {:?}", template);

    let config_content = match template {
        ConfigTemplate::Minimal => generate_minimal_config(),
        ConfigTemplate::Full => generate_full_config(),
        ConfigTemplate::Dev => generate_dev_config(),
        ConfigTemplate::Prod => generate_prod_config(),
    };

    std::fs::write(&config_path, config_content)?;
    println!("✅ Configuration file created successfully");

    Ok(())
}

/// Validate an existing configuration file
async fn validate_config(config: std::path::PathBuf) -> Result<()> {
    println!("🔍 Validating configuration: {}", config.display());

    if !config.exists() {
        anyhow::bail!("Configuration file not found: {}", config.display());
    }

    let content = std::fs::read_to_string(&config)?;
    let _parsed: toml::Value = toml::from_str(&content)?;

    println!("✅ Configuration is valid");
    Ok(())
}

/// Show configuration content
async fn show_config(config: Option<std::path::PathBuf>) -> Result<()> {
    let config_path = config.unwrap_or_else(|| "mcp-probe.toml".into());

    println!("📄 Configuration: {}", config_path.display());

    if !config_path.exists() {
        println!("❌ Configuration file not found");
        return Ok(());
    }

    let content = std::fs::read_to_string(&config_path)?;
    println!("\n{}", content);

    Ok(())
}

/// Generate minimal configuration template
fn generate_minimal_config() -> String {
    r#"# MCP Probe Configuration (Minimal)

[transport]
type = "stdio"
command = "python"
args = ["server.py"]

[client]
name = "mcp-probe"
version = "0.1.0"

[timeouts]
connection = "30s"
request = "10s"
"#
    .to_string()
}

/// Generate full configuration template
fn generate_full_config() -> String {
    r#"# MCP Probe Configuration (Full)

[transport]
type = "stdio"
command = "python"
args = ["server.py"]
working_dir = "/path/to/server"
timeout = "30s"

[transport.environment]
PYTHONPATH = "/usr/local/lib/python3.9/site-packages"

[client]
name = "mcp-probe"
version = "0.1.0"

[capabilities]
tools = true
resources = true
prompts = true
logging = true

[timeouts]
connection = "30s"
request = "10s"
initialization = "60s"

[debug]
show_raw_messages = false
save_session = true
session_dir = "./sessions"

[logging]
level = "info"
format = "pretty"
file = "mcp-probe.log"
"#
    .to_string()
}

/// Generate development configuration template
fn generate_dev_config() -> String {
    r#"# MCP Probe Configuration (Development)

[transport]
type = "stdio"
command = "python"
args = ["-m", "pip", "install", "-e", ".", "&&", "python", "server.py"]
working_dir = "./dev-server"
timeout = "60s"

[client]
name = "mcp-probe-dev"
version = "0.1.0-dev"

[debug]
show_raw_messages = true
save_session = true
session_dir = "./dev-sessions"

[logging]
level = "debug"
format = "pretty"
"#
    .to_string()
}

/// Generate production configuration template
fn generate_prod_config() -> String {
    r#"# MCP Probe Configuration (Production)

[transport]
type = "http_sse"
base_url = "https://api.example.com/mcp"
timeout = "120s"

[transport.headers]
"User-Agent" = "mcp-probe/0.1.0"
"Accept" = "application/json"

[transport.auth]
type = "bearer"
token = "${MCP_AUTH_TOKEN}"

[client]
name = "mcp-probe"
version = "0.1.0"

[timeouts]
connection = "60s"
request = "30s"
initialization = "120s"

[logging]
level = "info"
format = "json"
file = "/var/log/mcp-probe.log"
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_templates_valid_toml() {
        // Test that all templates generate valid TOML
        let templates = [
            generate_minimal_config(),
            generate_full_config(),
            generate_dev_config(),
            generate_prod_config(),
        ];

        for template in &templates {
            toml::from_str::<toml::Value>(template).expect("Template should generate valid TOML");
        }
    }
}
