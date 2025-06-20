//! Validation command implementation for MCP server compliance

use anyhow::Result;
use crate::cli::{ValidateArgs, Severity};
use super::validation::{ValidationEngine, ValidationConfig, ValidationStatus};
use std::time::Duration;

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
    
    // Configure validation engine based on command arguments
    let mut config = ValidationConfig {
        custom_rules: args.rules.clone(),
        ..Default::default()
    };
    
    // Adjust configuration based on severity level
    match args.severity {
        Severity::Info => {
            config.strict_schema_validation = false;
            config.test_error_conditions = false;
        }
        Severity::Warning => {
            config.strict_schema_validation = true;
            config.test_error_conditions = false;
        }
        Severity::Error => {
            config.strict_schema_validation = true;
            config.test_error_conditions = true;
        }
        Severity::Critical => {
            config.strict_schema_validation = true;
            config.test_error_conditions = true;
            config.test_timeout = Duration::from_secs(60);
            config.total_timeout = Duration::from_secs(600);
        }
    }
    
    // Create and run validation engine
    let mut validator = ValidationEngine::new(transport_config)
        .with_config(config);
    
    println!("🚀 Starting validation engine...");
    
    match validator.validate().await {
        Ok(report) => {
            // Display results
            display_validation_results(&report.results, &args.severity);
            
            // Generate report if requested
            if let Some(report_path) = &args.report {
                generate_validation_report(&report, report_path)?;
                println!("📄 Validation report saved to: {}", report_path.display());
            }
            
            // Print summary
            let summary = &report.summary;
            println!("\n📊 Validation Summary:");
            println!("Total tests: {}", summary.total_tests);
            println!("Passed: {} ({}%)", summary.passed, summary.compliance_percentage.round());
            println!("Warnings: {}", summary.warnings);
            println!("Errors: {}", summary.errors);
            println!("Critical: {}", summary.critical);
            
            if summary.compliance_percentage >= 90.0 {
                println!("✅ Server validation completed successfully!");
            } else if summary.compliance_percentage >= 70.0 {
                println!("⚠️ Server validation completed with warnings");
            } else {
                println!("❌ Server validation failed - multiple issues found");
            }
            
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Validation failed: {}", e);
            Err(e)
        }
    }
}

/// Display validation results to the console
fn display_validation_results(results: &[super::validation::ValidationResult], severity_filter: &Severity) {
    println!("\n📊 Validation Results:");
    println!("{:-<80}", "");
    
    // Filter results based on severity
    let filtered_results: Vec<_> = results.iter()
        .filter(|result| should_display_result(result, severity_filter))
        .collect();
    
    // Group by category
    let mut categories: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();
    for result in &filtered_results {
        let category = format!("{:?}", result.category);
        categories.entry(category).or_default().push(result);
    }
    
    // Display by category
    for (category, category_results) in categories {
        println!("\n🏷️  {}", category);
        println!("{:-<40}", "");
        
        for result in category_results {
            let duration_str = if result.duration.as_millis() > 0 {
                format!(" ({}ms)", result.duration.as_millis())
            } else {
                String::new()
            };
            
            println!("{} {} - {}{}", 
                    result.status.icon(), 
                    result.test_name, 
                    result.message,
                    duration_str);
            
            // Show details for failures
            if matches!(result.status, ValidationStatus::Error | ValidationStatus::Critical) && result.details.is_some() {
                if let Ok(details_str) = serde_json::to_string_pretty(result.details.as_ref().unwrap()) {
                    let truncated = if details_str.len() > 200 {
                        format!("{}...", &details_str[..200])
                    } else {
                        details_str
                    };
                    println!("   Details: {}", truncated);
                }
            }
        }
    }
    
    println!("{:-<80}", "");
}

/// Determine if a result should be displayed based on severity filter
fn should_display_result(result: &super::validation::ValidationResult, severity_filter: &Severity) -> bool {
    match severity_filter {
        Severity::Info => true, // Show all results
        Severity::Warning => !matches!(result.status, ValidationStatus::Pass),
        Severity::Error => matches!(result.status, ValidationStatus::Error | ValidationStatus::Critical),
        Severity::Critical => matches!(result.status, ValidationStatus::Critical),
    }
}

/// Generate a validation report file
fn generate_validation_report(report: &super::validation::ValidationReport, path: &std::path::Path) -> Result<()> {
    // Generate different formats based on file extension
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("json");
    
    match extension {
        "json" => {
            let json_content = serde_json::to_string_pretty(report)?;
            std::fs::write(path, json_content)?;
        }
        "yaml" | "yml" => {
            let yaml_content = serde_yaml::to_string(report)?;
            std::fs::write(path, yaml_content)?;
        }
        "md" | "markdown" => {
            let markdown_content = generate_markdown_report(report)?;
            std::fs::write(path, markdown_content)?;
        }
        _ => {
            // Default to JSON
            let json_content = serde_json::to_string_pretty(report)?;
            std::fs::write(path, json_content)?;
        }
    }
    
    Ok(())
}

