use std::path::Path;
use std::fs;
use regex::Regex;
use once_cell::sync::Lazy;
use serde_json::json;
use crate::graph::{Node, Edge, create_node, create_edge};

static HEADING_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^#{1,6}\s+(.+)$").unwrap()
});
static TASK_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?im)^\s*[-*]\s+\[([ x])\]\s+(.+)$|(?im)\b(TODO|FIXME|HACK|NOTE|IMPORTANT):?\s*(.+)$").unwrap()
});
static DECISION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(decided|decision|resolved|agreed|chosen|picked|conclusion):?\s*(.{5,100})").unwrap()
});
static PERSON_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b([A-Z][a-z]{1,15}(?:\s+[A-Z][a-z]{1,15}){1,3})\b").unwrap()
});
static CODE_BLOCK_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"```(\w*)\n([\s\S]*?)```").unwrap()
});
static LINK_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[\[([^\]]+)\]\]|\[([^\]]+)\]\(([^)]+)\)").unwrap()
});

pub struct IngestResult {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub vault_markdown: String,
}

pub fn ingest_file(file_path: &str, project: &str) -> anyhow::Result<IngestResult> {
    let path = Path::new(file_path);
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let content = match ext.as_str() {
        "pdf" => extract_pdf_text(file_path)?,
        _ => fs::read_to_string(file_path)
            .map_err(|e| anyhow::anyhow!("Cannot read file: {}", e))?,
    };

    let title = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Untitled")
        .to_string();

    ingest_content(&title, file_path, &content, project, &ext)
}

pub fn ingest_content(
    title: &str,
    file_path: &str,
    content: &str,
    project: &str,
    file_type: &str,
) -> anyhow::Result<IngestResult> {
    let mut nodes: Vec<Node> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();

    // Create the root file node
    let file_metadata = json!({
        "project": project,
        "file_type": file_type,
        "word_count": content.split_whitespace().count(),
        "char_count": content.len(),
    }).to_string();

    let file_node = create_node(
        title,
        "file",
        Some(truncate(content, 8000)),
        Some(file_path.to_string()),
        Some(file_metadata),
    );
    let file_id = file_node.id.clone();
    nodes.push(file_node);

    // --- Extract: Headings as concept nodes ---
    for cap in HEADING_RE.captures_iter(content) {
        let text = cap[1].trim().to_string();
        if text.len() > 2 && text.len() < 120 {
            let n = create_node(
                &text,
                "concept",
                None,
                None,
                Some(json!({"project": project}).to_string()),
            );
            edges.push(create_edge(&file_id, &n.id, "contains", 1.0));
            nodes.push(n);
        }
    }

    // --- Extract: Wikilinks and markdown links ---
    for cap in LINK_RE.captures_iter(content) {
        let linked = cap.get(1).or(cap.get(2)).map(|m| m.as_str()).unwrap_or("");
        if !linked.is_empty() && linked.len() < 100 {
            let n = create_node(
                linked,
                "concept",
                None,
                None,
                Some(json!({"project": project, "linked": true}).to_string()),
            );
            edges.push(create_edge(&file_id, &n.id, "references", 0.9));
            nodes.push(n);
        }
    }

    // --- Extract: Tasks (checklist items and TODO/FIXME) ---
    let task_re = Regex::new(r"(?im)^\s*[-*]\s+\[([ x])\]\s+(.+)$").unwrap();
    for cap in task_re.captures_iter(content) {
        let done = &cap[1] != " ";
        let text = cap[2].trim().to_string();
        if text.len() > 2 {
            let label = format!("[{}] {}", if done { "x" } else { " " }, &text[..text.len().min(80)]);
            let n = create_node(
                &label,
                "task",
                Some(text.clone()),
                None,
                Some(json!({"project": project, "done": done}).to_string()),
            );
            edges.push(create_edge(&file_id, &n.id, "contains-task", 1.0));
            nodes.push(n);
        }
    }

    let todo_re = Regex::new(r"(?im)\b(TODO|FIXME|HACK|NOTE|IMPORTANT):?\s*(.+)$").unwrap();
    for cap in todo_re.captures_iter(content) {
        let kind = cap[1].to_string();
        let text = cap[2].trim().to_string();
        if text.len() > 3 {
            let label = format!("[{}] {}", kind, &text[..text.len().min(70)]);
            let n = create_node(
                &label,
                "task",
                Some(text),
                None,
                Some(json!({"project": project, "task_kind": kind}).to_string()),
            );
            edges.push(create_edge(&file_id, &n.id, "contains-task", 0.9));
            nodes.push(n);
        }
    }

    // --- Extract: Decisions ---
    for cap in DECISION_RE.captures_iter(content) {
        let text = cap[2].trim().to_string();
        if text.len() > 5 {
            let label = format!("Decision: {}", &text[..text.len().min(70)]);
            let n = create_node(
                &label,
                "decision",
                Some(text),
                None,
                Some(json!({"project": project}).to_string()),
            );
            edges.push(create_edge(&file_id, &n.id, "contains-decision", 1.0));
            nodes.push(n);
        }
    }

    // --- Extract: Code blocks (for code files, label the language) ---
    if matches!(file_type, "md" | "markdown" | "txt" | "text") {
        for cap in CODE_BLOCK_RE.captures_iter(content) {
            let lang = cap[1].trim();
            let code = cap[2].trim();
            if !lang.is_empty() && code.len() > 20 {
                let label = format!("Code[{}]: {}", lang, &code[..code.len().min(50)]);
                let n = create_node(
                    &label,
                    "code",
                    Some(code[..code.len().min(2000)].to_string()),
                    None,
                    Some(json!({"project": project, "language": lang}).to_string()),
                );
                edges.push(create_edge(&file_id, &n.id, "contains-code", 0.8));
                nodes.push(n);
            }
        }
    }

    // --- Extract: People (proper name heuristic) ---
    // Only for prose-like files
    if matches!(file_type, "md" | "markdown" | "txt" | "text" | "pdf") {
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for cap in PERSON_RE.captures_iter(content) {
            let name = cap[1].trim().to_string();
            // Filter common false positives
            if name.len() > 5
                && name.len() < 50
                && !is_common_word(&name)
                && !seen.contains(&name)
            {
                seen.insert(name.clone());
                let n = create_node(
                    &name,
                    "person",
                    None,
                    None,
                    Some(json!({"project": project}).to_string()),
                );
                edges.push(create_edge(&file_id, &n.id, "mentions", 0.7));
                nodes.push(n);
            }
        }
    }

    let vault_md = generate_vault_markdown(title, file_path, content, &nodes);
    Ok(IngestResult { nodes, edges, vault_markdown: vault_md })
}

