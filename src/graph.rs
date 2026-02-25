//! Knowledge graph building and visualization.
//!
//! This module handles the construction of the knowledge graph from notes
//! and references, as well as the web-based D3.js visualization.

use crate::auth::is_logged_in;
use crate::graph_index;
use crate::models::{GraphEdge, GraphNode, GraphQuery, GraphStats, KnowledgeGraph};
use crate::notes::html_escape;
use crate::templates::{base_html, render_graph_js, graph_css, GraphRendererConfig, GraphDataSource};
use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use crate::AppState;

// ============================================================================
// Graph Building
// ============================================================================

pub fn build_knowledge_graph(query: &GraphQuery, db: &sled::Db) -> KnowledgeGraph {
    let indexed_nodes = graph_index::load_all_nodes(db).unwrap_or_default();
    let indexed_edges = graph_index::load_all_edges(db).unwrap_or_default();

    // Build raw edge maps
    let mut edge_counts: HashMap<(String, String), usize> = HashMap::new();
    let mut edge_types: HashMap<(String, String), String> = HashMap::new();

    for e in &indexed_edges {
        let key = (e.source.clone(), e.target.clone());
        *edge_counts.entry(key.clone()).or_insert(0) += e.weight as usize;
        edge_types.entry(key).or_insert_with(|| e.edge_type.clone());
    }

    // Calculate degrees
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut out_degree: HashMap<String, usize> = HashMap::new();
    for ((src, tgt), _) in &edge_counts {
        *out_degree.entry(src.clone()).or_insert(0) += 1;
        *in_degree.entry(tgt.clone()).or_insert(0) += 1;
    }

    // Find path if requested
    let path_nodes: HashSet<String> =
        if let (Some(start), Some(end)) = (&query.path_start, &query.path_end) {
            find_shortest_path(&edge_counts, start, end)
        } else {
            HashSet::new()
        };

    // Find nodes within depth if centered
    let reachable: HashSet<String> = if let Some(ref center) = query.center {
        find_reachable(&edge_counts, center, query.depth)
    } else {
        indexed_nodes.keys().cloned().collect()
    };

    // Build nodes with filtering
    let now = Utc::now();
    let mut graph_nodes = Vec::new();

    for (key, node) in &indexed_nodes {
        if !reachable.contains(key) && !path_nodes.contains(key) {
            continue;
        }

        if let Some(ref tf) = query.type_filter {
            if node.node_type != *tf {
                continue;
            }
        }

        if query.has_time && node.time_total == 0 {
            continue;
        }

        let indeg = *in_degree.get(key).unwrap_or(&0);
        let outdeg = *out_degree.get(key).unwrap_or(&0);
        let total_deg = indeg + outdeg;

        if let Some(min) = query.min_links {
            if total_deg <= min {
                continue;
            }
        }
        if let Some(max) = query.max_links {
            if total_deg >= max {
                continue;
            }
        }
        if query.orphans_only && total_deg > 0 {
            continue;
        }
        if query.hubs_only && total_deg < 5 {
            continue;
        }

        if let Some(ref cat_filter) = query.category_filter {
            if node.primary_category.as_deref() != Some(cat_filter) {
                continue;
            }
        }

        if let Some(days) = query.recent_days {
            let cutoff = now - chrono::Duration::days(days);
            if let Ok(modified) = DateTime::parse_from_rfc3339(&node.modified) {
                if modified < cutoff {
                    continue;
                }
            }
        }

        graph_nodes.push(GraphNode {
            id: key.clone(),
            title: node.title.clone(),
            node_type: node.node_type.clone(),
            short_label: node.short_label.clone(),
            date: node.date.clone(),
            time_total: node.time_total,
            primary_category: node.primary_category.clone(),
            in_degree: indeg,
            out_degree: outdeg,
            parent: node.parent_key.clone(),
        });
    }

    // Build edges (only between included nodes)
    let included: HashSet<String> = graph_nodes.iter().map(|n| n.id.clone()).collect();
    let annotations = graph_index::load_manual_edge_annotations(db).unwrap_or_default();
    let mut graph_edges = Vec::new();

    for ((src, tgt), weight) in &edge_counts {
        if included.contains(src) && included.contains(tgt) {
            let etype = edge_types
                .get(&(src.clone(), tgt.clone()))
                .cloned()
                .unwrap_or_else(|| "crosslink".to_string());
            let annotation = annotations.get(&(src.clone(), tgt.clone())).cloned();
            graph_edges.push(GraphEdge {
                source: src.clone(),
                target: tgt.clone(),
                weight: *weight,
                edge_type: etype,
                annotation,
            });
        }
    }

    // Calculate stats
    let total_nodes = graph_nodes.len();
    let total_edges = graph_edges.len();
    let orphan_count = graph_nodes
        .iter()
        .filter(|n| n.in_degree + n.out_degree == 0)
        .count();
    let hub_threshold = 5;
    let hub_count = graph_nodes
        .iter()
        .filter(|n| n.in_degree + n.out_degree >= hub_threshold)
        .count();
    let total_degree: usize = graph_nodes
        .iter()
        .map(|n| n.in_degree + n.out_degree)
        .sum();
    let avg_degree = if total_nodes > 0 {
        total_degree as f64 / total_nodes as f64
    } else {
        0.0
    };
    let max_degree = graph_nodes
        .iter()
        .map(|n| n.in_degree + n.out_degree)
        .max()
        .unwrap_or(0);

    KnowledgeGraph {
        nodes: graph_nodes,
        edges: graph_edges,
        stats: GraphStats {
            total_nodes,
            total_edges,
            orphan_count,
            hub_threshold,
            hub_count,
            avg_degree,
            max_degree,
        },
    }
}

