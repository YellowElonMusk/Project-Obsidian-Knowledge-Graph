use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::path::PathBuf;
use std::fs;
use rusqlite::Connection;
use tauri::{AppHandle, Manager, State};
use serde::{Deserialize, Serialize};

mod graph;
mod ingest;
mod mcp_server;
mod embeddings;

// ── App state ─────────────────────────────────────────────────────────────────

pub struct AppState {
    pub db: Mutex<Connection>,
    pub vault_path: PathBuf,
    pub mcp_port: u16,
    pub embedding_model: Arc<Mutex<Option<embeddings::EmbeddingModel>>>,
    pub model_download_progress: Arc<Mutex<Option<(u64, u64)>>>,
    pub mcp_connections: Arc<AtomicUsize>,
}

// ── Serializable return types ──────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct IngestSummary {
    pub nodes_added: usize,
    pub edges_added: usize,
    pub title: String,
}

#[derive(Serialize)]
pub struct ModelStatus {
    pub ready: bool,
    pub downloading: bool,
    pub progress_pct: u8,
}

// ── Tauri commands ─────────────────────────────────────────────────────────────

#[tauri::command]
fn get_mcp_port(state: State<AppState>) -> u16 {
    state.mcp_port
}

#[tauri::command]
fn get_mcp_connections(state: State<AppState>) -> usize {
    state.mcp_connections.load(Ordering::Relaxed)
}

#[tauri::command]
fn get_model_status(state: State<AppState>) -> ModelStatus {
    let ready = state.embedding_model.lock()
        .ok()
        .map(|lock| lock.is_some())
        .unwrap_or(false);

    let (downloading, progress_pct) = state.model_download_progress.lock()
        .ok()
        .and_then(|prog| *prog)
        .map(|(dl, total)| {
            let pct = if total > 0 {
                ((dl as f64 / total as f64) * 100.0).min(100.0) as u8
            } else {
                0u8
            };
            (true, pct)
        })
        .unwrap_or((false, 0u8));

    ModelStatus { ready, downloading, progress_pct }
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

    // Compute embeddings for each node if model is ready
    if let Ok(mut model_lock) = state.embedding_model.lock() {
        if let Some(model) = model_lock.as_mut() {
            for node in &result.nodes {
                let text = format!("{} {}",
                    node.label,
                    node.content.as_deref().unwrap_or(""));
                if let Ok(embedding) = model.embed(&text) {
                    let _ = graph::update_node_embedding(&db, &node.id, &embedding);
                }
            }
        }
    }

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

    // Compute embeddings for each node if model is ready
    if let Ok(mut model_lock) = state.embedding_model.lock() {
        if let Some(model) = model_lock.as_mut() {
            for node in &result.nodes {
                let text = format!("{} {}",
                    node.label,
                    node.content.as_deref().unwrap_or(""));
                if let Ok(embedding) = model.embed(&text) {
                    let _ = graph::update_node_embedding(&db, &node.id, &embedding);
                }
            }
        }
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

            // Shared state for embedding model and MCP connections
            let embedding_model: Arc<Mutex<Option<embeddings::EmbeddingModel>>> =
                Arc::new(Mutex::new(None));
            let model_download_progress: Arc<Mutex<Option<(u64, u64)>>> =
                Arc::new(Mutex::new(None));
            let mcp_connections: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));

            // Spawn MCP server
            let db_path_str = db_path.to_string_lossy().to_string();
            let model_for_mcp = Arc::clone(&embedding_model);
            let conns_for_mcp = Arc::clone(&mcp_connections);
            tauri::async_runtime::spawn(async move {
                mcp_server::start_mcp_server(db_path_str, mcp_port, model_for_mcp, conns_for_mcp).await;
            });

            // Spawn background task: download model if needed, then load it
            let model_for_task = Arc::clone(&embedding_model);
            let progress_for_task = Arc::clone(&model_download_progress);
            let data_dir_for_task = data_dir.clone();
            tauri::async_runtime::spawn(async move {
                if !embeddings::models_exist(&data_dir_for_task) {
                    let prog = Arc::clone(&progress_for_task);
                    if let Err(e) = embeddings::download_model_files(
                        &data_dir_for_task,
                        move |dl, total| {
                            if let Ok(mut p) = prog.lock() {
                                *p = Some((dl, total));
                            }
                        },
                    ).await {
                        eprintln!("[Cortex] Model download failed: {}", e);
                        return;
                    }
                }

                // Clear download progress
                if let Ok(mut p) = progress_for_task.lock() {
                    *p = None;
                }

                let (model_path, tokenizer_path) = embeddings::model_paths(&data_dir_for_task);
                match embeddings::EmbeddingModel::load(&model_path, &tokenizer_path) {
                    Ok(m) => {
                        if let Ok(mut lock) = model_for_task.lock() {
                            *lock = Some(m);
                        }
                        println!("[Cortex] Embedding model ready");
                    }
                    Err(e) => eprintln!("[Cortex] Embedding model load failed: {}", e),
                }
            });

            app.manage(AppState {
                db: Mutex::new(conn),
                vault_path,
                mcp_port,
                embedding_model,
                model_download_progress,
                mcp_connections,
            });

            // Set up system tray
            setup_tray(app.handle())?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_mcp_port,
            get_mcp_connections,
            get_model_status,
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
