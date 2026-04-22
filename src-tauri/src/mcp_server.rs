use tokio::net::TcpListener;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use rusqlite::Connection;
use chrono::Utc;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

impl JsonRpcResponse {
    fn success(id: Option<Value>, result: Value) -> Self {
        Self { jsonrpc: "2.0".into(), id, result: Some(result), error: None }
    }
    fn error_resp(id: Option<Value>, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(json!({"code": code, "message": message})),
        }
    }
}

pub async fn start_mcp_server(db_path: String, port: u16) {
    let listener = match TcpListener::bind(format!("127.0.0.1:{}", port)).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[Cortex MCP] Failed to bind port {}: {}", port, e);
            return;
        }
    };
    println!("[Cortex MCP] Server listening on 127.0.0.1:{}", port);

    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                println!("[Cortex MCP] Client connected: {}", addr);
                let db = db_path.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_client(socket, db).await {
                        eprintln!("[Cortex MCP] Client error: {}", e);
                    }
                });
            }
            Err(e) => eprintln!("[Cortex MCP] Accept error: {}", e),
        }
    }
}

async fn handle_client(socket: tokio::net::TcpStream, db_path: String) -> anyhow::Result<()> {
    let (reader, mut writer) = socket.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();

    while buf_reader.read_line(&mut line).await? > 0 {
        let trimmed = line.trim().to_string();
        line.clear();

        if trimmed.is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<JsonRpcRequest>(&trimmed) {
            Ok(req) => handle_rpc(req, &db_path).await,
            Err(e) => JsonRpcResponse::error_resp(None, -32700, &format!("Parse error: {}", e)),
        };

        let mut out = serde_json::to_string(&response)?;
        out.push('\n');
        writer.write_all(out.as_bytes()).await?;
    }
    Ok(())
}

async fn handle_rpc(req: JsonRpcRequest, db_path: &str) -> JsonRpcResponse {
    let id = req.id.clone();
    match req.method.as_str() {
        "initialize" => JsonRpcResponse::success(id, json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "cortex", "version": "0.1.0" }
        })),

        "tools/list" => JsonRpcResponse::success(id, json!({
            "tools": [
                {
                    "name": "graph_search",
                    "description": "Full-text search over Cortex knowledge graph nodes. Returns matching nodes with content.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string", "description": "Search query" }
                        },
                        "required": ["query"]
                    }
                },
                {
                    "name": "get_project_context",
                    "description": "Get all nodes and recent activity for a named project.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project_name": { "type": "string" }
                        },
                        "required": ["project_name"]
                    }
                },
                {
                    "name": "write_agent_memory",
                    "description": "Write an agent action and its result back to the knowledge graph for future recall.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "session_id": { "type": "string" },
                            "action": { "type": "string", "description": "What the agent did" },
                            "result": { "type": "string", "description": "What the result was" },
                            "project": { "type": "string" },
                            "nodes_touched": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "IDs or labels of nodes touched"
                            }
                        },
                        "required": ["session_id", "action", "result"]
                    }
                },
                {
                    "name": "get_last_session",
                    "description": "Retrieve what the agent did in the last session for a project.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "project_name": { "type": "string" }
                        },
                        "required": ["project_name"]
                    }
                },
                {
                    "name": "list_projects",
                    "description": "List all known projects in the Cortex graph.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                }
            ]
        })),

        "tools/call" => {
            let params = req.params.unwrap_or(json!({}));
            let name = params["name"].as_str().unwrap_or("").to_string();
            let args = params["arguments"].clone();
            match call_tool(&name, args, db_path) {
                Ok(text) => JsonRpcResponse::success(id, json!({
                    "content": [{ "type": "text", "text": text }]
                })),
                Err(e) => JsonRpcResponse::error_resp(id, -32603, &e.to_string()),
            }
        },

        // Notifications (no response needed but we send anyway)
        "notifications/initialized" => JsonRpcResponse::success(id, json!({})),

        _ => JsonRpcResponse::error_resp(id, -32601, "Method not found"),
    }
}

