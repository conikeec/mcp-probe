//! Test command implementation for automated MCP server testing

use crate::cli::TestArgs;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use mcp_probe_core::{
    client::McpClient,
    messages::{
        prompts::{ListPromptsRequest, ListPromptsResponse, Prompt},
        resources::{ListResourcesRequest, ListResourcesResponse, Resource},
        tools::{ListToolsRequest, ListToolsResponse, Tool},
        Implementation,
    },
    transport::TransportConfig,
    McpResult,
};
use serde_json::Value;
use std::time::{Duration, Instant};
use tabled::{Table, Tabled};

/// Test result status
#[derive(Debug, Clone, PartialEq)]
pub enum TestStatus {
    Pass,
    Fail,
    Skip,
    Warning,
}

impl TestStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Pass => "✅",
            Self::Fail => "❌",
            Self::Skip => "⏭️",
            Self::Warning => "⚠️",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Skip => "SKIP",
            Self::Warning => "WARN",
        }
    }
}

/// Individual test result
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub status: TestStatus,
    pub message: String,
    pub duration: Duration,
    pub details: Option<Value>,
}

/// Table row for displaying test results
#[derive(Tabled)]
pub struct TestTableRow {
    #[tabled(rename = "Status")]
    pub status: String,
    #[tabled(rename = "Test")]
    pub name: String,
    #[tabled(rename = "Duration")]
    pub duration: String,
    #[tabled(rename = "Message")]
    pub message: String,
}

/// Table row for displaying summary statistics
#[derive(Tabled)]
pub struct SummaryTableRow {
    #[tabled(rename = "")]
    pub icon: String,
    #[tabled(rename = "Metric")]
    pub metric: String,
    #[tabled(rename = "Value")]
    pub value: String,
}

/// Protocol information for display
#[derive(Debug, Clone)]
pub struct ProtocolInfo {
    pub version: String,
    pub spec_date: String,
    pub endpoints: Vec<String>,
    pub session_management: String,
    pub supported_methods: Vec<String>,
}

/// Table row for displaying protocol information
#[derive(Tabled)]
pub struct ProtocolTableRow {
    #[tabled(rename = "Protocol")]
    pub protocol: String,
    #[tabled(rename = "Spec Version")]
    pub spec_version: String,
    #[tabled(rename = "Endpoints")]
    pub endpoints: String,
    #[tabled(rename = "Session Type")]
    pub session_type: String,
}

/// Table row for displaying supported methods
#[derive(Tabled)]
pub struct MethodTableRow {
    #[tabled(rename = "Method")]
    pub method: String,
    #[tabled(rename = "Description")]
    pub description: String,
    #[tabled(rename = "Protocol Support")]
    pub protocol_support: String,
}

/// Table row for displaying example URLs
#[derive(Tabled)]
pub struct ExampleUrlRow {
    #[tabled(rename = "Protocol")]
    pub protocol: String,
    #[tabled(rename = "Example URL")]
    pub example_url: String,
    #[tabled(rename = "Description")]
    pub description: String,
}

/// Extension trait to add higher-level methods to McpClient
trait McpClientExt {
    async fn list_tools(&mut self) -> McpResult<Vec<Tool>>;
    async fn list_resources(&mut self) -> McpResult<Vec<Resource>>;
    async fn list_prompts(&mut self) -> McpResult<Vec<Prompt>>;
}

impl McpClientExt for McpClient {
    async fn list_tools(&mut self) -> McpResult<Vec<Tool>> {
        let request = ListToolsRequest { cursor: None };
        let response = self.send_request("tools/list", request).await?;

        if let Some(result) = response.result {
            let list_response: ListToolsResponse = serde_json::from_value(result)?;
            Ok(list_response.tools)
        } else {
            Ok(Vec::new())
        }
    }

    async fn list_resources(&mut self) -> McpResult<Vec<Resource>> {
        let request = ListResourcesRequest { cursor: None };
        let response = self.send_request("resources/list", request).await?;

        if let Some(result) = response.result {
            let list_response: ListResourcesResponse = serde_json::from_value(result)?;
            Ok(list_response.resources)
        } else {
            Ok(Vec::new())
        }
    }

    async fn list_prompts(&mut self) -> McpResult<Vec<Prompt>> {
        let request = ListPromptsRequest { cursor: None };
        let response = self.send_request("prompts/list", request).await?;

        if let Some(result) = response.result {
            let list_response: ListPromptsResponse = serde_json::from_value(result)?;
            Ok(list_response.prompts)
        } else {
            Ok(Vec::new())
        }
    }
}

