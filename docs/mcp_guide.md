# Model Context Protocol (MCP) - Complete Implementation Guide

## Corrected & Updated for Specification 2025-06-18

---

# Part 1: MCP Initialization and Transport Fundamentals

## Table of Contents

1. [Protocol Overview](#protocol-overview)
2. [Initialization Process](#initialization-process)
3. [Transport-Specific Initialization](#transport-specific-initialization)
4. [Capability Negotiation](#capability-negotiation)
5. [Core Features](#core-features)
6. [Advanced Features](#advanced-features)
7. [Complete Feature Reference](#complete-feature-reference)

---

## Protocol Overview

The Model Context Protocol (MCP) is a standardized protocol that enables secure, controlled interactions between AI models and external systems. Built on JSON-RPC 2.0, MCP establishes stateful connections with explicit capability negotiation before any feature usage.

### Key Principles

- **Initialization First**: All connections must begin with explicit initialization
- **Capability-Based**: Features are negotiated during initialization
- **Transport Agnostic**: Supports multiple transport mechanisms
- **Security First**: All interactions are controlled and auditable

### Current Transport Support (2025-06-18)

| Transport           | Status     | Use Case               | Initialization Pattern   |
| ------------------- | ---------- | ---------------------- | ------------------------ |
| **STDIO**           | ✅ Current | Local processes        | Direct connection        |
| **Streamable HTTP** | ✅ Current | Remote servers         | HTTP-based with sessions |
| **HTTP+SSE**        | ⚠️ Legacy  | Backward compatibility | Dual endpoint pattern    |

---

## Initialization Process

**Every MCP connection MUST begin with initialization.** This is non-negotiable and happens before any other protocol operations.

### Universal Initialization Flow

```
┌─────────┐                 ┌─────────┐
│  Client │                 │  Server │
└────┬────┘                 └────┬────┘
     │                           │
     │ 1. INITIALIZE REQUEST     │
     │ (Transport-specific)      │
     ├──────────────────────────>│
     │                           │
     │                     ┌─────┴─────┐
     │                     │ Process   │
     │                     │ capabilities,│
     │                     │ create     │
     │                     │ session    │
     │                     └─────┬─────┘
     │                           │
     │ 2. INITIALIZE RESPONSE    │
     │ (With server capabilities)│
     │<──────────────────────────┤
     │                           │
     │ 3. CONNECTION READY       │
     │ (Can now use negotiated   │
     │  capabilities)            │
     │                           │
```

### Standard Initialize Request Format

```json
{
  "jsonrpc": "2.0",
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-06-18",
    "capabilities": {
      "tools": {},
      "resources": {},
      "prompts": {},
      "notifications": { "supported": true }, // REQUIRED for list change notifications
      "pagination": { "supported": true },
      "completion": { "supported": true },
      "elicitation": { "supported": true }
    },
    "clientInfo": {
      "name": "ExampleClient",
      "version": "1.0.0"
    }
  },
  "id": "init-1"
}
```

### Standard Initialize Response Format

```json
{
  "jsonrpc": "2.0",
  "result": {
    "protocolVersion": "2025-06-18",
    "capabilities": {
      "tools": { "listChanged": true }, // Server will notify on tool changes
      "resources": {
        "subscribe": true,
        "listChanged": true // Server will notify on resource changes
      },
      "prompts": { "listChanged": true }, // Server will notify on prompt changes
      "notifications": { "progress": true }, // Server can send progress updates
      "pagination": { "cursor": true },
      "completion": { "arguments": true },
      "elicitation": { "supported": true }
    },
    "serverInfo": {
      "name": "ExampleServer",
      "version": "2.1.0"
    }
  },
  "id": "init-1"
}
```

---

## Transport-Specific Initialization

The initialization **content** is identical across transports, but the **delivery mechanism** differs significantly.

### 1. STDIO Transport Initialization

**Most Common**: Used for local MCP servers

```
┌─────────┐    stdin/stdout    ┌─────────┐
│  Client │<══════════════════>│  Server │
└────┬────┘    (JSON-RPC)     └────┬────┘
     │                             │
     │ Initialize Request          │
     │ (newline-delimited JSON)    │
     ├────────────────────────────>│
     │                             │
     │ Initialize Response         │
     │ (newline-delimited JSON)    │
     │<────────────────────────────┤
     │                             │
     │ ✓ Bidirectional Ready       │
```

**Implementation Example:**

```javascript
// Server (STDIO)
const transport = new StdioServerTransport();
await mcpServer.connect(transport);
// Initialization happens automatically on first message
```

### 2. Streamable HTTP Transport Initialization

**Current Standard**: Single endpoint, session-based

```
┌─────────┐                 ┌─────────┐
│  Client │                 │  Server │
└────┬────┘                 └────┬────┘
     │                           │
     │ POST /mcp                 │
     │ Content-Type: application/json
     │ Accept: application/json, text/event-stream
     │ Body: InitializeRequest   │
     ├──────────────────────────>│
     │                           │
     │                     ┌─────┴─────┐
     │                     │ Generate  │
     │                     │ session   │
     │                     │ ID        │
     │                     └─────┬─────┘
     │                           │
     │ HTTP 200 OK               │
     │ mcp-session-id: sess-abc123
     │ Content-Type: application/json
     │ Body: InitializeResult    │
     │<──────────────────────────┤
     │                           │
     │ ✓ Session Established     │
     │   All future requests     │
     │   include session header  │
```

**Key Characteristics:**

- **Single endpoint** (can be any path, `/mcp` is example)
- **Session management** via `mcp-session-id` header
- **Dynamic response type** (JSON or SSE based on server choice)
- **Stateless capable** (session ID optional for simple servers)

**Implementation Example:**

```javascript
// Client
const response = await fetch("/mcp", {
  method: "POST",
  headers: {
    "Content-Type": "application/json",
    Accept: "application/json, text/event-stream",
  },
  body: JSON.stringify(initializeRequest),
});

const sessionId = response.headers.get("mcp-session-id");
// Store sessionId for future requests
```

### 3. Legacy HTTP+SSE Transport Initialization

**Deprecated but Still Supported**: Dual endpoint pattern

```
┌─────────┐                 ┌─────────┐
│  Client │                 │  Server │
└────┬────┘                 └────┬────┘
     │                           │
     │ 1. GET /sse               │
     │ Accept: text/event-stream │
     ├──────────────────────────>│
     │                           │
     │ HTTP 200 OK               │
     │ Content-Type: text/event-stream
     │ SSE: endpoint event       │
     │ data: {"endpoint":"/message"}
     │<──────────────────────────┤
     │                           │
     │ 2. POST /message          │
     │ ?sessionId=sess-xyz       │
     │ Body: InitializeRequest   │
     │ ──────────────────────────>│
     │                           │
     │ HTTP 200 OK               │
     │ Body: InitializeResult    │
     │<──────────────────────────┤
     │                           │
     │ ✓ Dual Connection Ready   │
     │   SSE: Server→Client      │
     │   POST: Client→Server     │
```

**Key Characteristics:**

- **Dual endpoints**: `/sse` for server→client, `/message` for client→server
- **Session management** via URL query parameters
- **Persistent SSE connection** required
- **Complex connection management**

**Implementation Example:**

```javascript
// Client (Legacy)
// 1. Establish SSE connection
const eventSource = new EventSource("/sse");
let messageEndpoint;
let sessionId;

eventSource.addEventListener("endpoint", (event) => {
  const data = JSON.parse(event.data);
  messageEndpoint = data.endpoint;
  sessionId = data.sessionId;

  // 2. Send initialization
  fetch(`${messageEndpoint}?sessionId=${sessionId}`, {
    method: "POST",
    body: JSON.stringify(initializeRequest),
  });
});
```

---

## Transport Comparison: Initialization Differences

| Aspect                      | STDIO               | Streamable HTTP           | Legacy HTTP+SSE         |
| --------------------------- | ------------------- | ------------------------- | ----------------------- |
| **Connection Setup**        | Process spawn       | Single HTTP endpoint      | Dual HTTP endpoints     |
| **Session Management**      | N/A (process-bound) | `mcp-session-id` header   | `sessionId` query param |
| **Initialization Delivery** | Direct JSON-RPC     | HTTP POST to endpoint     | HTTP POST to `/message` |
| **Server→Client Setup**     | Bidirectional stdio | Optional SSE upgrade      | Required SSE at `/sse`  |
| **Endpoint Discovery**      | N/A                 | Fixed endpoint            | SSE endpoint event      |
| **Connection Persistence**  | Process lifetime    | Per-request or session    | Persistent SSE required |
| **Complexity**              | Low                 | Medium                    | High                    |
| **Scalability**             | Single process      | High (stateless possible) | Limited (persistent)    |

---

## Capability Negotiation

After successful initialization, both client and server know exactly which features are available for the session.

### Capability Categories

#### 1. **Core Capabilities** (Commonly Implemented)

```json
{
  "tools": {
    "listChanged": true // Server can notify when tool list changes
  },
  "resources": {
    "subscribe": true, // Client can subscribe to resource changes
    "listChanged": true // Server can notify when resource list changes
  },
  "prompts": {
    "listChanged": true // Server can notify when prompt list changes
  }
}
```

#### 2. **Notification Capabilities** (For Real-time Updates)

```json
{
  "notifications": {
    "supported": true, // Client can receive notifications (REQUIRED for list changes)
    "progress": true, // Server can send progress notifications
    "status": true // Server can send status updates
  }
}
```

#### 3. **Advanced Capabilities** (Optional)

```json
{
  "pagination": {
    "cursor": true // Server supports cursor-based pagination
  },
  "completion": {
    "arguments": true // Server can suggest argument completions
  },
  "elicitation": {
    "supported": true // Server supports progressive disclosure
  },
  "sampling": {
    "supported": true // Client can handle sampling requests
  }
}
```

### Capability Validation Rules

1. **Client declares what it can handle**
2. **Server declares what it provides**
3. **Intersection determines available features**
4. **Attempts to use non-negotiated features MUST result in errors**

#### Examples:

**Notification Support:**

```json
// Client declares
{"notifications": {"supported": true}}

// Server declares
{"notifications": {"progress": true, "status": true}}

// Result: Server can send progress and status notifications
```

**List Change Notifications:**

```json
// Client declares (REQUIRED)
{"notifications": {"supported": true}}

// Server declares
{
  "tools": {"listChanged": true},
  "resources": {"listChanged": true}
}

// Result: Server can send tool and resource list change notifications
// Client MUST automatically refresh lists when notified
```

**Elicitation Support:**

```json
// Client declares
{"elicitation": {"supported": true}}

// Server declares
{"elicitation": {"supported": true}}

// Result: Client can use */elicit methods for progressive disclosure
```

#### Critical Dependencies:

- **List Change Notifications** require `notifications.supported: true` on client
- **Progress Updates** require `notifications.progress: true` on server
- **Subscriptions** require `notifications.supported: true` on client
- **Elicitation** can work independently but benefits from proper capability declaration

---

## Core Features

Now that initialization and capabilities are established, here are the core MCP features:

### 1. Tools

**Purpose**: Enable models to perform actions in external systems
**Operations**: `tools/list` (discovery) + `tools/call` (execution)

#### Discovery Flow:

```json
{
  "method": "tools/list",
  "result": {
    "tools": [
      {
        "name": "file_read",
        "description": "Read file contents",
        "inputSchema": {
          "type": "object",
          "properties": {
            "path": { "type": "string" }
          },
          "required": ["path"]
        }
      }
    ]
  }
}
```

#### Execution Flow:

```json
{
  "method": "tools/call",
  "params": {
    "name": "file_read",
    "arguments": {
      "path": "/home/user/document.txt"
    }
  }
}
```

### 2. Resources

**Purpose**: Provide access to data and content
**Operations**: `resources/list` (discovery) + `resources/read` (access)

#### Discovery Flow:

```json
{
  "method": "resources/list",
  "result": {
    "resources": [
      {
        "uri": "file://./documents/readme.md",
        "name": "Project README",
        "description": "Project documentation",
        "mimeType": "text/markdown"
      }
    ]
  }
}
```

### 3. Prompts

**Purpose**: Provide reusable prompt templates
**Operations**: `prompts/list` (discovery) + `prompts/get` (retrieval)

#### Discovery Flow:

```json
{
  "method": "prompts/list",
  "result": {
    "prompts": [
      {
        "name": "code_review",
        "description": "Perform code review analysis",
        "arguments": [
          {
            "name": "language",
            "description": "Programming language",
            "required": true
          }
        ]
      }
    ]
  }
}
```

---

## Advanced Features

### 4. List Change Notifications

**Purpose**: Automatic updates when server capabilities change
**Critical Feature**: Keeps clients synchronized with dynamic server capabilities

#### How List Change Notifications Work:

**1. Capability Declaration (Server):**

```json
{
  "capabilities": {
    "tools": {
      "listChanged": true // Server will notify when tools change
    },
    "resources": {
      "listChanged": true // Server will notify when resources change
    },
    "prompts": {
      "listChanged": true // Server will notify when prompts change
    }
  }
}
```

**2. Capability Declaration (Client):**

```json
{
  "capabilities": {
    "notifications": {
      "supported": true // REQUIRED: Client can handle notifications
    }
  }
}
```

**3. Notification Types:**

##### Tools List Changes:

```json
// Sent when tools are added, removed, or modified
{
  "method": "notifications/tools/list_changed",
  "params": {}
}
```

##### Resources List Changes:

```json
// Sent when resources are added, removed, or modified
{
  "method": "notifications/resources/list_changed",
  "params": {}
}
```

##### Prompts List Changes:

```json
// Sent when prompts are added, removed, or modified
{
  "method": "notifications/prompts/list_changed",
  "params": {}
}
```

**4. Client Response Pattern:**

```json
// Client automatically refreshes when notified
{
  "method": "tools/list", // or resources/list, prompts/list
  "id": "refresh-after-change"
}
```

#### Use Cases for List Change Notifications:

- **Dynamic Tool Loading**: Server adds new tools based on user actions
- **Permission Changes**: User gains/loses access to certain capabilities
- **Plugin Management**: Server loads/unloads plugins dynamically
- **Resource Discovery**: New files/databases become available
- **Template Updates**: Prompt templates are modified or added

#### Implementation Example:

```javascript
// Server-side: Dynamic capability management
class DynamicMCPServer {
  async addTool(toolDefinition) {
    this.tools.set(toolDefinition.name, toolDefinition);

    // Automatically notify all clients
    await this.notifyClients("notifications/tools/list_changed", {});
  }

  async addResource(resourceDefinition) {
    this.resources.set(resourceDefinition.uri, resourceDefinition);

    // Automatically notify all clients
    await this.notifyClients("notifications/resources/list_changed", {});
  }
}

// Client-side: Automatic refresh handling
client.onNotification("notifications/tools/list_changed", async () => {
  const updatedTools = await client.request({ method: "tools/list" });
  updateUI(updatedTools.tools);
});
```

### 5. Subscriptions

**Purpose**: Enable real-time updates from server to client for specific events
**Requires**: Bidirectional communication capability

#### Subscription Flow:

```json
// 1. Client subscribes
{
  "method": "notifications/subscribe",
  "params": {
    "method": "resources/updated"
  }
}

// 2. Server acknowledges
{
  "result": {"subscribed": true}
}

// 3. Server sends updates (when events occur)
{
  "method": "notifications/resources/updated",
  "params": {
    "uri": "file://./documents/readme.md",
    "type": "modified"
  }
}
```

### 6. Progress Notifications

**Purpose**: Track long-running operations with real-time updates
**Transport Consideration**: Streamable HTTP can upgrade to SSE for streaming

#### Progress Flow:

```json
// Long-running tool call triggers progress
{
  "method": "notifications/progress",
  "params": {
    "progressToken": "operation_123",
    "progress": 0.75,
    "total": 100,
    "message": "Processing files 75 of 100..."
  }
}
```

### 7. Completion Support

**Purpose**: Help users complete arguments and parameters

#### Completion Flow:

```json
// Client requests completion
{
  "method": "completion/complete",
  "params": {
    "ref": {
      "type": "ref/tool",
      "name": "file_read"
    },
    "argument": {
      "name": "path",
      "value": "/home/user/doc"
    }
  }
}

// Server provides suggestions
{
  "result": {
    "completion": {
      "values": [
        "/home/user/documents/",
        "/home/user/docs/"
      ],
      "total": 2,
      "hasMore": false
    }
  }
}
```

### 8. Elicitation

**Purpose**: Progressive disclosure of information to reduce cognitive load
**How it works**: Servers provide minimal initial responses with options to request more detail

#### Elicitation Pattern:

Elicitation enables servers to provide concise initial responses while offering users the ability to request additional information on demand. This reduces information overload and improves user experience.

#### Initial Response with Elicitation:

```json
{
  "method": "resources/list",
  "result": {
    "resources": [
      {
        "uri": "collection://large-dataset",
        "name": "Customer Database",
        "description": "Contains 50,000 customer records"
      }
    ],
    "_meta": {
      "elicitation": {
        "available": true,
        "methods": ["expand", "filter", "search", "sample"]
      }
    }
  }
}
```

#### Client Elicitation Request:

```json
{
  "method": "resources/elicit",
  "params": {
    "uri": "collection://large-dataset",
    "elicitationMethod": "expand",
    "context": {
      "maxItems": 100,
      "filter": "status:active",
      "sortBy": "lastActivity"
    }
  }
}
```

#### Elicited Response:

```json
{
  "result": {
    "resources": [
      {
        "uri": "customer://001",
        "name": "Alice Johnson",
        "description": "Premium customer, last active today",
        "mimeType": "application/json"
      },
      {
        "uri": "customer://002",
        "name": "Bob Smith",
        "description": "Enterprise customer, last active yesterday",
        "mimeType": "application/json"
      }
      // ... up to 100 items as requested
    ],
    "_meta": {
      "elicitation": {
        "totalAvailable": 50000,
        "filtered": 12500,
        "returned": 100,
        "hasMore": true,
        "nextElicitation": {
          "methods": ["expand", "nextPage", "refine"]
        }
      }
    }
  }
}
```

#### Elicitation Use Cases:

1. **Large Dataset Preview**: Show summary first, expand on request
2. **Contextual Filtering**: Apply user-specific filters to reduce noise
3. **Progressive Detail**: Start with overview, drill down as needed
4. **Interactive Exploration**: Guide users through complex data structures

#### Elicitation Flow Diagram:

```
┌─────────┐                 ┌─────────┐
│ Client  │                 │ Server  │
└────┬────┘                 └────┬────┘
     │                           │
     │ 1. Initial Request        │
     │ resources/list            │
     ├──────────────────────────>│
     │                           │
     │ 2. Minimal Response       │
     │ + Elicitation Metadata    │
     │<──────────────────────────┤
     │                           │
     │ 3. User Chooses to        │
     │    Expand Specific Area   │
     │                           │
     │ 4. Elicit Request         │
     │ resources/elicit          │
     ├──────────────────────────>│
     │                           │
     │ 5. Detailed Response      │
     │ + Further Elicitation     │
     │   Options                 │
     │<──────────────────────────┤
     │                           │
```

#### Implementation Example:

```javascript
// Server-side elicitation handler
server.setRequestHandler(ResourcesElicitRequestSchema, async (request) => {
  const { uri, elicitationMethod, context } = request.params;

  switch (elicitationMethod) {
    case "expand":
      return await expandResourceDetails(uri, context);
    case "filter":
      return await filterResources(uri, context.filter);
    case "sample":
      return await sampleResources(uri, context.sampleSize || 10);
    default:
      throw new Error(`Unsupported elicitation method: ${elicitationMethod}`);
  }
});

async function expandResourceDetails(uri, context) {
  const resources = await getResourcesWithDetails(uri, {
    limit: context.maxItems || 50,
    filter: context.filter,
    sortBy: context.sortBy,
  });

  return {
    resources,
    _meta: {
      elicitation: {
        totalAvailable: await getTotalCount(uri),
        returned: resources.length,
        hasMore: resources.length === (context.maxItems || 50),
        nextElicitation: {
          methods: ["expand", "refine", "export"],
        },
      },
    },
  };
}
```

---

## Complete Feature Reference

| Feature                   | Discovery Method          | Operational Method                     | Purpose                  | Capability Required                                                   | Transport Notes        |
| ------------------------- | ------------------------- | -------------------------------------- | ------------------------ | --------------------------------------------------------------------- | ---------------------- |
| **Tools**                 | `tools/list`              | `tools/call`                           | Execute actions          | `tools: {}`                                                           | All transports         |
| **Resources**             | `resources/list`          | `resources/read`                       | Access data              | `resources: {}`                                                       | All transports         |
| **Prompts**               | `prompts/list`            | `prompts/get`                          | Template retrieval       | `prompts: {}`                                                         | All transports         |
| **Tool List Changes**     | _Automatic_               | `notifications/tools/list_changed`     | Dynamic tool updates     | `tools: {listChanged: true}` + `notifications: {supported: true}`     | Requires bidirectional |
| **Resource List Changes** | _Automatic_               | `notifications/resources/list_changed` | Dynamic resource updates | `resources: {listChanged: true}` + `notifications: {supported: true}` | Requires bidirectional |
| **Prompt List Changes**   | _Automatic_               | `notifications/prompts/list_changed`   | Dynamic prompt updates   | `prompts: {listChanged: true}` + `notifications: {supported: true}`   | Requires bidirectional |
| **Subscriptions**         | `notifications/subscribe` | `notifications/*`                      | Real-time updates        | `notifications: {supported: true}`                                    | Requires bidirectional |
| **Progress**              | _Built-in_                | `notifications/progress`               | Operation tracking       | `notifications: {progress: true}`                                     | SSE recommended        |
| **Completion**            | _Built-in_                | `completion/complete`                  | Argument assistance      | `completion: {}`                                                      | All transports         |
| **Elicitation**           | _Built-in_                | `*/elicit`                             | Progressive disclosure   | `elicitation: {}`                                                     | All transports         |
| **Pagination**            | _Parameter_               | _All list methods_                     | Large datasets           | `pagination: {}`                                                      | All transports         |
| **Logging**               | `logging/setLevel`        | `notifications/message`                | Debug output             | `logging: {}`                                                         | Optional               |

### 🔄 **Critical Feature: List Change Notifications**

**Why This Matters:**

- **Dynamic Capabilities**: Modern MCP servers can add/remove tools, resources, and prompts at runtime
- **Automatic Synchronization**: Clients stay current without manual refresh
- **Real-time Updates**: Changes are communicated immediately
- **User Experience**: UI stays accurate and responsive

**Implementation Requirements:**

1. **Server declares capability**: `{"tools": {"listChanged": true}}`
2. **Client supports notifications**: `{"notifications": {"supported": true}}`
3. **Bidirectional transport**: STDIO, Streamable HTTP, or Legacy SSE
4. **Automatic client refresh**: Client calls `tools/list` when notified

# Part 2: MCP Transport Implementation Guide

## Complete Reference for Transport Handlers - Updated for 2025-06-18

## Table of Contents

1. [Transport Overview](#transport-overview)
2. [STDIO Transport Implementation](#stdio-transport-implementation)
3. [Streamable HTTP Transport Implementation](#streamable-http-transport-implementation)
4. [Legacy HTTP+SSE Transport Implementation](#legacy-httpsse-transport-implementation)
5. [Transport Comparison and Migration](#transport-comparison-and-migration)
6. [Security Implementation](#security-implementation)
7. [Error Handling and Debugging](#error-handling-and-debugging)
8. [Complete Implementation Examples](#complete-implementation-examples)

---

## Transport Overview

MCP supports three primary transport mechanisms, each optimized for different deployment scenarios:

### Transport Evolution Timeline

| Specification Version | Transport       | Status     | Primary Use Case            |
| --------------------- | --------------- | ---------- | --------------------------- |
| **All Versions**      | STDIO           | ✅ Active  | Local process integration   |
| **2024-11-05**        | HTTP+SSE        | ⚠️ Legacy  | Remote servers (deprecated) |
| **2025-03-26+**       | Streamable HTTP | ✅ Current | Modern remote servers       |

### Transport Selection Matrix

| Scenario                     | Recommended Transport      | Reason                              |
| ---------------------------- | -------------------------- | ----------------------------------- |
| Local MCP server             | STDIO                      | Simplest, most efficient            |
| Remote MCP server (new)      | Streamable HTTP            | Modern, flexible, stateless-capable |
| Remote MCP server (existing) | HTTP+SSE → Streamable HTTP | Migrate for better scalability      |
| Browser-based client         | Streamable HTTP only       | CORS and security requirements      |
| Serverless deployment        | Streamable HTTP            | Stateless support                   |

---

## STDIO Transport Implementation

### Overview

STDIO transport uses standard input/output streams for bidirectional JSON-RPC communication. Each JSON-RPC message is newline-delimited.

### Message Format

```
{"jsonrpc":"2.0","method":"initialize",...}\n
{"jsonrpc":"2.0","result":{...},"id":"init-1"}\n
{"jsonrpc":"2.0","method":"tools/list",...}\n
```

### Complete Server Implementation

```javascript
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { Server } from "@modelcontextprotocol/sdk/server/index.js";

class StdioMCPServer {
  constructor() {
    this.server = new Server(
      {
        name: "stdio-example-server",
        version: "1.0.0",
      },
      {
        capabilities: {
          tools: { listChanged: true },
          resources: { subscribe: true },
          prompts: {},
          elicitation: { supported: true },
        },
      }
    );

    this.setupHandlers();
  }

  setupHandlers() {
    // Tool handlers
    this.server.setRequestHandler(ListToolsRequestSchema, async () => {
      return {
        tools: [
          {
            name: "echo",
            description: "Echo back the input",
            inputSchema: {
              type: "object",
              properties: {
                message: { type: "string" },
              },
              required: ["message"],
            },
          },
        ],
      };
    });

    this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
      const { name, arguments: args } = request.params;

      if (name === "echo") {
        return {
          content: [
            {
              type: "text",
              text: `Echo: ${args.message}`,
            },
          ],
        };
      }

      throw new Error(`Unknown tool: ${name}`);
    });

    // Elicitation handler example
    this.server.setRequestHandler(
      ResourcesElicitRequestSchema,
      async (request) => {
        const { uri, elicitationMethod, context } = request.params;

        if (elicitationMethod === "expand") {
          return {
            resources: [
              {
                uri: `${uri}/expanded`,
                name: "Expanded Resource",
                description: "Detailed view with elicited information",
              },
            ],
            _meta: {
              elicitation: {
                totalAvailable: 1000,
                returned: 1,
                hasMore: true,
              },
            },
          };
        }

        throw new Error(`Unsupported elicitation method: ${elicitationMethod}`);
      }
    );
  }

  async start() {
    const transport = new StdioServerTransport();
    await this.server.connect(transport);
    console.error("STDIO MCP Server started"); // Use stderr for logging
  }
}

// Start server
const server = new StdioMCPServer();
server.start().catch(console.error);
```

### Client Configuration

```json
{
  "mcpServers": {
    "stdio-server": {
      "command": "node",
      "args": ["./stdio-server.js"],
      "env": {
        "API_KEY": "your-api-key"
      }
    }
  }
}
```

### STDIO Flow Diagram

```
┌─────────────┐    stdin    ┌─────────────┐
│   Client    │ ═══════════>│   Server    │
│  Process    │             │  Process    │
│             │<════════════│             │
└─────────────┘   stdout    └─────────────┘
                                    │
                                stderr (logs)
                                    v
                              ┌─────────────┐
                              │   System    │
                              │    Logs     │
                              └─────────────┘
```

---

## Streamable HTTP Transport Implementation

### Overview

Modern HTTP-based transport using a single endpoint with optional SSE streaming. Supports both stateful (session-based) and stateless operation.

### Key Characteristics

- **Single endpoint** (any path, commonly `/mcp`)
- **Session management** via `mcp-session-id` header
- **Dynamic response types** (JSON or SSE)
- **HTTP methods**: POST (required), GET (optional), DELETE (optional)

### Complete Server Implementation

```javascript
import express from "express";
import { StreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/streamableHttp.js";
import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import crypto from "crypto";

class StreamableHTTPServer {
  constructor() {
    this.app = express();
    this.sessions = new Map();
    this.server = new Server(
      {
        name: "streamable-http-server",
        version: "1.0.0",
      },
      {
        capabilities: {
          tools: { listChanged: true },
          resources: { subscribe: true },
          prompts: {},
          notifications: { progress: true },
          elicitation: { supported: true },
        },
      }
    );

    this.setupMiddleware();
    this.setupRoutes();
    this.setupHandlers();
  }

  setupMiddleware() {
    // Security headers
    this.app.use((req, res, next) => {
      res.setHeader("Access-Control-Allow-Origin", "*");
      res.setHeader(
        "Access-Control-Allow-Methods",
        "GET, POST, DELETE, OPTIONS"
      );
      res.setHeader(
        "Access-Control-Allow-Headers",
        "Content-Type, Authorization, mcp-session-id, Accept"
      );
      res.setHeader(
        "Access-Control-Expose-Headers",
        "mcp-session-id, WWW-Authenticate"
      );

      if (req.method === "OPTIONS") {
        return res.status(200).end();
      }
      next();
    });

    this.app.use(express.json({ limit: "10mb" }));
  }

  setupRoutes() {
    // Main MCP endpoint - handles all JSON-RPC communication
    this.app.post("/mcp", async (req, res) => {
      try {
        await this.handleMCPRequest(req, res);
      } catch (error) {
        console.error("MCP request error:", error);
        res.status(500).json({
          jsonrpc: "2.0",
          error: {
            code: -32603,
            message: "Internal error",
            data: error.message,
          },
          id: req.body?.id || null,
        });
      }
    });

    // Optional: GET endpoint for server-initiated SSE streams
    this.app.get("/mcp", async (req, res) => {
      const sessionId = req.headers["mcp-session-id"];
      if (!sessionId || !this.sessions.has(sessionId)) {
        return res.status(404).json({ error: "Session not found" });
      }

      // Set up SSE for server-initiated messages
      res.setHeader("Content-Type", "text/event-stream");
      res.setHeader("Cache-Control", "no-cache");
      res.setHeader("Connection", "keep-alive");

      // Keep connection alive and allow server to send messages
      const keepAlive = setInterval(() => {
        res.write("event: ping\ndata: {}\n\n");
      }, 30000);

      req.on("close", () => {
        clearInterval(keepAlive);
      });
    });

    // Optional: Session termination
    this.app.delete("/mcp", (req, res) => {
      const sessionId = req.headers["mcp-session-id"];
      if (sessionId && this.sessions.has(sessionId)) {
        this.sessions.delete(sessionId);
        res.status(204).end();
      } else {
        res.status(404).json({ error: "Session not found" });
      }
    });

    // Health check
    this.app.get("/health", (req, res) => {
      res.json({
        status: "healthy",
        transport: "streamable-http",
        sessions: this.sessions.size,
        specification: "2025-06-18",
      });
    });
  }

  async handleMCPRequest(req, res) {
    const body = req.body;
    const sessionId = req.headers["mcp-session-id"];
    const isInitialize = body?.method === "initialize";

    if (isInitialize) {
      // Create new session
      const newSessionId = crypto.randomUUID();
      const transport = new StreamableHTTPServerTransport();

      this.sessions.set(newSessionId, {
        transport,
        lastActivity: Date.now(),
      });

      // Set session header in response
      res.setHeader("mcp-session-id", newSessionId);

      // Connect server to transport
      await this.server.connect(transport);
      await transport.handleRequest(req, res, body);
    } else {
      // Use existing session
      const session = this.sessions.get(sessionId);
      if (!session) {
        return res.status(404).json({
          jsonrpc: "2.0",
          error: {
            code: -32001,
            message: "Session not found",
          },
          id: body?.id || null,
        });
      }

      // Update activity timestamp
      session.lastActivity = Date.now();

      // Handle request with existing transport
      await session.transport.handleRequest(req, res, body);
    }
  }

  setupHandlers() {
    // Dynamic tools list handler
    this.server.setRequestHandler(ListToolsRequestSchema, async () => {
      return {
        tools: Array.from(this.tools.values()),
      };
    });

    // Tool execution handler
    this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
      const { name, arguments: args } = request.params;

      const tool = this.tools.get(name);
      if (!tool) {
        throw new Error(`Unknown tool: ${name}`);
      }

      if (name === "echo") {
        return {
          content: [{ type: "text", text: `Echo: ${args.message}` }],
        };
      }

      if (name === "long_task") {
        // Example: Send progress notifications during long operations
        const progressToken = crypto.randomUUID();

        // Send progress updates
        for (let i = 0; i <= 100; i += 25) {
          await this.server.notification({
            method: "notifications/progress",
            params: {
              progressToken,
              progress: i / 100,
              message: `Processing... ${i}%`,
            },
          });

          // Simulate work
          await new Promise((resolve) => setTimeout(resolve, 500));
        }

        return {
          content: [{ type: "text", text: "Task completed!" }],
        };
      }

      // Handle other dynamic tools
      return {
        content: [{ type: "text", text: `Executed tool: ${name}` }],
      };
    });

    // Elicitation handler for progressive disclosure
    this.server.setRequestHandler(
      ResourcesElicitRequestSchema,
      async (request) => {
        const { uri, elicitationMethod, context } = request.params;

        if (elicitationMethod === "expand") {
          // Simulate expanding a large dataset with context
          const limit = context?.maxItems || 50;
          const filter = context?.filter || "";

          return {
            resources: Array.from({ length: Math.min(limit, 100) }, (_, i) => ({
              uri: `${uri}/item-${i}`,
              name: `Resource Item ${i}`,
              description: `Expanded resource matching filter: ${filter}`,
              mimeType: "application/json",
            })),
            _meta: {
              elicitation: {
                totalAvailable: 10000,
                filtered: filter ? 500 : 10000,
                returned: Math.min(limit, 100),
                hasMore: limit < 100,
                nextElicitation: {
                  methods: ["expand", "filter", "sample"],
                },
              },
            },
          };
        }

        throw new Error(`Unsupported elicitation method: ${elicitationMethod}`);
      }
    );
  }

  // API methods for dynamic tool management
  async addToolViaAPI(toolDefinition) {
    await this.addTool(toolDefinition);
    console.log(`Tool '${toolDefinition.name}' added and clients notified`);
  }

  async removeToolViaAPI(toolName) {
    await this.removeTool(toolName);
    console.log(`Tool '${toolName}' removed and clients notified`);
  }

  start(port = 3000) {
    this.app.listen(port, () => {
      console.log(`Streamable HTTP MCP Server running on port ${port}`);
      console.log(`Endpoint: http://localhost:${port}/mcp`);
    });

    // Session cleanup
    setInterval(() => {
      const now = Date.now();
      const timeout = 30 * 60 * 1000; // 30 minutes

      for (const [sessionId, session] of this.sessions.entries()) {
        if (now - session.lastActivity > timeout) {
          this.sessions.delete(sessionId);
          console.log(`Cleaned up inactive session: ${sessionId}`);
        }
      }
    }, 5 * 60 * 1000); // Check every 5 minutes

    // Example: Add a new tool after 10 seconds
    setTimeout(async () => {
      await this.addToolViaAPI({
        name: "timestamp",
        description: "Get current timestamp",
        inputSchema: {
          type: "object",
          properties: {},
          required: [],
        },
      });
    }, 10000);

    // Example: Remove a tool after 30 seconds
    setTimeout(async () => {
      await this.removeToolViaAPI("echo");
    }, 30000);
  }
}

const server = new StreamableHTTPServer();
server.start();
```

### Streamable HTTP Flow Diagrams

#### Request-Response Pattern

```
┌─────────┐                 ┌─────────┐
│ Client  │                 │ Server  │
└────┬────┘                 └────┬────┘
     │                           │
     │ POST /mcp                 │
     │ mcp-session-id: sess-123  │
     │ Accept: application/json, text/event-stream
     │ Body: {"method":"tools/call",...}
     ├──────────────────────────>│
     │                           │
     │ ┌─ Simple Response ───────┤
     │ │ HTTP 200                │
     │ │ Content-Type: application/json
     │ │ Body: {"result":{...}}  │
     │ │<────────────────────────┤
     │ │                         │
     │ └─ OR Streaming ─────────┤
     │   HTTP 200               │
     │   Content-Type: text/event-stream
     │   event: message         │
     │   data: {"result":{...}} │
     │   <────────────────────────┤
     │                           │
```

#### Progress Notification Pattern

```
┌─────────┐                 ┌─────────┐
│ Client  │                 │ Server  │
└────┬────┘                 └────┬────┘
     │                           │
     │ POST /mcp                 │
     │ Body: Long-running tool   │
     ├──────────────────────────>│
     │                           │
     │ HTTP 200 OK               │
     │ Content-Type: text/event-stream
     │<──────────────────────────┤
     │                           │
     │ event: message            │
     │ data: {"method":"notifications/progress",
     │        "params":{"progress":0.25,...}}
     │<──────────────────────────┤
     │                           │
     │ event: message            │
     │ data: {"method":"notifications/progress",
     │        "params":{"progress":0.5,...}}
     │<──────────────────────────┤
     │                           │
     │ event: message            │
     │ data: {"result":{...},"id":"123"}
     │<──────────────────────────┤
     │                           │
```

#### Tool Change Notification Pattern

```
┌─────────┐                 ┌─────────┐
│ Client  │                 │ Server  │
└────┬────┘                 └────┬────┘
     │                           │
     │ ── Normal Operation ──    │
     │                           │
     │                           │ ╔═══════════════╗
     │                           │ ║ Server adds/  ║
     │                           │ ║ removes tool  ║
     │                           │ ║ dynamically   ║
     │                           │ ╚═══════════════╝
     │                           │
     │ Auto Notification         │
     │ Content-Type: text/event-stream
     │ event: message            │
     │ data: {                   │
     │   "method": "notifications/tools/list_changed",
     │   "params": {}            │
     │ }                         │
     │<══════════════════════════┤
     │                           │
     │ Client Auto-Refresh       │
     │ POST /mcp                 │
     │ Body: {"method":"tools/list"}
     ├──────────────────────────>│
     │                           │
     │ HTTP 200 OK               │
     │ Content-Type: application/json
     │ Body: {                   │
     │   "result": {             │
     │     "tools": [            │
     │       // Updated tool list│
     │     ]                     │
     │   }                       │
     │ }                         │
     │<──────────────────────────┤
     │                           │
     │ ✓ Client UI Updated       │
```

#### Client-Side Notification Handling

```javascript
// Example client implementation for handling tool changes
class MCPClient {
  constructor(serverUrl) {
    this.serverUrl = serverUrl;
    this.sessionId = null;
    this.availableTools = new Map();
  }

  async initialize() {
    const response = await fetch(`${this.serverUrl}/mcp`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Accept: "application/json, text/event-stream",
      },
      body: JSON.stringify({
        jsonrpc: "2.0",
        method: "initialize",
        params: {
          protocolVersion: "2025-06-18",
          capabilities: {
            notifications: { supported: true },
          },
          clientInfo: { name: "ExampleClient", version: "1.0.0" },
        },
        id: "init-1",
      }),
    });

    this.sessionId = response.headers.get("mcp-session-id");

    // Set up notification handler for tool changes
    this.setupNotificationHandler();

    // Load initial tools
    await this.refreshTools();
  }

  setupNotificationHandler() {
    // In a real implementation, you'd set up SSE or WebSocket
    // to listen for server notifications
    this.onNotification = async (notification) => {
      if (notification.method === "notifications/tools/list_changed") {
        console.log("Tools changed on server, refreshing...");
        await this.refreshTools();
        this.onToolsUpdated?.(this.availableTools);
      }
    };
  }

  async refreshTools() {
    const response = await fetch(`${this.serverUrl}/mcp`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "mcp-session-id": this.sessionId,
      },
      body: JSON.stringify({
        jsonrpc: "2.0",
        method: "tools/list",
        id: "tools-refresh-" + Date.now(),
      }),
    });

    const result = await response.json();

    // Update internal tool cache
    this.availableTools.clear();
    for (const tool of result.result.tools) {
      this.availableTools.set(tool.name, tool);
    }

    console.log(`Loaded ${this.availableTools.size} tools from server`);
  }

  // Callback for UI updates
  onToolsUpdated(tools) {
    // Override this method to update UI when tools change
    console.log("Available tools:", Array.from(tools.keys()));
  }
}

// Usage
const client = new MCPClient("http://localhost:3000");
client.onToolsUpdated = (tools) => {
  // Update your UI with the new tool list
  updateToolsInUI(Array.from(tools.values()));
};
await client.initialize();
```

```
┌─────────┐                 ┌─────────┐
│ Client  │                 │ Server  │
└────┬────┘                 └────┬────┘
     │                           │
     │ 1. Initial Request        │
     │ POST /mcp                 │
     │ Body: {"method":"resources/list"}
     ├──────────────────────────>│
     │                           │
     │ HTTP 200 OK               │
     │ Content-Type: application/json
     │ Body: {                   │
     │   "result": {             │
     │     "resources": [...],   │
     │     "_meta": {            │
     │       "elicitation": {    │
     │         "available": true,│
     │         "methods": [...]  │
     │       }                   │
     │     }                     │
     │   }                       │
     │ }                         │
     │<──────────────────────────┤
     │                           │
     │ 2. Elicitation Request    │
     │ POST /mcp                 │
     │ Body: {                   │
     │   "method": "resources/elicit",
     │   "params": {             │
     │     "uri": "collection://data",
     │     "elicitationMethod": "expand",
     │     "context": {...}      │
     │   }                       │
     │ }                         │
     ├──────────────────────────>│
     │                           │
     │ HTTP 200 OK               │
     │ Content-Type: application/json
     │ Body: {                   │
     │   "result": {             │
     │     "resources": [...detailed],
     │     "_meta": {            │
     │       "elicitation": {    │
     │         "totalAvailable": 10000,
     │         "returned": 50,   │
     │         "hasMore": true   │
     │       }                   │
     │     }                     │
     │   }                       │
     │ }                         │
     │<──────────────────────────┤
     │                           │
```

---

## Legacy HTTP+SSE Transport Implementation

### Overview

**Status: Deprecated but supported for backward compatibility**

Uses dual endpoints:

- `GET /sse` - Server-to-client event stream
- `POST /message` - Client-to-server messages (note: singular "message")

### Complete Server Implementation

```javascript
import express from "express";
import { SSEServerTransport } from "@modelcontextprotocol/sdk/server/sse.js";
import { Server } from "@modelcontextprotocol/sdk/server/index.js";

class LegacySSEServer {
  constructor() {
    this.app = express();
    this.activeTransports = new Map();
    this.server = new Server(
      {
        name: "legacy-sse-server",
        version: "1.0.0",
      },
      {
        capabilities: {
          tools: {},
          resources: {},
          prompts: {},
        },
      }
    );

    this.setupMiddleware();
    this.setupRoutes();
  }

  setupMiddleware() {
    this.app.use(express.json());
    this.app.use((req, res, next) => {
      res.setHeader("Access-Control-Allow-Origin", "*");
      res.setHeader("Access-Control-Allow-Methods", "GET, POST, OPTIONS");
      res.setHeader("Access-Control-Allow-Headers", "Content-Type");
      next();
    });
  }

  setupRoutes() {
    // SSE endpoint for server-to-client communication
    this.app.get("/sse", async (req, res) => {
      try {
        // Set SSE headers
        res.setHeader("Content-Type", "text/event-stream");
        res.setHeader("Cache-Control", "no-cache");
        res.setHeader("Connection", "keep-alive");

        // Create SSE transport with message endpoint
        const transport = new SSEServerTransport("/message", res);

        // Store transport for message endpoint lookup
        this.activeTransports.set(transport.sessionId, transport);

        // Connect MCP server
        await this.server.connect(transport);

        console.log(`SSE connection established: ${transport.sessionId}`);

        // Clean up on connection close
        req.on("close", () => {
          this.activeTransports.delete(transport.sessionId);
          console.log(`SSE connection closed: ${transport.sessionId}`);
        });
      } catch (error) {
        console.error("SSE connection error:", error);
        res.status(500).end();
      }
    });

    // Message endpoint for client-to-server communication
    this.app.post("/message", async (req, res) => {
      try {
        const sessionId = req.query.sessionId;
        const transport = this.activeTransports.get(sessionId);

        if (!transport) {
          return res.status(404).json({
            jsonrpc: "2.0",
            error: {
              code: -32001,
              message: "Session not found",
            },
            id: req.body?.id || null,
          });
        }

        // Handle the message through the transport
        await transport.handlePostMessage(req, res, req.body);
      } catch (error) {
        console.error("Message handling error:", error);
        res.status(500).json({
          jsonrpc: "2.0",
          error: {
            code: -32603,
            message: "Internal error",
          },
          id: req.body?.id || null,
        });
      }
    });
  }

  start(port = 3001) {
    this.app.listen(port, () => {
      console.log(`Legacy SSE MCP Server running on port ${port}`);
      console.log(`SSE endpoint: http://localhost:${port}/sse`);
      console.log(`Message endpoint: http://localhost:${port}/message`);
    });
  }
}

const server = new LegacySSEServer();
server.start();
```

### Legacy HTTP+SSE Flow Diagram

```
┌─────────┐                 ┌─────────┐
│ Client  │                 │ Server  │
└────┬────┘                 └────┬────┘
     │                           │
     │ 1. GET /sse               │
     │ Accept: text/event-stream │
     ├──────────────────────────>│
     │                           │
     │ HTTP 200 OK               │
     │ Content-Type: text/event-stream
     │ event: endpoint           │
     │ data: {"endpoint":"/message",
     │        "sessionId":"sess-xyz"}
     │<══════════════════════════┤ (SSE stream open)
     │                           │
     │ 2. POST /message          │
     │ ?sessionId=sess-xyz       │
     │ Body: InitializeRequest   │
     ├──────────────────────────>│
     │                           │
     │ HTTP 200 OK               │
     │ Body: InitializeResult    │
     │<──────────────────────────┤
     │                           │
     │ ── Ongoing Operations ──  │
     │                           │
     │ POST /message             │
     │ Body: ToolsListRequest    │
     ├──────────────────────────>│
     │                           │
     │ event: message            │
     │ data: ToolsListResult     │
     │<══════════════════════════┤ (via SSE)
     │                           │
```

---

## Transport Comparison and Migration

### Feature Comparison Matrix

| Feature                         | STDIO              | Streamable HTTP        | Legacy HTTP+SSE            |
| ------------------------------- | ------------------ | ---------------------- | -------------------------- |
| **Deployment**                  | Local only         | Local + Remote         | Local + Remote             |
| **Endpoints**                   | N/A                | Single (flexible path) | Dual (`/sse` + `/message`) |
| **Session Management**          | Process-bound      | Headers                | Query parameters           |
| **Bidirectional**               | ✅ Native          | ✅ Optional SSE        | ✅ Required SSE            |
| **Stateless Support**           | ❌                 | ✅                     | ❌                         |
| **Serverless Ready**            | ❌                 | ✅                     | ❌                         |
| **Progress Notifications**      | ✅                 | ✅                     | ✅                         |
| **Elicitation Support**         | ✅                 | ✅                     | ✅                         |
| **Connection Complexity**       | Low                | Medium                 | High                       |
| **Browser Support**             | ❌                 | ✅                     | ✅ Limited                 |
| **Infrastructure Requirements** | Process management | HTTP server            | HTTP + SSE handling        |

### Migration Path: Legacy SSE → Streamable HTTP

#### For Servers (Supporting Both)

```javascript
class DualTransportServer {
  constructor() {
    this.app = express();
    this.activeSessions = new Map(); // Streamable HTTP sessions
    this.activeTransports = new Map(); // Legacy SSE transports
  }

  setupRoutes() {
    // Modern Streamable HTTP endpoint
    this.app.post("/mcp", async (req, res) => {
      await this.handleStreamableHTTP(req, res);
    });

    // Legacy SSE endpoints (for backward compatibility)
    this.app.get("/sse", async (req, res) => {
      await this.handleLegacySSE(req, res);
    });

    this.app.post("/message", async (req, res) => {
      await this.handleLegacyMessage(req, res);
    });
  }
}
```

#### For Clients (Auto-Detection)

```javascript
class AdaptiveClient {
  async detectTransport(serverUrl) {
    try {
      // Try Streamable HTTP first
      const response = await fetch(`${serverUrl}/mcp`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Accept: "application/json, text/event-stream",
        },
        body: JSON.stringify({
          jsonrpc: "2.0",
          method: "initialize",
          params: {
            /* ... */
          },
          id: "init-1",
        }),
      });

      if (response.ok) {
        return "streamable-http";
      }
    } catch (error) {
      console.log("Streamable HTTP not available, trying legacy SSE");
    }

    try {
      // Fall back to legacy SSE
      const sseResponse = await fetch(`${serverUrl}/sse`);
      if (
        sseResponse.ok &&
        sseResponse.headers.get("content-type")?.includes("text/event-stream")
      ) {
        return "legacy-sse";
      }
    } catch (error) {
      console.log("Legacy SSE not available");
    }

    throw new Error("No supported transport found");
  }
}
```

---

## Security Implementation

### Authentication Requirements by Transport

| Transport           | Authentication Method | Implementation                    |
| ------------------- | --------------------- | --------------------------------- |
| **STDIO**           | Environment/Config    | API keys, file permissions        |
| **Streamable HTTP** | OAuth 2.1 (mandatory) | Bearer tokens, PKCE flow          |
| **Legacy HTTP+SSE** | OAuth 2.1 (mandatory) | Bearer tokens, session validation |

### OAuth 2.1 Implementation for HTTP Transports

```javascript
class SecureHTTPServer {
  constructor() {
    this.app = express();
    this.setupSecurity();
  }

  setupSecurity() {
    // OAuth discovery endpoint
    this.app.get("/.well-known/oauth-protected-resource", (req, res) => {
      res.json({
        resource: `${req.protocol}://${req.get("host")}/mcp`,
        authorization_servers: [`${req.protocol}://${req.get("host")}/auth`],
      });
    });

    // Token validation middleware
    this.app.use("/mcp", this.validateToken.bind(this));
  }

  async validateToken(req, res, next) {
    const authHeader = req.headers.authorization;

    if (!authHeader || !authHeader.startsWith("Bearer ")) {
      return res.status(401).json({
        error: "unauthorized",
        error_description: "Bearer token required",
        "WWW-Authenticate": 'Bearer realm="MCP", error="invalid_token"',
      });
    }

    const token = authHeader.substring(7);

    try {
      // Validate token (implement according to your OAuth setup)
      const tokenInfo = await this.validateAccessToken(token);
      req.user = tokenInfo;
      next();
    } catch (error) {
      res.status(401).json({
        error: "invalid_token",
        error_description: "Token validation failed",
      });
    }
  }

  async validateAccessToken(token) {
    // Implementation depends on your OAuth provider
    // Must validate:
    // 1. Token signature/encryption
    // 2. Token expiration
    // 3. Token audience (resource parameter)
    // 4. Token scope
    throw new Error("Implement token validation");
  }
}
```

### Security Headers and CORS

```javascript
function setupSecurityHeaders(app) {
  app.use((req, res, next) => {
    // CORS headers
    res.setHeader("Access-Control-Allow-Origin", "*"); // Restrict in production
    res.setHeader("Access-Control-Allow-Methods", "GET, POST, DELETE, OPTIONS");
    res.setHeader(
      "Access-Control-Allow-Headers",
      "Content-Type, Authorization, mcp-session-id"
    );

    // Security headers
    res.setHeader("X-Content-Type-Options", "nosniff");
    res.setHeader("X-Frame-Options", "DENY");
    res.setHeader("X-XSS-Protection", "1; mode=block");

    // Validate Origin for DNS rebinding protection
    const origin = req.headers.origin;
    if (origin && !isAllowedOrigin(origin)) {
      return res.status(403).json({ error: "Forbidden origin" });
    }

    next();
  });
}

function isAllowedOrigin(origin) {
  const allowedOrigins = ["http://localhost:3000", "https://yourdomain.com"];
  return allowedOrigins.includes(origin);
}
```

---

## Error Handling and Debugging

### Common Error Patterns

#### Transport-Specific Errors

```javascript
// Streamable HTTP errors
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32001,
    "message": "Session not found",
    "data": {
      "sessionId": "invalid-session-123",
      "suggestion": "Initialize a new session"
    }
  },
  "id": "req-456"
}

// Legacy SSE errors
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32001,
    "message": "SSE connection required",
    "data": {
      "endpoint": "/sse",
      "suggestion": "Establish SSE connection first"
    }
  },
  "id": "req-789"
}
```

### Debug Logging Implementation

```javascript
class DebugTransport {
  constructor(transport, debugLevel = "info") {
    this.transport = transport;
    this.debugLevel = debugLevel;
  }

  log(level, message, data = {}) {
    if (this.shouldLog(level)) {
      console.log(
        `[${new Date().toISOString()}] [${level.toUpperCase()}] ${message}`,
        data
      );
    }
  }

  shouldLog(level) {
    const levels = { error: 0, warn: 1, info: 2, debug: 3 };
    return levels[level] <= levels[this.debugLevel];
  }

  async handleRequest(req, res, body) {
    this.log("debug", "Incoming request", {
      method: body?.method,
      sessionId: req.headers["mcp-session-id"],
      contentType: req.headers["content-type"],
    });

    try {
      const result = await this.transport.handleRequest(req, res, body);
      this.log("debug", "Request completed successfully");
      return result;
    } catch (error) {
      this.log("error", "Request failed", {
        error: error.message,
        stack: error.stack,
      });
      throw error;
    }
  }
}
```

### Health Check Implementation

```javascript
class HealthMonitor {
  constructor(server) {
    this.server = server;
    this.metrics = {
      requests: 0,
      errors: 0,
      activeSessions: 0,
      startTime: Date.now(),
    };
  }

  getHealthStatus() {
    const uptime = Date.now() - this.metrics.startTime;
    const errorRate =
      this.metrics.requests > 0
        ? this.metrics.errors / this.metrics.requests
        : 0;

    return {
      status: errorRate < 0.1 ? "healthy" : "degraded",
      uptime: Math.floor(uptime / 1000),
      requests: this.metrics.requests,
      errors: this.metrics.errors,
      errorRate: Math.round(errorRate * 100) / 100,
      activeSessions: this.metrics.activeSessions,
      memoryUsage: process.memoryUsage(),
      timestamp: new Date().toISOString(),
    };
  }

  setupHealthEndpoint(app) {
    app.get("/health", (req, res) => {
      const health = this.getHealthStatus();
      const statusCode = health.status === "healthy" ? 200 : 503;
      res.status(statusCode).json(health);
    });
  }
}
```

---

## Complete Implementation Examples

### Production-Ready Streamable HTTP Server

```javascript
import express from "express";
import helmet from "helmet";
import rateLimit from "express-rate-limit";
import { StreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/streamableHttp.js";

class ProductionMCPServer {
  constructor(options = {}) {
    this.app = express();
    this.sessions = new Map();
    this.options = {
      port: 3000,
      sessionTimeout: 30 * 60 * 1000, // 30 minutes
      maxSessions: 1000,
      ...options,
    };

    this.setupSecurity();
    this.setupMiddleware();
    this.setupRoutes();
    this.setupCleanup();
  }

  setupSecurity() {
    // Security middleware
    this.app.use(
      helmet({
        contentSecurityPolicy: false, // Disable for MCP compatibility
        crossOriginEmbedderPolicy: false,
      })
    );

    // Rate limiting
    const limiter = rateLimit({
      windowMs: 15 * 60 * 1000, // 15 minutes
      max: 100, // Limit each IP to 100 requests per windowMs
      message: "Too many requests from this IP",
    });
    this.app.use("/mcp", limiter);

    // Request size limits
    this.app.use(
      express.json({
        limit: "10mb",
        verify: (req, res, buf) => {
          // Additional validation can be added here
        },
      })
    );
  }

  setupMiddleware() {
    // CORS with specific origins
    this.app.use((req, res, next) => {
      const allowedOrigins = process.env.ALLOWED_ORIGINS?.split(",") || ["*"];
      const origin = req.headers.origin;

      if (allowedOrigins.includes("*") || allowedOrigins.includes(origin)) {
        res.setHeader("Access-Control-Allow-Origin", origin || "*");
      }

      res.setHeader(
        "Access-Control-Allow-Methods",
        "GET, POST, DELETE, OPTIONS"
      );
      res.setHeader(
        "Access-Control-Allow-Headers",
        "Content-Type, Authorization, mcp-session-id"
      );
      res.setHeader("Access-Control-Expose-Headers", "mcp-session-id");

      if (req.method === "OPTIONS") {
        return res.status(200).end();
      }
      next();
    });

    // Request logging
    this.app.use((req, res, next) => {
      console.log(`${req.method} ${req.path} - ${req.ip}`);
      next();
    });
  }

  setupRoutes() {
    // Main MCP endpoint with full error handling
    this.app.post("/mcp", async (req, res) => {
      try {
        // Session limit check
        if (this.sessions.size >= this.options.maxSessions) {
          return res.status(503).json({
            jsonrpc: "2.0",
            error: {
              code: -32000,
              message: "Server overloaded",
              data: { maxSessions: this.options.maxSessions },
            },
            id: req.body?.id || null,
          });
        }

        await this.handleMCPRequest(req, res);
      } catch (error) {
        console.error("MCP request error:", error);

        if (!res.headersSent) {
          res.status(500).json({
            jsonrpc: "2.0",
            error: {
              code: -32603,
              message: "Internal error",
              data:
                process.env.NODE_ENV === "development"
                  ? error.message
                  : undefined,
            },
            id: req.body?.id || null,
          });
        }
      }
    });

    // Health and monitoring
    this.app.get("/health", (req, res) => {
      res.json({
        status: "healthy",
        sessions: this.sessions.size,
        uptime: process.uptime(),
        memory: process.memoryUsage(),
        version: process.env.npm_package_version || "1.0.0",
      });
    });

    // Graceful session cleanup
    this.app.delete("/mcp", (req, res) => {
      const sessionId = req.headers["mcp-session-id"];
      if (sessionId && this.sessions.has(sessionId)) {
        this.sessions.delete(sessionId);
        console.log(`Session terminated: ${sessionId}`);
        res.status(204).end();
      } else {
        res.status(404).json({ error: "Session not found" });
      }
    });
  }

  setupCleanup() {
    // Session cleanup interval
    setInterval(() => {
      const now = Date.now();
      let cleaned = 0;

      for (const [sessionId, session] of this.sessions.entries()) {
        if (now - session.lastActivity > this.options.sessionTimeout) {
          this.sessions.delete(sessionId);
          cleaned++;
        }
      }

      if (cleaned > 0) {
        console.log(`Cleaned up ${cleaned} inactive sessions`);
      }
    }, 5 * 60 * 1000); // Check every 5 minutes

    // Graceful shutdown
    process.on("SIGTERM", () => {
      console.log("Received SIGTERM, shutting down gracefully");
      this.server?.close(() => {
        console.log("Server closed");
        process.exit(0);
      });
    });
  }

  start() {
    this.server = this.app.listen(this.options.port, () => {
      console.log(`Production MCP Server running on port ${this.options.port}`);
      console.log(`Environment: ${process.env.NODE_ENV || "development"}`);
      console.log(`Max sessions: ${this.options.maxSessions}`);
    });
  }
}

// Start production server
if (require.main === module) {
  const server = new ProductionMCPServer({
    port: process.env.PORT || 3000,
    maxSessions: process.env.MAX_SESSIONS || 1000,
  });
  server.start();
}
```

## Tool Change Notification Summary

When tools change on the server side, clients/hosts are notified through MCP's **capability-based notification system**:

### 🔧 **Requirements:**

1. **Server Capability**: `{"tools": {"listChanged": true}}`
2. **Client Capability**: `{"notifications": {"supported": true}}`
3. **Bidirectional Transport**: STDIO, Streamable HTTP, or Legacy SSE

### 📡 **Notification Flow:**

1. **Server detects tool change** (add/remove/update)
2. **Server sends notification**: `"notifications/tools/list_changed"`
3. **Client receives notification** via transport
4. **Client automatically refreshes**: Calls `tools/list`
5. **Client updates UI** with new tool list

### ✅ **Supported Changes:**

- ✅ **New tools added**
- ✅ **Existing tools removed**
- ✅ **Tool schemas updated**
- ✅ **Tool availability changed**

### 🔄 **Similar Notifications Available:**

- **Resources**: `"notifications/resources/list_changed"` when resources change
- **Prompts**: `"notifications/prompts/list_changed"` when prompts change
- **Both require**: `{"resources": {"listChanged": true}}` or `{"prompts": {"listChanged": true}}`

### 🚀 **Benefits:**

- **Real-time updates** - No polling required
- **Automatic refresh** - Client stays in sync
- **Efficient** - Only notifies when changes occur
- **Reliable** - Built into MCP specification

This ensures that clients always have the most current list of available tools without manual refresh!