pub fn find_reachable(
    edges: &HashMap<(String, String), usize>,
    start: &str,
    max_depth: usize,
) -> HashSet<String> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back((start.to_string(), 0));
    visited.insert(start.to_string());

    // Build adjacency list (both directions for reachability)
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for (src, tgt) in edges.keys() {
        adj.entry(src.clone()).or_default().push(tgt.clone());
        adj.entry(tgt.clone()).or_default().push(src.clone());
    }

    while let Some((node, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }
        if let Some(neighbors) = adj.get(&node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    visited.insert(neighbor.clone());
                    queue.push_back((neighbor.clone(), depth + 1));
                }
            }
        }
    }

    visited
}

pub fn find_shortest_path(
    edges: &HashMap<(String, String), usize>,
    start: &str,
    end: &str,
) -> HashSet<String> {
    let mut visited = HashSet::new();
    let mut parent: HashMap<String, String> = HashMap::new();
    let mut queue = VecDeque::new();
    queue.push_back(start.to_string());
    visited.insert(start.to_string());

    // Build adjacency list (both directions)
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for (src, tgt) in edges.keys() {
        adj.entry(src.clone()).or_default().push(tgt.clone());
        adj.entry(tgt.clone()).or_default().push(src.clone());
    }

    while let Some(node) = queue.pop_front() {
        if node == end {
            // Reconstruct path
            let mut path = HashSet::new();
            let mut current = end.to_string();
            while current != start {
                path.insert(current.clone());
                if let Some(p) = parent.get(&current) {
                    current = p.clone();
                } else {
                    break;
                }
            }
            path.insert(start.to_string());
            return path;
        }

        if let Some(neighbors) = adj.get(&node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    visited.insert(neighbor.clone());
                    parent.insert(neighbor.clone(), node.clone());
                    queue.push_back(neighbor.clone());
                }
            }
        }
    }

    HashSet::new() // No path found
}

// ============================================================================
// Route Handlers
// ============================================================================

#[derive(Deserialize)]
pub struct GraphQueryParams {
    pub q: Option<String>,
}