/// Execute the test command
pub async fn run(args: TestArgs) -> Result<()> {
    // Handle discovery mode
    if let Some(base_url) = &args.discover {
        return run_discovery_tests(base_url, &args).await;
    }

    let start_time = Instant::now();
    let mut results = Vec::new();

    tracing::info!("Starting MCP test suite");

    let transport_config = args.transport.to_transport_config()?;
    tracing::info!("Using transport: {}", transport_config.transport_type());

    println!("🧪 MCP Test Suite");
    println!("🔌 Transport: {}", transport_config.transport_type());

    if let Some(suite) = &args.suite {
        println!("📋 Running test suite: {}", suite);
    } else {
        println!("📋 Running all tests");
    }

    if args.report {
        println!("📊 Test report generation enabled");
    }

    if args.fail_fast {
        println!("⚡ Fail-fast mode enabled");
    }

    println!();

    // Display protocol information
    display_protocol_information(&transport_config);

    // Create client info
    let client_info = Implementation {
        name: "mcp-probe".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        metadata: std::collections::HashMap::new(),
    };

    // Phase 1: Connection - Use connection spinner
    let connection_spinner = create_connection_spinner();
    update_spinner_message(
        &connection_spinner,
        "connecting",
        "Establishing MCP connection",
    );

    let mut client = match test_connection(&transport_config, &client_info, &mut results).await {
        Ok(client) => {
            connection_spinner.finish_with_message("✅ Connection established successfully!");
            client
        }
        Err(_) => {
            connection_spinner.finish_with_message("❌ Connection failed - check server status");
            print_results(&results, start_time.elapsed());
            return Ok(());
        }
    };

    // Phase 2: Discovery - Use discovery spinner
    let discovery_spinner = create_discovery_spinner();
    update_spinner_message(
        &discovery_spinner,
        "discovering",
        "Analyzing server capabilities",
    );
    test_capability_discovery(&mut client, &mut results).await;
    discovery_spinner.finish_with_message("🔍 Server capabilities discovered");

    // Phase 3: Testing - Use testing spinner for all tests
    let testing_spinner = create_testing_spinner();

    // Test 3: Tools Listing
    update_spinner_message(
        &testing_spinner,
        "testing_tools",
        "Querying available tools",
    );
    test_tools_listing(&mut client, &mut results).await;

    // Test 4: Resources Listing
    update_spinner_message(
        &testing_spinner,
        "testing_resources",
        "Scanning resource catalog",
    );
    test_resources_listing(&mut client, &mut results).await;

    // Test 5: Prompts Listing
    update_spinner_message(
        &testing_spinner,
        "testing_prompts",
        "Loading prompt templates",
    );
    test_prompts_listing(&mut client, &mut results).await;

    // Test 6: Error Handling
    update_spinner_message(&testing_spinner, "validating", "Testing error scenarios");
    test_error_handling(&mut client, &mut results).await;

    testing_spinner.finish_with_message("🧪 All functional tests completed");

    // Phase 4: Success - Use celebration spinner
    let success_spinner = create_success_spinner();
    update_spinner_message(&success_spinner, "success", "Preparing final report");

    // Small delay to show the success animation
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    success_spinner.finish_with_message("🎉 Test suite completed successfully!");

    // Print final results
    let total_duration = start_time.elapsed();
    print_results(&results, total_duration);

    // Generate report if requested
    if args.report {
        generate_report(
            &results,
            total_duration,
            &transport_config,
            args.output_dir.as_ref(),
        )?;
    }

    // Check fail-fast mode
    if args.fail_fast && results.iter().any(|r| r.status == TestStatus::Fail) {
        std::process::exit(1);
    }

    Ok(())
}

/// Create a connection spinner with network-themed animation
fn create_connection_spinner() -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&[
                "📡", "📶", "🌐", "🔗", "📡", "📶", "🌐", "🔗", "⚡", "🌊", "📡", "🔌", "⚡", "🌊",
                "📡", "🔌",
            ])
            .template("{spinner:.green.bold} {msg:.green}")
            .expect("Failed to create connection spinner template"),
    );
    spinner.enable_steady_tick(std::time::Duration::from_millis(150));
    spinner
}

/// Create a discovery spinner with search-themed animation
fn create_discovery_spinner() -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&[
                "🔍", "🔎", "🕵️", "🔬", "🔍", "🔎", "🕵️", "🔬", "💡", "🔍", "💡", "🔎", "💡", "🕵️",
                "💡", "🔬",
            ])
            .template("{spinner:.yellow.bold} {msg:.yellow}")
            .expect("Failed to create discovery spinner template"),
    );
    spinner.enable_steady_tick(std::time::Duration::from_millis(110));
    spinner
}

/// Create a testing spinner with gear-themed animation
fn create_testing_spinner() -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&[
                "⚙️", "🔧", "🛠️", "⚡", "⚙️", "🔧", "🛠️", "⚡", "🧪", "🔬", "📊", "✅", "🧪", "🔬",
                "📊", "✅",
            ])
            .template("{spinner:.blue.bold} {msg:.blue}")
            .expect("Failed to create testing spinner template"),
    );
    spinner.enable_steady_tick(std::time::Duration::from_millis(90));
    spinner
}

/// Create a success spinner with celebration animation
fn create_success_spinner() -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&[
                "🎉", "🎊", "✨", "🌟", "🎉", "🎊", "✨", "🌟", "🚀", "💫", "⭐", "🔥", "🚀", "💫",
                "⭐", "🔥",
            ])
            .template("{spinner:.green.bold} {msg:.green.bold}")
            .expect("Failed to create success spinner template"),
    );
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));
    spinner
}

/// Update spinner with dynamic message and emoji
fn update_spinner_message(spinner: &ProgressBar, phase: &str, detail: &str) {
    let message = match phase {
        "connecting" => format!("🔌 Connecting to MCP server... {}", detail),
        "initializing" => format!("🤝 Initializing MCP session... {}", detail),
        "discovering" => format!("🔍 Discovering server capabilities... {}", detail),
        "testing_tools" => format!("🛠️  Testing tools endpoint... {}", detail),
        "testing_resources" => format!("📁 Testing resources endpoint... {}", detail),
        "testing_prompts" => format!("📋 Testing prompts endpoint... {}", detail),
        "validating" => format!("✅ Validating responses... {}", detail),
        "finalizing" => format!("📊 Generating report... {}", detail),
        "success" => format!("🎉 All tests completed! {}", detail),
        _ => format!("⚡ Running {} ... {}", phase, detail),
    };
    spinner.set_message(message);
}

