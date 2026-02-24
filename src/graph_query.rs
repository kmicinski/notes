//! Ascent-powered knowledge graph query layer.
//!
//! Sled remains the persistence/write layer (unchanged). This module provides
//! a declarative Datalog query layer using the `ascent` crate, rebuilt per-request
//! from sled data. The write path (reconcile, reindex_note, remove_note, sync_citations)
//! is untouched.

use crate::graph_index::{self, IndexedEdge};
use crate::models::{GraphEdge, GraphNode, GraphQuery, GraphStats, KnowledgeGraph};
use ascent::{ascent_run, Dual};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};

// ============================================================================
// Helpers
// ============================================================================

/// Convert IndexedEdge list to bidirectional (source, target) pairs for Ascent.
fn load_edge_pairs(edges: &[IndexedEdge]) -> Vec<(String, String)> {
    let mut pairs = Vec::with_capacity(edges.len() * 2);
    for e in edges {
        pairs.push((e.source.clone(), e.target.clone()));
        pairs.push((e.target.clone(), e.source.clone()));
    }
    pairs
}

// ============================================================================
// Ascent Queries
// ============================================================================

/// Compute the set of nodes reachable from `center` within `max_depth` hops
/// over bidirectional edges. When no center is given, returns all node keys.
fn compute_reachable(
    edge_pairs: &[(String, String)],
    center: &Option<String>,
    max_depth: usize,
) -> HashSet<String> {
    let center = match center {
        Some(c) => c,
        None => return HashSet::new(), // caller will use all nodes
    };

    let edges: Vec<(String, String)> = edge_pairs.to_vec();
    let start = center.clone();
    let max_d = max_depth as u32;

    let result = ascent_run! {
        relation edge(String, String) = edges;
        relation reachable(String, u32);

        reachable(start.clone(), 0);

        reachable(y.clone(), d + 1) <--
            reachable(x, d),
            edge(x, y),
            if *d < max_d;
    };

    result.reachable.into_iter().map(|(node, _)| node).collect()
}

/// Compute the set of nodes on the shortest path between `start` and `end`
/// using Ascent lattice-based BFS, then reconstruct the path by greedy
/// backtracking through the distance map.
fn compute_shortest_path(
    edge_pairs: &[(String, String)],
    start: Option<&str>,
    end: Option<&str>,
) -> HashSet<String> {
    let (start, end) = match (start, end) {
        (Some(s), Some(e)) => (s, e),
        _ => return HashSet::new(),
    };

    let edges: Vec<(String, String, u32)> = edge_pairs
        .iter()
        .map(|(s, t)| (s.clone(), t.clone(), 1u32))
        .collect();
    let origin = start.to_string();

    // Phase 1: Compute BFS distances from start to all reachable nodes
    let result = ascent_run! {
        relation edge(String, String, u32) = edges;
        lattice dist(String, Dual<u32>);

        dist(origin.clone(), Dual(0u32));

        dist(y.clone(), Dual(d + w)) <--
            dist(x, ?Dual(d)),
            edge(x, y, w);
    };

    // Build distance map
    let dist_map: HashMap<String, u32> = result
        .dist
        .into_iter()
        .map(|(node, Dual(d))| (node, d))
        .collect();

    // Check if end is reachable
    let end_dist = match dist_map.get(end) {
        Some(&d) => d,
        None => return HashSet::new(),
    };

    // Phase 2: Reconstruct path by greedy backtracking from end to start
    // Build adjacency from the original (bidirectional) edge pairs
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for (s, t) in edge_pairs {
        adj.entry(s.as_str()).or_default().push(t.as_str());
    }

    let mut path = HashSet::new();
    let mut current = end.to_string();
    let mut current_dist = end_dist;
    path.insert(current.clone());

    while current != start {
        // Find a neighbor with distance == current_dist - 1
        let next = adj
            .get(current.as_str())
            .and_then(|neighbors| {
                neighbors.iter().find(|&&n| {
                    dist_map.get(n).copied() == Some(current_dist - 1)
                })
            })
            .map(|&n| n.to_string());

        match next {
            Some(n) => {
                current_dist -= 1;
                current = n;
                path.insert(current.clone());
            }
            None => break, // shouldn't happen if dist_map is correct
        }
    }

    path
}

// ============================================================================
// Main Query Entry Point
// ============================================================================

/// Build a KnowledgeGraph by querying sled data through Ascent.
///
/// This replaces `build_knowledge_graph()` in graph.rs with the same output type.
pub fn query_graph(query: &GraphQuery, db: &sled::Db) -> KnowledgeGraph {
    let indexed_nodes = graph_index::load_all_nodes(db).unwrap_or_default();
    let indexed_edges = graph_index::load_all_edges(db).unwrap_or_default();

    // Build edge metadata maps (same as original)
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

    // Build bidirectional edge pairs for Ascent queries
    let edge_pairs = load_edge_pairs(&indexed_edges);

    // Ascent: compute reachable nodes
    let reachable: HashSet<String> = if query.center.is_some() {
        compute_reachable(&edge_pairs, &query.center, query.depth)
    } else {
        indexed_nodes.keys().cloned().collect()
    };

    // Ascent: compute shortest path
    let path_nodes: HashSet<String> = compute_shortest_path(
        &edge_pairs,
        query.path_start.as_deref(),
        query.path_end.as_deref(),
    );

    // Apply scalar filters (same imperative logic as original)
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
