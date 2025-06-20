//! MCP Probe - Interactive Model Context Protocol debugger and client
//!
//! This CLI tool provides an interactive debugging interface for MCP servers,
//! allowing developers to test and validate their MCP implementations before
//! deploying to production LLM hosts.

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod cli;
mod commands;
mod config;
mod flows;
mod tui;
mod utils;

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging based on environment
    init_logging()?;

    // Parse command line arguments
    let cli = Cli::parse();

    // Execute the appropriate command
    match cli.command {
        Commands::Debug(debug_cmd) => debug_cmd.execute().await,
        Commands::Test(args) => commands::test::run(args).await,
        Commands::Config(args) => commands::config::run(args).await,
        Commands::Validate(args) => commands::validate::run(args).await,
        Commands::Export(args) => commands::export::run(args).await,
    }
}

/// Initialize structured logging based on environment variables and CLI options
fn init_logging() -> Result<()> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            // Default to info level with specific module filtering
            "mcp_probe=debug,mcp_core=debug,info".into()
        });

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();

    tracing::debug!("Logging initialized");
    Ok(())
} 