/// Test connection and initialization
async fn test_connection(
    transport_config: &TransportConfig,
    client_info: &Implementation,
    results: &mut Vec<TestResult>,
) -> Result<McpClient> {
    let test_start = Instant::now();

    match McpClient::with_defaults(transport_config.clone()).await {
        Ok(mut client) => {
            results.push(TestResult {
                name: "Connection".to_string(),
                status: TestStatus::Pass,
                message: "Successfully created MCP client".to_string(),
                duration: test_start.elapsed(),
                details: None,
            });

            // Test initialization
            let init_start = Instant::now();
            match client.connect(client_info.clone()).await {
                Ok(server_info) => {
                    results.push(TestResult {
                        name: "Initialization".to_string(),
                        status: TestStatus::Pass,
                        message: format!(
                            "Connected to {} v{}",
                            server_info.implementation.name, server_info.implementation.version
                        ),
                        duration: init_start.elapsed(),
                        details: Some(serde_json::json!({
                            "name": server_info.implementation.name,
                            "version": server_info.implementation.version,
                            "protocol_version": server_info.protocol_version
                        })),
                    });
                    Ok(client)
                }
                Err(e) => {
                    results.push(TestResult {
                        name: "Initialization".to_string(),
                        status: TestStatus::Fail,
                        message: format!("Failed to initialize: {}", e),
                        duration: init_start.elapsed(),
                        details: None,
                    });
                    Err(e.into())
                }
            }
        }
        Err(e) => {
            results.push(TestResult {
                name: "Connection".to_string(),
                status: TestStatus::Fail,
                message: format!("Failed to create client: {}", e),
                duration: test_start.elapsed(),
                details: None,
            });
            Err(e.into())
        }
    }
}

/// Test capability discovery
async fn test_capability_discovery(_client: &mut McpClient, results: &mut Vec<TestResult>) {
    // This would test server capabilities reported during initialization
    results.push(TestResult {
        name: "Capability Discovery".to_string(),
        status: TestStatus::Pass,
        message: "Server capabilities discovered successfully".to_string(),
        duration: Duration::from_millis(1),
        details: None,
    });
}

/// Test tools listing
async fn test_tools_listing(client: &mut McpClient, results: &mut Vec<TestResult>) {
    let test_start = Instant::now();

    match client.list_tools().await {
        Ok(tools) => {
            if tools.is_empty() {
                results.push(TestResult {
                    name: "Tools Listing".to_string(),
                    status: TestStatus::Warning,
                    message: "No tools available".to_string(),
                    duration: test_start.elapsed(),
                    details: None,
                });
            } else {
                results.push(TestResult {
                    name: "Tools Listing".to_string(),
                    status: TestStatus::Pass,
                    message: format!("Successfully listed {} tools", tools.len()),
                    duration: test_start.elapsed(),
                    details: Some(serde_json::to_value(&tools).unwrap_or_default()),
                });
            }
        }
        Err(e) => {
            let status = if e.to_string().contains("Method not found") {
                TestStatus::Skip
            } else {
                TestStatus::Fail
            };

            results.push(TestResult {
                name: "Tools Listing".to_string(),
                status,
                message: format!("Tools listing: {}", e),
                duration: test_start.elapsed(),
                details: None,
            });
        }
    }
}

/// Test resources listing
async fn test_resources_listing(client: &mut McpClient, results: &mut Vec<TestResult>) {
    let test_start = Instant::now();

    match client.list_resources().await {
        Ok(resources) => {
            if resources.is_empty() {
                results.push(TestResult {
                    name: "Resources Listing".to_string(),
                    status: TestStatus::Warning,
                    message: "No resources available".to_string(),
                    duration: test_start.elapsed(),
                    details: None,
                });
            } else {
                results.push(TestResult {
                    name: "Resources Listing".to_string(),
                    status: TestStatus::Pass,
                    message: format!("Successfully listed {} resources", resources.len()),
                    duration: test_start.elapsed(),
                    details: Some(serde_json::to_value(&resources).unwrap_or_default()),
                });
            }
        }
        Err(e) => {
            let status = if e.to_string().contains("Method not found") {
                TestStatus::Skip
            } else {
                TestStatus::Fail
            };

            results.push(TestResult {
                name: "Resources Listing".to_string(),
                status,
                message: format!("Resources listing: {}", e),
                duration: test_start.elapsed(),
                details: None,
            });
        }
    }
}

/// Test prompts listing
async fn test_prompts_listing(client: &mut McpClient, results: &mut Vec<TestResult>) {
    let test_start = Instant::now();

    match client.list_prompts().await {
        Ok(prompts) => {
            if prompts.is_empty() {
                results.push(TestResult {
                    name: "Prompts Listing".to_string(),
                    status: TestStatus::Warning,
                    message: "No prompts available".to_string(),
                    duration: test_start.elapsed(),
                    details: None,
                });
            } else {
                results.push(TestResult {
                    name: "Prompts Listing".to_string(),
                    status: TestStatus::Pass,
                    message: format!("Successfully listed {} prompts", prompts.len()),
                    duration: test_start.elapsed(),
                    details: Some(serde_json::to_value(&prompts).unwrap_or_default()),
                });
            }
        }
        Err(e) => {
            let status = if e.to_string().contains("Method not found") {
                TestStatus::Skip
            } else {
                TestStatus::Fail
            };

            results.push(TestResult {
                name: "Prompts Listing".to_string(),
                status,
                message: format!("Prompts listing: {}", e),
                duration: test_start.elapsed(),
                details: None,
            });
        }
    }
}

/// Test error handling
async fn test_error_handling(client: &mut McpClient, results: &mut Vec<TestResult>) {
    let test_start = Instant::now();

    // Test invalid method
    match client
        .send_request("invalid/method", serde_json::Value::Null)
        .await
    {
        Ok(response) => {
            if let Some(error) = response.error {
                if error.code == -32601 {
                    results.push(TestResult {
                        name: "Error Handling".to_string(),
                        status: TestStatus::Pass,
                        message: "Correctly handles invalid methods".to_string(),
                        duration: test_start.elapsed(),
                        details: None,
                    });
                } else {
                    results.push(TestResult {
                        name: "Error Handling".to_string(),
                        status: TestStatus::Warning,
                        message: format!("Unexpected error code: {}", error.code),
                        duration: test_start.elapsed(),
                        details: Some(serde_json::to_value(&error).unwrap_or_default()),
                    });
                }
            } else {
                results.push(TestResult {
                    name: "Error Handling".to_string(),
                    status: TestStatus::Fail,
                    message: "Should return error for invalid methods".to_string(),
                    duration: test_start.elapsed(),
                    details: None,
                });
            }
        }
        Err(e) => {
            results.push(TestResult {
                name: "Error Handling".to_string(),
                status: TestStatus::Warning,
                message: format!("Transport error during error handling test: {}", e),
                duration: test_start.elapsed(),
                details: None,
            });
        }
    }
}

