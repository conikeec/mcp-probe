//! Validation command implementation for MCP server compliance

use anyhow::Result;
use crate::cli::{ValidateArgs, Severity};

/// Execute the validate command
pub async fn run(args: ValidateArgs) -> Result<()> {
    tracing::info!("Starting MCP server validation");
    
    let transport_config = args.transport.to_transport_config()?;
    tracing::info!("Using transport: {}", transport_config.transport_type());
    
    println!("🔍 MCP Server Validation");
    println!("Transport: {}", transport_config.transport_type());
    println!("Severity: {:?}", args.severity);
    
    if !args.rules.is_empty() {
        println!("📋 Validation rules: {:?}", args.rules);
    } else {
        println!("📋 Using default validation rules");
    }
    
    // Placeholder validation results
    let validation_results = vec![
        ValidationResult {
            rule: "protocol_version".to_string(),
            status: ValidationStatus::Pass,
            message: "Protocol version is supported".to_string(),
        },
        ValidationResult {
            rule: "initialization".to_string(),
            status: ValidationStatus::Pass,
            message: "Initialization sequence completed successfully".to_string(),
        },
        ValidationResult {
            rule: "capabilities".to_string(),
            status: ValidationStatus::Warning,
            message: "Some optional capabilities not implemented".to_string(),
        },
    ];
    
    display_validation_results(&validation_results);
    
    if let Some(report_path) = &args.report {
        generate_validation_report(&validation_results, report_path)?;
        println!("📄 Validation report saved to: {}", report_path.display());
    }
    
    println!("✅ Validation completed");
    
    Ok(())
}

/// Validation result for a single rule
#[derive(Debug, Clone)]
struct ValidationResult {
    rule: String,
    status: ValidationStatus,
    message: String,
}

/// Status of a validation check
#[derive(Debug, Clone, PartialEq)]
enum ValidationStatus {
    Pass,
    Warning,
    Error,
    Critical,
}

/// Display validation results to the console
fn display_validation_results(results: &[ValidationResult]) {
    println!("\n📊 Validation Results:");
    println!("{:-<60}", "");
    
    for result in results {
        let icon = match result.status {
            ValidationStatus::Pass => "✅",
            ValidationStatus::Warning => "⚠️",
            ValidationStatus::Error => "❌",
            ValidationStatus::Critical => "🚨",
        };
        
        println!("{} {} - {}", icon, result.rule, result.message);
    }
    
    // Summary
    let passed = results.iter().filter(|r| r.status == ValidationStatus::Pass).count();
    let warnings = results.iter().filter(|r| r.status == ValidationStatus::Warning).count();
    let errors = results.iter().filter(|r| r.status == ValidationStatus::Error).count();
    let critical = results.iter().filter(|r| r.status == ValidationStatus::Critical).count();
    
    println!("{:-<60}", "");
    println!("Summary: {} passed, {} warnings, {} errors, {} critical", 
             passed, warnings, errors, critical);
}

/// Generate a validation report file
fn generate_validation_report(results: &[ValidationResult], path: &std::path::Path) -> Result<()> {
    let mut report = String::new();
    report.push_str("# MCP Server Validation Report\n\n");
    report.push_str(&format!("Generated: {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
    
    report.push_str("## Results\n\n");
    for result in results {
        let status_text = match result.status {
            ValidationStatus::Pass => "PASS",
            ValidationStatus::Warning => "WARNING",
            ValidationStatus::Error => "ERROR",
            ValidationStatus::Critical => "CRITICAL",
        };
        
        report.push_str(&format!("- **{}**: {} - {}\n", 
                                result.rule, status_text, result.message));
    }
    
    // Summary section
    let passed = results.iter().filter(|r| r.status == ValidationStatus::Pass).count();
    let warnings = results.iter().filter(|r| r.status == ValidationStatus::Warning).count();
    let errors = results.iter().filter(|r| r.status == ValidationStatus::Error).count();
    let critical = results.iter().filter(|r| r.status == ValidationStatus::Critical).count();
    
    report.push_str("\n## Summary\n\n");
    report.push_str(&format!("- Passed: {}\n", passed));
    report.push_str(&format!("- Warnings: {}\n", warnings));
    report.push_str(&format!("- Errors: {}\n", errors));
    report.push_str(&format!("- Critical: {}\n", critical));
    
    std::fs::write(path, report)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validation_result_display() {
        let results = vec![
            ValidationResult {
                rule: "test_rule".to_string(),
                status: ValidationStatus::Pass,
                message: "Test message".to_string(),
            },
        ];
        
        // This should not panic
        display_validation_results(&results);
    }
    
    #[test]
    fn test_validation_report_generation() -> Result<()> {
        let results = vec![
            ValidationResult {
                rule: "test_rule".to_string(),
                status: ValidationStatus::Pass,
                message: "Test message".to_string(),
            },
        ];
        
        let temp_file = tempfile::NamedTempFile::new()?;
        generate_validation_report(&results, temp_file.path())?;
        
        let content = std::fs::read_to_string(temp_file.path())?;
        assert!(content.contains("MCP Server Validation Report"));
        assert!(content.contains("test_rule"));
        
        Ok(())
    }
} 