//! Paths command implementation for directory management and cleanup

use crate::cli::{PathsAction, PathsArgs};
use crate::paths::get_mcp_probe_paths;
use anyhow::Result;
use std::process::Command;

/// Execute the paths command
pub async fn run(args: PathsArgs) -> Result<()> {
    match args.action {
        PathsAction::Show => show_directory_structure().await,
        PathsAction::Cleanup { days, force } => cleanup_old_files(days, force).await,
        PathsAction::Open => open_directory().await,
    }
}

/// Show the directory structure and usage information
async fn show_directory_structure() -> Result<()> {
    let paths = get_mcp_probe_paths()?;

    println!("📁 MCP Probe Directory Structure");
    println!("═════════════════════════════════");

    // Print directory structure
    paths.print_structure();

    // Show directory sizes and file counts
    println!("\n📊 Directory Usage:");
    show_directory_stats(&paths.logs_dir, "Logs")?;
    show_directory_stats(&paths.reports_dir, "Reports")?;
    show_directory_stats(&paths.sessions_dir, "Sessions")?;
    show_directory_stats(&paths.config_dir, "Config")?;

    // Show recent files
    println!("\n📋 Recent Files:");
    show_recent_files(&paths.logs_dir, "logs", 5)?;
    show_recent_files(&paths.reports_dir, "reports", 5)?;
    show_recent_files(&paths.sessions_dir, "sessions", 5)?;

    // Show cleanup recommendations
    println!("\n🧹 Cleanup Recommendations:");
    let old_files = count_old_files(&paths, 30)?;
    if old_files > 0 {
        println!("   • {} files older than 30 days found", old_files);
        println!("   • Run 'mcp-probe paths cleanup --days 30 --force' to clean up");
    } else {
        println!("   • No cleanup needed - all files are recent");
    }

    println!("\n💡 Useful Commands:");
    println!("   • mcp-probe paths cleanup --days 7 --force  # Clean files older than 7 days");
    println!("   • mcp-probe paths open                      # Open directory in file manager");
    println!(
        "   • ls -la {}                                # List all files",
        paths.home_dir.display()
    );

    Ok(())
}

/// Show statistics for a directory
fn show_directory_stats(dir: &std::path::Path, name: &str) -> Result<()> {
    if !dir.exists() {
        println!("   📁 {}: Not created yet", name);
        return Ok(());
    }

    let mut file_count = 0;
    let mut total_size = 0u64;

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.metadata()?.is_file() {
            file_count += 1;
            total_size += entry.metadata()?.len();
        }
    }

    let size_str = format_file_size(total_size);
    println!("   📁 {}: {} files, {} total", name, file_count, size_str);

    Ok(())
}

/// Show recent files in a directory
fn show_recent_files(dir: &std::path::Path, category: &str, count: usize) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    let mut files: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.metadata().map(|m| m.is_file()).unwrap_or(false))
        .collect();

    // Sort by modification time (newest first)
    files.sort_by_key(|entry| {
        entry
            .metadata()
            .and_then(|m| m.modified())
            .map(std::cmp::Reverse)
            .unwrap_or(std::cmp::Reverse(std::time::SystemTime::UNIX_EPOCH))
    });

    if files.is_empty() {
        return Ok(());
    }

    println!("   📋 Recent {} (showing up to {}):", category, count);
    for file in files.iter().take(count) {
        if let (Ok(metadata), Some(name)) = (file.metadata(), file.file_name().to_str()) {
            let size = format_file_size(metadata.len());
            let modified = metadata
                .modified()
                .map(|t| {
                    let duration = std::time::SystemTime::now()
                        .duration_since(t)
                        .unwrap_or_default();
                    let days = duration.as_secs() / 86400;
                    if days == 0 {
                        "today".to_string()
                    } else if days == 1 {
                        "1 day ago".to_string()
                    } else {
                        format!("{} days ago", days)
                    }
                })
                .unwrap_or_else(|_| "unknown".to_string());

            println!("      • {} ({}, {})", name, size, modified);
        }
    }

    Ok(())
}