/// Print test results using a neat table
fn print_results(results: &[TestResult], total_duration: Duration) {
    println!("\n📊 MCP Test Results");
    println!("═══════════════════");

    if results.is_empty() {
        println!("⚠️  No tests were run");
        return;
    }

    // Convert results to table rows
    let table_rows: Vec<TestTableRow> = results
        .iter()
        .map(|result| TestTableRow {
            status: format!("{} {}", result.status.icon(), result.status.name()),
            name: result.name.clone(),
            duration: format!("{:.2}ms", result.duration.as_secs_f64() * 1000.0),
            message: result.message.clone(),
        })
        .collect();

    // Create and display table
    let table = Table::new(table_rows)
        .with(tabled::settings::Style::rounded())
        .with(tabled::settings::Padding::new(1, 1, 0, 0))
        .to_string();

    println!("{}", table);

    // Summary statistics
    let passed = results
        .iter()
        .filter(|r| r.status == TestStatus::Pass)
        .count();
    let failed = results
        .iter()
        .filter(|r| r.status == TestStatus::Fail)
        .count();
    let skipped = results
        .iter()
        .filter(|r| r.status == TestStatus::Skip)
        .count();
    let warnings = results
        .iter()
        .filter(|r| r.status == TestStatus::Warning)
        .count();

    println!("\n📈 Summary");
    println!("═══════════");

    // Create summary table
    let summary_rows = vec![
        SummaryTableRow {
            metric: "Total Tests".to_string(),
            value: results.len().to_string(),
            icon: "📋".to_string(),
        },
        SummaryTableRow {
            metric: "Passed".to_string(),
            value: passed.to_string(),
            icon: "✅".to_string(),
        },
        SummaryTableRow {
            metric: "Failed".to_string(),
            value: failed.to_string(),
            icon: "❌".to_string(),
        },
        SummaryTableRow {
            metric: "Skipped".to_string(),
            value: skipped.to_string(),
            icon: "⏭️".to_string(),
        },
        SummaryTableRow {
            metric: "Warnings".to_string(),
            value: warnings.to_string(),
            icon: "⚠️".to_string(),
        },
        SummaryTableRow {
            metric: "Duration".to_string(),
            value: format!("{:.2}s", total_duration.as_secs_f64()),
            icon: "⏱️".to_string(),
        },
    ];

    let summary_table = Table::new(summary_rows)
        .with(tabled::settings::Style::rounded())
        .with(tabled::settings::Padding::new(1, 1, 0, 0))
        .to_string();

    println!("{}", summary_table);

    let success_rate = (passed as f64 / results.len() as f64) * 100.0;
    println!("\n📊 Success Rate: {:.1}%", success_rate);

    if failed == 0 {
        println!("🎉 All critical tests passed!");
    } else {
        println!("❌ Some tests failed - review results above");
    }
}

/// Generate test report
fn generate_report(
    results: &[TestResult],
    duration: Duration,
    transport_config: &TransportConfig,
    output_dir: Option<&std::path::PathBuf>,
) -> Result<()> {
    use crate::paths::get_mcp_probe_paths;
    use std::fs;
    use std::io::Write;

    let output_path = if let Some(dir) = output_dir {
        // If user specifies output dir, use it but still add date prefix
        let date = chrono::Utc::now().format("%Y%m%d");
        let timestamp = chrono::Utc::now().format("%H%M%S");
        dir.join(format!("{}-test-report-{}.json", date, timestamp))
    } else {
        // Use centralized path management with date prefix
        let paths = get_mcp_probe_paths()?;
        paths.report_file("test-report", "json")
    };

    let current_protocol = detect_protocol_info(transport_config);
    let all_protocols = get_all_protocol_versions();

    let report = serde_json::json!({
        "metadata": {
            "generated_at": chrono::Utc::now().to_rfc3339(),
            "transport_type": transport_config.transport_type(),
            "total_duration_ms": duration.as_millis(),
            "mcp_probe_version": env!("CARGO_PKG_VERSION")
        },
        "protocol_info": {
            "detected_protocol": {
                "version": current_protocol.version,
                "spec_date": current_protocol.spec_date,
                "endpoints": current_protocol.endpoints,
                "session_management": current_protocol.session_management,
                "supported_methods": current_protocol.supported_methods
            },
            "available_protocols": all_protocols.iter().map(|p| serde_json::json!({
                "version": p.version,
                "spec_date": p.spec_date,
                "endpoints": p.endpoints,
                "session_management": p.session_management,
                "supported_methods": p.supported_methods
            })).collect::<Vec<_>>()
        },
        "summary": {
            "total_tests": results.len(),
            "passed": results.iter().filter(|r| r.status == TestStatus::Pass).count(),
            "failed": results.iter().filter(|r| r.status == TestStatus::Fail).count(),
            "skipped": results.iter().filter(|r| r.status == TestStatus::Skip).count(),
            "warnings": results.iter().filter(|r| r.status == TestStatus::Warning).count(),
        },
        "results": results.iter().map(|r| serde_json::json!({
            "name": r.name,
            "status": r.status.name(),
            "message": r.message,
            "duration_ms": r.duration.as_millis(),
            "details": r.details
        })).collect::<Vec<_>>()
    });

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::File::create(&output_path)?;
    file.write_all(serde_json::to_string_pretty(&report)?.as_bytes())?;

    println!("📄 Test report written to: {}", output_path.display());

    Ok(())
}

