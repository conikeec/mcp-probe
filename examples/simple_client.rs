// Simple MCP Client Example
// This example shows how to connect to an MCP server and perform basic operations

use mcp_probe_core::{
    client::McpClient,
    transport::{TransportConfig, TransportFactory},
    messages::initialization::Implementation,
    error::McpError,
};
use std::collections::HashMap;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    println!("🚀 Starting Simple MCP Client Example");
    
    // 1. Configure the transport (change URL to match your MCP server)
    let server_url = std::env::var("MCP_SERVER_URL")
        .unwrap_or_else(|_| "http://localhost:3000/sse".to_string());
    
    println!("📡 Connecting to MCP server: {}", server_url);
    
    let transport_config = TransportConfig::http_sse(&server_url, None)?;
    
    // 2. Create transport
    let transport = TransportFactory::create_transport(transport_config).await?;
    
    // 3. Define client information
    let client_info = Implementation {
        name: "simple-mcp-client".to_string(),
        version: "1.0.0".to_string(),
        metadata: HashMap::from([
            ("description".to_string(), 
             serde_json::Value::String("A simple example MCP client".to_string())),
            ("author".to_string(), 
             serde_json::Value::String("MCP Probe SDK".to_string())),
        ]),
    };
    
    // 4. Create and initialize the MCP client
    let mut client = McpClient::new(transport, client_info);
    
    match client.initialize().await {
        Ok(_) => println!("✅ Successfully connected to MCP server"),
        Err(e) => {
            eprintln!("❌ Failed to connect: {}", e);
            return Err(e.into());
        }
    }
    
    // 5. Discover server capabilities
    println!("\n🔍 Discovering server capabilities...");
    
    // List tools
    match client.list_tools().await {
        Ok(tools_response) => {
            println!("🔧 Found {} tools:", tools_response.tools.len());
            for (i, tool) in tools_response.tools.iter().enumerate() {
                println!("  {}. {} - {}", 
                    i + 1, 
                    tool.name, 
                    tool.description.as_deref().unwrap_or("No description")
                );
            }
            
            // Try to call the first tool if available
            if let Some(first_tool) = tools_response.tools.first() {
                println!("\n🎯 Testing tool: {}", first_tool.name);
                
                // Example: try calling with empty parameters
                match client.call_tool(&first_tool.name, serde_json::Value::Null).await {
                    Ok(result) => {
                        println!("✅ Tool call successful:");
                        for content in result.content {
                            match content {
                                mcp_core::messages::tools::ToolResult::Text { text } => {
                                    println!("   📄 Text result: {}", text);
                                }
                                mcp_core::messages::tools::ToolResult::Image { data, mime_type } => {
                                    println!("   🖼️  Image result: {} bytes ({})", data.len(), mime_type);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("⚠️  Tool call failed (this may be expected if parameters are required): {}", e);
                    }
                }
            }
        }
        Err(e) => println!("❌ Failed to list tools: {}", e),
    }
    
    // List resources
    match client.list_resources().await {
        Ok(resources_response) => {
            println!("\n📁 Found {} resources:", resources_response.resources.len());
            for (i, resource) in resources_response.resources.iter().enumerate() {
                println!("  {}. {} - {}", 
                    i + 1, 
                    resource.name, 
                    resource.description.as_deref().unwrap_or("No description")
                );
            }
        }
        Err(e) => println!("❌ Failed to list resources: {}", e),
    }
    
    // List prompts
    match client.list_prompts().await {
        Ok(prompts_response) => {
            println!("\n💬 Found {} prompts:", prompts_response.prompts.len());
            for (i, prompt) in prompts_response.prompts.iter().enumerate() {
                println!("  {}. {} - {}", 
                    i + 1, 
                    prompt.name, 
                    prompt.description.as_deref().unwrap_or("No description")
                );
            }
        }
        Err(e) => println!("❌ Failed to list prompts: {}", e),
    }
    
    println!("\n🎉 Example completed successfully!");
    println!("💡 Try modifying this example to:");
    println!("   • Call specific tools with parameters");
    println!("   • Read specific resources");
    println!("   • Get prompts with arguments");
    println!("   • Handle errors more specifically");
    
    Ok(())
}

// Helper function to demonstrate error handling
async fn try_connect_with_retry(
    server_url: &str, 
    max_retries: u32
) -> Result<McpClient, McpError> {
    let mut attempts = 0;
    
    loop {
        attempts += 1;
        println!("🔄 Connection attempt {} of {}", attempts, max_retries + 1);
        
        let transport_config = TransportConfig::http_sse(server_url, None)?;
        let transport = match TransportFactory::create_transport(transport_config).await {
            Ok(transport) => transport,
            Err(e) if attempts <= max_retries => {
                println!("⚠️  Connection failed: {}. Retrying in 2 seconds...", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                continue;
            }
            Err(e) => return Err(e),
        };
        
        let client_info = Implementation {
            name: "simple-mcp-client".to_string(),
            version: "1.0.0".to_string(),
            metadata: HashMap::new(),
        };
        
        let mut client = McpClient::new(transport, client_info);
        
        match client.initialize().await {
            Ok(_) => {
                println!("✅ Connected successfully on attempt {}", attempts);
                return Ok(client);
            }
            Err(e) if attempts <= max_retries => {
                println!("⚠️  Initialization failed: {}. Retrying in 2 seconds...", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
} 