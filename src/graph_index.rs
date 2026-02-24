//! Sled-backed knowledge graph index.
//!
//! Maintains a materialized view of the knowledge graph in two sled trees:
//! - `kg:edges`: edge triples (source, target, type) → weight
//! - `kg:nodes`: note key → serialized IndexedNode metadata
//!
//! This replaces the expensive O(N × content_size) scan in `build_knowledge_graph`
//! with instant sled reads, while keeping the index in sync via incremental updates.

use crate::models::{CitationScanResult, Note, NoteType};
use crate::notes::extract_references;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

const EDGES_TREE: &str = "kg:edges";
const NODES_TREE: &str = "kg:nodes";
const CITATIONS_TREE: &str = "citations";
const MANUAL_EDGES_TREE: &str = "kg:manual_edges";

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedNode {
    pub title: String,
    pub node_type: String,
    pub short_label: String,
    pub date: Option<String>,
    pub time_total: u32,
    pub primary_category: Option<String>,
    pub parent_key: Option<String>,
    pub hidden: bool,
    pub modified: String,
    pub content_hash: String,
}

#[derive(Debug, Clone)]
pub struct IndexedEdge {
    pub source: String,
    pub target: String,
    pub edge_type: String,
    pub weight: u32,
}

pub struct ReconcileStats {
    pub reindexed: usize,
    pub removed: usize,
    pub unchanged: usize,
}

// ============================================================================
// Edge Key Encoding
// ============================================================================

fn encode_edge_key(source: &str, target: &str, edge_type: &str) -> Vec<u8> {
    let mut key = Vec::with_capacity(source.len() + target.len() + edge_type.len() + 2);
    key.extend_from_slice(source.as_bytes());
    key.push(0);
    key.extend_from_slice(target.as_bytes());
    key.push(0);
    key.extend_from_slice(edge_type.as_bytes());
    key
}

fn decode_edge_key(key: &[u8]) -> Option<(String, String, String)> {
    let parts: Vec<&[u8]> = key.split(|&b| b == 0).collect();
    if parts.len() != 3 {
        return None;
    }
    Some((
        String::from_utf8_lossy(parts[0]).to_string(),
        String::from_utf8_lossy(parts[1]).to_string(),
        String::from_utf8_lossy(parts[2]).to_string(),
    ))
}

// ============================================================================
// Helpers
// ============================================================================

fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn compute_short_label(note: &Note) -> String {
    if let NoteType::Paper(ref meta) = note.note_type {
        let eff = meta.effective_metadata(&note.title);
        if let Some(ref authors) = eff.authors {
            let first_author = authors
                .split(" and ")
                .next()
                .unwrap_or(authors)
                .split(',')
                .next()
                .unwrap_or(authors)
                .trim();
            let last_name = first_author
                .split_whitespace()
                .filter(|w| w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false))
                .last()
                .unwrap_or(first_author);
            if authors.contains(" and ") {
                return format!("{} et al.", last_name);
            } else {
                return last_name.to_string();
            }
        }
    }
    let t = &note.title;
    if t.len() > 16 {
        format!("{}…", &t[..16])
    } else {
        t.clone()
    }
}

/// Extract all edges for a single note: crosslinks from [@key] references and parent edge.
fn extract_edges_for_note(note: &Note, all_keys: &std::collections::HashSet<String>) -> Vec<(String, String, String, u32)> {
    let mut edges: HashMap<(String, String, String), u32> = HashMap::new();

    // Crosslinks from [@key] references
    let refs = extract_references(&note.full_file_content);
    for r in refs {
        if r != note.key && all_keys.contains(&r) {
            let key = (note.key.clone(), r, "crosslink".to_string());
            *edges.entry(key).or_insert(0) += 1;
        }
    }

    // Parent relationship
    if let Some(ref parent) = note.parent_key {
        if all_keys.contains(parent) {
            let key = (note.key.clone(), parent.clone(), "parent".to_string());
            *edges.entry(key).or_insert(0) += 1;
        }
    }

    edges
        .into_iter()
        .map(|((s, t, ty), w)| (s, t, ty, w))
        .collect()
}

fn build_indexed_node(note: &Note) -> IndexedNode {
    let node_type = match note.note_type {
        NoteType::Paper(_) => "paper",
        NoteType::Note => "note",
    };
    let time_total: u32 = note.time_entries.iter().map(|e| e.minutes).sum();
    let primary_category = note
        .time_entries
        .iter()
        .max_by_key(|e| e.minutes)
        .map(|e| e.category.to_string());

    IndexedNode {
        title: note.title.clone(),
        node_type: node_type.to_string(),
        short_label: compute_short_label(note),
        date: note.date.map(|d| d.to_string()),
        time_total,
        primary_category,
        parent_key: note.parent_key.clone(),
        hidden: note.hidden,
        modified: note.modified.to_rfc3339(),
        content_hash: content_hash(&note.full_file_content),
    }
}

