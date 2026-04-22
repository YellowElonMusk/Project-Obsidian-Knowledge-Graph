use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub node_type: String,
    pub content: Option<String>,
    pub file_path: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub relationship: String,
    pub weight: f64,
    pub created_at: i64,
    pub metadata: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphData {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeDetail {
    pub node: Node,
    pub neighbors: Vec<Node>,
    pub edges: Vec<Edge>,
}

pub fn init_db(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch("
        PRAGMA journal_mode=WAL;
        PRAGMA foreign_keys=ON;

        CREATE TABLE IF NOT EXISTS nodes (
            id TEXT PRIMARY KEY,
            label TEXT NOT NULL,
            node_type TEXT NOT NULL,
            content TEXT,
            file_path TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            metadata TEXT
        );

        CREATE TABLE IF NOT EXISTS edges (
            id TEXT PRIMARY KEY,
            source_id TEXT NOT NULL,
            target_id TEXT NOT NULL,
            relationship TEXT NOT NULL,
            weight REAL DEFAULT 1.0,
            created_at INTEGER NOT NULL,
            metadata TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_edges_source ON edges(source_id);
        CREATE INDEX IF NOT EXISTS idx_edges_target ON edges(target_id);
        CREATE INDEX IF NOT EXISTS idx_nodes_type ON nodes(node_type);
        CREATE INDEX IF NOT EXISTS idx_nodes_created ON nodes(created_at);

        CREATE VIRTUAL TABLE IF NOT EXISTS nodes_fts USING fts5(
            node_id UNINDEXED,
            label,
            content,
            tokenize='porter ascii'
        );
    ")?;
    Ok(())
}

pub fn create_node(
    label: &str,
    node_type: &str,
    content: Option<String>,
    file_path: Option<String>,
    metadata: Option<String>,
) -> Node {
    let now = Utc::now().timestamp_millis();
    Node {
        id: Uuid::new_v4().to_string(),
        label: label.to_string(),
        node_type: node_type.to_string(),
        content,
        file_path,
        created_at: now,
        updated_at: now,
        metadata,
    }
}

pub fn create_edge(source_id: &str, target_id: &str, relationship: &str, weight: f64) -> Edge {
    Edge {
        id: Uuid::new_v4().to_string(),
        source_id: source_id.to_string(),
        target_id: target_id.to_string(),
        relationship: relationship.to_string(),
        weight,
        created_at: Utc::now().timestamp_millis(),
        metadata: None,
    }
}

pub fn insert_node(conn: &Connection, node: &Node) -> anyhow::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO nodes (id, label, node_type, content, file_path, created_at, updated_at, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            node.id, node.label, node.node_type, node.content,
            node.file_path, node.created_at, node.updated_at, node.metadata
        ],
    )?;
    // Keep FTS index in sync
    conn.execute(
        "DELETE FROM nodes_fts WHERE node_id = ?1",
        params![node.id],
    )?;
    conn.execute(
        "INSERT INTO nodes_fts (node_id, label, content) VALUES (?1, ?2, ?3)",
        params![node.id, node.label, node.content.as_deref().unwrap_or("")],
    )?;
    Ok(())
}

pub fn insert_edge(conn: &Connection, edge: &Edge) -> anyhow::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO edges (id, source_id, target_id, relationship, weight, created_at, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            edge.id, edge.source_id, edge.target_id, edge.relationship,
            edge.weight, edge.created_at, edge.metadata
        ],
    )?;
    Ok(())
}

pub fn get_graph_data(conn: &Connection) -> anyhow::Result<GraphData> {
    let nodes = get_all_nodes(conn)?;
    let edges = get_all_edges(conn)?;
    Ok(GraphData { nodes, edges })
}

fn get_all_nodes(conn: &Connection) -> anyhow::Result<Vec<Node>> {
    let mut stmt = conn.prepare(
        "SELECT id, label, node_type, content, file_path, created_at, updated_at, metadata
         FROM nodes ORDER BY created_at DESC LIMIT 500"
    )?;
    let nodes = stmt.query_map([], |row| {
        Ok(Node {
            id: row.get(0)?,
            label: row.get(1)?,
            node_type: row.get(2)?,
            content: row.get(3)?,
            file_path: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            metadata: row.get(7)?,
        })
    })?.collect::<Result<Vec<_>>>()?;
    Ok(nodes)
}

fn get_all_edges(conn: &Connection) -> anyhow::Result<Vec<Edge>> {
    let mut stmt = conn.prepare(
        "SELECT id, source_id, target_id, relationship, weight, created_at, metadata FROM edges"
    )?;
    let edges = stmt.query_map([], |row| {
        Ok(Edge {
            id: row.get(0)?,
            source_id: row.get(1)?,
            target_id: row.get(2)?,
            relationship: row.get(3)?,
            weight: row.get(4)?,
            created_at: row.get(5)?,
            metadata: row.get(6)?,
        })
    })?.collect::<Result<Vec<_>>>()?;
    Ok(edges)
}