/// Clean up old files
async fn cleanup_old_files(days: u64, force: bool) -> Result<()> {
    let paths = get_mcp_probe_paths()?;

    println!("🧹 Cleaning up files older than {} days", days);

    if !force {
        println!("🔍 Dry run mode - no files will be deleted");
        println!("   Add --force to actually delete files");
    }

    let old_files = scan_old_files(&paths, days)?;

    if old_files.is_empty() {
        println!("✅ No files found older than {} days", days);
        return Ok(());
    }

    println!(
        "\n📋 Files to be {}:",
        if force {
            "deleted"
        } else {
            "deleted (dry run)"
        }
    );
    let mut total_size = 0u64;

    for (path, size, age_days) in &old_files {
        total_size += *size;
        let size_str = format_file_size(*size);
        println!(
            "   • {} ({}, {} days old)",
            path.display(),
            size_str,
            age_days
        );
    }

    println!("\n📊 Summary:");
    println!("   • {} files found", old_files.len());
    println!(
        "   • {} total space to be freed",
        format_file_size(total_size)
    );

    if force {
        println!("\n🗑️  Deleting files...");
        let mut deleted_count = 0;
        let mut error_count = 0;

        for (path, _, _) in old_files {
            match std::fs::remove_file(&path) {
                Ok(()) => {
                    deleted_count += 1;
                    println!("   ✅ Deleted: {}", path.display());
                }
                Err(e) => {
                    error_count += 1;
                    println!("   ❌ Failed to delete {}: {}", path.display(), e);
                }
            }
        }

        println!("\n📊 Cleanup Results:");
        println!("   • {} files deleted successfully", deleted_count);
        if error_count > 0 {
            println!("   • {} files failed to delete", error_count);
        }
        println!("   • {} space freed", format_file_size(total_size));
    } else {
        println!("\n💡 To actually delete these files, run:");
        println!("   mcp-probe paths cleanup --days {} --force", days);
    }

    Ok(())
}

/// Open the MCP Probe directory in the system file manager
async fn open_directory() -> Result<()> {
    let paths = get_mcp_probe_paths()?;

    println!(
        "📁 Opening MCP Probe directory: {}",
        paths.home_dir.display()
    );

    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(&paths.home_dir).spawn()?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer").arg(&paths.home_dir).spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        // Try various Linux file managers
        let file_managers = ["xdg-open", "nautilus", "dolphin", "thunar", "nemo"];
        let mut opened = false;

        for fm in &file_managers {
            if Command::new(fm).arg(&paths.home_dir).spawn().is_ok() {
                opened = true;
                break;
            }
        }

        if !opened {
            println!("⚠️  Could not find a suitable file manager to open the directory");
            println!(
                "   You can manually navigate to: {}",
                paths.home_dir.display()
            );
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        println!("⚠️  Opening directories is not supported on this platform");
        println!(
            "   You can manually navigate to: {}",
            paths.home_dir.display()
        );
    }

    Ok(())
}

/// Count old files for cleanup recommendations
fn count_old_files(paths: &crate::paths::McpProbePaths, days: u64) -> Result<usize> {
    let old_files = scan_old_files(paths, days)?;
    Ok(old_files.len())
}

/// Scan for old files in all directories
fn scan_old_files(
    paths: &crate::paths::McpProbePaths,
    days: u64,
) -> Result<Vec<(std::path::PathBuf, u64, u64)>> {
    let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(days * 86400);
    let mut old_files = Vec::new();

    for dir in [&paths.logs_dir, &paths.reports_dir, &paths.sessions_dir] {
        if !dir.exists() {
            continue;
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let metadata = entry.metadata()?;

            if metadata.is_file() {
                if let Ok(modified) = metadata.modified() {
                    if modified < cutoff {
                        let age_days = std::time::SystemTime::now()
                            .duration_since(modified)
                            .map(|d| d.as_secs() / 86400)
                            .unwrap_or(0);

                        old_files.push((entry.path(), metadata.len(), age_days));
                    }
                }
            }
        }
    }

    old_files.sort_by_key(|(_, _, age)| std::cmp::Reverse(*age));
    Ok(old_files)
}

/// Format file size in human-readable format
fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(0), "0 B");
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_file_size(1024 * 1024 * 1024), "1.0 GB");
    }
}
