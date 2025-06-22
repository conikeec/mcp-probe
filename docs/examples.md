---
layout: page
title: Examples and Use Cases
permalink: /examples/
---

# Examples and Use Cases

This page provides real-world examples of using MCP Probe to debug, test, and validate MCP servers across different scenarios.

## 🎯 Quick Start Examples

### Example 1: Debugging Playwright MCP Server

The Playwright MCP server provides browser automation capabilities. Let's debug it:

```bash
# Basic debugging (non-interactive)
mcp-probe debug --non-interactive --stdio npx @playwright/mcp@latest

# Interactive debugging with TUI
mcp-probe debug --stdio npx @playwright/mcp@latest

# Expected output:
# ✅ Connected to MCP server successfully!
# 📋 Tools (25): browser_navigate, browser_click, browser_type...
# 📁 Resources (0):
# 💬 Prompts (0):
```

### Example 2: Testing a Python MCP Server

Let's say you have a Python MCP server for file operations:

```bash
# Debug a Python MCP server
mcp-probe debug --stdio python --args file_server.py --working-dir ./my-mcp-server

# Run comprehensive tests
mcp-probe test --stdio python file_server.py --report --output-dir ./test-results

# Generate capability documentation
mcp-probe export --stdio python file_server.py --format markdown --output file-server-docs.md
```

### Example 3: HTTP+SSE Server Testing

For HTTP-based MCP servers:

```bash
# Debug HTTP+SSE server
mcp-probe debug --http-sse http://localhost:3000/sse

# With authentication
mcp-probe debug --http-sse https://api.example.com/mcp \
  --auth-header "Bearer your-api-token"

# Export capabilities with authentication
mcp-probe export --http-sse https://api.example.com/mcp \
  --auth-header "Bearer your-api-token" \
  --format json --output api-capabilities.json
```

## 🏢 Professional Use Cases

### Use Case 1: CI/CD Pipeline Integration

**Scenario**: Automatically test your MCP server in CI/CD before deployment.

```bash
#!/bin/bash
# ci-test-mcp-server.sh

set -e

echo "🧪 Testing MCP Server in CI..."

# Run comprehensive tests
mcp-probe test --stdio python server.py \
  --report \
  --output-dir ./ci-reports \
  --fail-fast \
  --timeout 120

# Validate protocol compliance
mcp-probe validate --stdio python server.py \
  --rules schema-validation,tool-parameters \
  --severity error \
  --report ./ci-reports/validation-report.md

# Export capabilities for documentation
mcp-probe export --stdio python server.py \
  --format html \
  --output ./ci-reports/capabilities.html

echo "✅ All MCP tests passed!"
```

**GitHub Actions Integration**:

```yaml
# .github/workflows/test-mcp.yml
name: Test MCP Server

on: [push, pull_request]

jobs:
  test-mcp:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install MCP Probe
        run: |
          curl -fsSL https://raw.githubusercontent.com/conikeec/mcp-probe/main/install.sh | bash
          echo "$HOME/.local/bin" >> $GITHUB_PATH
      
      - name: Test MCP Server
        run: ./ci-test-mcp-server.sh
      
      - name: Upload Test Reports
        uses: actions/upload-artifact@v3
        with:
          name: mcp-test-reports
          path: ./ci-reports/
```

### Use Case 2: Development Workflow

**Scenario**: Daily development and testing of an MCP server.

```bash
# Development script - dev-test.sh
#!/bin/bash

echo "🔧 MCP Development Testing"

# Quick smoke test
echo "Running smoke test..."
mcp-probe debug --non-interactive --stdio python server.py --timeout 10

# Interactive debugging for development
echo "Launching interactive debugger..."
mcp-probe debug --stdio python server.py \
  --save-session "dev-session-$(date +%Y%m%d)" \
  --show-raw

# Generate updated documentation
echo "Updating documentation..."
mcp-probe export --stdio python server.py \
  --format markdown \
  --output ./docs/api-capabilities.md \
  --include-timing
```

