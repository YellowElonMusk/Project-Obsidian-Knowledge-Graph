use std::sync::Mutex;
use std::path::PathBuf;
use std::fs;
use rusqlite::Connection;
use tauri::{AppHandle, Manager, State};
use serde::{Deserialize, Serialize};

mod graph;
mod ingest;
mod mcp_server;

// ── App state ─────────────────────────────────────────────────────────────────

pub struct AppState {
    pub db: Mutex<Connection>,
    pub vault_path: PathBuf,
    pub mcp_port: u16,
}

// ── Serializable return types ──────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct IngestSummary {
    pub nodes_added: usize,
    pub edges_added: usize,
    pub title: String,
}

// ── Tauri commands ─────────────────────────────────────────────────────────────

#[tauri::command]
fn get_mcp_port(state: State<AppState>) -> u16 {
    state.mcp_port
}

#[tauri::command]
fn ingest_file(
    path: String,
    project: Option<String>,
    state: State<AppState>,
) -> Result<IngestSummary, String> {
    let proj = project.as_deref().unwrap_or("default");
    let result = ingest::ingest_file(&path, proj).map_err(|e| e.to_string())?;

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let nodes_count = result.nodes.len();
    let edges_count = result.edges.len();
    let title = result.nodes.first().map(|n| n.label.clone()).unwrap_or_default();

    for node in &result.nodes {
        graph::insert_node(&db, node).map_err(|e| e.to_string())?;
    }
    for edge in &result.edges {
        graph::insert_edge(&db, edge).map_err(|e| e.to_string())?;
    }

    // Write vault markdown mirror
    let vault_file = state.vault_path.join(format!("{}.md", sanitize_filename(&title)));
    let _ = fs::write(&vault_file, &result.vault_markdown);

    Ok(IngestSummary { nodes_added: nodes_count, edges_added: edges_count, title })
}

#[tauri::command]
fn ingest_text(
    content: String,
    title: String,
    project: Option<String>,
    state: State<AppState>,
) -> Result<IngestSummary, String> {
    let proj = project.as_deref().unwrap_or("default");
    let result = ingest::ingest_content(&title, "", &content, proj, "text")
        .map_err(|e| e.to_string())?;

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let nodes_count = result.nodes.len();
    let edges_count = result.edges.len();

    for node in &result.nodes {
        graph::insert_node(&db, node).map_err(|e| e.to_string())?;
    }
    for edge in &result.edges {
        graph::insert_edge(&db, edge).map_err(|e| e.to_string())?;
    }

    let vault_file = state.vault_path.join(format!("{}.md", sanitize_filename(&title)));
    let _ = fs::write(&vault_file, &result.vault_markdown);

    Ok(IngestSummary { nodes_added: nodes_count, edges_added: edges_count, title })
}

#[tauri::command]
fn get_graph_data(state: State<AppState>) -> Result<graph::GraphData, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    graph::get_graph_data(&db).map_err(|e| e.to_string())
}

#[tauri::command]
fn search_graph(query: String, state: State<AppState>) -> Result<Vec<graph::Node>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    graph::search_nodes(&db, &query).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_node_detail(
    node_id: String,
    state: State<AppState>,
) -> Result<Option<graph::NodeDetail>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    graph::get_node_detail(&db, &node_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_projects(state: State<AppState>) -> Result<Vec<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    graph::get_projects(&db).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_project_context(
    project_name: String,
    state: State<AppState>,
) -> Result<graph::GraphData, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let nodes = graph::get_project_nodes(&db, &project_name).map_err(|e| e.to_string())?;
    // Get edges between these nodes
    let node_ids: std::collections::HashSet<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let all_edges = graph::get_graph_data(&db).map_err(|e| e.to_string())?.edges;
    let edges = all_edges.into_iter()
        .filter(|e| node_ids.contains(&e.source_id) && node_ids.contains(&e.target_id))
        .collect();
    Ok(graph::GraphData { nodes, edges })
}