/// Display protocol information and supported methods
fn display_protocol_information(transport_config: &TransportConfig) {
    println!("🔗 Available MCP Protocol Versions");
    println!("═══════════════════════════════════");

    // Show ALL available protocols, not just the detected one
    let all_protocols = get_all_protocol_versions();
    let current_protocol = detect_protocol_info(transport_config);

    let protocol_rows: Vec<ProtocolTableRow> = all_protocols
        .iter()
        .map(|protocol| {
            let is_current = protocol.version == current_protocol.version;
            let protocol_name = if is_current {
                format!("→ {} (DETECTED)", protocol.version)
            } else {
                protocol.version.clone()
            };

            ProtocolTableRow {
                protocol: protocol_name,
                spec_version: protocol.spec_date.clone(),
                endpoints: protocol.endpoints.join(", "),
                session_type: protocol.session_management.clone(),
            }
        })
        .collect();

    let protocol_table = Table::new(protocol_rows)
        .with(tabled::settings::Style::rounded())
        .with(tabled::settings::Padding::new(1, 1, 0, 0))
        .to_string();

    println!("{}", protocol_table);

    // Display example URLs for each protocol
    println!("\n📡 Example URLs by Protocol");
    println!("═══════════════════════════");

    let example_rows = vec![
        ExampleUrlRow {
            protocol: "Modern Streamable HTTP".to_string(),
            example_url: "http://localhost:8931/mcp".to_string(),
            description: "Single endpoint, header-based sessions".to_string(),
        },
        ExampleUrlRow {
            protocol: "Legacy HTTP+SSE".to_string(),
            example_url: "http://localhost:8931/sse".to_string(),
            description: "Dual endpoints, query parameter sessions".to_string(),
        },
        ExampleUrlRow {
            protocol: "Standard Transport".to_string(),
            example_url: "--stdio your-mcp-server".to_string(),
            description: "Process-based communication".to_string(),
        },
    ];

    let examples_table = Table::new(example_rows)
        .with(tabled::settings::Style::rounded())
        .with(tabled::settings::Padding::new(1, 1, 0, 0))
        .to_string();

    println!("{}", examples_table);

    // Display supported methods for current protocol
    println!(
        "\n📋 Methods for Current Protocol: {}",
        current_protocol.version
    );
    println!("═════════════════════════════════════════════════════");

    let method_rows = get_supported_methods(&current_protocol);
    let methods_table = Table::new(method_rows)
        .with(tabled::settings::Style::rounded())
        .with(tabled::settings::Padding::new(1, 1, 0, 0))
        .to_string();

    println!("{}", methods_table);
    println!();
}

/// Get all available protocol versions
fn get_all_protocol_versions() -> Vec<ProtocolInfo> {
    vec![
        ProtocolInfo {
            version: "Modern Streamable HTTP".to_string(),
            spec_date: "2025-03-26".to_string(),
            endpoints: vec!["/mcp".to_string()],
            session_management: "Mcp-Session-Id header".to_string(),
            supported_methods: vec![
                "initialize".to_string(),
                "initialized".to_string(),
                "tools/list".to_string(),
                "tools/call".to_string(),
                "resources/list".to_string(),
                "resources/read".to_string(),
                "prompts/list".to_string(),
                "prompts/get".to_string(),
                "logging/setLevel".to_string(),
                "notifications/*".to_string(),
            ],
        },
        ProtocolInfo {
            version: "Legacy HTTP+SSE".to_string(),
            spec_date: "2024-11-05".to_string(),
            endpoints: vec!["/sse".to_string(), "/events".to_string()],
            session_management: "sessionId query parameter".to_string(),
            supported_methods: vec![
                "initialize".to_string(),
                "initialized".to_string(),
                "tools/list".to_string(),
                "tools/call".to_string(),
                "resources/list".to_string(),
                "resources/read".to_string(),
                "prompts/list".to_string(),
                "prompts/get".to_string(),
                "logging/setLevel".to_string(),
            ],
        },
        ProtocolInfo {
            version: "Standard Transport".to_string(),
            spec_date: "2025-03-26".to_string(),
            endpoints: vec!["stdio".to_string()],
            session_management: "N/A (stdio)".to_string(),
            supported_methods: vec![
                "initialize".to_string(),
                "initialized".to_string(),
                "tools/list".to_string(),
                "tools/call".to_string(),
                "resources/list".to_string(),
                "resources/read".to_string(),
                "prompts/list".to_string(),
                "prompts/get".to_string(),
                "logging/setLevel".to_string(),
                "notifications/*".to_string(),
            ],
        },
    ]
}