### Use Case 3: Multi-Environment Testing

**Scenario**: Test the same MCP server across different environments.

```bash
#!/bin/bash
# multi-env-test.sh

ENVIRONMENTS=("development" "staging" "production")
SERVERS=("http://dev-api.example.com" "http://staging-api.example.com" "http://api.example.com")

for i in "${!ENVIRONMENTS[@]}"; do
  ENV=${ENVIRONMENTS[$i]}
  SERVER=${SERVERS[$i]}
  
  echo "🌍 Testing $ENV environment..."
  
  # Test the environment
  mcp-probe test --http-sse "$SERVER/mcp" \
    --auth-header "Bearer $MCP_TOKEN" \
    --report \
    --output-dir "./reports/$ENV" \
    --timeout 60
  
  # Export capabilities
  mcp-probe export --http-sse "$SERVER/mcp" \
    --auth-header "Bearer $MCP_TOKEN" \
    --format json \
    --output "./reports/$ENV/capabilities.json"
done

echo "✅ Multi-environment testing complete!"
```

## 🔧 Advanced Examples

### Example 4: Custom Configuration

Create a configuration file for your team's standard setup:

```toml
# ~/.mcp-probe/config/team-config.toml
[defaults]
transport = "stdio"
timeout = 45
max_retries = 3
output_format = "pretty"

[stdio]
working_dir = "/workspace/mcp-servers"
environment = { 
  "DEBUG" = "1", 
  "LOG_LEVEL" = "info",
  "MCP_SERVER_CONFIG" = "/workspace/config/server.json"
}

[http]
headers = { 
  "User-Agent" = "mcp-probe/0.2.4 (team-config)",
  "X-Team" = "backend-team"
}
timeout = 60

[validation]
rules = ["schema-validation", "tool-parameters", "resource-uris"]
severity = "warning"

[output]
include_timing = true
include_raw = false
auto_save_sessions = true
```

Use the configuration:

```bash
# Use team configuration
mcp-probe debug --stdio python server.py --config ~/.mcp-probe/config/team-config.toml

# Override specific settings
mcp-probe debug --stdio python server.py \
  --config ~/.mcp-probe/config/team-config.toml \
  --timeout 120 \
  --show-raw
```

### Example 5: Automated Capability Comparison

**Scenario**: Compare capabilities between different versions of your MCP server.

```bash
#!/bin/bash
# compare-versions.sh

VERSION_1="v1.0.0"
VERSION_2="v1.1.0"

echo "📊 Comparing MCP server versions $VERSION_1 vs $VERSION_2"

# Export capabilities for version 1
git checkout $VERSION_1
mcp-probe export --stdio python server.py \
  --format json \
  --output "./comparisons/capabilities-$VERSION_1.json"

# Export capabilities for version 2  
git checkout $VERSION_2
mcp-probe export --stdio python server.py \
  --format json \
  --output "./comparisons/capabilities-$VERSION_2.json"

# Compare using jq (or your preferred tool)
echo "🔍 Capability differences:"
jq -s 'def diff(a; b): 
  {
    added: (b.tools // [] | map(.name)) - (a.tools // [] | map(.name)),
    removed: (a.tools // [] | map(.name)) - (b.tools // [] | map(.name)),
    modified: [
      (a.tools // []) as $a_tools |
      (b.tools // []) as $b_tools |
      $a_tools[] | select(.name as $name | $b_tools[] | select(.name == $name and . != $a_tools[] | select(.name == $name))) | .name
    ]
  };
  diff(.[0]; .[1])' \
  "./comparisons/capabilities-$VERSION_1.json" \
  "./comparisons/capabilities-$VERSION_2.json"
```

### Example 6: Load Testing MCP Server

**Scenario**: Test how your MCP server handles multiple concurrent connections.

