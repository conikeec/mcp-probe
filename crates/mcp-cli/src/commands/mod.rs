//! Command implementations for MCP Probe CLI
//!
//! This module contains the implementation of all CLI commands including
//! debug, test, config, validate, and export operations.

use anyhow::Result;

pub mod config;
pub mod debug;
pub mod export;
pub mod paths;
pub mod test;
pub mod validate;

// Validation engine module
pub mod validation;

/// Common result type for all command operations
#[allow(dead_code)]
pub type CommandResult<T = ()> = Result<T>;

/// Command execution context shared across all commands
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CommandContext {
    /// Whether to use colored output
    pub use_color: bool,
    /// Verbosity level (0-3)
    pub verbosity: u8,
    /// Output format preference
    pub output_format: crate::cli::OutputFormat,
}

impl CommandContext {
    /// Create a new command context from CLI arguments
    #[allow(dead_code)]
    pub fn new(cli: &crate::cli::Cli) -> Self {
        Self {
            use_color: !cli.no_color,
            verbosity: cli.verbose,
            output_format: cli.output.clone(),
        }
    }

    /// Check if we should show verbose output
    #[allow(dead_code)]
    pub fn is_verbose(&self) -> bool {
        self.verbosity > 0
    }

    /// Check if we should show debug output
    #[allow(dead_code)]
    pub fn is_debug(&self) -> bool {
        self.verbosity > 1
    }

    /// Check if we should show trace output
    #[allow(dead_code)]
    pub fn is_trace(&self) -> bool {
        self.verbosity > 2
    }
}