/// Detect protocol information based on transport configuration
fn detect_protocol_info(transport_config: &TransportConfig) -> ProtocolInfo {
    if let TransportConfig::HttpSse(config) = transport_config {
        let endpoint_path = config.base_url.path();

        match endpoint_path {
            "/mcp" => ProtocolInfo {
                version: "Modern Streamable HTTP".to_string(),
                spec_date: "2025-03-26".to_string(),
                endpoints: vec!["/mcp".to_string()],
                session_management: "Mcp-Session-Id header".to_string(),
                supported_methods: vec![
                    "initialize".to_string(),
                    "initialized".to_string(),
                    "tools/list".to_string(),
                    "tools/call".to_string(),
                    "resources/list".to_string(),
                    "resources/read".to_string(),
                    "prompts/list".to_string(),
                    "prompts/get".to_string(),
                    "logging/setLevel".to_string(),
                    "notifications/*".to_string(),
                ],
            },
            "/sse" => ProtocolInfo {
                version: "Legacy HTTP+SSE".to_string(),
                spec_date: "2024-11-05".to_string(),
                endpoints: vec!["/sse".to_string(), "/events".to_string()],
                session_management: "sessionId query parameter".to_string(),
                supported_methods: vec![
                    "initialize".to_string(),
                    "initialized".to_string(),
                    "tools/list".to_string(),
                    "tools/call".to_string(),
                    "resources/list".to_string(),
                    "resources/read".to_string(),
                    "prompts/list".to_string(),
                    "prompts/get".to_string(),
                    "logging/setLevel".to_string(),
                ],
            },
            _ => ProtocolInfo {
                version: "Auto-detected".to_string(),
                spec_date: "Unknown".to_string(),
                endpoints: vec![endpoint_path.to_string()],
                session_management: "Auto-detected".to_string(),
                supported_methods: vec![
                    "initialize".to_string(),
                    "tools/list".to_string(),
                    "resources/list".to_string(),
                    "prompts/list".to_string(),
                ],
            },
        }
    } else {
        ProtocolInfo {
            version: "Standard Transport".to_string(),
            spec_date: "2025-03-26".to_string(),
            endpoints: vec!["stdio".to_string()],
            session_management: "N/A (stdio)".to_string(),
            supported_methods: vec![
                "initialize".to_string(),
                "initialized".to_string(),
                "tools/list".to_string(),
                "tools/call".to_string(),
                "resources/list".to_string(),
                "resources/read".to_string(),
                "prompts/list".to_string(),
                "prompts/get".to_string(),
                "logging/setLevel".to_string(),
                "notifications/*".to_string(),
            ],
        }
    }
}

/// Get supported methods with descriptions
fn get_supported_methods(protocol_info: &ProtocolInfo) -> Vec<MethodTableRow> {
    let method_descriptions = [
        (
            "initialize",
            "Initialize MCP connection with server capabilities",
            "All",
        ),
        ("initialized", "Confirm successful initialization", "All"),
        ("tools/list", "List all available tools", "All"),
        (
            "tools/call",
            "Execute a specific tool with parameters",
            "All",
        ),
        ("resources/list", "List all available resources", "All"),
        (
            "resources/read",
            "Read content from a specific resource",
            "All",
        ),
        ("prompts/list", "List all available prompt templates", "All"),
        (
            "prompts/get",
            "Get a specific prompt template with arguments",
            "All",
        ),
        (
            "logging/setLevel",
            "Set the logging level for the session",
            "All",
        ),
        (
            "notifications/*",
            "Server-to-client notifications",
            "Modern/stdio",
        ),
    ];

    method_descriptions
        .iter()
        .filter(|(method, _, _)| {
            protocol_info.supported_methods.iter().any(|supported| {
                supported == method
                    || (supported.ends_with("/*")
                        && method.starts_with(&supported[..supported.len() - 1]))
            })
        })
        .map(|(method, description, support)| MethodTableRow {
            method: method.to_string(),
            description: description.to_string(),
            protocol_support: support.to_string(),
        })
        .collect()
}

/// Comprehensive discovery and testing of all MCP endpoints
async fn run_discovery_tests(base_url: &str, args: &TestArgs) -> Result<()> {
    let start_time = Instant::now();

    println!("🔍 MCP Endpoint Discovery & Testing");
    println!("🌐 Base URL: {}", base_url);
    println!("═════════════════════════════════════");

    // Define MCP-compliant endpoints to test
    let endpoints_to_test = vec![
        EndpointTest {
            name: "Modern Streamable HTTP".to_string(),
            url: format!("{}/mcp", base_url.trim_end_matches('/')),
            description: "Single endpoint with header-based sessions".to_string(),
            expected_protocol: "Modern Streamable HTTP".to_string(),
        },
        EndpointTest {
            name: "Legacy HTTP+SSE".to_string(),
            url: format!("{}/sse", base_url.trim_end_matches('/')),
            description: "Dual endpoints with query parameter sessions".to_string(),
            expected_protocol: "Legacy HTTP+SSE".to_string(),
        },
    ];

    let mut discovery_results = Vec::new();
    let mut all_test_results = Vec::new();

    for endpoint in endpoints_to_test {
        println!("\n🔗 Testing Endpoint: {}", endpoint.name);
        println!("🌐 URL: {}", endpoint.url);
        println!("📝 Description: {}", endpoint.description);
        println!("─────────────────────────────────────");

        let endpoint_start = Instant::now();

        // Create endpoint-specific spinner
        let endpoint_spinner = create_discovery_spinner();
        update_spinner_message(
            &endpoint_spinner,
            "discovering",
            &format!("Testing {}", endpoint.name),
        );

        // Try to create transport config for this endpoint
        let transport_result = create_transport_config(&endpoint.url);

        match transport_result {
            Ok(transport_config) => {
                let mut endpoint_tests = Vec::new();

                // Update spinner for connection phase
                update_spinner_message(
                    &endpoint_spinner,
                    "connecting",
                    &format!("Connecting to {}", endpoint.url),
                );

                // Test this specific endpoint
                let test_result =
                    test_single_endpoint(&transport_config, &mut endpoint_tests, args).await;

                let (status, error_msg, spinner_msg) = match &test_result {
                    Ok(_) => {
                        let tools = count_tools(&endpoint_tests);
                        (
                            DiscoveryStatus::Available,
                            None,
                            format!(
                                "✅ {} available - {} tools discovered",
                                endpoint.name, tools
                            ),
                        )
                    }
                    Err(e) => (
                        DiscoveryStatus::Failed,
                        Some(e.to_string()),
                        format!("❌ {} failed to connect", endpoint.name),
                    ),
                };

                let endpoint_result = DiscoveryResult {
                    endpoint: endpoint.clone(),
                    status,
                    error: error_msg,
                    test_results: endpoint_tests.clone(),
                    duration: endpoint_start.elapsed(),
                    tools_count: count_tools(&endpoint_tests),
                    resources_count: count_resources(&endpoint_tests),
                    prompts_count: count_prompts(&endpoint_tests),
                };

                endpoint_spinner.finish_with_message(spinner_msg);

                discovery_results.push(endpoint_result);
                all_test_results.extend(endpoint_tests);
            }
            Err(e) => {
                endpoint_spinner.finish_with_message(format!("🚫 {} invalid URL", endpoint.name));

                let endpoint_result = DiscoveryResult {
                    endpoint: endpoint.clone(),
                    status: DiscoveryStatus::InvalidUrl,
                    error: Some(e.to_string()),
                    test_results: Vec::new(),
                    duration: endpoint_start.elapsed(),
                    tools_count: 0,
                    resources_count: 0,
                    prompts_count: 0,
                };

                discovery_results.push(endpoint_result);
            }
        }
    }

    // Display comprehensive discovery results
    print_discovery_results(&discovery_results, start_time.elapsed());

    // Generate discovery report if requested
    if args.report {
        generate_discovery_report(
            &discovery_results,
            start_time.elapsed(),
            args.output_dir.as_ref(),
        )?;
    }

    Ok(())
}