#[tauri::command]
fn get_last_session(
    project_name: String,
    state: State<AppState>,
) -> Result<Option<graph::Node>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    graph::get_last_session(&db, &project_name).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_agent_memory(
    session_id: String,
    action: String,
    result: String,
    project: Option<String>,
    nodes_touched: Option<Vec<String>>,
    state: State<AppState>,
) -> Result<String, String> {
    let proj = project.as_deref().unwrap_or("default");
    let touched = nodes_touched.unwrap_or_default();
    let now = chrono::Utc::now().timestamp_millis();
    let content = format!(
        "Action: {}\n\nResult: {}\n\nNodes touched: {}",
        action, result,
        if touched.is_empty() { "none".into() } else { touched.join(", ") }
    );
    let node = graph::Node {
        id: uuid::Uuid::new_v4().to_string(),
        label: format!("AgentMem: {}", &action[..action.len().min(60)]),
        node_type: "agent_memory".to_string(),
        content: Some(content),
        file_path: None,
        created_at: now,
        updated_at: now,
        metadata: Some(serde_json::json!({
            "session_id": session_id,
            "project": proj,
            "nodes_touched": touched
        }).to_string()),
    };
    let db = state.db.lock().map_err(|e| e.to_string())?;
    graph::insert_node(&db, &node).map_err(|e| e.to_string())?;
    Ok(node.id)
}

#[tauri::command]
fn create_session(
    project_name: String,
    state: State<AppState>,
) -> Result<String, String> {
    let now = chrono::Utc::now();
    let node = graph::create_node(
        &format!("Session {}", now.format("%Y-%m-%d %H:%M")),
        "session",
        None,
        None,
        Some(serde_json::json!({
            "project": project_name,
            "started_at": now.timestamp_millis()
        }).to_string()),
    );
    let id = node.id.clone();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    graph::insert_node(&db, &node).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
fn get_recent_sessions(
    limit: Option<usize>,
    state: State<AppState>,
) -> Result<Vec<graph::Node>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    graph::get_recent_sessions(&db, limit.unwrap_or(20)).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_vault_path(state: State<AppState>) -> String {
    state.vault_path.to_string_lossy().to_string()
}

// ── App entry point ────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()
                .expect("Cannot resolve app data dir");
            fs::create_dir_all(&data_dir).expect("Cannot create data dir");

            let vault_path = data_dir.join("vault");
            fs::create_dir_all(&vault_path).expect("Cannot create vault dir");

            let db_path = data_dir.join("cortex.db");
            let conn = Connection::open(&db_path).expect("Cannot open database");
            graph::init_db(&conn).expect("Cannot initialize database");

            let mcp_port: u16 = 7340;

            // Spawn MCP server
            let db_path_str = db_path.to_string_lossy().to_string();
            tauri::async_runtime::spawn(async move {
                mcp_server::start_mcp_server(db_path_str, mcp_port).await;
            });

            app.manage(AppState {
                db: Mutex::new(conn),
                vault_path,
                mcp_port,
            });

            // Set up system tray
            setup_tray(app.handle())?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_mcp_port,
            ingest_file,
            ingest_text,
            get_graph_data,
            search_graph,
            get_node_detail,
            list_projects,
            get_project_context,
            get_last_session,
            write_agent_memory,
            create_session,
            get_recent_sessions,
            get_vault_path,
        ])
        .run(tauri::generate_context!())
        .expect("Error running Cortex");
}

fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{MenuBuilder, MenuItemBuilder};
    use tauri::tray::{TrayIconBuilder, TrayIconEvent};

    let show = MenuItemBuilder::with_id("show", "Open Cortex").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
    let menu = MenuBuilder::new(app).items(&[&show, &quit]).build()?;

    TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Cortex — Knowledge Graph")
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { .. } = event {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    if w.is_visible().unwrap_or(false) {
                        let _ = w.hide();
                    } else {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .take(64)
        .collect()
}
