//! Export command implementation for session data and reports

use crate::cli::{ExportArgs, ExportFormat};
use anyhow::Result;

/// Execute the export command
pub async fn run(args: ExportArgs) -> Result<()> {
    tracing::info!("Starting session export");

    if !args.session.exists() {
        anyhow::bail!("Session file not found: {}", args.session.display());
    }

    println!("📤 Exporting session: {}", args.session.display());
    println!("📝 Format: {:?}", args.format);

    // Read session data (placeholder)
    let session_data = std::fs::read_to_string(&args.session)?;

    // Convert to requested format
    let exported_data = match args.format {
        ExportFormat::Json => export_as_json(&session_data, &args)?,
        ExportFormat::Yaml => export_as_yaml(&session_data, &args)?,
        ExportFormat::Markdown => export_as_markdown(&session_data, &args)?,
        ExportFormat::Html => export_as_html(&session_data, &args)?,
        ExportFormat::Csv => export_as_csv(&session_data, &args)?,
    };

    // Output to file or stdout
    if let Some(output_path) = &args.output {
        std::fs::write(output_path, exported_data)?;
        println!("✅ Export saved to: {}", output_path.display());
    } else {
        println!("\n{}", exported_data);
    }

    Ok(())
}

/// Export session data as JSON
fn export_as_json(session_data: &str, args: &ExportArgs) -> Result<String> {
    let mut export = serde_json::json!({
        "format": "mcp-probe-session-export",
        "version": "1.0",
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "session_data": session_data
    });

    if args.include_timing {
        export["timing_info"] = serde_json::json!({
            "export_duration_ms": 0,
            "include_timing": true
        });
    }

    if args.include_raw {
        export["raw_messages"] = serde_json::json!({
            "included": true,
            "note": "Raw protocol messages included in session data"
        });
    }

    Ok(serde_json::to_string_pretty(&export)?)
}

/// Export session data as YAML
fn export_as_yaml(session_data: &str, args: &ExportArgs) -> Result<String> {
    let export = serde_yaml::Value::Mapping({
        let mut map = serde_yaml::Mapping::new();
        map.insert(
            serde_yaml::Value::String("format".to_string()),
            serde_yaml::Value::String("mcp-probe-session-export".to_string()),
        );
        map.insert(
            serde_yaml::Value::String("version".to_string()),
            serde_yaml::Value::String("1.0".to_string()),
        );
        map.insert(
            serde_yaml::Value::String("exported_at".to_string()),
            serde_yaml::Value::String(chrono::Utc::now().to_rfc3339()),
        );
        map.insert(
            serde_yaml::Value::String("session_data".to_string()),
            serde_yaml::Value::String(session_data.to_string()),
        );

        if args.include_timing {
            map.insert(
                serde_yaml::Value::String("include_timing".to_string()),
                serde_yaml::Value::Bool(true),
            );
        }

        if args.include_raw {
            map.insert(
                serde_yaml::Value::String("include_raw".to_string()),
                serde_yaml::Value::Bool(true),
            );
        }

        map
    });

    Ok(serde_yaml::to_string(&export)?)
}

/// Export session data as Markdown report
fn export_as_markdown(session_data: &str, args: &ExportArgs) -> Result<String> {
    let mut report = String::new();

    report.push_str("# MCP Session Export Report\n\n");
    report.push_str(&format!(
        "**Exported:** {}\n\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));

    report.push_str("## Session Information\n\n");
    report.push_str(&format!("- **Include Timing:** {}\n", args.include_timing));
    report.push_str(&format!(
        "- **Include Raw Messages:** {}\n",
        args.include_raw
    ));
    report.push('\n');

    report.push_str("## Session Data\n\n");
    report.push_str("```\n");
    report.push_str(session_data);
    report.push_str("\n```\n");

    Ok(report)
}

/// Export session data as HTML report
fn export_as_html(session_data: &str, args: &ExportArgs) -> Result<String> {
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>MCP Session Export Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; }}
        .header {{ border-bottom: 2px solid #333; padding-bottom: 10px; }}
        .section {{ margin: 20px 0; }}
        .session-data {{ background: #f5f5f5; padding: 15px; border-radius: 5px; }}
        pre {{ white-space: pre-wrap; word-wrap: break-word; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>MCP Session Export Report</h1>
        <p><strong>Exported:</strong> {}</p>
    </div>
    
    <div class="section">
        <h2>Session Information</h2>
        <ul>
            <li><strong>Include Timing:</strong> {}</li>
            <li><strong>Include Raw Messages:</strong> {}</li>
        </ul>
    </div>
    
    <div class="section">
        <h2>Session Data</h2>
        <div class="session-data">
            <pre>{}</pre>
        </div>
    </div>
</body>
</html>"#,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        args.include_timing,
        args.include_raw,
        html_escape::encode_text(session_data)
    );

    Ok(html)
}

/// Export session data as CSV
fn export_as_csv(session_data: &str, _args: &ExportArgs) -> Result<String> {
    let mut csv = String::new();

    // CSV header
    csv.push_str("timestamp,event_type,data\n");

    // Parse session data and convert to CSV rows (placeholder)
    let timestamp = chrono::Utc::now().to_rfc3339();
    csv.push_str(&format!(
        "{},session_data,\"{}\"\n",
        timestamp,
        session_data.replace('"', "\"\"")
    ));

    Ok(csv)
}

#[cfg(test)]
mod tests {
    use super::*;
    // Test utilities

    #[test]
    fn test_export_formats() -> Result<()> {
        let session_data = "test session data";
        let args = ExportArgs {
            session: "test.session".into(),
            format: ExportFormat::Json,
            output: None,
            include_raw: false,
            include_timing: false,
        };

        // Test JSON export
        let json_result = export_as_json(session_data, &args)?;
        assert!(json_result.contains("mcp-probe-session-export"));

        // Test YAML export
        let yaml_result = export_as_yaml(session_data, &args)?;
        assert!(yaml_result.contains("mcp-probe-session-export"));

        // Test Markdown export
        let md_result = export_as_markdown(session_data, &args)?;
        assert!(md_result.contains("# MCP Session Export Report"));

        // Test HTML export
        let html_result = export_as_html(session_data, &args)?;
        assert!(html_result.contains("<html>"));

        // Test CSV export
        let csv_result = export_as_csv(session_data, &args)?;
        assert!(csv_result.contains("timestamp,event_type,data"));

        Ok(())
    }
}