```bash
#!/bin/bash
# load-test.sh

CONCURRENT_SESSIONS=5
TEST_DURATION=60

echo "⚡ Load testing MCP server with $CONCURRENT_SESSIONS concurrent sessions"

# Function to run a single test session
run_test_session() {
  local session_id=$1
  echo "Starting session $session_id..."
  
  mcp-probe test --stdio python server.py \
    --timeout $TEST_DURATION \
    --output-dir "./load-test/session-$session_id" \
    --non-interactive \
    --report 2>&1 | tee "./load-test/session-$session_id.log"
}

# Create output directory
mkdir -p ./load-test

# Run concurrent sessions
for i in $(seq 1 $CONCURRENT_SESSIONS); do
  run_test_session $i &
done

# Wait for all sessions to complete
wait

echo "📊 Analyzing load test results..."

# Aggregate results
echo "Session Results:" > ./load-test/summary.txt
for i in $(seq 1 $CONCURRENT_SESSIONS); do
  if [ -f "./load-test/session-$i.log" ]; then
    echo "Session $i: $(grep -o "✅.*successfully" "./load-test/session-$i.log" | head -1)" >> ./load-test/summary.txt
  fi
done

cat ./load-test/summary.txt
```

## 🎨 Real-World Integration Examples

### Example 7: Documentation Generation

**Scenario**: Generate beautiful API documentation from your MCP server.

```bash
#!/bin/bash
# generate-docs.sh

echo "📚 Generating MCP Server Documentation"

# Create docs directory structure
mkdir -p ./generated-docs/{json,markdown,html}

# Export in multiple formats
mcp-probe export --stdio python server.py --format json --output ./generated-docs/json/api.json
mcp-probe export --stdio python server.py --format markdown --output ./generated-docs/markdown/api.md  
mcp-probe export --stdio python server.py --format html --output ./generated-docs/html/api.html

# Create index page
cat > ./generated-docs/index.html << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>MCP Server API Documentation</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .format-links { margin: 20px 0; }
        .format-links a { margin-right: 15px; padding: 8px 16px; 
                         background: #007bff; color: white; text-decoration: none; border-radius: 4px; }
    </style>
</head>
<body>
    <h1>MCP Server API Documentation</h1>
    <p>Generated on $(date)</p>
    
    <div class="format-links">
        <a href="html/api.html">HTML Documentation</a>
        <a href="markdown/api.md">Markdown</a>
        <a href="json/api.json">JSON Schema</a>
    </div>
</body>
</html>
EOF

echo "✅ Documentation generated in ./generated-docs/"
```

### Example 8: Monitoring and Alerting

**Scenario**: Set up monitoring for your MCP server health.

```bash
#!/bin/bash
# mcp-health-check.sh

SERVER_URL="http://localhost:3000/sse"
WEBHOOK_URL="https://hooks.slack.com/your/webhook/url"

echo "🏥 Checking MCP server health..."

# Run health check
if mcp-probe debug --non-interactive --http-sse "$SERVER_URL" --timeout 30; then
  echo "✅ MCP server is healthy"
  exit 0
else
  echo "❌ MCP server health check failed"
  
  # Send alert to Slack
  curl -X POST -H 'Content-type: application/json' \
    --data '{"text":"🚨 MCP Server Health Check Failed"}' \
    "$WEBHOOK_URL"
  
  exit 1
fi
```

**Cron job setup**:

```bash
# Add to crontab for every 5 minutes
# crontab -e
*/5 * * * * /path/to/mcp-health-check.sh >> /var/log/mcp-health.log 2>&1
```

## 🔬 Debugging Scenarios

### Scenario 1: Connection Issues