pub fn search_nodes(conn: &Connection, query: &str) -> anyhow::Result<Vec<Node>> {
    // Try FTS5 first
    let fts_result = conn.prepare(
        "SELECT n.id, n.label, n.node_type, n.content, n.file_path, n.created_at, n.updated_at, n.metadata
         FROM nodes n
         JOIN nodes_fts f ON n.id = f.node_id
         WHERE nodes_fts MATCH ?1
         ORDER BY rank
         LIMIT 20"
    );

    match fts_result {
        Ok(mut stmt) => {
            match stmt.query_map(params![query], |row| {
                Ok(Node {
                    id: row.get(0)?,
                    label: row.get(1)?,
                    node_type: row.get(2)?,
                    content: row.get(3)?,
                    file_path: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    metadata: row.get(7)?,
                })
            }) {
                Ok(rows) => {
                    let results = rows.collect::<Result<Vec<_>>>()?;
                    if !results.is_empty() {
                        return Ok(results);
                    }
                }
                Err(_) => {}
            }
        }
        Err(_) => {}
    }

    // Fallback: LIKE search
    let pattern = format!("%{}%", query);
    let mut stmt = conn.prepare(
        "SELECT id, label, node_type, content, file_path, created_at, updated_at, metadata
         FROM nodes WHERE label LIKE ?1 OR content LIKE ?1
         ORDER BY created_at DESC LIMIT 20"
    )?;
    let nodes = stmt.query_map(params![pattern], |row| {
        Ok(Node {
            id: row.get(0)?,
            label: row.get(1)?,
            node_type: row.get(2)?,
            content: row.get(3)?,
            file_path: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            metadata: row.get(7)?,
        })
    })?.collect::<Result<Vec<_>>>()?;
    Ok(nodes)
}

pub fn get_node(conn: &Connection, node_id: &str) -> anyhow::Result<Option<Node>> {
    let mut stmt = conn.prepare(
        "SELECT id, label, node_type, content, file_path, created_at, updated_at, metadata
         FROM nodes WHERE id = ?1"
    )?;
    let mut rows = stmt.query(params![node_id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Node {
            id: row.get(0)?,
            label: row.get(1)?,
            node_type: row.get(2)?,
            content: row.get(3)?,
            file_path: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            metadata: row.get(7)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_node_detail(conn: &Connection, node_id: &str) -> anyhow::Result<Option<NodeDetail>> {
    let node = match get_node(conn, node_id)? {
        Some(n) => n,
        None => return Ok(None),
    };

    let mut edge_stmt = conn.prepare(
        "SELECT id, source_id, target_id, relationship, weight, created_at, metadata
         FROM edges WHERE source_id = ?1 OR target_id = ?1"
    )?;
    let edges: Vec<Edge> = edge_stmt.query_map(params![node_id], |row| {
        Ok(Edge {
            id: row.get(0)?,
            source_id: row.get(1)?,
            target_id: row.get(2)?,
            relationship: row.get(3)?,
            weight: row.get(4)?,
            created_at: row.get(5)?,
            metadata: row.get(6)?,
        })
    })?.collect::<Result<Vec<_>>>()?;

    let mut neighbor_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for edge in &edges {
        if edge.source_id != node_id {
            neighbor_ids.insert(edge.source_id.clone());
        }
        if edge.target_id != node_id {
            neighbor_ids.insert(edge.target_id.clone());
        }
    }

    let mut neighbors = Vec::new();
    for id in &neighbor_ids {
        if let Some(n) = get_node(conn, id)? {
            neighbors.push(n);
        }
    }

    Ok(Some(NodeDetail { node, neighbors, edges }))
}

pub fn get_projects(conn: &Connection) -> anyhow::Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT json_extract(metadata, '$.project') as project
         FROM nodes
         WHERE json_extract(metadata, '$.project') IS NOT NULL
         ORDER BY project"
    )?;
    let projects: Vec<String> = stmt.query_map([], |row| {
        row.get(0)
    })?.collect::<Result<Vec<_>>>()?;
    Ok(projects)
}

pub fn get_project_nodes(conn: &Connection, project: &str) -> anyhow::Result<Vec<Node>> {
    let mut stmt = conn.prepare(
        "SELECT id, label, node_type, content, file_path, created_at, updated_at, metadata
         FROM nodes
         WHERE json_extract(metadata, '$.project') = ?1
         ORDER BY created_at DESC
         LIMIT 100"
    )?;
    let nodes = stmt.query_map(params![project], |row| {
        Ok(Node {
            id: row.get(0)?,
            label: row.get(1)?,
            node_type: row.get(2)?,
            content: row.get(3)?,
            file_path: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            metadata: row.get(7)?,
        })
    })?.collect::<Result<Vec<_>>>()?;
    Ok(nodes)
}

pub fn get_last_session(conn: &Connection, project: &str) -> anyhow::Result<Option<Node>> {
    let mut stmt = conn.prepare(
        "SELECT id, label, node_type, content, file_path, created_at, updated_at, metadata
         FROM nodes
         WHERE node_type IN ('session', 'agent_memory')
           AND json_extract(metadata, '$.project') = ?1
         ORDER BY created_at DESC
         LIMIT 1"
    )?;
    let mut rows = stmt.query(params![project])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Node {
            id: row.get(0)?,
            label: row.get(1)?,
            node_type: row.get(2)?,
            content: row.get(3)?,
            file_path: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            metadata: row.get(7)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_recent_sessions(conn: &Connection, limit: usize) -> anyhow::Result<Vec<Node>> {
    let mut stmt = conn.prepare(
        "SELECT id, label, node_type, content, file_path, created_at, updated_at, metadata
         FROM nodes
         WHERE node_type IN ('session', 'agent_memory')
         ORDER BY created_at DESC
         LIMIT ?1"
    )?;
    let nodes = stmt.query_map(params![limit as i64], |row| {
        Ok(Node {
            id: row.get(0)?,
            label: row.get(1)?,
            node_type: row.get(2)?,
            content: row.get(3)?,
            file_path: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            metadata: row.get(7)?,
        })
    })?.collect::<Result<Vec<_>>>()?;
    Ok(nodes)
}