pub async fn graph_page(
    Query(params): Query<GraphQueryParams>,
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Html<String> {
    let logged_in = is_logged_in(&jar, &state.db);
    let query_str = params.q.as_deref().unwrap_or("");
    let query = GraphQuery::parse(query_str);
    let graph = crate::graph_query::query_graph(&query, &state.db);
    let has_center = query.center.is_some();

    // Build notes list for autocomplete
    let notes_list: Vec<serde_json::Value> = state.notes_map().values().map(|n| {
        let nt = match n.note_type {
            crate::models::NoteType::Paper(_) => "paper",
            crate::models::NoteType::Note => "note",
        };
        serde_json::json!({"key": n.key, "title": n.title, "node_type": nt})
    }).collect();
    let notes_json = serde_json::to_string(&notes_list).unwrap_or("[]".to_string());

    let graph_json = serde_json::to_string(&graph).unwrap_or("{}".to_string());

    let config = GraphRendererConfig {
        container_selector: "#graph-container".into(),
        center_key: query.center.clone(),
        is_mini: false,
        logged_in,
        show_arrows: has_center,
        show_edge_tooltips: true,
        auto_fit: has_center,
        max_nodes: 0,
        data_source: GraphDataSource::Inline { graph_json },
        notes_json: if logged_in { Some(notes_json) } else { None },
    };

    let graph_script = render_graph_js(&config);
    let graph_styles = graph_css();

    let page_styles = r#"
        .graph-container {
            position: relative;
            border: 1px solid var(--border);
            border-radius: 4px;
            background: var(--accent);
            height: calc(100vh - 280px);
            min-height: 400px;
        }
        .graph-container svg { width: 100%; height: 100%; }
        .graph-controls {
            display: flex;
            gap: 1rem;
            align-items: center;
            flex-wrap: wrap;
            margin-bottom: 1rem;
        }
        .graph-query-input {
            flex: 1;
            min-width: 300px;
            padding: 0.5rem 0.75rem;
            border: 1px solid var(--border);
            border-radius: 4px;
            background: var(--bg);
            color: var(--fg);
            font-family: monospace;
            font-size: 0.9rem;
        }
        .graph-stats {
            display: flex;
            gap: 1.5rem;
            font-size: 0.85rem;
            color: var(--muted);
            margin-bottom: 0.5rem;
        }
        .graph-stats span { display: flex; align-items: center; gap: 0.3rem; }
        .query-description {
            font-size: 0.9rem;
            color: var(--muted);
            margin-bottom: 1rem;
            font-style: italic;
        }
        .graph-help {
            font-size: 0.8rem;
            color: var(--muted);
            margin-top: 1rem;
            padding: 0.75rem;
            background: var(--accent);
            border-radius: 4px;
        }
        .graph-help code {
            background: var(--bg);
            padding: 0.1rem 0.3rem;
            border-radius: 2px;
            font-size: 0.85em;
        }
        .graph-help-grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
            gap: 0.5rem;
            margin-top: 0.5rem;
        }
    "#;

    let html = format!(
        r##"
        <style>{page_styles}{graph_styles}</style>
        <h1>Knowledge Graph</h1>

        <div class="graph-controls">
            <form action="/graph" method="get" style="display: flex; gap: 0.5rem; flex: 1;">
                <input type="text" name="q" class="graph-query-input"
                       value="{query_escaped}"
                       placeholder="Query: from:KEY depth:2 type:paper has:time orphans hubs">
                <button class="btn" type="submit">Apply</button>
                <a href="/graph" class="btn secondary">Reset</a>
            </form>
        </div>

        <div class="query-description">Showing: {query_desc}</div>

        <div class="graph-stats">
            <span><strong>{nodes}</strong> nodes</span>
            <span><strong>{edges}</strong> edges</span>
            <span><strong>{orphans}</strong> orphans</span>
            <span><strong>{hubs}</strong> hubs (≥{hub_threshold} links)</span>
            <span>avg degree: <strong>{avg_deg:.1}</strong></span>
        </div>

        <div class="graph-container" id="graph-container"></div>

        <div class="graph-help">
            <strong>Query Language</strong> &nbsp; <em style="font-weight:normal">(drag from green handle to link nodes)</em>
            <div class="graph-help-grid">
                <span><code>from:KEY</code> — Center on node</span>
                <span><code>depth:N</code> — Expand N hops</span>
                <span><code>type:paper</code> — Filter by type</span>
                <span><code>type:note</code> — Only notes</span>
                <span><code>has:time</code> — With time tracking</span>
                <span><code>links:>N</code> — Min connections</span>
                <span><code>links:&lt;N</code> — Max connections</span>
                <span><code>orphans</code> — Disconnected only</span>
                <span><code>hubs</code> — Highly connected</span>
                <span><code>path:A->B</code> — Shortest path</span>
                <span><code>category:X</code> — By time category</span>
                <span><code>recent:N</code> — Last N days</span>
            </div>
        </div>

        {graph_script}
        "##,
        page_styles = page_styles,
        graph_styles = graph_styles,
        query_escaped = html_escape(query_str),
        query_desc = query.describe(),
        nodes = graph.stats.total_nodes,
        edges = graph.stats.total_edges,
        orphans = graph.stats.orphan_count,
        hubs = graph.stats.hub_count,
        hub_threshold = graph.stats.hub_threshold,
        avg_deg = graph.stats.avg_degree,
        graph_script = graph_script,
    );

    Html(base_html("Knowledge Graph", &html, None, logged_in))
}

pub async fn graph_api(
    Query(params): Query<GraphQueryParams>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let query_str = params.q.as_deref().unwrap_or("");
    let query = GraphQuery::parse(query_str);
    let graph = crate::graph_query::query_graph(&query, &state.db);

    (
        [("content-type", "application/json")],
        serde_json::to_string(&graph).unwrap_or("{}".to_string()),
    )
        .into_response()
}
