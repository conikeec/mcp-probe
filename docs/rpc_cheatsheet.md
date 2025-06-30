## MCP Server Testing with curl + jq

### Basic MCP Protocol Operations

#### Initialize Session

```bash
# Initialize MCP session and get session ID
SESSION_ID=$(curl -s -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{"tools":{"listChanged":true},"resources":{"subscribe":true,"listChanged":true},"prompts":{"listChanged":true}},"clientInfo":{"name":"example-client","version":"1.0.0"}}}' \
  http://localhost:3000 | jq -r '.result.sessionId // empty')

echo "Session ID: $SESSION_ID"
```

#### List Available Tools

```bash
# List all tools with session
curl -s -X POST -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  http://localhost:3000 | jq '.result.tools[] | .name'
```

#### Search for Specific Tools

```bash
# Search for tools containing "github" in the name
curl -s -X POST -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  http://localhost:3000 | jq '.result.tools[] | select(.name | contains("github")) | .name'

# Search for tools with specific patterns
curl -s -X POST -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  http://localhost:3000 | jq '.result.tools[] | select(.name | test("repos.*list")) | {name, description}'
```

#### Get Tool Details

```bash
# Get details for a specific tool
TOOL_NAME="Z2l0aHViX2FwaTpnaXRodWJfYXBpOjAuMS4w.github.repos/list-for-authenticated-user"
curl -s -X POST -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/get\",\"params\":{\"name\":\"$TOOL_NAME\"}}" \
  http://localhost:3000 | jq '.result'
```

#### Call Tools

```bash
# Call a tool with parameters
curl -s -X POST -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/call\",\"params\":{\"name\":\"$TOOL_NAME\",\"arguments\":{}}}" \
  http://localhost:3000 | jq '.result'
```

### Advanced jq Queries

#### Extract Tool Names by Pattern

```bash
# Get all GitHub repository tools
curl -s -X POST -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  http://localhost:3000 | jq -r '.result.tools[] | select(.name | contains("repos")) | .name'

# Get tools with their descriptions in a table format
curl -s -X POST -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  http://localhost:3000 | jq -r '.result.tools[] | "\(.name)\t\(.description // "No description")"' | column -t
```

#### Filter and Format Tool Information

```bash
# Get tools with parameter counts
curl -s -X POST -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  http://localhost:3000 | jq '.result.tools[] | {name, param_count: (.inputSchema.properties // {} | length)}'

# Get tools grouped by namespace
curl -s -X POST -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  http://localhost:3000 | jq 'group_by(.result.tools[].name | split(".")[0]) | map({namespace: .[0].name | split(".")[0], tools: map(.name)})'
```

### Resource and Prompt Operations

#### List Resources

```bash
# List all resources
curl -s -X POST -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","id":5,"method":"resources/list"}' \
  http://localhost:3000 | jq '.result.resources[] | {uri, name, description}'
```

#### List Prompts

```bash
# List all prompts
curl -s -X POST -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","id":6,"method":"prompts/list"}' \
  http://localhost:3000 | jq '.result.prompts[] | {name, description, arguments}'
```

### Error Handling and Debugging

#### Check for Errors in Responses

```bash
# Check if response contains errors
curl -s -X POST -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","id":7,"method":"invalid/method"}' \
  http://localhost:3000 | jq 'if .error then {error: .error} else {success: true} end'
```

#### Validate Session

```bash
# Test if session is still valid
curl -s -X POST -H "Content-Type: application/json" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -d '{"jsonrpc":"2.0","id":8,"method":"ping"}' \
  http://localhost:3000 | jq '.result // .error'
```

### Useful One-Liners

```bash
# Quick tool search
alias mcp-tools='curl -s -X POST -H "Content-Type: application/json" -H "Mcp-Session-Id: $SESSION_ID" -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}" http://localhost:3000 | jq -r ".result.tools[] | .name"'

# Quick tool call
mcp-call() {
  curl -s -X POST -H "Content-Type: application/json" \
    -H "Mcp-Session-Id: $SESSION_ID" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"$1\",\"arguments\":${2:-{}}}}" \
    http://localhost:3000 | jq '.result'
}

# Usage: mcp-call "tool-name" '{"param":"value"}'
```