// ============================================================================
// Edge CRUD
// ============================================================================

fn delete_edges_by_source(edges_tree: &sled::Tree, source: &str) -> sled::Result<()> {
    let mut prefix = source.as_bytes().to_vec();
    prefix.push(0);
    let to_remove: Vec<sled::IVec> = edges_tree
        .scan_prefix(&prefix)
        .filter_map(|r| r.ok())
        .map(|(k, _)| k)
        .collect();
    for k in to_remove {
        edges_tree.remove(&k)?;
    }
    Ok(())
}

fn delete_edges_by_target(edges_tree: &sled::Tree, target: &str) -> sled::Result<()> {
    // Full scan — fine for ~2000 edges
    let target_bytes = target.as_bytes();
    let to_remove: Vec<sled::IVec> = edges_tree
        .iter()
        .filter_map(|r| r.ok())
        .filter(|(k, _)| {
            // Check if the target segment matches
            let parts: Vec<&[u8]> = k.split(|&b| b == 0).collect();
            parts.len() == 3 && parts[1] == target_bytes
        })
        .map(|(k, _)| k)
        .collect();
    for k in to_remove {
        edges_tree.remove(&k)?;
    }
    Ok(())
}

fn insert_edge(edges_tree: &sled::Tree, source: &str, target: &str, edge_type: &str, weight: u32) -> sled::Result<()> {
    let key = encode_edge_key(source, target, edge_type);
    edges_tree.insert(key, &weight.to_le_bytes())?;
    Ok(())
}

// ============================================================================
// Lifecycle: Reconcile, Reindex, Remove
// ============================================================================

/// Full reconciliation at startup. Compares sled index with notes on disk.
pub fn reconcile(db: &sled::Db, notes: &[Note]) -> Result<ReconcileStats, String> {
    use rayon::prelude::*;

    let edges_tree = db.open_tree(EDGES_TREE).map_err(|e| e.to_string())?;
    let nodes_tree = db.open_tree(NODES_TREE).map_err(|e| e.to_string())?;

    let all_keys: std::collections::HashSet<String> = notes.iter().map(|n| n.key.clone()).collect();
    let notes_map: HashMap<String, &Note> = notes.iter().map(|n| (n.key.clone(), n)).collect();

    // Parallel: compute hashes, check staleness, build nodes + extract edges for changed notes
    let note_updates: Vec<(String, IndexedNode, Vec<(String, String, String, u32)>)> = notes
        .par_iter()
        .filter_map(|note| {
            let hash = content_hash(&note.full_file_content);
            let needs_reindex = match nodes_tree.get(note.key.as_bytes()) {
                Ok(Some(data)) => match serde_json::from_slice::<IndexedNode>(&data) {
                    Ok(existing) => existing.content_hash != hash,
                    Err(_) => true,
                },
                _ => true,
            };

            if needs_reindex {
                let indexed = build_indexed_node(note);
                let new_edges = extract_edges_for_note(note, &all_keys);
                Some((note.key.clone(), indexed, new_edges))
            } else {
                None
            }
        })
        .collect();

    let reindexed = note_updates.len();
    let unchanged = notes.len() - reindexed;

    // Sequential: batch sled writes
    for (key, indexed, new_edges) in &note_updates {
        let json = serde_json::to_vec(indexed).map_err(|e| e.to_string())?;
        nodes_tree
            .insert(key.as_bytes(), json)
            .map_err(|e| e.to_string())?;

        delete_edges_by_source(&edges_tree, key).map_err(|e| e.to_string())?;
        for (s, t, ty, w) in new_edges {
            insert_edge(&edges_tree, s, t, ty, *w).map_err(|e| e.to_string())?;
        }
    }

    // Remove nodes/edges for deleted notes (in sled but not on disk)
    let stale_keys: Vec<String> = nodes_tree
        .iter()
        .filter_map(|r| r.ok())
        .filter_map(|(k, _)| {
            let key = String::from_utf8_lossy(&k).to_string();
            if !notes_map.contains_key(&key) {
                Some(key)
            } else {
                None
            }
        })
        .collect();

    let removed = stale_keys.len();
    for key in &stale_keys {
        nodes_tree
            .remove(key.as_bytes())
            .map_err(|e| e.to_string())?;
        delete_edges_by_source(&edges_tree, key).map_err(|e| e.to_string())?;
        delete_edges_by_target(&edges_tree, key).map_err(|e| e.to_string())?;
    }

    // Sync all citation edges
    sync_all_citations(db)?;

    Ok(ReconcileStats {
        reindexed,
        removed,
        unchanged,
    })
}

