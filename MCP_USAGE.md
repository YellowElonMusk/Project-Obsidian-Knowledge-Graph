# Cortex MCP Server

Cortex exposes a local MCP server on **`localhost:7340`** (JSON-RPC 2.0 over TCP).

## Connect from Claude Code / any MCP client

Add this to your `~/.claude/claude_desktop_config.json` or MCP config:

```json
{
  "mcpServers": {
    "cortex": {
      "command": "node",
      "args": ["-e", "
const net = require('net');
const readline = require('readline');
const sock = net.connect(7340, '127.0.0.1');
const rl = readline.createInterface({ input: process.stdin });
rl.on('line', line => sock.write(line + '\\n'));
sock.on('data', d => process.stdout.write(d));
      "]
    }
  }
}
```

## Available Tools

### `graph_search(query: string)`
Full-text search over all ingested nodes. Returns matching nodes with content preview.

### `get_project_context(project_name: string)`
Returns all nodes tagged to a project with their types and IDs.

### `write_agent_memory(session_id, action, result, project?, nodes_touched?)`
Writes agent actions/results back into the graph. Called after completing tasks so future sessions can recall what was done.

### `get_last_session(project_name: string)`
Returns the most recent agent memory or session node for a project.

### `list_projects()`
Lists all projects currently in the knowledge graph.

## Example JSON-RPC calls

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"graph_search","arguments":{"query":"authentication"}}}
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"write_agent_memory","arguments":{"session_id":"s1","action":"Fixed auth bug","result":"Added JWT validation in middleware","project":"myapp"}}}
```

## Test with netcat / wscat

```bash
# Quick test
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | nc localhost 7340
```