/// Generate markdown report
fn generate_markdown_report(report: &super::validation::ValidationReport) -> Result<String> {
    let mut content = String::new();
    
    // Header
    content.push_str("# MCP Server Validation Report\n\n");
    content.push_str(&format!("**Generated:** {}\n", report.metadata.generated_at.format("%Y-%m-%d %H:%M:%S UTC")));
    content.push_str(&format!("**Validator Version:** {}\n", report.metadata.validator_version));
    content.push_str(&format!("**Transport:** {}\n", report.metadata.transport_type));
    content.push_str(&format!("**Duration:** {:.2}s\n\n", report.metadata.total_duration.as_secs_f64()));
    
    // Summary
    content.push_str("## Summary\n\n");
    content.push_str(&format!("- **Total Tests:** {}\n", report.summary.total_tests));
    content.push_str(&format!("- **Passed:** {} ({:.1}%)\n", report.summary.passed, report.summary.compliance_percentage));
    content.push_str(&format!("- **Warnings:** {}\n", report.summary.warnings));
    content.push_str(&format!("- **Errors:** {}\n", report.summary.errors));
    content.push_str(&format!("- **Critical:** {}\n", report.summary.critical));
    content.push_str(&format!("- **Skipped:** {}\n\n", report.summary.skipped));
    
    // Results by category
    content.push_str("## Detailed Results\n\n");
    
    let mut categories: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();
    for result in &report.results {
        let category = format!("{:?}", result.category);
        categories.entry(category).or_default().push(result);
    }
    
    for (category, results) in categories {
        content.push_str(&format!("### {}\n\n", category));
        
        for result in results {
            let status_emoji = result.status.icon();
            content.push_str(&format!("- {} **{}**: {}\n", status_emoji, result.test_name, result.message));
            
            if result.duration.as_millis() > 0 {
                content.push_str(&format!("  - Duration: {}ms\n", result.duration.as_millis()));
            }
        }
        content.push('\n');
    }
    
    // Performance metrics
    content.push_str("## Performance\n\n");
    content.push_str(&format!("- **Initialization Time:** {}ms\n", report.performance.initialization_time.as_millis()));
    content.push_str(&format!("- **Average Request Time:** {}ms\n", report.performance.average_request_time.as_millis()));
    content.push_str(&format!("- **Total Requests:** {}\n", report.performance.total_requests));
    content.push_str(&format!("- **Failed Requests:** {}\n", report.performance.failed_requests));
    content.push_str(&format!("- **Timeouts:** {}\n", report.performance.timeouts));
    
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::validation::*;
    // Test utilities
    
    #[test]
    fn test_severity_filtering() {
        let result_pass = ValidationResult {
            test_id: "test".to_string(),
            test_name: "Test".to_string(),
            category: ValidationCategory::Protocol,
            status: ValidationStatus::Pass,
            message: "Test message".to_string(),
            details: None,
            duration: std::time::Duration::from_millis(1),
            timestamp: chrono::Utc::now(),
        };
        
        let result_error = ValidationResult {
            test_id: "test".to_string(),
            test_name: "Test".to_string(), 
            category: ValidationCategory::Protocol,
            status: ValidationStatus::Error,
            message: "Test message".to_string(),
            details: None,
            duration: std::time::Duration::from_millis(1),
            timestamp: chrono::Utc::now(),
        };
        
        // Info shows all
        assert!(should_display_result(&result_pass, &Severity::Info));
        assert!(should_display_result(&result_error, &Severity::Info));
        
        // Error only shows errors
        assert!(!should_display_result(&result_pass, &Severity::Error));
        assert!(should_display_result(&result_error, &Severity::Error));
    }
    
    #[tokio::test]
    async fn test_markdown_report_generation() -> Result<()> {
        // ValidationEngine is defined in this file
        
        let report = ValidationReport {
            metadata: ReportMetadata {
                generated_at: chrono::Utc::now(),
                validator_version: "1.0.0".to_string(),
                transport_type: "stdio".to_string(),
                total_duration: Duration::from_secs(60),
                config: ValidationConfig::default(),
            },
            summary: ValidationSummary {
                total_tests: 5,
                passed: 4,
                info: 0,
                warnings: 1,
                errors: 0,
                critical: 0,
                skipped: 0,
                compliance_percentage: 80.0,
            },
            results: vec![],
            server_info: None,
            performance: PerformanceMetrics {
                initialization_time: Duration::from_millis(100),
                average_request_time: Duration::from_millis(50),
                total_requests: 5,
                failed_requests: 0,
                timeouts: 0,
            },
        };
        
        let markdown = generate_markdown_report(&report)?;
        assert!(markdown.contains("# MCP Server Validation Report"));
        assert!(markdown.contains("**Total Tests:** 5"));
        assert!(markdown.contains("80.0%"));
        
        Ok(())
    }
} 