/// Reindex a single note. Returns true if the note was actually updated.
pub fn reindex_note(db: &sled::Db, note: &Note, all_keys: &std::collections::HashSet<String>) -> Result<bool, String> {
    let edges_tree = db.open_tree(EDGES_TREE).map_err(|e| e.to_string())?;
    let nodes_tree = db.open_tree(NODES_TREE).map_err(|e| e.to_string())?;

    let hash = content_hash(&note.full_file_content);

    // Check if unchanged
    if let Ok(Some(data)) = nodes_tree.get(note.key.as_bytes()) {
        if let Ok(existing) = serde_json::from_slice::<IndexedNode>(&data) {
            if existing.content_hash == hash {
                return Ok(false);
            }
        }
    }

    // Update node
    let indexed = build_indexed_node(note);
    let json = serde_json::to_vec(&indexed).map_err(|e| e.to_string())?;
    nodes_tree.insert(note.key.as_bytes(), json).map_err(|e| e.to_string())?;

    // Update edges
    delete_edges_by_source(&edges_tree, &note.key).map_err(|e| e.to_string())?;
    let new_edges = extract_edges_for_note(note, all_keys);
    for (s, t, ty, w) in new_edges {
        insert_edge(&edges_tree, &s, &t, &ty, w).map_err(|e| e.to_string())?;
    }

    Ok(true)
}

