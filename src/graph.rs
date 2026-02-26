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
            authors: node.authors.clone(),
            year: node.year,
            venue: node.venue.clone(),
        });
    }

    // Build edges (only between included nodes)
    let included: HashSet<String> = graph_nodes.iter().map(|n| n.id.clone()).collect();
    let annotations = graph_index::load_all_edge_annotations(db).unwrap_or_default();
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

    // Build notes list for autocomplete (enriched with scholarly metadata)
    let notes_list: Vec<serde_json::Value> = state.notes_map().values().map(|n| {
        let (nt, authors, year, venue, short_label) = match &n.note_type {
            crate::models::NoteType::Paper(meta) => {
                let eff = meta.effective_metadata(&n.title);
                ("paper", eff.authors, eff.year, eff.venue, crate::graph_index::compute_short_label_pub(n))
            }
            crate::models::NoteType::Note => ("note", None, None, None, crate::graph_index::compute_short_label_pub(n)),
        };
        serde_json::json!({
            "key": n.key,
            "title": n.title,
            "node_type": nt,
            "authors": authors,
            "year": year,
            "venue": venue,
            "short_label": short_label,
            "date": n.date.map(|d| d.to_string()),
        })
    }).collect();
    let notes_json = serde_json::to_string(&notes_list).unwrap_or("[]".to_string());

    let graph_json = serde_json::to_string(&graph).unwrap_or("{}".to_string());

    let config = GraphRendererConfig {
        container_selector: "#graph-container".into(),
        center_key: query.center.clone(),
        is_mini: false,
        logged_in,
        show_arrows: true,
        show_edge_tooltips: true,
        auto_fit: has_center,
        max_nodes: 0,
        data_source: GraphDataSource::Inline { graph_json },
        notes_json: if logged_in { Some(notes_json) } else { None },
    };

    let graph_script = render_graph_js(&config);
    let graph_styles = graph_css();

    let page_styles = r#"
        /* Full-screen immersive graph */
        .container { max-width: none; padding: 0; margin: 0; }
        .nav-bar { position: fixed; top: 0; left: 0; right: 0; z-index: 200; opacity: 0.92; }
        .graph-fullscreen {
            position: fixed;
            top: 0; left: 0; right: 0; bottom: 0;
            background: var(--accent);
        }
        .graph-fullscreen svg { width: 100%; height: 100%; }

        /* Floating query bar */
        .graph-query-bar {
            position: fixed;
            top: 52px; left: 50%; transform: translateX(-50%);
            z-index: 150;
            display: flex;
            gap: 0.4rem;
            align-items: center;
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 0.35rem 0.5rem;
            box-shadow: 0 4px 20px rgba(0,0,0,0.12);
            width: min(700px, calc(100vw - 2rem));
        }
        .graph-query-input {
            flex: 1;
            min-width: 0;
            padding: 0.4rem 0.6rem;
            border: 1px solid var(--border);
            border-radius: 4px;
            background: var(--bg);
            color: var(--fg);
            font-family: "SF Mono", "Consolas", monospace;
            font-size: 0.85rem;
        }
        .graph-query-input:focus {
            outline: none;
            border-color: var(--link);
            box-shadow: 0 0 0 2px rgba(38, 139, 210, 0.15);
        }
        .graph-query-bar .qb-btn {
            padding: 0.35rem 0.65rem;
            border: 1px solid var(--base1);
            border-radius: 4px;
            background: var(--blue);
            color: var(--base3);
            cursor: pointer;
            font-size: 0.8rem;
            font-family: inherit;
            text-decoration: none;
            display: inline-block;
        }
        .graph-query-bar .qb-btn:hover { background: var(--cyan); border-color: var(--cyan); }
        .graph-query-bar .qb-btn.secondary { background: var(--base2); color: var(--base00); border-color: var(--base1); }
        .graph-query-bar .qb-btn.secondary:hover { background: var(--base3); }
        .graph-query-bar .qb-help-toggle {
            background: none; border: none; cursor: pointer;
            color: var(--muted); font-size: 1rem; padding: 0.2rem 0.4rem;
            border-radius: 4px;
        }
        .graph-query-bar .qb-help-toggle:hover { color: var(--fg); background: var(--accent); }

        /* Floating stats pill */
        .graph-stats-pill {
            position: fixed;
            top: 52px; right: 12px;
            z-index: 150;
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 6px;
            padding: 0.3rem 0.6rem;
            font-size: 0.75rem;
            color: var(--muted);
            box-shadow: 0 2px 8px rgba(0,0,0,0.08);
            display: flex; gap: 0.6rem; align-items: center;
        }
        .graph-stats-pill strong { color: var(--fg); }

        /* Floating query description */
        .graph-query-desc {
            position: fixed;
            bottom: 40px; left: 50%; transform: translateX(-50%);
            z-index: 150;
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 6px;
            padding: 0.25rem 0.75rem;
            font-size: 0.78rem;
            color: var(--muted);
            font-style: italic;
            box-shadow: 0 2px 8px rgba(0,0,0,0.08);
            white-space: nowrap;
        }

        /* Floating help panel */
        .graph-help-overlay {
            display: none;
            position: fixed;
            top: 100px; left: 50%; transform: translateX(-50%);
            z-index: 300;
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 1rem 1.25rem;
            box-shadow: 0 8px 32px rgba(0,0,0,0.2);
            width: min(560px, calc(100vw - 2rem));
            max-height: calc(100vh - 160px);
            overflow-y: auto;
        }
        .graph-help-overlay.visible { display: block; }
        .graph-help-overlay .help-header {
            display: flex; justify-content: space-between; align-items: center;
            margin-bottom: 0.6rem;
        }
        .graph-help-overlay .help-header h3 { margin: 0; font-size: 0.95rem; }
        .graph-help-overlay .help-close {
            background: none; border: none; cursor: pointer;
            color: var(--muted); font-size: 1.2rem; padding: 0; line-height: 1;
        }
        .graph-help-overlay .help-close:hover { color: var(--fg); }
        .graph-help-overlay code {
            background: var(--accent);
            padding: 0.1rem 0.3rem;
            border-radius: 2px;
            font-size: 0.82em;
        }
        .graph-help-grid {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
            gap: 0.4rem;
            font-size: 0.82rem;
            color: var(--muted);
        }
    "#;

    let html = format!(
        r##"
        <style>{page_styles}{graph_styles}</style>

        <div class="graph-fullscreen" id="graph-container"></div>

        <div class="graph-query-bar">
            <form action="/graph" method="get" style="display: contents;">
                <input type="text" name="q" class="graph-query-input"
                       value="{query_escaped}"
                       placeholder="from:KEY depth:2 type:paper author:NAME year:2024 hubs orphans">
                <button class="qb-btn" type="submit">Apply</button>
                <a href="/graph" class="qb-btn secondary">Reset</a>
            </form>
            <button class="qb-help-toggle" onclick="document.querySelector('.graph-help-overlay').classList.toggle('visible')" title="Query help">?</button>
        </div>

        <div class="graph-stats-pill">
            <span><strong>{nodes}</strong> nodes</span>
            <span><strong>{edges}</strong> edges</span>
            <span><strong>{orphans}</strong> orphans</span>
            <span><strong>{hubs}</strong> hubs</span>
            <span>avg\u00a0<strong>{avg_deg:.1}</strong></span>
        </div>

        <div class="graph-query-desc">Showing: {query_desc}</div>

        <div class="graph-help-overlay">
            <div class="help-header">
                <h3>Query Language</h3>
                <button class="help-close" onclick="this.closest('.graph-help-overlay').classList.remove('visible')">&times;</button>
            </div>
            <div class="graph-help-grid">
                <span><code>from:KEY</code> Center on node</span>
                <span><code>depth:N</code> Expand N hops</span>
                <span><code>type:paper</code> Filter by type</span>
                <span><code>type:note</code> Only notes</span>
                <span><code>has:time</code> With time tracking</span>
                <span><code>links:&gt;N</code> Min connections</span>
                <span><code>links:&lt;N</code> Max connections</span>
                <span><code>orphans</code> Disconnected only</span>
                <span><code>hubs</code> Highly connected</span>
                <span><code>path:A-&gt;B</code> Shortest path</span>
                <span><code>category:X</code> By time category</span>
                <span><code>recent:N</code> Last N days</span>
                <span><code>author:NAME</code> By author name</span>
                <span><code>venue:NAME</code> By venue/journal</span>
                <span><code>year:YYYY</code> By year</span>
                <span><code>year:YYYY-YYYY</code> Year range</span>
                <span><code>title:TEXT</code> Search titles</span>
            </div>
            <div style="margin-top: 0.6rem; font-size: 0.78rem; color: var(--muted);">
                Drag from green handle to link nodes. Click any edge to annotate.
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