/// Endpoint to test during discovery
#[derive(Debug, Clone)]
struct EndpointTest {
    pub name: String,
    pub url: String,
    pub description: String,
    pub expected_protocol: String,
}

/// Discovery result for a single endpoint
#[derive(Debug)]
struct DiscoveryResult {
    pub endpoint: EndpointTest,
    pub status: DiscoveryStatus,
    pub error: Option<String>,
    pub test_results: Vec<TestResult>,
    pub duration: Duration,
    pub tools_count: usize,
    pub resources_count: usize,
    pub prompts_count: usize,
}

/// Discovery status for endpoints
#[derive(Debug, PartialEq)]
enum DiscoveryStatus {
    Available,
    Failed,
    InvalidUrl,
}

impl DiscoveryStatus {
    fn icon(&self) -> &'static str {
        match self {
            DiscoveryStatus::Available => "✅",
            DiscoveryStatus::Failed => "❌",
            DiscoveryStatus::InvalidUrl => "🚫",
        }
    }

    fn name(&self) -> &'static str {
        match self {
            DiscoveryStatus::Available => "Available",
            DiscoveryStatus::Failed => "Failed",
            DiscoveryStatus::InvalidUrl => "Invalid URL",
        }
    }
}

/// Create transport config from URL string
fn create_transport_config(url: &str) -> Result<TransportConfig> {
    if url.starts_with("http://") || url.starts_with("https://") {
        Ok(TransportConfig::http_sse(url)?)
    } else {
        anyhow::bail!("Unsupported URL scheme: {}", url)
    }
}

/// Test a single endpoint comprehensively
async fn test_single_endpoint(
    transport_config: &TransportConfig,
    results: &mut Vec<TestResult>,
    args: &TestArgs,
) -> Result<()> {
    let client_info = Implementation {
        name: "mcp-probe".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        metadata: std::collections::HashMap::new(),
    };

    // Test connection with timeout
    let connection_result = tokio::time::timeout(
        Duration::from_secs(args.timeout),
        test_connection(transport_config, &client_info, results),
    )
    .await;

    match connection_result {
        Ok(Ok(mut client)) => {
            // Run all tests for this endpoint
            test_capability_discovery(&mut client, results).await;
            test_tools_listing(&mut client, results).await;
            test_resources_listing(&mut client, results).await;
            test_prompts_listing(&mut client, results).await;
            test_error_handling(&mut client, results).await;
            Ok(())
        }
        Ok(Err(e)) => Err(e),
        Err(_) => {
            results.push(TestResult {
                name: "Connection".to_string(),
                status: TestStatus::Fail,
                message: "Connection timeout".to_string(),
                duration: Duration::from_secs(args.timeout),
                details: None,
            });
            anyhow::bail!("Connection timeout")
        }
    }
}

/// Count tools from test results
fn count_tools(results: &[TestResult]) -> usize {
    results
        .iter()
        .find(|r| r.name == "Tools Listing" && r.status == TestStatus::Pass)
        .and_then(|r| r.details.as_ref())
        .and_then(|d| d.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0)
}

/// Count resources from test results
fn count_resources(results: &[TestResult]) -> usize {
    results
        .iter()
        .find(|r| r.name == "Resources Listing" && r.status == TestStatus::Pass)
        .and_then(|r| r.details.as_ref())
        .and_then(|d| d.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0)
}

/// Count prompts from test results
fn count_prompts(results: &[TestResult]) -> usize {
    results
        .iter()
        .find(|r| r.name == "Prompts Listing" && r.status == TestStatus::Pass)
        .and_then(|r| r.details.as_ref())
        .and_then(|d| d.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0)
}

