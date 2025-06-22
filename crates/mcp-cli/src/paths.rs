//! Path management for MCP Probe file system organization
//!
//! This module provides centralized path management for all MCP Probe file operations,
//! ensuring a clean and organized folder structure in the user's home directory.

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Central path manager for MCP Probe file system organization
#[derive(Debug, Clone)]
pub struct McpProbePaths {
    /// MCP Probe home directory (e.g., ~/.mcp-probe)
    pub home_dir: PathBuf,
    /// Logs directory for all log files
    pub logs_dir: PathBuf,
    /// Reports directory for all generated reports
    pub reports_dir: PathBuf,
    /// Sessions directory for session files
    pub sessions_dir: PathBuf,
    /// Config directory for configuration files
    pub config_dir: PathBuf,
}

impl McpProbePaths {
    /// Create a new path manager and ensure all directories exist
    pub fn new() -> Result<Self> {
        let home_dir = Self::get_mcp_probe_home()?;

        let paths = Self {
            logs_dir: home_dir.join("logs"),
            reports_dir: home_dir.join("reports"),
            sessions_dir: home_dir.join("sessions"),
            config_dir: home_dir.join("config"),
            home_dir,
        };

        // Ensure all directories exist
        paths.ensure_directories_exist()?;

        Ok(paths)
    }