```bash
# Debug connection problems with verbose logging
RUST_LOG=debug mcp-probe debug --stdio python server.py --show-raw --timeout 60

# Check specific transport issues
mcp-probe debug --http-sse http://localhost:3000/sse --show-raw 2>&1 | grep -E "(ERROR|WARN|connection|timeout)"

# Test with different timeout values
for timeout in 10 30 60 120; do
  echo "Testing with ${timeout}s timeout..."
  if timeout $((timeout + 5)) mcp-probe debug --non-interactive --stdio python server.py --timeout $timeout; then
    echo "✅ Success with ${timeout}s timeout"
    break
  else
    echo "❌ Failed with ${timeout}s timeout"
  fi
done
```

### Scenario 2: Protocol Validation Issues

```bash
# Comprehensive validation with detailed output
mcp-probe validate --stdio python server.py \
  --rules schema-validation,tool-parameters,resource-uris,prompt-validation \
  --severity info \
  --report validation-detailed.md

# Focus on specific validation aspects
mcp-probe validate --stdio python server.py \
  --rules tool-parameters \
  --severity error \
  --report tool-validation.md

# Compare validation results between versions
diff <(mcp-probe validate --stdio python server-v1.py --rules all) \
     <(mcp-probe validate --stdio python server-v2.py --rules all)
```

## 🔗 Integration Examples

### Example 9: Docker Integration

```dockerfile
# Dockerfile for MCP testing environment
FROM rust:1.75

# Install MCP Probe
RUN cargo install mcp-cli

# Install Python for testing
RUN apt-get update && apt-get install -y python3 python3-pip

# Copy test scripts
COPY test-scripts/ /usr/local/bin/
RUN chmod +x /usr/local/bin/*.sh

# Set working directory
WORKDIR /workspace

# Default command
CMD ["mcp-probe", "--help"]
```

**Usage**:

```bash
# Build testing image
docker build -t mcp-tester .

# Run tests in container
docker run --rm -v $(pwd):/workspace mcp-tester \
  mcp-probe test --stdio python server.py --report
```

### Example 10: VS Code Integration

Create a VS Code task for MCP debugging:

```json
// .vscode/tasks.json
{
    "version": "2.0.0",
    "tasks": [
        {
            "label": "Debug MCP Server",
            "type": "shell",
            "command": "mcp-probe",
            "args": [
                "debug",
                "--stdio",
                "python",
                "${workspaceFolder}/server.py"
            ],
            "group": "test",
            "presentation": {
                "echo": true,
                "reveal": "always",
                "focus": false,
                "panel": "new"
            },
            "problemMatcher": []
        },
        {
            "label": "Test MCP Server",
            "type": "shell",
            "command": "mcp-probe",
            "args": [
                "test",
                "--stdio",
                "python",
                "${workspaceFolder}/server.py",
                "--report",
                "--output-dir",
                "${workspaceFolder}/test-reports"
            ],
            "group": "test"
        }
    ]
}
```

## 📊 Performance Testing

### Example 11: Benchmarking MCP Operations

```bash
#!/bin/bash
# benchmark-mcp.sh

echo "⏱️ Benchmarking MCP server performance"

# Test tool listing performance
echo "Testing tool listing..."
time mcp-probe debug --non-interactive --stdio python server.py > /dev/null

# Test with different message sizes
for size in small medium large; do
  echo "Testing with $size payload..."
  MCP_TEST_SIZE=$size time mcp-probe test --stdio python server.py --timeout 300
done

# Memory usage testing
echo "Memory usage analysis..."
/usr/bin/time -v mcp-probe debug --stdio python server.py --timeout 60 2>&1 | grep -E "(Maximum resident|User time|System time)"
```

---

## 🎯 Next Steps

These examples should give you a solid foundation for using MCP Probe in various scenarios. For more advanced usage:

1. **[Read the Documentation](documentation.html)** - Learn about all available features
2. **[API Reference](api-reference.html)** - Detailed technical documentation  
3. **[Contributing](contributing.html)** - Help improve MCP Probe

**Have a specific use case not covered here?** [Open an issue](https://github.com/conikeec/mcp-probe/issues) and we'll help you figure it out! 