fn call_tool(name: &str, args: Value, db_path: &str) -> anyhow::Result<String> {
    let conn = Connection::open(db_path)
        .map_err(|e| anyhow::anyhow!("DB open failed: {}", e))?;

    match name {
        "graph_search" => {
            let query = args["query"].as_str().unwrap_or("");
            if query.is_empty() {
                return Ok("Please provide a search query.".into());
            }
            let nodes = crate::graph::search_nodes(&conn, query)?;
            if nodes.is_empty() {
                return Ok(format!("No results found for '{}'.", query));
            }
            let result = nodes.iter().enumerate().map(|(i, n)| {
                let preview = n.content.as_deref()
                    .unwrap_or("")
                    .chars()
                    .take(300)
                    .collect::<String>();
                format!(
                    "{}. **{}** [{}]\n   ID: {}\n   {}",
                    i + 1, n.label, n.node_type, n.id, preview
                )
            }).collect::<Vec<_>>().join("\n\n");
            Ok(format!("Found {} results:\n\n{}", nodes.len(), result))
        }

        "get_project_context" => {
            let project = args["project_name"].as_str().unwrap_or("default");
            let nodes = crate::graph::get_project_nodes(&conn, project)?;
            if nodes.is_empty() {
                return Ok(format!("No data found for project '{}'. Try ingesting some files first.", project));
            }
            let summary = nodes.iter().map(|n| {
                format!("- [{}] {} ({})", n.node_type, n.label, n.id)
            }).collect::<Vec<_>>().join("\n");
            Ok(format!("**Project: {}** — {} nodes\n\n{}", project, nodes.len(), summary))
        }

        "write_agent_memory" => {
            let session_id = args["session_id"].as_str().unwrap_or(&Uuid::new_v4().to_string()).to_string();
            let action = args["action"].as_str().unwrap_or("").to_string();
            let result = args["result"].as_str().unwrap_or("").to_string();
            let project = args["project"].as_str().unwrap_or("default").to_string();
            let nodes_touched: Vec<String> = args["nodes_touched"]
                .as_array()
                .map(|a| a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect())
                .unwrap_or_default();

            let content = format!(
                "Action: {}\n\nResult: {}\n\nNodes touched: {}",
                action, result,
                if nodes_touched.is_empty() { "none".into() } else { nodes_touched.join(", ") }
            );

            let now = Utc::now().timestamp_millis();
            let node = crate::graph::Node {
                id: Uuid::new_v4().to_string(),
                label: format!("AgentMem: {}", &action[..action.len().min(60)]),
                node_type: "agent_memory".to_string(),
                content: Some(content),
                file_path: None,
                created_at: now,
                updated_at: now,
                metadata: Some(json!({
                    "session_id": session_id,
                    "project": project,
                    "nodes_touched": nodes_touched
                }).to_string()),
            };
            crate::graph::insert_node(&conn, &node)?;
            Ok(format!("Memory stored. Node ID: {}", node.id))
        }

        "get_last_session" => {
            let project = args["project_name"].as_str().unwrap_or("default");
            match crate::graph::get_last_session(&conn, project)? {
                Some(node) => Ok(format!(
                    "**Last Session for '{}'**\n\n{}\n\n---\n{}",
                    project,
                    node.label,
                    node.content.as_deref().unwrap_or("(no details)")
                )),
                None => Ok(format!("No sessions recorded for project '{}'.", project)),
            }
        }

        "list_projects" => {
            let projects = crate::graph::get_projects(&conn)?;
            if projects.is_empty() {
                Ok("No projects found. Ingest files to get started.".into())
            } else {
                Ok(format!(
                    "**Projects ({}):**\n{}",
                    projects.len(),
                    projects.iter().map(|p| format!("- {}", p)).collect::<Vec<_>>().join("\n")
                ))
            }
        }

        _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
    }
}