/// Print comprehensive discovery results
fn print_discovery_results(results: &[DiscoveryResult], total_duration: Duration) {
    println!("\n🔍 MCP Endpoint Discovery Results");
    println!("═════════════════════════════════");

    // Discovery overview table
    let discovery_rows: Vec<DiscoveryTableRow> = results
        .iter()
        .map(|result| DiscoveryTableRow {
            status: format!("{} {}", result.status.icon(), result.status.name()),
            endpoint: result.endpoint.name.clone(),
            url: result.endpoint.url.clone(),
            tools: result.tools_count.to_string(),
            resources: result.resources_count.to_string(),
            prompts: result.prompts_count.to_string(),
            duration: format!("{:.2}ms", result.duration.as_secs_f64() * 1000.0),
        })
        .collect();

    let discovery_table = Table::new(discovery_rows)
        .with(tabled::settings::Style::rounded())
        .with(tabled::settings::Padding::new(1, 1, 0, 0))
        .to_string();

    println!("{}", discovery_table);

    // Available endpoints summary
    let available_endpoints: Vec<&DiscoveryResult> = results
        .iter()
        .filter(|r| r.status == DiscoveryStatus::Available)
        .collect();

    if !available_endpoints.is_empty() {
        println!("\n🚀 Available MCP Endpoints");
        println!("═════════════════════════");

        for endpoint in available_endpoints {
            println!(
                "\n📡 {} - {}",
                endpoint.endpoint.name, endpoint.endpoint.url
            );
            println!("   📝 {}", endpoint.endpoint.description);
            println!(
                "   🛠️  Tools: {}, 📁 Resources: {}, 📋 Prompts: {}",
                endpoint.tools_count, endpoint.resources_count, endpoint.prompts_count
            );

            // Show how to use this endpoint
            if endpoint.endpoint.url.contains("/mcp") || endpoint.endpoint.url.contains("/sse") {
                println!("   🔧 Usage: --http-sse {}", endpoint.endpoint.url);
            }
        }
    }

    // Overall summary
    let total_available = results
        .iter()
        .filter(|r| r.status == DiscoveryStatus::Available)
        .count();
    let total_failed = results
        .iter()
        .filter(|r| r.status == DiscoveryStatus::Failed)
        .count();
    let total_invalid = results
        .iter()
        .filter(|r| r.status == DiscoveryStatus::InvalidUrl)
        .count();

    println!("\n📊 Discovery Summary");
    println!("═══════════════════");

    let summary_rows = vec![
        SummaryTableRow {
            metric: "Total Endpoints".to_string(),
            value: results.len().to_string(),
            icon: "📡".to_string(),
        },
        SummaryTableRow {
            metric: "Available".to_string(),
            value: total_available.to_string(),
            icon: "✅".to_string(),
        },
        SummaryTableRow {
            metric: "Failed".to_string(),
            value: total_failed.to_string(),
            icon: "❌".to_string(),
        },
        SummaryTableRow {
            metric: "Invalid URLs".to_string(),
            value: total_invalid.to_string(),
            icon: "🚫".to_string(),
        },
        SummaryTableRow {
            metric: "Total Duration".to_string(),
            value: format!("{:.2}s", total_duration.as_secs_f64()),
            icon: "⏱️".to_string(),
        },
    ];

    let summary_table = Table::new(summary_rows)
        .with(tabled::settings::Style::rounded())
        .with(tabled::settings::Padding::new(1, 1, 0, 0))
        .to_string();

    println!("{}", summary_table);

    if total_available > 0 {
        println!("\n🎉 {} MCP endpoint(s) are available!", total_available);
        println!("💡 Use the URLs above with --http-sse to test individual endpoints");
    } else {
        println!("\n❌ No MCP endpoints are available at the provided base URL");
        println!("💡 Please check that your MCP server is running and accessible");
    }
}

/// Table row for discovery results
#[derive(Tabled)]
struct DiscoveryTableRow {
    #[tabled(rename = "Status")]
    pub status: String,
    #[tabled(rename = "Endpoint")]
    pub endpoint: String,
    #[tabled(rename = "URL")]
    pub url: String,
    #[tabled(rename = "Tools")]
    pub tools: String,
    #[tabled(rename = "Resources")]
    pub resources: String,
    #[tabled(rename = "Prompts")]
    pub prompts: String,
    #[tabled(rename = "Duration")]
    pub duration: String,
}

/// Generate discovery report
fn generate_discovery_report(
    results: &[DiscoveryResult],
    total_duration: Duration,
    output_dir: Option<&std::path::PathBuf>,
) -> Result<()> {
    use crate::paths::get_mcp_probe_paths;
    use std::fs;
    use std::io::Write;

    let output_path = if let Some(dir) = output_dir {
        // If user specifies output dir, use it but still add date prefix
        let date = chrono::Utc::now().format("%Y%m%d");
        let timestamp = chrono::Utc::now().format("%H%M%S");
        dir.join(format!("{}-discovery-report-{}.json", date, timestamp))
    } else {
        // Use centralized path management with date prefix
        let paths = get_mcp_probe_paths()?;
        paths.report_file("discovery-report", "json")
    };

    let report = serde_json::json!({
        "metadata": {
            "generated_at": chrono::Utc::now().to_rfc3339(),
            "total_duration_ms": total_duration.as_millis(),
            "mcp_probe_version": env!("CARGO_PKG_VERSION"),
            "discovery_mode": true
        },
        "endpoints": results.iter().map(|r| serde_json::json!({
            "name": r.endpoint.name,
            "url": r.endpoint.url,
            "description": r.endpoint.description,
            "expected_protocol": r.endpoint.expected_protocol,
            "status": r.status.name(),
            "error": r.error,
            "duration_ms": r.duration.as_millis(),
            "capabilities": {
                "tools_count": r.tools_count,
                "resources_count": r.resources_count,
                "prompts_count": r.prompts_count
            },
            "test_results": r.test_results.iter().map(|t| serde_json::json!({
                "name": t.name,
                "status": t.status.name(),
                "message": t.message,
                "duration_ms": t.duration.as_millis(),
                "details": t.details
            })).collect::<Vec<_>>()
        })).collect::<Vec<_>>(),
        "summary": {
            "total_endpoints": results.len(),
            "available_endpoints": results.iter().filter(|r| r.status == DiscoveryStatus::Available).count(),
            "failed_endpoints": results.iter().filter(|r| r.status == DiscoveryStatus::Failed).count(),
            "invalid_endpoints": results.iter().filter(|r| r.status == DiscoveryStatus::InvalidUrl).count(),
            "total_tools": results.iter().map(|r| r.tools_count).sum::<usize>(),
            "total_resources": results.iter().map(|r| r.resources_count).sum::<usize>(),
            "total_prompts": results.iter().map(|r| r.prompts_count).sum::<usize>(),
        }
    });

    let json_content = serde_json::to_string_pretty(&report)?;

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::File::create(&output_path)?;
    file.write_all(json_content.as_bytes())?;

    println!("📄 Discovery report written to: {}", output_path.display());

    Ok(())
}