/// Remove a note from the graph index.
pub fn remove_note(db: &sled::Db, key: &str) -> Result<(), String> {
    let edges_tree = db.open_tree(EDGES_TREE).map_err(|e| e.to_string())?;
    let nodes_tree = db.open_tree(NODES_TREE).map_err(|e| e.to_string())?;

    nodes_tree.remove(key.as_bytes()).map_err(|e| e.to_string())?;
    delete_edges_by_source(&edges_tree, key).map_err(|e| e.to_string())?;
    delete_edges_by_target(&edges_tree, key).map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Citation Sync
// ============================================================================

/// Sync citation edges for a single note from the citations cache tree into kg:edges.
pub fn sync_citations(db: &sled::Db, source_key: &str) -> Result<(), String> {
    let edges_tree = db.open_tree(EDGES_TREE).map_err(|e| e.to_string())?;
    let nodes_tree = db.open_tree(NODES_TREE).map_err(|e| e.to_string())?;
    let citations_tree = db.open_tree(CITATIONS_TREE).map_err(|e| e.to_string())?;

    // First, remove existing citation edges from this source
    let prefix = {
        let mut p = source_key.as_bytes().to_vec();
        p.push(0);
        p
    };
    let citation_suffix = b"citation";
    let to_remove: Vec<sled::IVec> = edges_tree
        .scan_prefix(&prefix)
        .filter_map(|r| r.ok())
        .filter(|(k, _)| {
            let parts: Vec<&[u8]> = k.split(|&b| b == 0).collect();
            parts.len() == 3 && parts[2] == citation_suffix
        })
        .map(|(k, _)| k)
        .collect();
    for k in to_remove {
        edges_tree.remove(&k).map_err(|e| e.to_string())?;
    }

    // Load citation scan result for this note
    if let Ok(Some(data)) = citations_tree.get(source_key.as_bytes()) {
        if let Ok(result) = serde_json::from_slice::<CitationScanResult>(&data) {
            for m in &result.matches {
                // Only add if both source and target exist in the index
                let source_exists = nodes_tree.contains_key(source_key.as_bytes()).unwrap_or(false);
                let target_exists = nodes_tree.contains_key(m.target_key.as_bytes()).unwrap_or(false);
                if source_exists && target_exists {
                    // Only add citation edge if no existing crosslink/parent edge exists
                    let crosslink_key = encode_edge_key(source_key, &m.target_key, "crosslink");
                    let parent_key = encode_edge_key(source_key, &m.target_key, "parent");
                    let has_existing = edges_tree.contains_key(&crosslink_key).unwrap_or(false)
                        || edges_tree.contains_key(&parent_key).unwrap_or(false);
                    if !has_existing {
                        insert_edge(&edges_tree, source_key, &m.target_key, "citation", 1)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Sync all citation edges from the citations tree.
pub fn sync_all_citations(db: &sled::Db) -> Result<usize, String> {
    let citations_tree = db.open_tree(CITATIONS_TREE).map_err(|e| e.to_string())?;

    let source_keys: Vec<String> = citations_tree
        .iter()
        .filter_map(|r| r.ok())
        .filter_map(|(k, _)| String::from_utf8(k.to_vec()).ok())
        .collect();

    let count = source_keys.len();
    for key in &source_keys {
        sync_citations(db, key)?;
    }
    Ok(count)
}

// ============================================================================
// Manual Edges
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualEdgeValue {
    pub annotation: Option<String>,
    pub created: String,
}

/// Add a manual edge between two notes.
pub fn add_manual_edge(db: &sled::Db, source: &str, target: &str, annotation: Option<String>) -> Result<(), String> {
    let tree = db.open_tree(MANUAL_EDGES_TREE).map_err(|e| e.to_string())?;
    let key = format!("{}\0{}", source, target);
    let value = ManualEdgeValue {
        annotation,
        created: chrono::Utc::now().to_rfc3339(),
    };
    let json = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
    tree.insert(key.as_bytes(), json).map_err(|e| e.to_string())?;
    Ok(())
}

/// Remove a manual edge.
pub fn remove_manual_edge(db: &sled::Db, source: &str, target: &str) -> Result<(), String> {
    let tree = db.open_tree(MANUAL_EDGES_TREE).map_err(|e| e.to_string())?;
    let key = format!("{}\0{}", source, target);
    tree.remove(key.as_bytes()).map_err(|e| e.to_string())?;
    Ok(())
}

/// Load all manual edges.
pub fn load_manual_edges(db: &sled::Db) -> Result<Vec<IndexedEdge>, String> {
    let tree = db.open_tree(MANUAL_EDGES_TREE).map_err(|e| e.to_string())?;
    let mut edges = Vec::new();

    for entry in tree.iter() {
        let (k, _v) = entry.map_err(|e| e.to_string())?;
        let key_str = String::from_utf8_lossy(&k);
        if let Some((source, target)) = key_str.split_once('\0') {
            edges.push(IndexedEdge {
                source: source.to_string(),
                target: target.to_string(),
                edge_type: "manual".to_string(),
                weight: 1,
            });
        }
    }

    Ok(edges)
}

/// Load annotation for a manual edge, if it exists.
pub fn get_manual_edge_annotation(db: &sled::Db, source: &str, target: &str) -> Result<Option<String>, String> {
    let tree = db.open_tree(MANUAL_EDGES_TREE).map_err(|e| e.to_string())?;
    let key = format!("{}\0{}", source, target);
    match tree.get(key.as_bytes()).map_err(|e| e.to_string())? {
        Some(data) => {
            let val: ManualEdgeValue = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
            Ok(val.annotation)
        }
        None => Ok(None),
    }
}

/// Load all manual edge annotations as a map.
pub fn load_manual_edge_annotations(db: &sled::Db) -> Result<HashMap<(String, String), String>, String> {
    let tree = db.open_tree(MANUAL_EDGES_TREE).map_err(|e| e.to_string())?;
    let mut annotations = HashMap::new();

    for entry in tree.iter() {
        let (k, v) = entry.map_err(|e| e.to_string())?;
        let key_str = String::from_utf8_lossy(&k);
        if let Some((source, target)) = key_str.split_once('\0') {
            if let Ok(val) = serde_json::from_slice::<ManualEdgeValue>(&v) {
                if let Some(ann) = val.annotation {
                    annotations.insert((source.to_string(), target.to_string()), ann);
                }
            }
        }
    }

    Ok(annotations)
}

// ============================================================================
// Queries
// ============================================================================

/// Load all edges from the kg:edges tree, including manual edges.
pub fn load_all_edges(db: &sled::Db) -> Result<Vec<IndexedEdge>, String> {
    let edges_tree = db.open_tree(EDGES_TREE).map_err(|e| e.to_string())?;
    let mut edges = Vec::new();

    for entry in edges_tree.iter() {
        let (k, v) = entry.map_err(|e| e.to_string())?;
        if let Some((source, target, edge_type)) = decode_edge_key(&k) {
            let weight = if v.len() == 4 {
                u32::from_le_bytes([v[0], v[1], v[2], v[3]])
            } else {
                1
            };
            edges.push(IndexedEdge {
                source,
                target,
                edge_type,
                weight,
            });
        }
    }

    // Merge manual edges
    if let Ok(manual) = load_manual_edges(db) {
        edges.extend(manual);
    }

    Ok(edges)
}

/// Load all nodes from the kg:nodes tree.
pub fn load_all_nodes(db: &sled::Db) -> Result<HashMap<String, IndexedNode>, String> {
    let nodes_tree = db.open_tree(NODES_TREE).map_err(|e| e.to_string())?;
    let mut nodes = HashMap::new();

    for entry in nodes_tree.iter() {
        let (k, v) = entry.map_err(|e| e.to_string())?;
        let key = String::from_utf8_lossy(&k).to_string();
        if let Ok(node) = serde_json::from_slice::<IndexedNode>(&v) {
            nodes.insert(key, node);
        }
    }

    Ok(nodes)
}