    /// Get the MCP Probe home directory
    fn get_mcp_probe_home() -> Result<PathBuf> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());

        Ok(Path::new(&home).join(".mcp-probe"))
    }

    /// Ensure all required directories exist
    fn ensure_directories_exist(&self) -> Result<()> {
        for dir in [
            &self.home_dir,
            &self.logs_dir,
            &self.reports_dir,
            &self.sessions_dir,
            &self.config_dir,
        ] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }

    /// Get a log file path with timestamp
    pub fn log_file(&self, name: &str) -> PathBuf {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        self.logs_dir.join(format!("{}-{}.log", name, timestamp))
    }

    /// Get a debug log file path (no timestamp for TUI mode)
    pub fn debug_log_file(&self) -> PathBuf {
        self.logs_dir.join("mcp-probe-debug.log")
    }

    /// Get a report file path with date prefix
    pub fn report_file(&self, name: &str, extension: &str) -> PathBuf {
        let date = chrono::Utc::now().format("%Y%m%d");
        let timestamp = chrono::Utc::now().format("%H%M%S");
        self.reports_dir
            .join(format!("{}-{}-{}.{}", date, name, timestamp, extension))
    }

    /// Get a dated report file path (just date, no time)
    #[allow(dead_code)]
    pub fn dated_report_file(&self, name: &str, extension: &str) -> PathBuf {
        let date = chrono::Utc::now().format("%Y%m%d");
        self.reports_dir
            .join(format!("{}-{}.{}", date, name, extension))
    }

    /// Get a session file path
    #[allow(dead_code)]
    pub fn session_file(&self, name: &str) -> PathBuf {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        self.sessions_dir
            .join(format!("{}-{}.json", name, timestamp))
    }

    /// Get a config file path
    pub fn config_file(&self, name: &str) -> PathBuf {
        self.config_dir.join(format!("{}.toml", name))
    }

    /// Get the default config file path
    pub fn default_config_file(&self) -> PathBuf {
        self.config_file("mcp-probe")
    }

    /// Create a custom output directory under reports
    #[allow(dead_code)]
    pub fn custom_output_dir(&self, name: &str) -> Result<PathBuf> {
        let dir = self.reports_dir.join(name);
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// Get a temporary file path in the home directory
    #[allow(dead_code)]
    pub fn temp_file(&self, name: &str, extension: &str) -> PathBuf {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        self.home_dir
            .join(format!("{}-{}.{}", name, timestamp, extension))
    }

    /// Clean up old files based on age
    #[allow(dead_code)]
    pub fn cleanup_old_files(&self, days_to_keep: u64) -> Result<()> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days_to_keep as i64);

        for dir in [&self.logs_dir, &self.reports_dir, &self.sessions_dir] {
            self.cleanup_directory(dir, cutoff)?;
        }

        Ok(())
    }

    /// Clean up files in a specific directory older than cutoff date
    #[allow(dead_code)]
    fn cleanup_directory(&self, dir: &Path, cutoff: chrono::DateTime<chrono::Utc>) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let metadata = entry.metadata()?;

            if metadata.is_file() {
                if let Ok(modified) = metadata.modified() {
                    let modified_utc: chrono::DateTime<chrono::Utc> = modified.into();
                    if modified_utc < cutoff {
                        if let Err(e) = std::fs::remove_file(entry.path()) {
                            tracing::warn!("Failed to remove old file {:?}: {}", entry.path(), e);
                        } else {
                            tracing::debug!("Cleaned up old file: {:?}", entry.path());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get a relative path from the home directory
    #[allow(dead_code)]
    pub fn relative_to_home(&self, path: &Path) -> Option<PathBuf> {
        path.strip_prefix(&self.home_dir)
            .ok()
            .map(|p| p.to_path_buf())
    }

    /// Print the directory structure for debugging
    pub fn print_structure(&self) {
        println!("📁 MCP Probe Directory Structure:");
        println!("   🏠 Home: {}", self.home_dir.display());
        println!("   📄 Logs: {}", self.logs_dir.display());
        println!("   📊 Reports: {}", self.reports_dir.display());
        println!("   💾 Sessions: {}", self.sessions_dir.display());
        println!("   ⚙️  Config: {}", self.config_dir.display());
    }
}

impl Default for McpProbePaths {
    fn default() -> Self {
        Self::new().expect("Failed to create MCP Probe paths")
    }
}

/// Get the global MCP Probe paths instance
pub fn get_mcp_probe_paths() -> Result<McpProbePaths> {
    McpProbePaths::new()
}

/// Helper function to get a report file path with date prefix
#[allow(dead_code)]
pub fn get_report_path(name: &str, extension: &str) -> Result<PathBuf> {
    let paths = get_mcp_probe_paths()?;
    Ok(paths.report_file(name, extension))
}

/// Helper function to get a log file path with timestamp
#[allow(dead_code)]
pub fn get_log_path(name: &str) -> Result<PathBuf> {
    let paths = get_mcp_probe_paths()?;
    Ok(paths.log_file(name))
}

/// Helper function to get a session file path with timestamp
#[allow(dead_code)]
pub fn get_session_path(name: &str) -> Result<PathBuf> {
    let paths = get_mcp_probe_paths()?;
    Ok(paths.session_file(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_generation() -> Result<()> {
        let paths = McpProbePaths::new()?;

        // Test log file paths
        let log_path = paths.log_file("test");
        assert!(log_path.to_string_lossy().contains("test-"));
        assert!(log_path.extension().unwrap() == "log");

        // Test report file paths
        let report_path = paths.report_file("test-report", "json");
        assert!(report_path.to_string_lossy().contains("test-report"));
        assert!(report_path.extension().unwrap() == "json");

        // Test dated report file paths
        let dated_report = paths.dated_report_file("daily-report", "json");
        assert!(dated_report.to_string_lossy().contains("daily-report"));

        // Test session file paths
        let session_path = paths.session_file("debug-session");
        assert!(session_path.to_string_lossy().contains("debug-session"));
        assert!(session_path.extension().unwrap() == "json");

        Ok(())
    }

    #[test]
    fn test_directory_creation() -> Result<()> {
        let paths = McpProbePaths::new()?;

        // Check that all directories exist
        assert!(paths.home_dir.exists());
        assert!(paths.logs_dir.exists());
        assert!(paths.reports_dir.exists());
        assert!(paths.sessions_dir.exists());
        assert!(paths.config_dir.exists());

        Ok(())
    }

    #[test]
    fn test_helper_functions() -> Result<()> {
        let report_path = get_report_path("test", "json")?;
        assert!(report_path.extension().unwrap() == "json");

        let log_path = get_log_path("test")?;
        assert!(log_path.extension().unwrap() == "log");

        let session_path = get_session_path("test")?;
        assert!(session_path.extension().unwrap() == "json");

        Ok(())
    }
}
