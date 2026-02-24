//! Knowledge graph building and visualization.
//!
//! This module handles the construction of the knowledge graph from notes
//! and references, as well as the web-based D3.js visualization.

use crate::auth::is_logged_in;
use crate::graph_index;
use crate::models::{GraphEdge, GraphNode, GraphQuery, GraphStats, KnowledgeGraph};
use crate::notes::html_escape;
use crate::templates::base_html;
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

    let graph_styles = r#"
        .graph-container {
            position: relative;
            border: 1px solid var(--border);
            border-radius: 4px;
            background: var(--accent);
            height: calc(100vh - 280px);
            min-height: 400px;
        }

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

        .node-tooltip {
            position: absolute;
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 4px;
            padding: 0.5rem 0.75rem;
            font-size: 0.85rem;
            pointer-events: none;
            z-index: 1000;
            box-shadow: 0 2px 8px rgba(0,0,0,0.15);
            max-width: 300px;
        }

        .node-tooltip .title { font-weight: 600; margin-bottom: 0.25rem; }
        .node-tooltip .meta { color: var(--muted); font-size: 0.8rem; }

        svg { width: 100%; height: 100%; }

        .link { stroke: var(--border); stroke-opacity: 0.6; }
        .link.citation { stroke-dasharray: 5,3; stroke: #b58900; stroke-opacity: 0.5; }
        .link.manual { stroke: #859900; stroke-opacity: 0.7; }
        .link.highlighted { stroke: var(--link); stroke-opacity: 1; stroke-width: 2px; }

        .node circle { cursor: pointer; stroke: var(--bg); stroke-width: 1.5px; }
        .node.note circle { fill: var(--link); }
        .node.paper circle { fill: #f4a460; }
        .node.orphan circle { fill: var(--muted); opacity: 0.6; }
        .node.hub circle { stroke: var(--fg); stroke-width: 2px; }
        .node.selected circle { stroke: #fff; stroke-width: 3px; }

        .node text {
            font-size: 10px;
            fill: var(--fg);
            pointer-events: none;
            text-anchor: middle;
            dominant-baseline: middle;
        }

        .legend {
            position: absolute;
            bottom: 10px;
            left: 10px;
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 4px;
            padding: 0.5rem;
            font-size: 0.75rem;
        }

        .legend-item { display: flex; align-items: center; gap: 0.4rem; margin: 0.2rem 0; }
        .legend-dot { width: 10px; height: 10px; border-radius: 50%; }
        .legend-dot.note { background: var(--link); }
        .legend-dot.paper { background: #f4a460; }

        .graph-ctx-menu {
            position: fixed;
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 4px;
            padding: 0.25rem 0;
            z-index: 2000;
            box-shadow: 0 2px 12px rgba(0,0,0,0.2);
            min-width: 180px;
        }
        .graph-ctx-menu-item {
            padding: 0.4rem 0.75rem;
            cursor: pointer;
            font-size: 0.85rem;
            color: var(--fg);
        }
        .graph-ctx-menu-item:hover {
            background: var(--accent);
        }

        .link-modal-overlay {
            position: fixed; top: 0; left: 0; right: 0; bottom: 0;
            background: rgba(0,0,0,0.5);
            z-index: 3000;
            display: flex; align-items: center; justify-content: center;
        }
        .link-modal {
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 1.5rem;
            width: 460px;
            max-width: 90vw;
            box-shadow: 0 4px 20px rgba(0,0,0,0.3);
        }
        .link-modal h3 { margin: 0 0 1rem; font-size: 1.1rem; }
        .link-modal label { display: block; margin-bottom: 0.3rem; font-size: 0.85rem; font-weight: 600; }
        .link-modal input, .link-modal textarea {
            width: 100%; box-sizing: border-box;
            padding: 0.5rem; border: 1px solid var(--border);
            border-radius: 4px; background: var(--accent); color: var(--fg);
            font-size: 0.9rem; font-family: inherit;
        }
        .link-modal textarea { height: 60px; resize: vertical; }
        .link-modal-actions { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 1rem; }
        .autocomplete-wrap { position: relative; }
        .autocomplete-dropdown {
            position: absolute; top: 100%; left: 0; right: 0;
            background: var(--bg); border: 1px solid var(--border);
            border-radius: 0 0 4px 4px; max-height: 200px; overflow-y: auto;
            z-index: 3001; box-shadow: 0 4px 8px rgba(0,0,0,0.15);
        }
        .autocomplete-item {
            padding: 0.4rem 0.5rem; cursor: pointer; font-size: 0.85rem;
            display: flex; justify-content: space-between; align-items: center;
        }
        .autocomplete-item:hover, .autocomplete-item.active { background: var(--accent); }
        .autocomplete-badge {
            font-size: 0.7rem; padding: 0.1rem 0.3rem; border-radius: 3px;
            background: var(--border); color: var(--muted);
        }
    "#;

    let graph_json = serde_json::to_string(&graph).unwrap_or("{}".to_string());

    let html = format!(
        r##"
        <style>{styles}</style>
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

        <div class="graph-container" id="graph-container">
            <svg id="graph-svg"></svg>
            <div class="legend">
                <div class="legend-item"><div class="legend-dot note"></div>Note</div>
                <div class="legend-item"><div class="legend-dot paper"></div>Paper</div>
                <div class="legend-item"><svg width="30" height="12"><line x1="0" y1="6" x2="30" y2="6" stroke="#b58900" stroke-dasharray="4,3" stroke-width="1.5"/></svg> Citation</div>
                <div class="legend-item"><svg width="30" height="12"><line x1="0" y1="6" x2="30" y2="6" stroke="#859900" stroke-width="1.5"/></svg> Manual link</div>
            </div>
        </div>

        <div class="graph-help">
            <strong>Query Language</strong> &nbsp; <em style="font-weight:normal">(shift+click a node to add a link)</em>
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

        <script src="https://d3js.org/d3.v7.min.js"></script>
        <script>
            const graphData = {graph_json};
            const allNotes = {notes_json};
            const isLoggedIn = {logged_in_js};
            const hasCenter = {has_center_js};
            const container = document.getElementById('graph-container');
            const svg = d3.select('#graph-svg');
            const width = container.clientWidth;
            const height = container.clientHeight;

            // Build node map for edge tooltips
            const nodeMap = {{}};
            graphData.nodes.forEach(n => {{ nodeMap[n.id] = n; }});

            // Create tooltip
            const tooltip = d3.select('body').append('div')
                .attr('class', 'node-tooltip')
                .style('display', 'none');

            // Define arrowhead markers
            const defs = svg.append('defs');

            function addMarker(id, color) {{
                defs.append('marker')
                    .attr('id', id)
                    .attr('viewBox', '0 -7 14 14')
                    .attr('refX', 14)
                    .attr('refY', 0)
                    .attr('markerWidth', 10)
                    .attr('markerHeight', 10)
                    .attr('markerUnits', 'userSpaceOnUse')
                    .attr('orient', 'auto')
                    .append('path')
                    .attr('d', 'M0,-5L12,0L0,5')
                    .attr('fill', color);
            }}

            addMarker('arrow-default', '#93a1a1');
            addMarker('arrow-citation', '#b58900');
            addMarker('arrow-manual', '#859900');

            // Force simulation
            const simulation = d3.forceSimulation(graphData.nodes)
                .force('link', d3.forceLink(graphData.edges)
                    .id(d => d.id)
                    .distance(80))
                .force('charge', d3.forceManyBody().strength(-200))
                .force('center', d3.forceCenter(width / 2, height / 2))
                .force('collision', d3.forceCollide().radius(30));

            // Create link groups
            const linkG = svg.append('g');
            const link = linkG
                .selectAll('line')
                .data(graphData.edges)
                .join('line')
                .attr('class', d => {{
                    let cls = 'link';
                    if (d.edge_type === 'citation') cls += ' citation';
                    if (d.edge_type === 'manual') cls += ' manual';
                    return cls;
                }})
                .attr('stroke-width', d => Math.sqrt(d.weight) * 1.5)
                .attr('marker-end', d => {{
                    if (!hasCenter) return null;
                    if (d.edge_type === 'citation') return 'url(#arrow-citation)';
                    if (d.edge_type === 'manual') return 'url(#arrow-manual)';
                    return 'url(#arrow-default)';
                }});

            // Edge hover tooltips
            link.on('mouseover', function(event, d) {{
                const srcNode = (typeof d.source === 'object') ? d.source : nodeMap[d.source];
                const tgtNode = (typeof d.target === 'object') ? d.target : nodeMap[d.target];
                const srcTitle = srcNode ? srcNode.title : d.source;
                const tgtTitle = tgtNode ? tgtNode.title : d.target;

                const typeLabels = {{
                    crosslink: 'cites ([@ref])',
                    parent: 'child of',
                    citation: 'PDF citation',
                    manual: 'linked by user'
                }};
                const typeLabel = typeLabels[d.edge_type] || d.edge_type;

                let html = '<div class="title">' + srcTitle + ' &rarr; ' + tgtTitle + '</div>';
                html += '<div class="meta">Type: ' + typeLabel;
                if (d.annotation) html += '<br>' + d.annotation;
                html += '</div>';

                d3.select(this).attr('stroke-width', 4).attr('stroke-opacity', 1);
                tooltip.style('display', 'block').html(html)
                    .style('left', (event.pageX + 10) + 'px')
                    .style('top', (event.pageY - 10) + 'px');
            }})
            .on('mouseout', function(event, d) {{
                d3.select(this).attr('stroke-width', Math.sqrt(d.weight) * 1.5).attr('stroke-opacity', null);
                tooltip.style('display', 'none');
            }});

            // Create node groups
            const nodeG = svg.append('g');
            const node = nodeG
                .selectAll('g')
                .data(graphData.nodes)
                .join('g')
                .attr('class', d => {{
                    let cls = 'node ' + d.node_type;
                    if (d.in_degree + d.out_degree === 0) cls += ' orphan';
                    if (d.in_degree + d.out_degree >= {hub_threshold}) cls += ' hub';
                    return cls;
                }})
                .call(d3.drag()
                    .on('start', dragstarted)
                    .on('drag', dragged)
                    .on('end', dragended));

            // Add circles to nodes
            function nodeRadius(d) {{
                const base = 8;
                const degree = (d.in_degree || 0) + (d.out_degree || 0);
                return base + Math.sqrt(degree) * 3;
            }}

            node.append('circle')
                .attr('r', nodeRadius);

            // Add labels
            node.append('text')
                .text(d => d.title.length > 15 ? d.title.substring(0, 15) + '...' : d.title)
                .attr('dy', d => -(12 + Math.sqrt(d.in_degree + d.out_degree) * 3));

            // Node hover interactions
            node.on('mouseover', function(event, d) {{
                d3.select(this).classed('selected', true);
                link.classed('highlighted', l => l.source.id === d.id || l.target.id === d.id);

                tooltip.style('display', 'block')
                    .html(`
                        <div class="title">${{d.title}}</div>
                        <div class="meta">
                            Type: ${{d.node_type}}<br>
                            Links: ${{d.in_degree}} in, ${{d.out_degree}} out
                            ${{d.time_total > 0 ? '<br>Time: ' + Math.floor(d.time_total/60) + 'h ' + (d.time_total%60) + 'm' : ''}}
                            ${{d.primary_category ? '<br>Category: ' + d.primary_category : ''}}
                        </div>
                    `)
                    .style('left', (event.pageX + 10) + 'px')
                    .style('top', (event.pageY - 10) + 'px');
            }})
            .on('mouseout', function() {{
                d3.select(this).classed('selected', false);
                link.classed('highlighted', false);
                tooltip.style('display', 'none');
            }})
            .on('click', function(event, d) {{
                if (event.shiftKey && isLoggedIn) {{
                    // Shift+click opens the link creation modal
                    event.preventDefault();
                    event.stopPropagation();
                    openLinkModal(d);
                    return;
                }}
                window.location.href = '/note/' + d.id;
            }})
            .on('dblclick', function(event, d) {{
                window.location.href = '/graph?q=from:' + d.id + ' depth:2';
            }});

            // Link creation modal
            function openLinkModal(sourceNode) {{
                d3.selectAll('.link-modal-overlay').remove();

                const overlay = d3.select('body').append('div')
                    .attr('class', 'link-modal-overlay');

                const modal = overlay.append('div')
                    .attr('class', 'link-modal');

                modal.append('h3').text('Add link from "' + sourceNode.title + '"');

                modal.append('label').text('Target note');
                const acWrap = modal.append('div').attr('class', 'autocomplete-wrap');
                const targetInput = acWrap.append('input')
                    .attr('type', 'text')
                    .attr('id', 'link-target-input')
                    .attr('placeholder', 'Search notes...')
                    .attr('autocomplete', 'off');
                const dropdown = acWrap.append('div')
                    .attr('class', 'autocomplete-dropdown')
                    .style('display', 'none');

                let selectedKey = null;
                let selectedIdx = -1;
                let currentMatches = [];

                function renderDropdown(matches) {{
                    currentMatches = matches;
                    selectedIdx = -1;
                    dropdown.html('');
                    if (matches.length === 0) {{
                        dropdown.style('display', 'none');
                        return;
                    }}
                    dropdown.style('display', 'block');
                    matches.forEach((m, i) => {{
                        const item = dropdown.append('div')
                            .attr('class', 'autocomplete-item')
                            .on('click', () => selectTarget(m));
                        item.append('span').text(m.title);
                        item.append('span').attr('class', 'autocomplete-badge').text(m.node_type);
                    }});
                }}

                function selectTarget(m) {{
                    selectedKey = m.key;
                    targetInput.property('value', m.title);
                    dropdown.style('display', 'none');
                }}

                targetInput.on('input', function() {{
                    selectedKey = null;
                    const q = this.value.toLowerCase().trim();
                    if (q.length === 0) {{ dropdown.style('display', 'none'); return; }}
                    const matches = allNotes
                        .filter(n => n.key !== sourceNode.id &&
                            (n.title.toLowerCase().includes(q) || n.key.toLowerCase().includes(q)))
                        .slice(0, 10);
                    renderDropdown(matches);
                }});

                targetInput.on('keydown', function(event) {{
                    if (event.key === 'ArrowDown') {{
                        event.preventDefault();
                        if (currentMatches.length > 0) {{
                            selectedIdx = Math.min(selectedIdx + 1, currentMatches.length - 1);
                            dropdown.selectAll('.autocomplete-item').classed('active', (d, i) => i === selectedIdx);
                        }}
                    }} else if (event.key === 'ArrowUp') {{
                        event.preventDefault();
                        selectedIdx = Math.max(selectedIdx - 1, 0);
                        dropdown.selectAll('.autocomplete-item').classed('active', (d, i) => i === selectedIdx);
                    }} else if (event.key === 'Enter') {{
                        event.preventDefault();
                        if (selectedIdx >= 0 && selectedIdx < currentMatches.length) {{
                            selectTarget(currentMatches[selectedIdx]);
                        }}
                    }} else if (event.key === 'Escape') {{
                        dropdown.style('display', 'none');
                    }}
                }});

                modal.append('div').style('height', '0.75rem');
                modal.append('label').text('Annotation (optional)');
                const annInput = modal.append('textarea')
                    .attr('id', 'link-annotation')
                    .attr('placeholder', 'Describe this link...');

                const actions = modal.append('div').attr('class', 'link-modal-actions');
                actions.append('button')
                    .attr('class', 'btn secondary')
                    .text('Cancel')
                    .on('click', () => overlay.remove());

                const submitBtn = actions.append('button')
                    .attr('class', 'btn')
                    .text('Add Link')
                    .on('click', () => {{
                        if (!selectedKey) {{
                            targetInput.node().focus();
                            return;
                        }}
                        const annotation = annInput.property('value').trim() || null;
                        submitBtn.attr('disabled', true).text('Adding...');
                        fetch('/api/graph/edge', {{
                            method: 'POST',
                            headers: {{ 'Content-Type': 'application/json' }},
                            body: JSON.stringify({{ source: sourceNode.id, target: selectedKey, annotation }})
                        }}).then(r => {{
                            if (r.ok) {{
                                overlay.remove();
                                window.location.reload();
                            }} else {{
                                r.text().then(t => alert('Error: ' + t));
                                submitBtn.attr('disabled', null).text('Add Link');
                            }}
                        }}).catch(e => {{
                            alert('Network error: ' + e);
                            submitBtn.attr('disabled', null).text('Add Link');
                        }});
                    }});

                // Close on overlay click
                overlay.on('click', function(event) {{
                    if (event.target === overlay.node()) overlay.remove();
                }});

                // Close on Escape
                function escHandler(event) {{
                    if (event.key === 'Escape') {{
                        overlay.remove();
                        document.removeEventListener('keydown', escHandler);
                    }}
                }}
                document.addEventListener('keydown', escHandler);

                // Focus target input
                setTimeout(() => targetInput.node().focus(), 50);
            }}

            // Update positions on simulation tick
            simulation.on('tick', () => {{
                link
                    .attr('x1', d => d.source.x)
                    .attr('y1', d => d.source.y)
                    .attr('x2', d => {{
                        if (!hasCenter) return d.target.x;
                        const dx = d.target.x - d.source.x;
                        const dy = d.target.y - d.source.y;
                        const dist = Math.sqrt(dx*dx + dy*dy) || 1;
                        const r = nodeRadius(d.target);
                        return d.target.x - dx * (r / dist);
                    }})
                    .attr('y2', d => {{
                        if (!hasCenter) return d.target.y;
                        const dx = d.target.x - d.source.x;
                        const dy = d.target.y - d.source.y;
                        const dist = Math.sqrt(dx*dx + dy*dy) || 1;
                        const r = nodeRadius(d.target);
                        return d.target.y - dy * (r / dist);
                    }});

                node.attr('transform', d => {{
                    d.x = Math.max(20, Math.min(width - 20, d.x));
                    d.y = Math.max(20, Math.min(height - 20, d.y));
                    return `translate(${{d.x}},${{d.y}})`;
                }});
            }});

            // Drag functions
            function dragstarted(event, d) {{
                if (!event.active) simulation.alphaTarget(0.3).restart();
                d.fx = d.x;
                d.fy = d.y;
            }}

            function dragged(event, d) {{
                d.fx = event.x;
                d.fy = event.y;
            }}

            function dragended(event, d) {{
                if (!event.active) simulation.alphaTarget(0);
                d.fx = null;
                d.fy = null;
            }}

            // Zoom support
            const zoom = d3.zoom()
                .scaleExtent([0.3, 3])
                .on('zoom', (event) => {{
                    linkG.attr('transform', event.transform);
                    nodeG.attr('transform', event.transform);
                }});

            svg.call(zoom);

            // Handle window resize
            window.addEventListener('resize', () => {{
                const newWidth = container.clientWidth;
                const newHeight = container.clientHeight;
                simulation.force('center', d3.forceCenter(newWidth / 2, newHeight / 2));
                simulation.alpha(0.3).restart();
            }});
        </script>
        "##,
        styles = graph_styles,
        query_escaped = html_escape(query_str),
        query_desc = query.describe(),
        nodes = graph.stats.total_nodes,
        edges = graph.stats.total_edges,
        orphans = graph.stats.orphan_count,
        hubs = graph.stats.hub_count,
        hub_threshold = graph.stats.hub_threshold,
        avg_deg = graph.stats.avg_degree,
        graph_json = graph_json,
        notes_json = notes_json,
        logged_in_js = if logged_in { "true" } else { "false" },
        has_center_js = if has_center { "true" } else { "false" },
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
