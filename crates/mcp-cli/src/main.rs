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
mod search;
mod tui;

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments first to check if we're in TUI mode
    let cli = Cli::parse();

    // Initialize logging based on command type
    let tui_mode = matches!(cli.command, Commands::Debug(ref cmd) if !cmd.non_interactive);
    init_logging(tui_mode)?;

    // Log the startup
    tracing::info!("MCP Probe starting up, TUI mode: {}", tui_mode);
    tracing::debug!("Command: {:?}", cli.command);

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
fn init_logging(tui_mode: bool) -> Result<()> {
    if tui_mode {
        // In TUI mode, use immediate file logging (synchronous to ensure writes)
        let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "mcp_probe=debug,mcp_core=debug,info".into());

        // Create a simple file writer that writes immediately
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open("mcp-probe-debug.log")?;

        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(log_file)
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .with_ansi(false); // No ANSI codes in log file

        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .init();

        // Write an initial log to confirm logging is working
        tracing::info!("=== MCP Probe Debug Log Started ===");
    } else {
        // Normal mode with full logging
        let env_filter =
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
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
    }

    Ok(())
}