fn is_common_word(s: &str) -> bool {
    matches!(
        s,
        "The" | "This" | "That" | "These" | "Those" | "There" | "Their" |
        "They" | "What" | "When" | "Where" | "Which" | "While" | "With" |
        "From" | "Have" | "Been" | "Will" | "Would" | "Could" | "Should" |
        "More" | "Some" | "Each" | "Such" | "Into" | "About" | "After" |
        "Before" | "Under" | "Over" | "Also" | "Here" | "Just" | "Like" |
        "Make" | "Made" | "Many" | "Much" | "Most" | "Only" | "Other" |
        "Same" | "Your" | "Our" | "His" | "Her" | "Its" | "Both" |
        "True" | "False" | "Note" | "Create" | "Update" | "Delete" |
        "Figure" | "Table" | "Section" | "Chapter" | "Example"
    )
}

fn extract_pdf_text(path: &str) -> anyhow::Result<String> {
    let bytes = fs::read(path)?;
    // Attempt to extract text from PDF streams
    // This is a minimal heuristic extractor for plain PDFs
    let mut result = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'(' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] != b')' {
                if bytes[j] >= 32 && bytes[j] < 127 {
                    result.push(bytes[j] as char);
                }
                j += 1;
            }
            result.push(' ');
            i = j;
        }
        i += 1;
    }
    if result.trim().is_empty() {
        Ok(format!("[Binary PDF: {} bytes — text extraction limited]", bytes.len()))
    } else {
        Ok(result)
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

fn generate_vault_markdown(title: &str, file_path: &str, content: &str, nodes: &[Node]) -> String {
    let mut md = format!("# {}\n\n", title);
    md.push_str(&format!("> Source: `{}`\n\n", file_path));

    let concepts: Vec<_> = nodes.iter()
        .filter(|n| n.node_type == "concept")
        .collect();
    let tasks: Vec<_> = nodes.iter()
        .filter(|n| n.node_type == "task")
        .collect();
    let decisions: Vec<_> = nodes.iter()
        .filter(|n| n.node_type == "decision")
        .collect();
    let people: Vec<_> = nodes.iter()
        .filter(|n| n.node_type == "person")
        .collect();

    if !concepts.is_empty() {
        md.push_str("## Concepts\n");
        for n in &concepts {
            md.push_str(&format!("- [[{}]]\n", n.label));
        }
        md.push('\n');
    }
    if !people.is_empty() {
        md.push_str("## People\n");
        for n in &people {
            md.push_str(&format!("- [[{}]]\n", n.label));
        }
        md.push('\n');
    }
    if !tasks.is_empty() {
        md.push_str("## Tasks\n");
        for n in &tasks {
            md.push_str(&format!("- {}\n", n.label));
        }
        md.push('\n');
    }
    if !decisions.is_empty() {
        md.push_str("## Decisions\n");
        for n in &decisions {
            md.push_str(&format!("- {}\n", n.label));
        }
        md.push('\n');
    }

    md.push_str("## Content\n\n");
    if content.len() > 6000 {
        md.push_str(&content[..6000]);
        md.push_str("\n\n…[truncated]");
    } else {
        md.push_str(content);
    }
    md
}
