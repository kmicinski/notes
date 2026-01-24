use axum::{
    extract::{Path, Query, State},
    http::{header::SET_COOKIE, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, NaiveDate, Utc};
use hmac::{Hmac, Mac};
use pulldown_cmark::Parser;
use rand::Rng;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sled::Db;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    env,
    fs,
    path::PathBuf,
    process::Command,
    sync::Arc,
    time::Duration,
};
use walkdir::WalkDir;

type HmacSha256 = Hmac<Sha256>;

// ============================================================================
// Configuration
// ============================================================================

const NOTES_DIR: &str = "content";
const DB_PATH: &str = ".notes_db";
const SESSION_COOKIE: &str = "notes_session";
const SESSION_TTL_HOURS: i64 = 24;

// ============================================================================
// Data Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub key: String,
    pub path: PathBuf,
    pub title: String,
    pub date: Option<NaiveDate>,
    pub note_type: NoteType,
    pub parent_key: Option<String>,
    pub time_entries: Vec<TimeEntry>,
    pub raw_content: String,
    pub full_file_content: String,
    pub modified: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NoteType {
    Note,
    Paper(PaperMeta),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaperMeta {
    pub bib_key: String,
    pub bibtex: Option<String>,
    pub authors: Option<String>,
    pub venue: Option<String>,
    pub year: Option<i32>,
    pub sources: Vec<PaperSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaperSource {
    pub source_type: String,  // "arxiv", "doi", "url"
    pub identifier: String,   // The arxiv ID, DOI, or URL
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEntry {
    pub date: NaiveDate,
    pub minutes: u32,
    pub category: TimeCategory,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum TimeCategory {
    Programming,
    Teaching,
    Reading,
    Writing,
    Service,
    Other(String),
}

impl std::fmt::Display for TimeCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeCategory::Programming => write!(f, "programming"),
            TimeCategory::Teaching => write!(f, "teaching"),
            TimeCategory::Reading => write!(f, "reading"),
            TimeCategory::Writing => write!(f, "writing"),
            TimeCategory::Service => write!(f, "service"),
            TimeCategory::Other(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommit {
    pub hash: String,
    pub date: DateTime<Utc>,
    pub message: String,
    pub author: String,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub note: Note,
    pub matches: Vec<SearchMatch>,
}

#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub line_number: usize,
    pub line_content: String,
}

// ============================================================================
// Knowledge Graph Data Structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub title: String,
    pub node_type: String,        // "note" or "paper"
    pub date: Option<String>,
    pub time_total: u32,          // Total minutes tracked
    pub primary_category: Option<String>,
    pub in_degree: usize,         // Incoming links
    pub out_degree: usize,        // Outgoing links
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub weight: usize,  // Number of times referenced
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub stats: GraphStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub orphan_count: usize,      // Nodes with no connections
    pub hub_threshold: usize,     // Degree considered "hub"
    pub hub_count: usize,
    pub avg_degree: f64,
    pub max_degree: usize,
}

// ============================================================================
// Smart Add Data Structures
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct SmartAddRequest {
    pub input: String,
}

#[derive(Debug, Clone)]
pub enum InputType {
    ArxivUrl { arxiv_id: String },
    DoiUrl { doi: String },
    GenericUrl { url: String },
    PlainText { text: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalMatch {
    pub key: String,
    pub title: String,
    pub match_type: String,  // "exact", "title", "arxiv_id"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalResult {
    pub title: String,
    pub authors: Option<String>,
    pub year: Option<i32>,
    pub venue: Option<String>,
    pub bib_key: String,
    pub bibtex: Option<String>,
    pub suggested_filename: String,
    pub source: String,  // "arxiv", "crossref", "claude"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartAddResult {
    pub input_type: String,
    pub local_match: Option<LocalMatch>,
    pub external_result: Option<ExternalResult>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SmartAddCreateRequest {
    pub title: String,
    pub filename: String,
    pub bib_key: String,
    pub authors: Option<String>,
    pub year: Option<i32>,
    pub venue: Option<String>,
    pub bibtex: Option<String>,
    pub arxiv_id: Option<String>,
    pub doi: Option<String>,
}

// ============================================================================
// Graph Query Language
// ============================================================================
//
// Query syntax (composable, space-separated):
//   from:KEY        - Center view on node KEY, show its neighborhood
//   depth:N         - Expand N hops from center (default 2)
//   type:paper      - Filter to only papers
//   type:note       - Filter to only notes
//   has:time        - Only nodes with time tracking
//   links:>N        - Only nodes with more than N connections
//   links:<N        - Only nodes with fewer than N connections
//   orphans         - Show only disconnected nodes
//   hubs            - Show only highly connected nodes (>5 links)
//   path:A->B       - Highlight shortest path between A and B
//   cluster:type    - Group nodes by type
//   cluster:parent  - Group nodes by parent hierarchy
//   category:X      - Filter by primary time category
//   recent:N        - Only nodes modified in last N days

#[derive(Debug, Clone, Default)]
pub struct GraphQuery {
    pub center: Option<String>,
    pub depth: usize,
    pub type_filter: Option<String>,
    pub has_time: bool,
    pub min_links: Option<usize>,
    pub max_links: Option<usize>,
    pub orphans_only: bool,
    pub hubs_only: bool,
    pub path_start: Option<String>,
    pub path_end: Option<String>,
    pub cluster_by: Option<String>,
    pub category_filter: Option<String>,
    pub recent_days: Option<i64>,
}

impl GraphQuery {
    pub fn parse(query: &str) -> Self {
        let mut gq = GraphQuery {
            depth: 99, // Default: show all
            ..Default::default()
        };

        for part in query.split_whitespace() {
            if let Some(key) = part.strip_prefix("from:") {
                gq.center = Some(key.to_string());
                if gq.depth == 99 { gq.depth = 2; } // Default depth when centered
            } else if let Some(d) = part.strip_prefix("depth:") {
                gq.depth = d.parse().unwrap_or(2);
            } else if let Some(t) = part.strip_prefix("type:") {
                gq.type_filter = Some(t.to_string());
            } else if part == "has:time" {
                gq.has_time = true;
            } else if let Some(l) = part.strip_prefix("links:>") {
                gq.min_links = l.parse().ok();
            } else if let Some(l) = part.strip_prefix("links:<") {
                gq.max_links = l.parse().ok();
            } else if part == "orphans" {
                gq.orphans_only = true;
            } else if part == "hubs" {
                gq.hubs_only = true;
            } else if let Some(path) = part.strip_prefix("path:") {
                if let Some((a, b)) = path.split_once("->") {
                    gq.path_start = Some(a.to_string());
                    gq.path_end = Some(b.to_string());
                }
            } else if let Some(c) = part.strip_prefix("cluster:") {
                gq.cluster_by = Some(c.to_string());
            } else if let Some(cat) = part.strip_prefix("category:") {
                gq.category_filter = Some(cat.to_string());
            } else if let Some(days) = part.strip_prefix("recent:") {
                gq.recent_days = days.parse().ok();
            }
        }

        gq
    }

    pub fn describe(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref c) = self.center {
            parts.push(format!("centered on {}", c));
        }
        if self.depth < 99 {
            parts.push(format!("{} hops", self.depth));
        }
        if let Some(ref t) = self.type_filter {
            parts.push(format!("type={}", t));
        }
        if self.has_time {
            parts.push("with time tracking".to_string());
        }
        if let Some(n) = self.min_links {
            parts.push(format!("links>{}", n));
        }
        if self.orphans_only {
            parts.push("orphans only".to_string());
        }
        if self.hubs_only {
            parts.push("hubs only".to_string());
        }
        if self.path_start.is_some() && self.path_end.is_some() {
            parts.push(format!("path {}→{}",
                self.path_start.as_ref().unwrap(),
                self.path_end.as_ref().unwrap()));
        }

        if parts.is_empty() {
            "Full graph".to_string()
        } else {
            parts.join(", ")
        }
    }
}

// ============================================================================
// Graph Building
// ============================================================================

fn extract_references(content: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = content.chars().collect();

    while i < chars.len() {
        if i + 1 < chars.len() && chars[i] == '[' && chars[i + 1] == '@' {
            let start = i + 2;
            let mut end = start;
            while end < chars.len() && chars[end] != ']' {
                end += 1;
            }
            if end < chars.len() {
                let key: String = chars[start..end].iter().collect();
                if !key.is_empty() {
                    refs.push(key);
                }
            }
            i = end + 1;
        } else {
            i += 1;
        }
    }

    refs
}

fn build_knowledge_graph(notes: &[Note], query: &GraphQuery) -> KnowledgeGraph {
    let notes_map: HashMap<String, &Note> = notes.iter().map(|n| (n.key.clone(), n)).collect();

    // Build raw edges with counts
    let mut edge_counts: HashMap<(String, String), usize> = HashMap::new();
    for note in notes {
        let refs = extract_references(&note.full_file_content);
        for r in refs {
            if notes_map.contains_key(&r) {
                *edge_counts.entry((note.key.clone(), r)).or_insert(0) += 1;
            }
        }
        // Also count parent relationships
        if let Some(ref parent) = note.parent_key {
            if notes_map.contains_key(parent) {
                *edge_counts.entry((note.key.clone(), parent.clone())).or_insert(0) += 1;
            }
        }
    }

    // Calculate degrees
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut out_degree: HashMap<String, usize> = HashMap::new();
    for ((src, tgt), _) in &edge_counts {
        *out_degree.entry(src.clone()).or_insert(0) += 1;
        *in_degree.entry(tgt.clone()).or_insert(0) += 1;
    }

    // Find path if requested
    let path_nodes: HashSet<String> = if let (Some(start), Some(end)) = (&query.path_start, &query.path_end) {
        find_shortest_path(&edge_counts, start, end)
    } else {
        HashSet::new()
    };

    // Find nodes within depth if centered
    let reachable: HashSet<String> = if let Some(ref center) = query.center {
        find_reachable(&edge_counts, center, query.depth)
    } else {
        notes.iter().map(|n| n.key.clone()).collect()
    };

    // Build nodes with filtering
    let now = Utc::now();
    let mut graph_nodes = Vec::new();

    for note in notes {
        // Apply filters
        if !reachable.contains(&note.key) && !path_nodes.contains(&note.key) {
            continue;
        }

        let node_type = match note.note_type {
            NoteType::Paper(_) => "paper",
            NoteType::Note => "note",
        };

        if let Some(ref tf) = query.type_filter {
            if node_type != tf {
                continue;
            }
        }

        let time_total: u32 = note.time_entries.iter().map(|e| e.minutes).sum();
        if query.has_time && time_total == 0 {
            continue;
        }

        let indeg = *in_degree.get(&note.key).unwrap_or(&0);
        let outdeg = *out_degree.get(&note.key).unwrap_or(&0);
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

        // Category filter
        let primary_cat = note.time_entries.iter()
            .max_by_key(|e| e.minutes)
            .map(|e| e.category.to_string());

        if let Some(ref cat_filter) = query.category_filter {
            if primary_cat.as_deref() != Some(cat_filter) {
                continue;
            }
        }

        // Recent filter
        if let Some(days) = query.recent_days {
            let cutoff = now - chrono::Duration::days(days);
            if note.modified < cutoff {
                continue;
            }
        }

        graph_nodes.push(GraphNode {
            id: note.key.clone(),
            title: note.title.clone(),
            node_type: node_type.to_string(),
            date: note.date.map(|d| d.to_string()),
            time_total,
            primary_category: primary_cat,
            in_degree: indeg,
            out_degree: outdeg,
            parent: note.parent_key.clone(),
        });
    }

    // Build edges (only between included nodes)
    let included: HashSet<String> = graph_nodes.iter().map(|n| n.id.clone()).collect();
    let mut graph_edges = Vec::new();

    for ((src, tgt), weight) in &edge_counts {
        if included.contains(src) && included.contains(tgt) {
            graph_edges.push(GraphEdge {
                source: src.clone(),
                target: tgt.clone(),
                weight: *weight,
            });
        }
    }

    // Calculate stats
    let total_nodes = graph_nodes.len();
    let total_edges = graph_edges.len();
    let orphan_count = graph_nodes.iter().filter(|n| n.in_degree + n.out_degree == 0).count();
    let hub_threshold = 5;
    let hub_count = graph_nodes.iter().filter(|n| n.in_degree + n.out_degree >= hub_threshold).count();
    let total_degree: usize = graph_nodes.iter().map(|n| n.in_degree + n.out_degree).sum();
    let avg_degree = if total_nodes > 0 { total_degree as f64 / total_nodes as f64 } else { 0.0 };
    let max_degree = graph_nodes.iter().map(|n| n.in_degree + n.out_degree).max().unwrap_or(0);

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

fn find_reachable(edges: &HashMap<(String, String), usize>, start: &str, max_depth: usize) -> HashSet<String> {
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

fn find_shortest_path(edges: &HashMap<(String, String), usize>, start: &str, end: &str) -> HashSet<String> {
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
// Session Management
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Session {
    created: i64,
    expires: i64,
    nonce: String,
}

fn get_secret_key() -> Option<Vec<u8>> {
    env::var("NOTES_PASSWORD").ok().map(|p| p.into_bytes())
}

fn is_auth_enabled() -> bool {
    get_secret_key().is_some()
}

fn create_session() -> Option<String> {
    let secret = get_secret_key()?;
    let now = Utc::now().timestamp();
    let expires = now + (SESSION_TTL_HOURS * 3600);
    let nonce: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(16)
        .map(char::from)
        .collect();

    let session = Session { created: now, expires, nonce };
    let session_json = serde_json::to_string(&session).ok()?;

    let mut mac = HmacSha256::new_from_slice(&secret).ok()?;
    mac.update(session_json.as_bytes());
    let signature = hex_encode(mac.finalize().into_bytes().as_slice());

    Some(format!("{}.{}", base64_encode(&session_json), signature))
}

fn verify_session(token: &str, secret: &[u8]) -> bool {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 2 {
        return false;
    }

    let session_json = match base64_decode(parts[0]) {
        Some(s) => s,
        None => return false,
    };

    // Verify signature
    let mut mac = match HmacSha256::new_from_slice(secret) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(session_json.as_bytes());
    let expected_sig = hex_encode(mac.finalize().into_bytes().as_slice());

    if parts[1] != expected_sig {
        return false;
    }

    // Check expiration
    let session: Session = match serde_json::from_str(&session_json) {
        Ok(s) => s,
        Err(_) => return false,
    };

    Utc::now().timestamp() < session.expires
}

fn is_logged_in(jar: &CookieJar) -> bool {
    let secret = match get_secret_key() {
        Some(s) => s,
        None => return false,
    };

    match jar.get(SESSION_COOKIE) {
        Some(cookie) => verify_session(cookie.value(), &secret),
        None => false,
    }
}

fn base64_encode(s: &str) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = s.as_bytes();
    let mut result = String::new();

    for chunk in bytes.chunks(3) {
        let mut n: u32 = 0;
        for (i, &byte) in chunk.iter().enumerate() {
            n |= (byte as u32) << (16 - i * 8);
        }

        let chars_to_add = chunk.len() + 1;
        for i in 0..4 {
            if i < chars_to_add {
                result.push(CHARS[((n >> (18 - i * 6)) & 0x3F) as usize] as char);
            } else {
                result.push('=');
            }
        }
    }

    result
}

fn base64_decode(s: &str) -> Option<String> {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let s = s.trim_end_matches('=');
    let mut result = Vec::new();

    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;

    for c in s.chars() {
        let val = CHARS.iter().position(|&x| x == c as u8)? as u32;
        buffer = (buffer << 6) | val;
        bits += 6;

        if bits >= 8 {
            bits -= 8;
            result.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }

    String::from_utf8(result).ok()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// ============================================================================
// Frontmatter Parsing
// ============================================================================

#[derive(Debug, Default)]
struct Frontmatter {
    title: Option<String>,
    date: Option<NaiveDate>,
    note_type: Option<String>,
    parent: Option<String>,
    bib_key: Option<String>,
    bibtex: Option<String>,
    authors: Option<String>,
    venue: Option<String>,
    year: Option<i32>,
    time: Vec<TimeEntry>,
    sources: Vec<PaperSource>,
}

fn parse_frontmatter(content: &str) -> (Frontmatter, String) {
    let mut fm = Frontmatter::default();
    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() || lines[0].trim() != "---" {
        return (fm, content.to_string());
    }

    let mut end_idx = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            end_idx = Some(i);
            break;
        }
    }

    let end_idx = match end_idx {
        Some(i) => i,
        None => return (fm, content.to_string()),
    };

    let mut current_key: Option<String> = None;
    let mut multiline_value = String::new();
    let mut in_time_block = false;
    let mut time_entries: Vec<TimeEntry> = Vec::new();
    let mut current_time: Option<(NaiveDate, u32, TimeCategory, Option<String>)> = None;

    for line in &lines[1..end_idx] {
        let trimmed = line.trim();

        if in_time_block {
            if trimmed.starts_with("- date:") {
                if let Some((date, mins, cat, desc)) = current_time.take() {
                    time_entries.push(TimeEntry {
                        date,
                        minutes: mins,
                        category: cat,
                        description: desc,
                    });
                }
                if let Some(date_str) = trimmed.strip_prefix("- date:") {
                    if let Ok(date) = NaiveDate::parse_from_str(date_str.trim(), "%Y-%m-%d") {
                        current_time = Some((date, 0, TimeCategory::Other("unset".into()), None));
                    }
                }
                continue;
            } else if trimmed.starts_with("minutes:") {
                if let Some(ref mut t) = current_time {
                    if let Ok(mins) = trimmed.strip_prefix("minutes:").unwrap().trim().parse() {
                        t.1 = mins;
                    }
                }
                continue;
            } else if trimmed.starts_with("category:") {
                if let Some(ref mut t) = current_time {
                    let cat_str = trimmed.strip_prefix("category:").unwrap().trim();
                    t.2 = match cat_str {
                        "programming" => TimeCategory::Programming,
                        "teaching" => TimeCategory::Teaching,
                        "reading" => TimeCategory::Reading,
                        "writing" => TimeCategory::Writing,
                        "service" => TimeCategory::Service,
                        other => TimeCategory::Other(other.to_string()),
                    };
                }
                continue;
            } else if trimmed.starts_with("description:") {
                if let Some(ref mut t) = current_time {
                    t.3 = Some(trimmed.strip_prefix("description:").unwrap().trim().to_string());
                }
                continue;
            } else if !trimmed.is_empty() && !trimmed.starts_with('-') && !line.starts_with("  ") && !line.starts_with("\t") {
                if let Some((date, mins, cat, desc)) = current_time.take() {
                    time_entries.push(TimeEntry {
                        date,
                        minutes: mins,
                        category: cat,
                        description: desc,
                    });
                }
                in_time_block = false;
            }
        }

        if line.starts_with("  ") || line.starts_with("\t") {
            if current_key.is_some() {
                multiline_value.push_str(trimmed);
                multiline_value.push('\n');
            }
            continue;
        }

        if let Some(ref key) = current_key {
            if !multiline_value.is_empty() && key.as_str() == "bibtex" {
                fm.bibtex = Some(multiline_value.trim().to_string());
                multiline_value.clear();
            }
        }

        if let Some((key, value)) = trimmed.split_once(':') {
            let key = key.trim().to_lowercase();
            let value = value.trim();

            current_key = Some(key.clone());

            match key.as_str() {
                "title" => fm.title = Some(value.to_string()),
                "date" => {
                    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
                        fm.date = Some(date);
                    }
                }
                "type" => fm.note_type = Some(value.to_string()),
                "parent" => fm.parent = Some(value.to_string()),
                "bib_key" | "bibkey" => fm.bib_key = Some(value.to_string()),
                "authors" => fm.authors = Some(value.to_string()),
                "venue" => fm.venue = Some(value.to_string()),
                "year" => {
                    if let Ok(y) = value.parse() {
                        fm.year = Some(y);
                    }
                }
                "bibtex" => {
                    if !value.starts_with('|') && !value.is_empty() {
                        fm.bibtex = Some(value.to_string());
                    }
                }
                "arxiv" => {
                    if !value.is_empty() {
                        fm.sources.push(PaperSource {
                            source_type: "arxiv".to_string(),
                            identifier: value.to_string(),
                        });
                    }
                }
                "doi" => {
                    if !value.is_empty() {
                        fm.sources.push(PaperSource {
                            source_type: "doi".to_string(),
                            identifier: value.to_string(),
                        });
                    }
                }
                "url" | "source_url" => {
                    if !value.is_empty() {
                        fm.sources.push(PaperSource {
                            source_type: "url".to_string(),
                            identifier: value.to_string(),
                        });
                    }
                }
                "time" => {
                    in_time_block = true;
                }
                _ => {}
            }
        }
    }

    if let Some((date, mins, cat, desc)) = current_time.take() {
        time_entries.push(TimeEntry {
            date,
            minutes: mins,
            category: cat,
            description: desc,
        });
    }
    fm.time = time_entries;

    if let Some(ref key) = current_key {
        if !multiline_value.is_empty() && key.as_str() == "bibtex" {
            fm.bibtex = Some(multiline_value.trim().to_string());
        }
    }

    let body = lines[end_idx + 1..].join("\n");
    (fm, body)
}

// ============================================================================
// Key Generation
// ============================================================================

fn generate_key(path: &PathBuf) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(path.to_string_lossy().as_bytes());
    let result = hasher.finalize();
    hex_encode(&result[..3])
}

// ============================================================================
// Note Loading
// ============================================================================

fn load_note(path: &PathBuf, notes_dir: &PathBuf) -> Option<Note> {
    let content = fs::read_to_string(path).ok()?;
    let relative_path = path.strip_prefix(notes_dir).ok()?.to_path_buf();
    let key = generate_key(&relative_path);

    let (fm, body) = parse_frontmatter(&content);

    let title = fm.title.unwrap_or_else(|| {
        relative_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".to_string())
    });

    let metadata = fs::metadata(path).ok()?;
    let modified: DateTime<Utc> = metadata.modified().ok()?.into();

    let note_type = if fm.note_type.as_deref() == Some("paper") || fm.bib_key.is_some() {
        NoteType::Paper(PaperMeta {
            bib_key: fm.bib_key.unwrap_or_else(|| key.clone()),
            bibtex: fm.bibtex,
            authors: fm.authors,
            venue: fm.venue,
            year: fm.year,
            sources: fm.sources,
        })
    } else {
        NoteType::Note
    };

    Some(Note {
        key,
        path: relative_path,
        title,
        date: fm.date,
        note_type,
        parent_key: fm.parent,
        time_entries: fm.time,
        raw_content: body,
        full_file_content: content,
        modified,
    })
}

fn load_all_notes(notes_dir: &PathBuf) -> Vec<Note> {
    let mut notes = Vec::new();

    for entry in WalkDir::new(notes_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().map(|e| e == "md").unwrap_or(false) {
            if let Some(note) = load_note(&path.to_path_buf(), notes_dir) {
                notes.push(note);
            }
        }
    }

    notes.sort_by(|a, b| b.modified.cmp(&a.modified));
    notes
}

// ============================================================================
// Full-Text Search
// ============================================================================

fn search_notes(notes: &[Note], query: &str) -> Vec<SearchResult> {
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for note in notes {
        let mut matches = Vec::new();

        if note.title.to_lowercase().contains(&query_lower) {
            matches.push(SearchMatch {
                line_number: 0,
                line_content: format!("Title: {}", note.title),
            });
        }

        for (i, line) in note.full_file_content.lines().enumerate() {
            if line.to_lowercase().contains(&query_lower) {
                matches.push(SearchMatch {
                    line_number: i + 1,
                    line_content: line.to_string(),
                });
            }
        }

        if !matches.is_empty() {
            results.push(SearchResult {
                note: note.clone(),
                matches,
            });
        }
    }

    results
}

// ============================================================================
// Cross-link Processing
// ============================================================================

fn process_crosslinks(content: &str, notes: &HashMap<String, Note>) -> String {
    let mut result = content.to_string();
    let mut replacements = Vec::new();

    let mut i = 0;
    while i < result.len() {
        if let Some(start) = result[i..].find("[@") {
            let abs_start = i + start;
            if let Some(end) = result[abs_start..].find(']') {
                let abs_end = abs_start + end + 1;
                let key = &result[abs_start + 2..abs_end - 1];

                if let Some(note) = notes.get(key) {
                    let replacement = format!(
                        r#"<a href="/note/{}" class="crosslink" title="{}">{}</a>"#,
                        key,
                        html_escape(&note.title),
                        html_escape(&note.title)
                    );
                    replacements.push((abs_start, abs_end, replacement));
                }
                i = abs_end;
            } else {
                i += 1;
            }
        } else {
            break;
        }
    }

    for (start, end, replacement) in replacements.into_iter().rev() {
        result.replace_range(start..end, &replacement);
    }

    result
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn js_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace('$', "\\$")
}

// ============================================================================
// Markdown Rendering
// ============================================================================

fn render_markdown(content: &str) -> String {
    let parser = Parser::new(content);
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);
    html_output
}

// ============================================================================
// Git Integration
// ============================================================================

fn get_git_history(file_path: &PathBuf, notes_dir: &PathBuf) -> Vec<GitCommit> {
    let full_path = notes_dir.join(file_path);

    let output = Command::new("git")
        .args(["log", "--format=%H|%aI|%an|%s", "--follow", "--"])
        .arg(&full_path)
        .current_dir(notes_dir)
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(4, '|').collect();
            if parts.len() == 4 {
                let date = DateTime::parse_from_rfc3339(parts[1])
                    .ok()?
                    .with_timezone(&Utc);
                Some(GitCommit {
                    hash: parts[0].to_string(),
                    date,
                    author: parts[2].to_string(),
                    message: parts[3].to_string(),
                })
            } else {
                None
            }
        })
        .collect()
}

fn get_file_at_commit(file_path: &PathBuf, commit_hash: &str, notes_dir: &PathBuf) -> Option<String> {
    let output = Command::new("git")
        .args(["show", &format!("{}:{}", commit_hash, file_path.display())])
        .current_dir(notes_dir)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

// ============================================================================
// Bibliography Export
// ============================================================================

fn generate_bibliography(notes: &[Note]) -> String {
    let mut bib = String::new();

    for note in notes {
        if let NoteType::Paper(ref paper) = note.note_type {
            if let Some(ref bibtex) = paper.bibtex {
                bib.push_str(bibtex);
                bib.push_str("\n\n");
            } else {
                bib.push_str(&format!("@misc{{{},\n", paper.bib_key));
                bib.push_str(&format!("  title = {{{}}},\n", note.title));
                if let Some(ref authors) = paper.authors {
                    bib.push_str(&format!("  author = {{{}}},\n", authors));
                }
                if let Some(year) = paper.year {
                    bib.push_str(&format!("  year = {{{}}},\n", year));
                }
                if let Some(ref venue) = paper.venue {
                    bib.push_str(&format!("  howpublished = {{{}}},\n", venue));
                }
                bib.push_str("}\n\n");
            }
        }
    }

    bib
}

// ============================================================================
// Smart Add: Input Detection & Local Search
// ============================================================================

fn detect_input_type(input: &str) -> InputType {
    let input = input.trim();

    // Check for arXiv patterns
    if let Some(arxiv_id) = extract_arxiv_id(input) {
        return InputType::ArxivUrl { arxiv_id };
    }

    // Check for DOI patterns
    if let Some(doi) = extract_doi(input) {
        return InputType::DoiUrl { doi };
    }

    // Check for URL patterns
    if input.starts_with("http://") || input.starts_with("https://") {
        return InputType::GenericUrl { url: input.to_string() };
    }

    InputType::PlainText { text: input.to_string() }
}

fn extract_arxiv_id(input: &str) -> Option<String> {
    // Match arxiv URLs or bare IDs
    // Formats: arxiv.org/abs/2301.00001, arxiv.org/pdf/2301.00001.pdf, 2301.00001, arXiv:2301.00001
    let patterns = [
        r"arxiv\.org/(?:abs|pdf)/(\d{4}\.\d{4,5})",
        r"arxiv\.org/(?:abs|pdf)/([a-z-]+/\d{7})",
        r"arXiv:(\d{4}\.\d{4,5})",
        r"^(\d{4}\.\d{4,5})$",
    ];

    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(input) {
                if let Some(m) = caps.get(1) {
                    return Some(m.as_str().to_string());
                }
            }
        }
    }
    None
}

fn extract_doi(input: &str) -> Option<String> {
    // Match DOI patterns from various sources
    // DOI format: 10.XXXX/... where XXXX is 4+ digits
    let patterns = [
        // Standard DOI URLs
        r#"(?:doi\.org|dx\.doi\.org)/?(10\.\d{4,}/[^\s\]"'<>]+)"#,
        // ACM Digital Library: dl.acm.org/doi/10.1145/...
        r#"dl\.acm\.org/doi/(10\.\d{4,}/[^\s\]"'<>]+)"#,
        // IEEE Xplore: ieeexplore.ieee.org/document/... doesn't have DOI in URL, skip
        // Springer: link.springer.com/article/10.1007/...
        r#"link\.springer\.com/(?:article|chapter)/(10\.\d{4,}/[^\s\]"'<>]+)"#,
        // Wiley: onlinelibrary.wiley.com/doi/10.1002/...
        r#"onlinelibrary\.wiley\.com/doi/(?:abs/|full/)?(10\.\d{4,}/[^\s\]"'<>]+)"#,
        // Nature: nature.com/articles/... (DOI embedded)
        r#"nature\.com/articles/(10\.\d{4,}/[^\s\]"'<>]+)"#,
        // Science Direct / Elsevier
        r#"sciencedirect\.com/science/article/pii/[^?]+\?.*doi=(10\.\d{4,}/[^\s\]"'<>&]+)"#,
        // PLOS
        r#"journals\.plos\.org/\w+/article\?id=(10\.\d{4,}/[^\s\]"'<>&]+)"#,
        // Generic: any URL containing a DOI pattern
        r#"/(10\.\d{4,}/[^\s\]"'<>/?#]+)"#,
        // Bare DOI
        r#"^(10\.\d{4,}/[^\s\]"'<>]+)$"#,
    ];

    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(input) {
                if let Some(m) = caps.get(1) {
                    // Clean up the DOI (remove trailing punctuation)
                    let doi = m.as_str().trim_end_matches(|c| c == '.' || c == ',' || c == ';');
                    return Some(doi.to_string());
                }
            }
        }
    }
    None
}

fn search_local_for_match(notes: &[Note], input: &str, input_type: &InputType) -> Option<LocalMatch> {
    let input_lower = input.to_lowercase();

    // Helper to check if a note has a matching source
    let check_source = |note: &Note, source_type: &str, identifier: &str| -> bool {
        if let NoteType::Paper(ref paper) = note.note_type {
            for source in &paper.sources {
                if source.source_type == source_type && source.identifier == identifier {
                    return true;
                }
            }
        }
        // Also check content for backwards compatibility
        note.full_file_content.contains(identifier)
    };

    // For arXiv IDs, check sources and content
    if let InputType::ArxivUrl { arxiv_id } = input_type {
        for note in notes {
            if check_source(note, "arxiv", arxiv_id) {
                return Some(LocalMatch {
                    key: note.key.clone(),
                    title: note.title.clone(),
                    match_type: "arxiv".to_string(),
                });
            }
        }
    }

    // For DOIs, check sources and content
    if let InputType::DoiUrl { doi } = input_type {
        for note in notes {
            if check_source(note, "doi", doi) {
                return Some(LocalMatch {
                    key: note.key.clone(),
                    title: note.title.clone(),
                    match_type: "doi".to_string(),
                });
            }
        }
    }

    // For any URL, try to extract a DOI and search for it
    if let Some(doi) = extract_doi(input) {
        for note in notes {
            if check_source(note, "doi", &doi) {
                return Some(LocalMatch {
                    key: note.key.clone(),
                    title: note.title.clone(),
                    match_type: "doi".to_string(),
                });
            }
        }
    }

    // Check for title matches
    for note in notes {
        let note_title_lower = note.title.to_lowercase();

        // Exact title match
        if note_title_lower == input_lower {
            return Some(LocalMatch {
                key: note.key.clone(),
                title: note.title.clone(),
                match_type: "exact".to_string(),
            });
        }
    }

    // Fuzzy title match (for papers, check title + authors + year)
    for note in notes {
        if let NoteType::Paper(ref paper) = note.note_type {
            let note_title_lower = note.title.to_lowercase();

            // Check if input contains significant portion of title
            let title_words: Vec<&str> = note_title_lower.split_whitespace().collect();
            let input_words: Vec<&str> = input_lower.split_whitespace().collect();

            if title_words.len() >= 3 {
                let matching_words = title_words.iter()
                    .filter(|w| w.len() > 3 && input_words.contains(w))
                    .count();

                if matching_words >= title_words.len() * 2 / 3 {
                    // Also check author/year if available
                    let mut score = matching_words;
                    if let Some(ref authors) = paper.authors {
                        if input_lower.contains(&authors.to_lowercase().split(',').next().unwrap_or("").trim().to_lowercase()) {
                            score += 2;
                        }
                    }
                    if let Some(year) = paper.year {
                        if input.contains(&year.to_string()) {
                            score += 2;
                        }
                    }
                    if score >= title_words.len() * 2 / 3 + 2 {
                        return Some(LocalMatch {
                            key: note.key.clone(),
                            title: note.title.clone(),
                            match_type: "title".to_string(),
                        });
                    }
                }
            }
        }
    }

    None
}

fn generate_bib_key(title: &str, authors: Option<&str>, year: Option<i32>) -> String {
    // Format: lastname + year + keyword
    // Example: vaswani2017attention

    let lastname = authors
        .and_then(|a| a.split(',').next())
        .and_then(|a| a.split_whitespace().last())
        .unwrap_or("unknown")
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphabetic())
        .collect::<String>();

    let year_str = year.map(|y| y.to_string()).unwrap_or_default();

    // Get first significant word from title (skip common words)
    let skip_words = ["a", "an", "the", "on", "of", "for", "to", "in", "with", "and", "is", "are"];
    let keyword = title
        .split_whitespace()
        .find(|w| !skip_words.contains(&w.to_lowercase().as_str()) && w.len() > 2)
        .unwrap_or("paper")
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphabetic())
        .collect::<String>();

    format!("{}{}{}", lastname, year_str, keyword)
}

fn generate_suggested_filename(title: &str) -> String {
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .take(6)
        .collect::<Vec<_>>()
        .join("-");

    format!("{}.md", slug)
}

// ============================================================================
// Smart Add: External API Integration
// ============================================================================

async fn query_arxiv_api(arxiv_id: &str) -> Option<ExternalResult> {
    let url = format!("https://export.arxiv.org/api/query?id_list={}", arxiv_id);

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build() {
        Ok(c) => c,
        Err(_) => return None,
    };

    let response = match client.get(&url).send().await {
        Ok(r) => r,
        Err(_) => return None,
    };

    let text = match response.text().await {
        Ok(t) => t,
        Err(_) => return None,
    };

    // Parse XML response - need to extract from <entry> not the feed
    // The feed has its own <title> which is "arXiv Query: ..."
    // We want the <title> inside <entry>
    let entry = extract_xml_tag(&text, "entry")?;

    let title = extract_xml_tag(&entry, "title")
        .map(|t| t.trim().replace('\n', " ").replace("  ", " "))
        .filter(|t| !t.starts_with("Error") && !t.is_empty())?;

    let authors: Vec<String> = extract_all_xml_tags(&entry, "name")
        .into_iter()
        .map(|s| s.trim().to_string())
        .collect();
    let authors_str = if authors.is_empty() { None } else { Some(authors.join(" and ")) };

    let published = extract_xml_tag(&entry, "published");
    let year = published.and_then(|p| p.get(..4).and_then(|y| y.parse().ok()));

    let bib_key = generate_bib_key(&title, authors_str.as_deref(), year);
    let suggested_filename = generate_suggested_filename(&title);

    // Generate bibtex
    let bibtex = format!(
        "@article{{{},\n  title = {{{}}},\n  author = {{{}}},\n  year = {{{}}},\n  eprint = {{{}}},\n  archivePrefix = {{arXiv}},\n}}",
        bib_key,
        title,
        authors_str.as_deref().unwrap_or(""),
        year.unwrap_or(0),
        arxiv_id
    );

    Some(ExternalResult {
        title,
        authors: authors_str,
        year,
        venue: Some("arXiv".to_string()),
        bib_key,
        bibtex: Some(bibtex),
        suggested_filename,
        source: "arxiv".to_string(),
    })
}

async fn query_crossref_api(doi: &str) -> Option<ExternalResult> {
    let url = format!("https://api.crossref.org/works/{}", doi);

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build() {
        Ok(c) => c,
        Err(_) => return None,
    };

    let response = match client.get(&url)
        .header("User-Agent", "NotesApp/1.0 (mailto:user@example.com)")
        .send().await {
        Ok(r) => r,
        Err(_) => return None,
    };

    let json: serde_json::Value = match response.json().await {
        Ok(j) => j,
        Err(_) => return None,
    };

    let message = json.get("message")?;

    let title = message.get("title")
        .and_then(|t| t.as_array())
        .and_then(|a| a.first())
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())?;

    let authors: Vec<String> = message.get("author")
        .and_then(|a| a.as_array())
        .map(|authors| {
            authors.iter()
                .filter_map(|a| {
                    let given = a.get("given").and_then(|g| g.as_str()).unwrap_or("");
                    let family = a.get("family").and_then(|f| f.as_str()).unwrap_or("");
                    if family.is_empty() { None } else { Some(format!("{} {}", given, family).trim().to_string()) }
                })
                .collect()
        })
        .unwrap_or_default();
    let authors_str = if authors.is_empty() { None } else { Some(authors.join(" and ")) };

    let year = message.get("published")
        .or_else(|| message.get("published-print"))
        .or_else(|| message.get("published-online"))
        .and_then(|p| p.get("date-parts"))
        .and_then(|d| d.as_array())
        .and_then(|a| a.first())
        .and_then(|a| a.as_array())
        .and_then(|a| a.first())
        .and_then(|y| y.as_i64())
        .map(|y| y as i32);

    let venue = message.get("container-title")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let bib_key = generate_bib_key(&title, authors_str.as_deref(), year);
    let suggested_filename = generate_suggested_filename(&title);

    // Generate bibtex
    let bibtex = format!(
        "@article{{{},\n  title = {{{}}},\n  author = {{{}}},\n  year = {{{}}},\n  doi = {{{}}},\n  journal = {{{}}},\n}}",
        bib_key,
        title,
        authors_str.as_deref().unwrap_or(""),
        year.unwrap_or(0),
        doi,
        venue.as_deref().unwrap_or("")
    );

    Some(ExternalResult {
        title,
        authors: authors_str,
        year,
        venue,
        bib_key,
        bibtex: Some(bibtex),
        suggested_filename,
        source: "crossref".to_string(),
    })
}

async fn query_crossref_by_title(title: &str) -> Option<ExternalResult> {
    let encoded_title = urlencoding::encode(title);
    let url = format!("https://api.crossref.org/works?query.title={}&rows=1", encoded_title);

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build() {
        Ok(c) => c,
        Err(_) => return None,
    };

    let response = match client.get(&url)
        .header("User-Agent", "NotesApp/1.0 (mailto:user@example.com)")
        .send().await {
        Ok(r) => r,
        Err(_) => return None,
    };

    let json: serde_json::Value = match response.json().await {
        Ok(j) => j,
        Err(_) => return None,
    };

    let items = json.get("message")?.get("items")?.as_array()?;
    let item = items.first()?;

    let found_title = item.get("title")
        .and_then(|t| t.as_array())
        .and_then(|a| a.first())
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())?;

    // Check if title is similar enough
    let title_lower = title.to_lowercase();
    let found_lower = found_title.to_lowercase();
    if !found_lower.contains(&title_lower) && !title_lower.contains(&found_lower) {
        // Titles too different
        return None;
    }

    let doi = item.get("DOI").and_then(|d| d.as_str())?;
    query_crossref_api(doi).await
}

/// Fetch a URL and extract paper metadata from HTML meta tags
async fn fetch_and_extract_metadata(url: &str) -> Option<ExternalResult> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (compatible; NotesApp/1.0)")
        .build()
        .ok()?;

    let response = client.get(url).send().await.ok()?;
    let html = response.text().await.ok()?;

    // First, try to extract DOI from the page and use CrossRef
    if let Some(doi) = extract_doi_from_html(&html) {
        if let Some(result) = query_crossref_api(&doi).await {
            return Some(result);
        }
    }

    // Extract metadata from HTML meta tags
    let title = extract_meta_content(&html, &[
        "citation_title",
        "DC.title",
        "og:title",
        "twitter:title",
    ]).or_else(|| extract_html_title(&html))?;

    // Skip if title looks like an error page
    if title.to_lowercase().contains("access denied")
        || title.to_lowercase().contains("404")
        || title.to_lowercase().contains("not found") {
        return None;
    }

    let authors = extract_meta_content(&html, &["citation_author", "DC.creator", "author"])
        .or_else(|| extract_all_meta_content(&html, "citation_author"));

    let year = extract_meta_content(&html, &["citation_publication_date", "citation_date", "DC.date", "article:published_time"])
        .and_then(|d| d.get(..4).and_then(|y| y.parse().ok()));

    let venue = extract_meta_content(&html, &["citation_journal_title", "citation_conference_title", "DC.publisher", "og:site_name"]);

    let bib_key = generate_bib_key(&title, authors.as_deref(), year);
    let suggested_filename = generate_suggested_filename(&title);

    let bibtex = format!(
        "@misc{{{},\n  title = {{{}}},\n  author = {{{}}},\n  year = {{{}}},\n  howpublished = {{\\url{{{}}}}},\n}}",
        bib_key,
        title,
        authors.as_deref().unwrap_or(""),
        year.unwrap_or(0),
        url
    );

    Some(ExternalResult {
        title,
        authors,
        year,
        venue,
        bib_key,
        bibtex: Some(bibtex),
        suggested_filename,
        source: "webpage".to_string(),
    })
}

fn extract_doi_from_html(html: &str) -> Option<String> {
    // Look for DOI in meta tags
    let doi_patterns = [
        r#"name="citation_doi"\s+content="([^"]+)""#,
        r#"name="DC.identifier"\s+content="([^"]+)""#,
        r#"name="doi"\s+content="([^"]+)""#,
        r#"content="([^"]+)"\s+name="citation_doi""#,
        r#"data-doi="([^"]+)""#,
    ];

    for pattern in doi_patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(html) {
                if let Some(m) = caps.get(1) {
                    let doi = m.as_str();
                    // Validate it looks like a DOI
                    if doi.starts_with("10.") {
                        return Some(doi.to_string());
                    }
                }
            }
        }
    }

    // Also look for DOI in text content
    if let Ok(re) = Regex::new(r#"(?:doi|DOI)[:\s]+(?:https?://(?:dx\.)?doi\.org/)?(10\.\d{4,}/[^\s<"']+)"#) {
        if let Some(caps) = re.captures(html) {
            if let Some(m) = caps.get(1) {
                return Some(m.as_str().trim_end_matches(|c| c == '.' || c == ',').to_string());
            }
        }
    }

    None
}

fn extract_meta_content(html: &str, names: &[&str]) -> Option<String> {
    for name in names {
        // Try both name="X" content="Y" and content="Y" name="X" orders
        let patterns = [
            format!(r#"(?i)<meta[^>]*name=["']{}["'][^>]*content=["']([^"']+)["']"#, regex::escape(name)),
            format!(r#"(?i)<meta[^>]*content=["']([^"']+)["'][^>]*name=["']{}["']"#, regex::escape(name)),
            format!(r#"(?i)<meta[^>]*property=["']{}["'][^>]*content=["']([^"']+)["']"#, regex::escape(name)),
            format!(r#"(?i)<meta[^>]*content=["']([^"']+)["'][^>]*property=["']{}["']"#, regex::escape(name)),
        ];

        for pattern in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(caps) = re.captures(html) {
                    if let Some(m) = caps.get(1) {
                        let content = m.as_str().trim();
                        if !content.is_empty() {
                            return Some(html_entity_decode(content));
                        }
                    }
                }
            }
        }
    }
    None
}

fn extract_all_meta_content(html: &str, name: &str) -> Option<String> {
    let pattern = format!(r#"(?i)<meta[^>]*name=["']{}["'][^>]*content=["']([^"']+)["']"#, regex::escape(name));
    let mut authors = Vec::new();

    if let Ok(re) = Regex::new(&pattern) {
        for caps in re.captures_iter(html) {
            if let Some(m) = caps.get(1) {
                authors.push(html_entity_decode(m.as_str().trim()));
            }
        }
    }

    if authors.is_empty() {
        None
    } else {
        Some(authors.join(" and "))
    }
}

fn extract_html_title(html: &str) -> Option<String> {
    let pattern = r"(?i)<title[^>]*>([^<]+)</title>";
    if let Ok(re) = Regex::new(pattern) {
        if let Some(caps) = re.captures(html) {
            if let Some(m) = caps.get(1) {
                let title = html_entity_decode(m.as_str().trim());
                // Clean up common suffixes
                let title = title
                    .split(" | ").next().unwrap_or(&title)
                    .split(" - ").next().unwrap_or(&title)
                    .trim();
                if !title.is_empty() {
                    return Some(title.to_string());
                }
            }
        }
    }
    None
}

fn html_entity_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
}

async fn query_claude_for_url(url: &str) -> Option<ExternalResult> {
    let prompt = format!(
        "Extract paper/article metadata from this URL: {}\n\n\
        Return ONLY a JSON object with these fields (no other text):\n\
        {{\"title\": \"...\", \"authors\": \"Author1 and Author2\", \"year\": 2024, \"venue\": \"...\"}}\n\n\
        If you cannot access or parse the URL, return: {{\"error\": \"cannot access\"}}",
        url
    );

    let output = tokio::task::spawn_blocking(move || {
        Command::new("claude")
            .args(["-p", &prompt])
            .output()
    }).await.ok()?.ok()?;

    if !output.status.success() {
        return None;
    }

    let response = String::from_utf8_lossy(&output.stdout);

    // Try to extract JSON from response
    let json_start = response.find('{')?;
    let json_end = response.rfind('}')?;
    let json_str = &response[json_start..=json_end];

    let json: serde_json::Value = serde_json::from_str(json_str).ok()?;

    if json.get("error").is_some() {
        return None;
    }

    let title = json.get("title").and_then(|t| t.as_str()).map(|s| s.to_string())?;
    let authors = json.get("authors").and_then(|a| a.as_str()).map(|s| s.to_string());
    let year = json.get("year").and_then(|y| y.as_i64()).map(|y| y as i32);
    let venue = json.get("venue").and_then(|v| v.as_str()).map(|s| s.to_string());

    let bib_key = generate_bib_key(&title, authors.as_deref(), year);
    let suggested_filename = generate_suggested_filename(&title);

    let bibtex = format!(
        "@misc{{{},\n  title = {{{}}},\n  author = {{{}}},\n  year = {{{}}},\n  howpublished = {{\\url{{{}}}}},\n}}",
        bib_key,
        title,
        authors.as_deref().unwrap_or(""),
        year.unwrap_or(0),
        url
    );

    Some(ExternalResult {
        title,
        authors,
        year,
        venue,
        bib_key,
        bibtex: Some(bibtex),
        suggested_filename,
        source: "claude".to_string(),
    })
}

fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{}>", tag);
    let end_tag = format!("</{}>", tag);

    let start = xml.find(&start_tag)? + start_tag.len();
    let end = xml[start..].find(&end_tag)? + start;

    Some(xml[start..end].to_string())
}

fn extract_all_xml_tags(xml: &str, tag: &str) -> Vec<String> {
    let start_tag = format!("<{}>", tag);
    let end_tag = format!("</{}>", tag);
    let mut results = Vec::new();
    let mut search_start = 0;

    while let Some(start_pos) = xml[search_start..].find(&start_tag) {
        let abs_start = search_start + start_pos + start_tag.len();
        if let Some(end_pos) = xml[abs_start..].find(&end_tag) {
            results.push(xml[abs_start..abs_start + end_pos].to_string());
            search_start = abs_start + end_pos + end_tag.len();
        } else {
            break;
        }
    }

    results
}

// ============================================================================
// Application State
// ============================================================================

#[derive(Clone)]
struct AppState {
    notes_dir: PathBuf,
    #[allow(dead_code)]
    db: Db,
}

impl AppState {
    fn new() -> Self {
        let notes_dir = PathBuf::from(NOTES_DIR);
        fs::create_dir_all(&notes_dir).ok();

        let db = sled::open(DB_PATH).expect("Failed to open database");

        Self { notes_dir, db }
    }

    fn load_notes(&self) -> Vec<Note> {
        load_all_notes(&self.notes_dir)
    }

    fn notes_map(&self) -> HashMap<String, Note> {
        self.load_notes()
            .into_iter()
            .map(|n| (n.key.clone(), n))
            .collect()
    }
}

// ============================================================================
// HTML Templates
// ============================================================================

const STYLE: &str = r#"
/* Solarized Light Theme */
:root {
    --base03: #002b36;
    --base02: #073642;
    --base01: #586e75;
    --base00: #657b83;
    --base0: #839496;
    --base1: #93a1a1;
    --base2: #eee8d5;
    --base3: #fdf6e3;

    --yellow: #b58900;
    --orange: #cb4b16;
    --red: #dc322f;
    --magenta: #d33682;
    --violet: #6c71c4;
    --blue: #268bd2;
    --cyan: #2aa198;
    --green: #859900;

    --bg: var(--base3);
    --fg: var(--base00);
    --muted: var(--base1);
    --border: var(--base2);
    --link: var(--blue);
    --link-hover: var(--cyan);
    --accent: var(--base2);
    --paper-bg: #f5ecd5;
    --code-bg: var(--base2);
    --highlight: #f7f2e2;
}

* { box-sizing: border-box; margin: 0; padding: 0; }

body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
    line-height: 1.6;
    color: var(--fg);
    background: var(--bg);
}

.container {
    max-width: 900px;
    margin: 0 auto;
    padding: 1rem;
}

a { color: var(--link); text-decoration: none; }
a:hover { color: var(--link-hover); text-decoration: underline; }

h1, h2, h3 { font-weight: 600; margin-top: 1.5em; margin-bottom: 0.5em; }
h1 { font-size: 1.5rem; }

.nav-bar {
    position: sticky;
    top: 0;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    padding: 0.5rem 1rem;
    display: flex;
    gap: 1rem;
    align-items: center;
    flex-wrap: wrap;
    z-index: 100;
}

.nav-bar a, .nav-bar button { font-size: 0.9rem; }
.nav-bar .spacer { flex: 1; }

.nav-bar button {
    background: none;
    border: none;
    color: var(--link);
    cursor: pointer;
    font-family: inherit;
}
.nav-bar button:hover { color: var(--link-hover); text-decoration: underline; }

.search-box {
    display: flex;
    gap: 0.5rem;
}

.search-box input {
    padding: 0.4rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--fg);
    font-size: 0.9rem;
    width: 180px;
}

.search-box button {
    padding: 0.4rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--accent);
    color: var(--fg);
    cursor: pointer;
    font-size: 0.9rem;
}

.note-list { list-style: none; }

.note-item {
    padding: 0.75rem 0;
    border-bottom: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 1rem;
}

.note-item:last-child { border-bottom: none; }
.note-item .title { font-size: 1rem; }
.note-item .meta { font-size: 0.8rem; color: var(--muted); white-space: nowrap; }
.note-item .key { font-family: monospace; font-size: 0.7rem; color: var(--muted); margin-left: 0.5rem; }
.note-item.paper { background: var(--paper-bg); margin: 0 -1rem; padding-left: 1rem; padding-right: 1rem; }

.type-badge {
    font-size: 0.65rem;
    padding: 0.1rem 0.4rem;
    background: var(--accent);
    border-radius: 3px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-right: 0.5rem;
    vertical-align: middle;
}

.note-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
    flex-wrap: wrap;
    gap: 0.5rem;
}

.note-header h1 { margin: 0; flex: 1; }

.mode-toggle {
    display: flex;
    gap: 0;
    border: 1px solid var(--border);
    border-radius: 4px;
    overflow: hidden;
}

.mode-toggle button {
    padding: 0.4rem 1rem;
    border: none;
    background: var(--accent);
    color: var(--fg);
    cursor: pointer;
    font-size: 0.85rem;
    font-family: inherit;
}

.mode-toggle button.active {
    background: var(--link);
    color: white;
}

.mode-toggle button:hover:not(.active) {
    background: var(--border);
}

.note-content { margin-top: 1rem; }
.note-content pre {
    background: var(--accent);
    padding: 1rem;
    overflow-x: auto;
    border-radius: 4px;
    margin: 1rem 0;
}
.note-content code {
    font-family: "SF Mono", "Consolas", "Liberation Mono", monospace;
    font-size: 0.9em;
}
.note-content p code {
    background: var(--accent);
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
}
.note-content blockquote {
    border-left: 3px solid var(--border);
    margin: 1rem 0;
    padding-left: 1rem;
    color: var(--muted);
}
.note-content ul, .note-content ol {
    margin: 1rem 0;
    padding-left: 1.5rem;
}
.note-content p { margin: 1rem 0; }

.crosslink {
    background: var(--accent);
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
    font-size: 0.9em;
}

.meta-block {
    background: var(--accent);
    padding: 0.75rem 1rem;
    margin-bottom: 1rem;
    border-radius: 4px;
    font-size: 0.85rem;
}
.meta-block dt { font-weight: 600; display: inline; }
.meta-block dd { display: inline; margin: 0; margin-right: 1.5rem; }

.time-table { width: 100%; border-collapse: collapse; font-size: 0.85rem; margin-top: 1rem; }
.time-table th, .time-table td { padding: 0.5rem; text-align: left; border-bottom: 1px solid var(--border); }
.time-table th { font-weight: 600; }

.history-list { font-size: 0.85rem; }
.history-item { padding: 0.5rem 0; border-bottom: 1px solid var(--border); }
.history-item:last-child { border-bottom: none; }
.history-hash { font-family: monospace; color: var(--muted); }

.sub-notes { margin-top: 1rem; padding-top: 1rem; border-top: 1px solid var(--border); }
.sub-notes h3 { font-size: 1rem; margin-top: 0; }

.time-summary { margin-top: 2rem; }
.time-bar { display: flex; height: 24px; border-radius: 4px; overflow: hidden; margin: 0.5rem 0; }
.time-segment { height: 100%; }
.time-legend { display: flex; flex-wrap: wrap; gap: 1rem; font-size: 0.8rem; margin-top: 0.5rem; }
.time-legend-item { display: flex; align-items: center; gap: 0.3rem; }
.time-legend-color { width: 12px; height: 12px; border-radius: 2px; }

.cat-programming { background: var(--blue); }
.cat-teaching { background: var(--green); }
.cat-reading { background: var(--orange); }
.cat-writing { background: var(--magenta); }
.cat-service { background: var(--base1); }
.cat-other { background: var(--base0); }

.search-results .match {
    font-family: monospace;
    font-size: 0.85rem;
    background: var(--accent);
    padding: 0.25rem 0.5rem;
    margin: 0.25rem 0;
    border-radius: 3px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.search-results .match .line-num {
    color: var(--muted);
    margin-right: 0.5rem;
}
.search-results .result-group {
    margin-bottom: 1.5rem;
}

.editor-container {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 500;
    background: #fdf6e3; /* solarized-light base3 */
}

.editor-header {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 48px;
    background: #eee8d5; /* solarized-light base2 */
    border-bottom: 1px solid #93a1a1;
    display: flex;
    align-items: center;
    padding: 0 1rem;
    gap: 1rem;
    z-index: 501;
}

.editor-header h1 {
    margin: 0;
    font-size: 1rem;
    font-weight: 500;
    color: #657b83; /* solarized-light base00 */
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.editor-header .btn {
    padding: 0.4rem 0.8rem;
    font-size: 0.85rem;
}

.editor-status {
    font-size: 0.8rem;
    color: #93a1a1; /* solarized-light base1 */
    display: flex;
    align-items: center;
    gap: 0.5rem;
}

.editor-status.saving { color: #268bd2; } /* solarized blue */
.editor-status.saved { color: #859900; } /* solarized green */
.editor-status.error { color: #dc322f; } /* solarized red */
.editor-status.pending { color: #b58900; } /* solarized yellow */

.editor-status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: currentColor;
}

#monaco-editor {
    position: absolute;
    top: 48px;
    left: 0;
    right: 0;
    bottom: 0;
}

.editor-actions-old {
    margin-top: 1rem;
    display: flex;
    gap: 1rem;
    align-items: center;
}

.btn {
    padding: 0.5rem 1rem;
    border: 1px solid var(--base1);
    border-radius: 4px;
    background: var(--blue);
    color: var(--base3);
    cursor: pointer;
    font-size: 0.9rem;
    font-family: inherit;
    text-decoration: none;
    display: inline-block;
}

.btn:hover { background: var(--cyan); border-color: var(--cyan); }
.btn.secondary { background: var(--base2); color: var(--base00); border-color: var(--base1); }
.btn.secondary:hover { background: var(--base3); }

.save-status {
    font-size: 0.85rem;
    color: var(--muted);
}

.save-status.saving { color: var(--link); }
.save-status.saved { color: #4a4; }
.save-status.error { color: #c44; }

.login-form {
    max-width: 300px;
    margin: 4rem auto;
    padding: 2rem;
    background: var(--accent);
    border-radius: 8px;
}

.login-form h1 {
    margin-top: 0;
    margin-bottom: 1.5rem;
    text-align: center;
}

.login-form input {
    width: 100%;
    padding: 0.75rem;
    margin-bottom: 1rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--fg);
    font-size: 1rem;
}

.login-form button {
    width: 100%;
    padding: 0.75rem;
    background: var(--link);
    color: white;
    border: none;
    border-radius: 4px;
    font-size: 1rem;
    cursor: pointer;
}

.login-form button:hover { background: var(--link-hover); }

.message {
    padding: 0.75rem 1rem;
    border-radius: 4px;
    margin-bottom: 1rem;
}
.message.error { background: #fdf2f2; color: var(--red); border: 1px solid var(--red); }
.message.success { background: #f5f9f5; color: var(--green); border: 1px solid var(--green); }

.back-link {
    display: inline-block;
    margin-bottom: 1rem;
    font-size: 0.9rem;
}

/* Floating Action Button */
.fab {
    position: fixed;
    bottom: 2rem;
    right: 2rem;
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: var(--link);
    color: white;
    border: none;
    box-shadow: 0 4px 12px rgba(0,0,0,0.3);
    z-index: 1000;
    cursor: pointer;
    font-size: 1.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 0.2s, background 0.2s;
}
.fab:hover {
    background: var(--link-hover);
    transform: scale(1.1);
}

/* Smart Add Modal */
.smart-modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0,0,0,0.5);
    z-index: 1001;
    display: none;
    align-items: center;
    justify-content: center;
}
.smart-modal-overlay.active {
    display: flex;
}

.smart-modal {
    background: var(--bg);
    border-radius: 8px;
    width: 90%;
    max-width: 600px;
    max-height: 90vh;
    overflow-y: auto;
    box-shadow: 0 8px 32px rgba(0,0,0,0.3);
}

.smart-modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 1.5rem;
    border-bottom: 1px solid var(--border);
}
.smart-modal-header h2 {
    margin: 0;
    font-size: 1.25rem;
}
.smart-modal-close {
    background: none;
    border: none;
    font-size: 1.5rem;
    cursor: pointer;
    color: var(--muted);
    padding: 0;
    line-height: 1;
}
.smart-modal-close:hover {
    color: var(--fg);
}

.smart-modal-body {
    padding: 1.5rem;
}

.smart-input-group {
    margin-bottom: 1rem;
}
.smart-input-group label {
    display: block;
    margin-bottom: 0.5rem;
    font-weight: 600;
    font-size: 0.9rem;
}
.smart-input-group input {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--fg);
    font-size: 1rem;
}
.smart-input-group small {
    display: block;
    margin-top: 0.25rem;
    font-size: 0.8rem;
    color: var(--muted);
}

.smart-loading {
    display: none;
    align-items: center;
    gap: 0.5rem;
    padding: 1rem;
    color: var(--muted);
}
.smart-loading.active {
    display: flex;
}
.smart-spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--border);
    border-top-color: var(--link);
    border-radius: 50%;
    animation: spin 1s linear infinite;
}
@keyframes spin {
    to { transform: rotate(360deg); }
}

.smart-result {
    display: none;
    padding: 1rem;
    background: var(--accent);
    border-radius: 4px;
    margin-top: 1rem;
}
.smart-result.active {
    display: block;
}
.smart-result.match {
    background: var(--paper-bg);
}
.smart-result.error {
    background: #422;
    color: #faa;
}

.smart-result h3 {
    margin: 0 0 0.5rem 0;
    font-size: 1rem;
}
.smart-result-meta {
    font-size: 0.85rem;
    color: var(--muted);
    margin-bottom: 1rem;
}
.smart-result-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
}

.smart-form {
    display: none;
    margin-top: 1rem;
}
.smart-form.active {
    display: block;
}
.smart-form-row {
    display: flex;
    gap: 1rem;
    margin-bottom: 1rem;
}
.smart-form-row .smart-input-group {
    flex: 1;
    margin-bottom: 0;
}
.smart-form textarea {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--fg);
    font-family: monospace;
    font-size: 0.85rem;
    min-height: 100px;
    resize: vertical;
}
"#;

fn nav_bar(search_query: Option<&str>, logged_in: bool) -> String {
    let query_val = search_query.unwrap_or("");
    let auth_link = if logged_in {
        r#"<a href="/logout">Logout</a>"#
    } else if is_auth_enabled() {
        r#"<a href="/login">Login</a>"#
    } else {
        ""
    };

    format!(
        r#"<nav class="nav-bar">
            <a href="/">All</a>
            <a href="/papers">Papers</a>
            <a href="/time">Time</a>
            <a href="/graph">Graph</a>
            <a href="/bibliography.bib">Bib</a>
            <span class="spacer"></span>
            <form class="search-box" action="/search" method="get">
                <input type="text" name="q" placeholder="Search..." value="{}">
                <button type="submit">Go</button>
            </form>
            {}
        </nav>"#,
        html_escape(query_val),
        auth_link
    )
}

fn smart_add_html() -> &'static str {
    r##"
    <!-- Smart Add FAB -->
    <button class="fab" onclick="openSmartAdd()" title="Smart Add">+</button>

    <!-- Smart Add Modal -->
    <div class="smart-modal-overlay" id="smart-modal-overlay" onclick="if(event.target===this)closeSmartAdd()">
        <div class="smart-modal">
            <div class="smart-modal-header">
                <h2>Smart Add</h2>
                <button class="smart-modal-close" onclick="closeSmartAdd()">&times;</button>
            </div>
            <div class="smart-modal-body">
                <div class="smart-input-group">
                    <label for="smart-input">Paste URL, arXiv ID, DOI, or paper title</label>
                    <input type="text" id="smart-input" placeholder="https://arxiv.org/abs/... or 10.1000/... or paper title"
                           onkeydown="if(event.key==='Enter')performSmartLookup()">
                    <small>Press Enter or wait to auto-detect</small>
                </div>

                <div class="smart-loading" id="smart-loading">
                    <div class="smart-spinner"></div>
                    <span>Looking up...</span>
                </div>

                <div class="smart-result" id="smart-result"></div>

                <div class="smart-form" id="smart-form">
                    <h3>Create Paper Note</h3>
                    <div class="smart-form-row">
                        <div class="smart-input-group">
                            <label for="smart-title">Title</label>
                            <input type="text" id="smart-title">
                        </div>
                    </div>
                    <div class="smart-form-row">
                        <div class="smart-input-group">
                            <label for="smart-filename">Filename</label>
                            <input type="text" id="smart-filename" placeholder="paper-title.md">
                        </div>
                        <div class="smart-input-group">
                            <label for="smart-bibkey">Bib Key</label>
                            <input type="text" id="smart-bibkey" placeholder="author2024keyword">
                        </div>
                    </div>
                    <div class="smart-form-row">
                        <div class="smart-input-group">
                            <label for="smart-authors">Authors</label>
                            <input type="text" id="smart-authors" placeholder="Author One and Author Two">
                        </div>
                    </div>
                    <div class="smart-form-row">
                        <div class="smart-input-group">
                            <label for="smart-year">Year</label>
                            <input type="number" id="smart-year" placeholder="2024">
                        </div>
                        <div class="smart-input-group">
                            <label for="smart-venue">Venue</label>
                            <input type="text" id="smart-venue" placeholder="Conference/Journal">
                        </div>
                    </div>
                    <div class="smart-input-group">
                        <label for="smart-bibtex">BibTeX</label>
                        <textarea id="smart-bibtex" placeholder="@article{...}"></textarea>
                    </div>
                    <div class="smart-result-actions">
                        <button class="btn" onclick="createFromSmartAdd()">Create Note</button>
                        <button class="btn secondary" onclick="closeSmartAdd()">Cancel</button>
                    </div>
                </div>
            </div>
        </div>
    </div>

    <script>
    let smartDebounceTimer = null;

    function openSmartAdd() {
        document.getElementById('smart-modal-overlay').classList.add('active');
        document.getElementById('smart-input').focus();
        document.getElementById('smart-input').value = '';
        document.getElementById('smart-result').classList.remove('active');
        document.getElementById('smart-form').classList.remove('active');
    }

    function closeSmartAdd() {
        document.getElementById('smart-modal-overlay').classList.remove('active');
        document.getElementById('smart-loading').classList.remove('active');
    }

    document.getElementById('smart-input').addEventListener('input', function() {
        clearTimeout(smartDebounceTimer);
        smartDebounceTimer = setTimeout(performSmartLookup, 800);
    });

    // Auto-generate filename from title when typing manually
    document.getElementById('smart-title').addEventListener('input', function() {
        const title = this.value;
        const slug = title.toLowerCase()
            .replace(/[^a-z0-9\s-]/g, '')
            .replace(/\s+/g, '-')
            .replace(/-+/g, '-')
            .substring(0, 50);
        if (slug) {
            document.getElementById('smart-filename').value = slug + '.md';
            // Also generate a basic bib_key if year is set
            const year = document.getElementById('smart-year').value;
            const authors = document.getElementById('smart-authors').value;
            const firstWord = title.split(/\s+/).find(w => w.length > 3 && !['the','and','for','with'].includes(w.toLowerCase())) || 'paper';
            let bibkey = firstWord.toLowerCase().replace(/[^a-z]/g, '');
            if (year) bibkey = year + bibkey;
            if (authors) {
                const lastName = authors.split(/[,\s]+/)[0].toLowerCase().replace(/[^a-z]/g, '');
                if (lastName) bibkey = lastName + bibkey.replace(/^\d+/, year || '');
            }
            document.getElementById('smart-bibkey').value = bibkey;
        }
    });

    async function performSmartLookup() {
        const input = document.getElementById('smart-input').value.trim();
        if (!input) return;

        const loading = document.getElementById('smart-loading');
        const result = document.getElementById('smart-result');
        const form = document.getElementById('smart-form');

        loading.classList.add('active');
        result.classList.remove('active');
        form.classList.remove('active');

        try {
            const response = await fetch('/api/smart-add/lookup', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ input: input })
            });

            // Handle non-OK responses
            if (!response.ok) {
                loading.classList.remove('active');
                showManualEntryOption(result, 'Server error: ' + response.status);
                return;
            }

            // Try to parse JSON, with fallback for invalid responses
            let data;
            try {
                const text = await response.text();
                if (!text || text.trim() === '') {
                    loading.classList.remove('active');
                    showManualEntryOption(result, 'Empty response from server');
                    return;
                }
                data = JSON.parse(text);
            } catch (parseErr) {
                loading.classList.remove('active');
                showManualEntryOption(result, 'Invalid response from server');
                return;
            }

            loading.classList.remove('active');

            if (data.error) {
                result.innerHTML = '<h3>Error</h3><p>' + escapeHtml(data.error) + '</p>';
                result.className = 'smart-result active error';
                return;
            }

            if (data.local_match) {
                // Check if we have a new source to attach
                const canAttach = data.external_result && data.input_type !== 'text';
                const sourceInfo = data.input_type === 'arxiv' ? 'arXiv' :
                                   data.input_type === 'doi' ? 'DOI' : 'source';

                let attachBtn = '';
                if (canAttach) {
                    attachBtn = `<button class="btn" onclick="attachSourceToNote('${escapeHtml(data.local_match.key)}', '${escapeHtml(data.input_type)}', '${escapeHtml(getSourceIdentifier(data))}')">Attach ${sourceInfo}</button>`;
                }

                result.innerHTML = `
                    <h3>Existing Note Found</h3>
                    <p><strong>${escapeHtml(data.local_match.title)}</strong></p>
                    <p class="smart-result-meta">Match type: ${escapeHtml(data.local_match.match_type)}</p>
                    ${canAttach ? '<p class="smart-result-meta">A new source was found that can be attached to this note.</p>' : ''}
                    <div class="smart-result-actions">
                        <a href="/note/${escapeHtml(data.local_match.key)}" class="btn">View Note</a>
                        ${attachBtn}
                        <button class="btn secondary" onclick="showFormForManualEntry()">Create New</button>
                    </div>
                `;
                result.className = 'smart-result active match';

                // Store match info for potential attachment
                window.currentMatch = data.local_match;
                window.currentInputType = data.input_type;
                window.currentInput = document.getElementById('smart-input').value.trim();

                // Still populate form in case user wants to add anyway
                if (data.external_result) {
                    populateForm(data.external_result);
                }
                return;
            }

            if (data.external_result) {
                populateForm(data.external_result);

                // Store source identifiers for when we create the note
                window.detectedArxivId = null;
                window.detectedDoi = null;
                if (data.input_type === 'arxiv') {
                    window.detectedArxivId = getSourceIdentifier(data);
                } else if (data.input_type === 'doi') {
                    window.detectedDoi = getSourceIdentifier(data);
                }

                result.innerHTML = `
                    <h3>Found: ${escapeHtml(data.external_result.title)}</h3>
                    <p class="smart-result-meta">
                        Source: ${escapeHtml(data.external_result.source)}
                        ${data.external_result.authors ? ' | ' + escapeHtml(data.external_result.authors) : ''}
                        ${data.external_result.year ? ' (' + data.external_result.year + ')' : ''}
                    </p>
                `;
                result.className = 'smart-result active';
                form.classList.add('active');
                return;
            }

            // No external result, offer manual entry or regular note
            result.innerHTML = `
                <h3>No paper metadata found</h3>
                <p>External APIs didn't return results. You can enter details manually or create a regular note.</p>
                <div class="smart-result-actions">
                    <button class="btn" onclick="showFormForManualEntry()">Enter Paper Details</button>
                    <a href="/new" class="btn secondary">Create Regular Note</a>
                </div>
            `;
            result.className = 'smart-result active';

        } catch (e) {
            loading.classList.remove('active');
            showManualEntryOption(result, 'Network error: ' + e.message);
        }
    }

    function escapeHtml(str) {
        if (!str) return '';
        return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
    }

    function showManualEntryOption(result, message) {
        result.innerHTML = `
            <h3>Lookup unavailable</h3>
            <p>${escapeHtml(message)}</p>
            <p>You can still create a note manually:</p>
            <div class="smart-result-actions">
                <button class="btn" onclick="showFormForManualEntry()">Enter Details Manually</button>
                <a href="/new" class="btn secondary">Use Simple Form</a>
            </div>
        `;
        result.className = 'smart-result active';
    }

    function getSourceIdentifier(data) {
        const input = document.getElementById('smart-input').value.trim();
        if (data.input_type === 'arxiv') {
            // Extract arxiv ID from URL or raw input
            const match = input.match(/(\d{{4}}\.\d{{4,5}})/);
            return match ? match[1] : input;
        } else if (data.input_type === 'doi') {
            // Extract DOI
            const match = input.match(/(10\.\d{{4,}}\/[^\s]+)/);
            return match ? match[1] : input;
        }
        return input;
    }

    async function attachSourceToNote(noteKey, sourceType, identifier) {
        const result = document.getElementById('smart-result');
        result.innerHTML = '<p>Attaching source...</p>';

        try {
            const response = await fetch('/api/smart-add/attach', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    note_key: noteKey,
                    source_type: sourceType,
                    identifier: identifier
                })
            });

            const data = await response.json();
            if (data.success) {
                result.innerHTML = `
                    <h3>Source Attached!</h3>
                    <p>The ${sourceType} source has been added to the note.</p>
                    <div class="smart-result-actions">
                        <a href="/note/${escapeHtml(noteKey)}" class="btn">View Note</a>
                        <button class="btn secondary" onclick="closeSmartAdd()">Close</button>
                    </div>
                `;
                result.className = 'smart-result active';
            } else {
                result.innerHTML = `<h3>Error</h3><p>${escapeHtml(data.error || 'Unknown error')}</p>`;
                result.className = 'smart-result active error';
            }
        } catch (e) {
            result.innerHTML = `<h3>Error</h3><p>Failed to attach source: ${escapeHtml(e.message)}</p>`;
            result.className = 'smart-result active error';
        }
    }

    function showFormForManualEntry() {
        document.getElementById('smart-result').classList.remove('active');
        document.getElementById('smart-form').classList.add('active');
        document.getElementById('smart-title').focus();
    }

    function populateForm(ext) {
        document.getElementById('smart-title').value = ext.title || '';
        document.getElementById('smart-filename').value = ext.suggested_filename || '';
        document.getElementById('smart-bibkey').value = ext.bib_key || '';
        document.getElementById('smart-authors').value = ext.authors || '';
        document.getElementById('smart-year').value = ext.year || '';
        document.getElementById('smart-venue').value = ext.venue || '';
        document.getElementById('smart-bibtex').value = ext.bibtex || '';
    }

    async function createFromSmartAdd() {
        const data = {
            title: document.getElementById('smart-title').value,
            filename: document.getElementById('smart-filename').value,
            bib_key: document.getElementById('smart-bibkey').value,
            authors: document.getElementById('smart-authors').value || null,
            year: parseInt(document.getElementById('smart-year').value) || null,
            venue: document.getElementById('smart-venue').value || null,
            bibtex: document.getElementById('smart-bibtex').value || null,
            arxiv_id: window.detectedArxivId || null,
            doi: window.detectedDoi || null
        };

        if (!data.title || !data.filename || !data.bib_key) {
            alert('Title, filename, and bib_key are required');
            return;
        }

        try {
            const response = await fetch('/api/smart-add/create', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(data)
            });

            const result = await response.json();

            if (result.error) {
                alert('Error: ' + result.error);
                return;
            }

            if (result.key) {
                window.location.href = '/note/' + result.key + '?edit=true';
            }
        } catch (e) {
            alert('Failed to create note: ' + e.message);
        }
    }
    </script>
    "##
}

fn base_html(title: &str, content: &str, search_query: Option<&str>, logged_in: bool) -> String {
    let fab_html = if logged_in { smart_add_html() } else { "" };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <style>{STYLE}</style>
</head>
<body>
    {nav}
    <div class="container">
        {content}
    </div>
    {fab}
</body>
</html>"#,
        title = html_escape(title),
        nav = nav_bar(search_query, logged_in),
        fab = fab_html,
    )
}

// ============================================================================
// Route Handlers
// ============================================================================

async fn index(State(state): State<Arc<AppState>>, jar: CookieJar) -> Html<String> {
    let logged_in = is_logged_in(&jar);
    let notes = state.load_notes();

    let mut list_html = String::from("<ul class=\"note-list\">");

    for note in &notes {
        let is_paper = matches!(note.note_type, NoteType::Paper(_));
        let class = if is_paper { "note-item paper" } else { "note-item" };
        let type_badge = if is_paper {
            "<span class=\"type-badge\">paper</span>"
        } else {
            ""
        };

        list_html.push_str(&format!(
            r#"<li class="{class}">
                <span>
                    {type_badge}
                    <a href="/note/{key}" class="title">{title}</a>
                    <span class="key">[@{key}]</span>
                </span>
                <span class="meta">{modified}</span>
            </li>"#,
            class = class,
            key = note.key,
            title = html_escape(&note.title),
            modified = note.modified.format("%Y-%m-%d %H:%M"),
        ));
    }

    list_html.push_str("</ul>");

    Html(base_html("Notes", &list_html, None, logged_in))
}

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
}

async fn search(
    Query(query): Query<SearchQuery>,
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Html<String> {
    let logged_in = is_logged_in(&jar);
    let q = query.q.unwrap_or_default();

    if q.is_empty() {
        return Html(base_html("Search", "<p>Enter a search term.</p>", Some(&q), logged_in));
    }

    let notes = state.load_notes();
    let results = search_notes(&notes, &q);

    let mut html = format!(
        "<h1>Search: \"{}\"</h1><p>{} results</p><div class=\"search-results\">",
        html_escape(&q),
        results.len()
    );

    for result in results {
        html.push_str(&format!(
            r#"<div class="result-group">
                <a href="/note/{}">{}</a>
                <span class="key">[@{}]</span>"#,
            result.note.key,
            html_escape(&result.note.title),
            result.note.key
        ));

        for m in result.matches.iter().take(5) {
            let content = if m.line_content.len() > 100 {
                format!("{}...", &m.line_content[..100])
            } else {
                m.line_content.clone()
            };
            html.push_str(&format!(
                r#"<div class="match"><span class="line-num">{}:</span>{}</div>"#,
                m.line_number,
                html_escape(&content)
            ));
        }

        if result.matches.len() > 5 {
            html.push_str(&format!(
                "<div class=\"match\">... and {} more matches</div>",
                result.matches.len() - 5
            ));
        }

        html.push_str("</div>");
    }

    html.push_str("</div>");

    Html(base_html(&format!("Search: {}", q), &html, Some(&q), logged_in))
}

#[derive(Deserialize)]
struct NoteQuery {
    edit: Option<bool>,
}

async fn view_note(
    Path(key): Path<String>,
    Query(query): Query<NoteQuery>,
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Response {
    let logged_in = is_logged_in(&jar);
    let notes_map = state.notes_map();

    let note = match notes_map.get(&key) {
        Some(n) => n,
        None => return (StatusCode::NOT_FOUND, "Note not found").into_response(),
    };

    let edit_mode = query.edit.unwrap_or(false) && logged_in;

    if edit_mode {
        return render_editor(note, &notes_map, logged_in).into_response();
    }

    render_view(note, &notes_map, &state.notes_dir, logged_in).into_response()
}

fn render_view(note: &Note, notes_map: &HashMap<String, Note>, notes_dir: &PathBuf, logged_in: bool) -> Html<String> {
    let mut meta_html = String::from("<dl class=\"meta-block\">");
    meta_html.push_str(&format!("<dt>Key:</dt><dd><code>[@{}]</code></dd>", note.key));

    if let Some(date) = note.date {
        meta_html.push_str(&format!("<dt>Date:</dt><dd>{}</dd>", date.format("%Y-%m-%d")));
    }

    if let NoteType::Paper(ref paper) = note.note_type {
        meta_html.push_str(&format!("<dt>Bib Key:</dt><dd><code>{}</code></dd>", paper.bib_key));
        if let Some(ref authors) = paper.authors {
            meta_html.push_str(&format!("<dt>Authors:</dt><dd>{}</dd>", html_escape(authors)));
        }
        if let Some(year) = paper.year {
            meta_html.push_str(&format!("<dt>Year:</dt><dd>{}</dd>", year));
        }
        if let Some(ref venue) = paper.venue {
            meta_html.push_str(&format!("<dt>Venue:</dt><dd>{}</dd>", html_escape(venue)));
        }
        if !paper.sources.is_empty() {
            meta_html.push_str("<dt>Sources:</dt><dd>");
            for (i, source) in paper.sources.iter().enumerate() {
                if i > 0 { meta_html.push_str(" · "); }
                let link = match source.source_type.as_str() {
                    "arxiv" => format!("<a href=\"https://arxiv.org/abs/{}\" target=\"_blank\">arXiv:{}</a>",
                        html_escape(&source.identifier), html_escape(&source.identifier)),
                    "doi" => format!("<a href=\"https://doi.org/{}\" target=\"_blank\">DOI:{}</a>",
                        html_escape(&source.identifier), html_escape(&source.identifier)),
                    _ => format!("<a href=\"{}\" target=\"_blank\">{}</a>",
                        html_escape(&source.identifier), html_escape(&source.identifier)),
                };
                meta_html.push_str(&link);
            }
            meta_html.push_str("</dd>");
        }
    }

    if let Some(ref parent_key) = note.parent_key {
        if let Some(parent) = notes_map.get(parent_key) {
            meta_html.push_str(&format!(
                "<dt>Parent:</dt><dd><a href=\"/note/{}\">{}</a></dd>",
                parent_key,
                html_escape(&parent.title)
            ));
        }
    }

    meta_html.push_str("</dl>");

    let content_with_links = process_crosslinks(&note.raw_content, notes_map);
    let rendered_content = render_markdown(&content_with_links);

    let mut time_html = String::new();
    if !note.time_entries.is_empty() {
        time_html.push_str("<h2>Time Log</h2><table class=\"time-table\">");
        time_html.push_str("<tr><th>Date</th><th>Minutes</th><th>Category</th><th>Description</th></tr>");

        for entry in &note.time_entries {
            time_html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                entry.date.format("%Y-%m-%d"),
                entry.minutes,
                entry.category,
                entry.description.as_deref().unwrap_or("-")
            ));
        }
        time_html.push_str("</table>");
    }

    let sub_notes: Vec<_> = notes_map
        .values()
        .filter(|n| n.parent_key.as_ref() == Some(&note.key))
        .collect();

    let mut sub_notes_html = String::new();
    if !sub_notes.is_empty() {
        sub_notes_html.push_str("<div class=\"sub-notes\"><h3>Sub-notes</h3><ul>");
        for sub in sub_notes {
            sub_notes_html.push_str(&format!(
                "<li><a href=\"/note/{}\">{}</a></li>",
                sub.key,
                html_escape(&sub.title)
            ));
        }
        sub_notes_html.push_str("</ul></div>");
    }

    let history = get_git_history(&note.path, notes_dir);
    let mut history_html = String::new();
    if !history.is_empty() {
        history_html.push_str("<h2>Edit History</h2><div class=\"history-list\">");
        for commit in history.iter().take(10) {
            history_html.push_str(&format!(
                "<div class=\"history-item\">
                    <span class=\"history-hash\">{}</span>
                    <span>{}</span>
                    <a href=\"/note/{}/history/{}\">view</a>
                    <br><small>{} &mdash; {}</small>
                </div>",
                &commit.hash[..7],
                html_escape(&commit.message),
                note.key,
                &commit.hash[..7],
                commit.date.format("%Y-%m-%d %H:%M"),
                html_escape(&commit.author)
            ));
        }
        history_html.push_str("</div>");
    }

    let mode_toggle = if logged_in {
        format!(
            r#"<div class="mode-toggle">
                <button class="active">View</button>
                <button onclick="window.location.href='/note/{}?edit=true'">Edit</button>
            </div>"#,
            note.key
        )
    } else {
        String::new()
    };

    let full_html = format!(
        r#"<div class="note-header">
            <h1>{}</h1>
            {}
        </div>
        {}
        <div class="note-content">{}</div>
        {}{}{}
        "#,
        html_escape(&note.title),
        mode_toggle,
        meta_html,
        rendered_content,
        time_html,
        sub_notes_html,
        history_html
    );

    Html(base_html(&note.title, &full_html, None, logged_in))
}

fn render_editor(note: &Note, _notes_map: &HashMap<String, Note>, _logged_in: bool) -> Html<String> {
    let content_escaped = js_escape(&note.full_file_content);

    // Editor has its own full-screen layout, doesn't use base_html
    let html = format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Editing: {title}</title>
    <style>
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{ overflow: hidden; }}

        .editor-container {{
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            background: #fdf6e3;
        }}

        .editor-header {{
            position: absolute;
            top: 0;
            left: 0;
            right: 0;
            height: 48px;
            background: #eee8d5;
            border-bottom: 1px solid #93a1a1;
            display: flex;
            align-items: center;
            padding: 0 1rem;
            gap: 1rem;
            z-index: 10;
        }}

        .editor-header h1 {{
            margin: 0;
            font-size: 1rem;
            font-weight: 500;
            color: #657b83;
            flex: 1;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
        }}

        .btn {{
            padding: 0.4rem 0.8rem;
            border: 1px solid #93a1a1;
            border-radius: 4px;
            background: #fdf6e3;
            color: #657b83;
            cursor: pointer;
            font-size: 0.85rem;
            font-family: inherit;
            text-decoration: none;
            display: inline-block;
        }}
        .btn:hover {{ background: #eee8d5; }}
        .btn.primary {{
            background: #268bd2;
            color: #fdf6e3;
            border-color: #268bd2;
        }}
        .btn.primary:hover {{ background: #1a6fa3; }}

        .editor-status {{
            font-size: 0.8rem;
            color: #93a1a1;
            display: flex;
            align-items: center;
            gap: 0.5rem;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            min-width: 120px;
        }}
        .editor-status.saving {{ color: #268bd2; }}
        .editor-status.saved {{ color: #859900; }}
        .editor-status.error {{ color: #dc322f; }}
        .editor-status.pending {{ color: #b58900; }}

        .emacs-badge {{
            font-size: 0.65rem;
            font-weight: 600;
            padding: 0.2rem 0.4rem;
            background: #6c71c4; /* solarized violet */
            color: #fdf6e3;
            border-radius: 3px;
            font-family: monospace;
            letter-spacing: 0.05em;
        }}

        .editor-status-dot {{
            width: 8px;
            height: 8px;
            border-radius: 50%;
            background: currentColor;
            flex-shrink: 0;
        }}

        #monaco-editor {{
            position: absolute;
            top: 48px;
            left: 0;
            right: 0;
            bottom: 0;
        }}
    </style>
</head>
<body>
    <div class="editor-container">
        <div class="editor-header">
            <h1>{title}</h1>
            <span class="emacs-badge" id="emacs-badge" style="display:none;">EMACS</span>
            <div class="editor-status" id="editor-status">
                <span class="editor-status-dot"></span>
                <span id="status-text">Ready</span>
            </div>
            <button class="btn primary" onclick="saveNote(false)">Save</button>
            <a href="/note/{key}" class="btn">Done</a>
        </div>
        <div id="monaco-editor"></div>
    </div>

    <script src="https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.45.0/min/vs/loader.min.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/monaco-emacs@0.3.0/dist/monaco-emacs.min.js"></script>
    <script>
        let editor;
        let emacsMode;
        let lastSavedContent = `{content}`;
        let autoSaveTimer = null;
        let hasUnsavedChanges = false;
        const noteKey = "{key}";
        const AUTO_SAVE_DELAY = 90000; // 90 seconds

        require.config({{ paths: {{ vs: 'https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.45.0/min/vs' }} }});

        require(['vs/editor/editor.main'], function() {{
            // Define solarized-light theme
            monaco.editor.defineTheme('solarized-light', {{
                base: 'vs',
                inherit: true,
                rules: [
                    {{ token: '', foreground: '657b83', background: 'fdf6e3' }},
                    {{ token: 'comment', foreground: '93a1a1', fontStyle: 'italic' }},
                    {{ token: 'keyword', foreground: '859900' }},
                    {{ token: 'string', foreground: '2aa198' }},
                    {{ token: 'number', foreground: 'd33682' }},
                    {{ token: 'type', foreground: 'b58900' }},
                    {{ token: 'function', foreground: '268bd2' }},
                    {{ token: 'variable', foreground: '268bd2' }},
                    {{ token: 'constant', foreground: 'cb4b16' }},
                    {{ token: 'markup.heading', foreground: 'cb4b16', fontStyle: 'bold' }},
                    {{ token: 'markup.bold', fontStyle: 'bold' }},
                    {{ token: 'markup.italic', fontStyle: 'italic' }},
                    {{ token: 'markup.underline', fontStyle: 'underline' }},
                ],
                colors: {{
                    'editor.background': '#fdf6e3',
                    'editor.foreground': '#657b83',
                    'editor.lineHighlightBackground': '#eee8d5',
                    'editor.selectionBackground': '#eee8d5',
                    'editorCursor.foreground': '#657b83',
                    'editorLineNumber.foreground': '#93a1a1',
                    'editorLineNumber.activeForeground': '#657b83',
                    'editorIndentGuide.background': '#eee8d5',
                    'editorWhitespace.foreground': '#eee8d5',
                }}
            }});

            editor = monaco.editor.create(document.getElementById('monaco-editor'), {{
                value: `{content}`,
                language: 'markdown',
                theme: 'solarized-light',
                fontSize: 15,
                lineNumbers: 'on',
                wordWrap: 'on',
                minimap: {{ enabled: false }},
                scrollBeyondLastLine: true,
                automaticLayout: true,
                tabSize: 2,
                insertSpaces: true,
                renderWhitespace: 'selection',
                lineHeight: 1.7,
                padding: {{ top: 16, bottom: 16 }},
                fontFamily: '"SF Mono", "Consolas", "Liberation Mono", monospace',
                cursorBlinking: 'smooth',
                cursorSmoothCaretAnimation: 'on',
                smoothScrolling: true,
                renderLineHighlight: 'line',
                occurrencesHighlight: 'off',
                folding: false,
            }});

            // Track changes for auto-save
            editor.onDidChangeModelContent(() => {{
                const currentContent = editor.getValue();
                if (currentContent !== lastSavedContent) {{
                    hasUnsavedChanges = true;
                    updateStatus('pending', 'Unsaved changes');
                    scheduleAutoSave();
                }}
            }});

            // Ctrl/Cmd+S to save (works alongside Emacs C-x C-s)
            editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, function() {{
                saveNote(false);
            }});

            // Enable Emacs keybindings
            if (typeof MonacoEmacs !== 'undefined') {{
                emacsMode = new MonacoEmacs.EmacsExtension(editor);
                emacsMode.onDidMarkChange(function(marked) {{
                    // Visual feedback for mark mode could go here
                }});
                emacsMode.start();

                // Add C-x C-s for save (Emacs style)
                emacsMode.addCommand('C-x C-s', function() {{
                    saveNote(false);
                }});

                // Add C-x C-c to exit editor
                emacsMode.addCommand('C-x C-c', function() {{
                    if (hasUnsavedChanges) {{
                        if (confirm('You have unsaved changes. Save before leaving?')) {{
                            saveNote(false);
                        }}
                    }}
                    window.location.href = '/note/{key}';
                }});

                // Show Emacs badge
                document.getElementById('emacs-badge').style.display = 'inline-block';
                console.log('Emacs mode enabled');
            }} else {{
                console.warn('MonacoEmacs not loaded, using default keybindings');
            }}

            // Focus editor
            editor.focus();
        }});

        function scheduleAutoSave() {{
            if (autoSaveTimer) clearTimeout(autoSaveTimer);
            autoSaveTimer = setTimeout(() => {{
                if (hasUnsavedChanges) {{
                    saveNote(true); // auto-save with git commit
                }}
            }}, AUTO_SAVE_DELAY);
        }}

        function updateStatus(state, text) {{
            const statusEl = document.getElementById('editor-status');
            const textEl = document.getElementById('status-text');
            statusEl.className = 'editor-status ' + state;
            textEl.textContent = text;
        }}

        async function saveNote(isAutoSave) {{
            if (!editor) return;

            const currentContent = editor.getValue();
            if (currentContent === lastSavedContent) {{
                updateStatus('saved', 'No changes');
                return;
            }}

            updateStatus('saving', isAutoSave ? 'Auto-saving...' : 'Saving...');

            try {{
                const response = await fetch('/api/note/' + noteKey, {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{
                        content: currentContent,
                        auto_commit: isAutoSave
                    }})
                }});

                if (response.ok) {{
                    lastSavedContent = currentContent;
                    hasUnsavedChanges = false;
                    const now = new Date();
                    const timeStr = now.toLocaleTimeString('en-US', {{ hour: 'numeric', minute: '2-digit' }});
                    updateStatus('saved', 'Saved at ' + timeStr);
                }} else {{
                    const err = await response.text();
                    updateStatus('error', 'Save failed');
                    console.error('Save error:', err);
                }}
            }} catch (e) {{
                updateStatus('error', 'Save failed');
                console.error('Save error:', e);
            }}
        }}

        // Warn before leaving with unsaved changes
        window.addEventListener('beforeunload', (e) => {{
            if (hasUnsavedChanges) {{
                e.preventDefault();
                e.returnValue = '';
            }}
        }});
    </script>
</body>
</html>"##,
        title = html_escape(&note.title),
        key = note.key,
        content = content_escaped,
    );

    Html(html)
}

#[derive(Deserialize)]
struct SaveNoteBody {
    content: String,
    #[serde(default)]
    auto_commit: bool,
}

async fn save_note(
    Path(key): Path<String>,
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    axum::Json(body): axum::Json<SaveNoteBody>,
) -> Response {
    if !is_logged_in(&jar) {
        return (StatusCode::UNAUTHORIZED, "Not logged in").into_response();
    }

    let notes_map = state.notes_map();

    let note = match notes_map.get(&key) {
        Some(n) => n,
        None => return (StatusCode::NOT_FOUND, "Note not found").into_response(),
    };

    let full_path = state.notes_dir.join(&note.path);
    let note_path = note.path.clone();

    if let Err(e) = fs::write(&full_path, &body.content) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save: {}", e)).into_response();
    }

    // Make git commit if auto_commit is true
    if body.auto_commit {
        let notes_dir = state.notes_dir.clone();
        tokio::task::spawn_blocking(move || {
            // Format: "automatic save from notes: Sat Jan 24, 3:35PM"
            let now = chrono::Local::now();
            let commit_msg = format!(
                "automatic save from notes: {}",
                now.format("%a %b %d, %-I:%M%p")
            );

            // Stage the file
            let _ = Command::new("git")
                .args(["add", &note_path.to_string_lossy()])
                .current_dir(&notes_dir)
                .output();

            // Commit
            let _ = Command::new("git")
                .args(["commit", "-m", &commit_msg])
                .current_dir(&notes_dir)
                .output();
        });
    }

    (StatusCode::OK, "Saved").into_response()
}

async fn view_note_history(
    Path((key, commit)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Response {
    let logged_in = is_logged_in(&jar);
    let notes_map = state.notes_map();

    let note = match notes_map.get(&key) {
        Some(n) => n,
        None => return (StatusCode::NOT_FOUND, "Note not found").into_response(),
    };

    let content = match get_file_at_commit(&note.path, &commit, &state.notes_dir) {
        Some(c) => c,
        None => return (StatusCode::NOT_FOUND, "Commit not found").into_response(),
    };

    let (_, body) = parse_frontmatter(&content);
    let rendered = render_markdown(&body);

    let html = format!(
        "<a href=\"/note/{}\" class=\"back-link\">&larr; Back to current version</a>
        <h1>{} <small style=\"color: var(--muted); font-weight: normal;\">@ {}</small></h1>
        <div class=\"note-content\">{}</div>",
        key,
        html_escape(&note.title),
        &commit,
        rendered
    );

    Html(base_html(&format!("{} (history)", note.title), &html, None, logged_in)).into_response()
}

async fn login_page(jar: CookieJar) -> Response {
    if is_logged_in(&jar) {
        return Redirect::to("/").into_response();
    }

    let html = r#"
        <div class="login-form">
            <h1>Login</h1>
            <form method="POST" action="/login">
                <input type="password" name="password" placeholder="Password" autofocus required>
                <button type="submit">Login</button>
            </form>
        </div>
    "#;

    Html(base_html("Login", html, None, false)).into_response()
}

#[derive(Deserialize)]
struct LoginForm {
    password: String,
}

async fn login_submit(
    axum::Form(form): axum::Form<LoginForm>,
) -> Response {
    if !is_auth_enabled() {
        let html = r#"<div class="message error">Authentication not configured.</div>"#;
        return Html(base_html("Error", html, None, false)).into_response();
    }

    let password = env::var("NOTES_PASSWORD").unwrap_or_default();
    if form.password != password {
        let html = r#"
            <div class="login-form">
                <div class="message error">Invalid password.</div>
                <h1>Login</h1>
                <form method="POST" action="/login">
                    <input type="password" name="password" placeholder="Password" autofocus required>
                    <button type="submit">Login</button>
                </form>
            </div>
        "#;
        return Html(base_html("Login", html, None, false)).into_response();
    }

    let session_token = match create_session() {
        Some(t) => t,
        None => {
            let html = r#"<div class="message error">Failed to create session.</div>"#;
            return Html(base_html("Error", html, None, false)).into_response();
        }
    };

    let cookie = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
        SESSION_COOKIE,
        session_token,
        SESSION_TTL_HOURS * 3600
    );

    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, cookie.parse().unwrap());

    (headers, Redirect::to("/")).into_response()
}

async fn logout() -> Response {
    let cookie = format!("{}=; Path=/; HttpOnly; Max-Age=0", SESSION_COOKIE);

    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, cookie.parse().unwrap());

    (headers, Redirect::to("/")).into_response()
}

async fn new_note_page(jar: CookieJar) -> Response {
    if !is_logged_in(&jar) {
        return Redirect::to("/login").into_response();
    }

    let today = Utc::now().format("%Y-%m-%d").to_string();

    let html = format!(
        r##"
        <h1>New Note</h1>
        <form method="POST" action="/new" class="new-note-form">
            <div class="form-group">
                <label for="title">Title</label>
                <input type="text" id="title" name="title" required autofocus
                       placeholder="My New Note">
            </div>

            <div class="form-group">
                <label for="filename">Filename</label>
                <input type="text" id="filename" name="filename" required
                       placeholder="my-new-note.md" pattern="[a-z0-9\-/]+\.md">
                <small>Use lowercase, hyphens, optional subdirectory (e.g., <code>projects/my-idea.md</code>)</small>
            </div>

            <div class="form-group">
                <label for="note_type">Type</label>
                <select id="note_type" name="note_type">
                    <option value="note">Note</option>
                    <option value="paper">Paper</option>
                </select>
            </div>

            <div class="form-group">
                <label for="date">Date</label>
                <input type="date" id="date" name="date" value="{}">
            </div>

            <div class="form-group" id="paper-fields" style="display: none;">
                <label for="bib_key">Bib Key</label>
                <input type="text" id="bib_key" name="bib_key" placeholder="smith2024attention">

                <label for="authors">Authors</label>
                <input type="text" id="authors" name="authors" placeholder="Smith, John and Doe, Jane">

                <label for="year">Year</label>
                <input type="number" id="year" name="year" placeholder="2024">

                <label for="venue">Venue</label>
                <input type="text" id="venue" name="venue" placeholder="NeurIPS">
            </div>

            <div class="form-actions">
                <button type="submit" class="btn">Create Note</button>
                <a href="/" class="btn secondary">Cancel</a>
            </div>
        </form>

        <style>
            .new-note-form {{ max-width: 500px; }}
            .form-group {{ margin-bottom: 1rem; }}
            .form-group label {{ display: block; margin-bottom: 0.25rem; font-weight: 600; font-size: 0.9rem; }}
            .form-group input, .form-group select {{
                width: 100%;
                padding: 0.5rem 0.75rem;
                border: 1px solid var(--border);
                border-radius: 4px;
                background: var(--bg);
                color: var(--fg);
                font-size: 1rem;
                margin-bottom: 0.25rem;
            }}
            .form-group small {{ font-size: 0.8rem; color: var(--muted); }}
            .form-group small code {{ background: var(--accent); padding: 0.1rem 0.3rem; border-radius: 2px; }}
            .form-actions {{ display: flex; gap: 1rem; margin-top: 1.5rem; }}
            #paper-fields {{ padding: 1rem; background: var(--paper-bg); border-radius: 4px; margin-top: 0.5rem; }}
            #paper-fields label {{ margin-top: 0.75rem; }}
            #paper-fields label:first-child {{ margin-top: 0; }}
        </style>

        <script>
            const typeSelect = document.getElementById('note_type');
            const paperFields = document.getElementById('paper-fields');
            const titleInput = document.getElementById('title');
            const filenameInput = document.getElementById('filename');

            typeSelect.addEventListener('change', function() {{
                paperFields.style.display = this.value === 'paper' ? 'block' : 'none';
            }});

            // Auto-generate filename from title
            titleInput.addEventListener('input', function() {{
                const slug = this.value
                    .toLowerCase()
                    .replace(/[^a-z0-9\s-]/g, '')
                    .replace(/\s+/g, '-')
                    .replace(/-+/g, '-')
                    .trim();
                if (slug) {{
                    filenameInput.value = slug + '.md';
                }}
            }});
        </script>
        "##,
        today
    );

    Html(base_html("New Note", &html, None, true)).into_response()
}

#[derive(Deserialize)]
struct NewNoteForm {
    title: String,
    filename: String,
    note_type: String,
    date: Option<String>,
    bib_key: Option<String>,
    authors: Option<String>,
    year: Option<String>,
    venue: Option<String>,
}

async fn create_note(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    axum::Form(form): axum::Form<NewNoteForm>,
) -> Response {
    if !is_logged_in(&jar) {
        return Redirect::to("/login").into_response();
    }

    // Validate filename
    let filename = form.filename.trim();
    if filename.is_empty() || !filename.ends_with(".md") {
        let html = r#"<div class="message error">Invalid filename. Must end with .md</div>"#;
        return Html(base_html("Error", html, None, true)).into_response();
    }

    // Check for path traversal
    if filename.contains("..") {
        let html = r#"<div class="message error">Invalid filename.</div>"#;
        return Html(base_html("Error", html, None, true)).into_response();
    }

    let file_path = state.notes_dir.join(filename);

    // Check if file already exists
    if file_path.exists() {
        let html = format!(
            r#"<div class="message error">A note with filename '{}' already exists.</div>
            <a href="/new">Go back</a>"#,
            html_escape(filename)
        );
        return Html(base_html("Error", &html, None, true)).into_response();
    }

    // Create parent directories if needed
    if let Some(parent) = file_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            let html = format!(r#"<div class="message error">Failed to create directory: {}</div>"#, e);
            return Html(base_html("Error", &html, None, true)).into_response();
        }
    }

    // Build frontmatter
    let mut frontmatter = format!("---\ntitle: {}\n", form.title);

    if let Some(ref date) = form.date {
        if !date.is_empty() {
            frontmatter.push_str(&format!("date: {}\n", date));
        }
    }

    if form.note_type == "paper" {
        frontmatter.push_str("type: paper\n");
        if let Some(ref bib_key) = form.bib_key {
            if !bib_key.is_empty() {
                frontmatter.push_str(&format!("bib_key: {}\n", bib_key));
            }
        }
        if let Some(ref authors) = form.authors {
            if !authors.is_empty() {
                frontmatter.push_str(&format!("authors: {}\n", authors));
            }
        }
        if let Some(ref year) = form.year {
            if !year.is_empty() {
                frontmatter.push_str(&format!("year: {}\n", year));
            }
        }
        if let Some(ref venue) = form.venue {
            if !venue.is_empty() {
                frontmatter.push_str(&format!("venue: {}\n", venue));
            }
        }
    }

    frontmatter.push_str("---\n\n");

    // Write the file
    if let Err(e) = fs::write(&file_path, &frontmatter) {
        let html = format!(r#"<div class="message error">Failed to create note: {}</div>"#, e);
        return Html(base_html("Error", &html, None, true)).into_response();
    }

    // Get the key of the new note
    let relative_path = PathBuf::from(filename);
    let key = generate_key(&relative_path);

    // Redirect to edit the new note
    Redirect::to(&format!("/note/{}?edit=true", key)).into_response()
}

async fn papers(State(state): State<Arc<AppState>>, jar: CookieJar) -> Html<String> {
    let logged_in = is_logged_in(&jar);
    let notes = state.load_notes();
    let papers: Vec<_> = notes.iter().filter(|n| matches!(n.note_type, NoteType::Paper(_))).collect();

    let mut html = String::from("<h1>Papers</h1><ul class=\"note-list\">");

    for note in papers {
        if let NoteType::Paper(ref paper) = note.note_type {
            let authors = paper.authors.as_deref().unwrap_or("Unknown");
            let year = paper.year.map(|y| y.to_string()).unwrap_or_default();

            html.push_str(&format!(
                r#"<li class="note-item paper">
                    <span>
                        <a href="/note/{}" class="title">{}</a>
                        <br><small>{} {}</small>
                        <br><code class="key">{}</code>
                    </span>
                </li>"#,
                note.key,
                html_escape(&note.title),
                html_escape(authors),
                year,
                paper.bib_key
            ));
        }
    }

    html.push_str("</ul>");

    Html(base_html("Papers", &html, None, logged_in))
}

async fn time_tracking(State(state): State<Arc<AppState>>, jar: CookieJar) -> Html<String> {
    let logged_in = is_logged_in(&jar);
    let notes = state.load_notes();

    let mut totals: HashMap<TimeCategory, u32> = HashMap::new();
    let mut entries_by_date: HashMap<NaiveDate, Vec<(&Note, &TimeEntry)>> = HashMap::new();

    for note in &notes {
        for entry in &note.time_entries {
            *totals.entry(entry.category.clone()).or_insert(0) += entry.minutes;
            entries_by_date
                .entry(entry.date)
                .or_default()
                .push((note, entry));
        }
    }

    let total_minutes: u32 = totals.values().sum();

    let mut bar_html = String::from("<div class=\"time-bar\">");
    let mut legend_html = String::from("<div class=\"time-legend\">");

    if total_minutes > 0 {
        let categories = [
            (TimeCategory::Programming, "programming"),
            (TimeCategory::Teaching, "teaching"),
            (TimeCategory::Reading, "reading"),
            (TimeCategory::Writing, "writing"),
            (TimeCategory::Service, "service"),
        ];

        for (cat, class) in &categories {
            if let Some(&mins) = totals.get(cat) {
                let pct = (mins as f64 / total_minutes as f64) * 100.0;
                bar_html.push_str(&format!(
                    "<div class=\"time-segment cat-{}\" style=\"width: {:.1}%\" title=\"{}: {} mins\"></div>",
                    class, pct, class, mins
                ));
                legend_html.push_str(&format!(
                    "<span class=\"time-legend-item\"><span class=\"time-legend-color cat-{}\"></span>{}: {}h {}m</span>",
                    class, class, mins / 60, mins % 60
                ));
            }
        }
    }

    bar_html.push_str("</div>");
    legend_html.push_str("</div>");

    let mut dates: Vec<_> = entries_by_date.keys().collect();
    dates.sort_by(|a, b| b.cmp(a));

    let mut entries_html = String::from("<h2>Recent Entries</h2><table class=\"time-table\">");
    entries_html.push_str("<tr><th>Date</th><th>Note</th><th>Category</th><th>Minutes</th><th>Description</th></tr>");

    for date in dates.iter().take(20) {
        if let Some(entries) = entries_by_date.get(date) {
            for (note, entry) in entries {
                entries_html.push_str(&format!(
                    "<tr><td>{}</td><td><a href=\"/note/{}\">{}</a></td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    entry.date.format("%Y-%m-%d"),
                    note.key,
                    html_escape(&note.title),
                    entry.category,
                    entry.minutes,
                    entry.description.as_deref().unwrap_or("-")
                ));
            }
        }
    }
    entries_html.push_str("</table>");

    let html = format!(
        "<h1>Time Tracking</h1>
        <div class=\"time-summary\">
            <p>Total tracked: <strong>{}h {}m</strong></p>
            {}{}
        </div>
        {}",
        total_minutes / 60,
        total_minutes % 60,
        bar_html,
        legend_html,
        entries_html
    );

    Html(base_html("Time Tracking", &html, None, logged_in))
}

async fn bibliography(State(state): State<Arc<AppState>>) -> Response {
    let notes = state.load_notes();
    let bib = generate_bibliography(&notes);

    (
        [("content-type", "text/plain; charset=utf-8")],
        bib
    ).into_response()
}

// ============================================================================
// Graph Visualization
// ============================================================================

#[derive(Deserialize)]
struct GraphQueryParams {
    q: Option<String>,
}

async fn graph_page(
    Query(params): Query<GraphQueryParams>,
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Html<String> {
    let logged_in = is_logged_in(&jar);
    let query_str = params.q.as_deref().unwrap_or("");
    let query = GraphQuery::parse(query_str);
    let notes = state.load_notes();
    let graph = build_knowledge_graph(&notes, &query);

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
            </div>
        </div>

        <div class="graph-help">
            <strong>Query Language</strong>
            <div class="graph-help-grid">
                <span><code>from:KEY</code> — Center on node</span>
                <span><code>depth:N</code> — Expand N hops</span>
                <span><code>type:paper</code> — Filter by type</span>
                <span><code>type:note</code> — Only notes</span>
                <span><code>has:time</code> — With time tracking</span>
                <span><code>links:>N</code> — Min connections</span>
                <span><code>links:<N</code> — Max connections</span>
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
            const container = document.getElementById('graph-container');
            const svg = d3.select('#graph-svg');
            const width = container.clientWidth;
            const height = container.clientHeight;

            // Create tooltip
            const tooltip = d3.select('body').append('div')
                .attr('class', 'node-tooltip')
                .style('display', 'none');

            // Force simulation
            const simulation = d3.forceSimulation(graphData.nodes)
                .force('link', d3.forceLink(graphData.edges)
                    .id(d => d.id)
                    .distance(80))
                .force('charge', d3.forceManyBody().strength(-200))
                .force('center', d3.forceCenter(width / 2, height / 2))
                .force('collision', d3.forceCollide().radius(30));

            // Create links
            const link = svg.append('g')
                .selectAll('line')
                .data(graphData.edges)
                .join('line')
                .attr('class', 'link')
                .attr('stroke-width', d => Math.sqrt(d.weight) * 1.5);

            // Create node groups
            const node = svg.append('g')
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
            node.append('circle')
                .attr('r', d => {{
                    const base = 8;
                    const degree = d.in_degree + d.out_degree;
                    return base + Math.sqrt(degree) * 3;
                }});

            // Add labels (only for nodes with enough connections or when zoomed)
            node.append('text')
                .text(d => d.title.length > 15 ? d.title.substring(0, 15) + '...' : d.title)
                .attr('dy', d => -(12 + Math.sqrt(d.in_degree + d.out_degree) * 3));

            // Hover interactions
            node.on('mouseover', function(event, d) {{
                d3.select(this).classed('selected', true);

                // Highlight connected links
                link.classed('highlighted', l => l.source.id === d.id || l.target.id === d.id);

                // Show tooltip
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
                // Navigate to note on click
                window.location.href = '/note/' + d.id;
            }})
            .on('dblclick', function(event, d) {{
                // Center graph on this node
                window.location.href = '/graph?q=from:' + d.id + ' depth:2';
            }});

            // Update positions on simulation tick
            simulation.on('tick', () => {{
                link
                    .attr('x1', d => d.source.x)
                    .attr('y1', d => d.source.y)
                    .attr('x2', d => d.target.x)
                    .attr('y2', d => d.target.y);

                node.attr('transform', d => {{
                    // Keep nodes within bounds
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
                    svg.selectAll('g').attr('transform', event.transform);
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
    );

    Html(base_html("Knowledge Graph", &html, None, logged_in))
}

async fn graph_api(
    Query(params): Query<GraphQueryParams>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let query_str = params.q.as_deref().unwrap_or("");
    let query = GraphQuery::parse(query_str);
    let notes = state.load_notes();
    let graph = build_knowledge_graph(&notes, &query);

    (
        [("content-type", "application/json")],
        serde_json::to_string(&graph).unwrap_or("{}".to_string())
    ).into_response()
}

// ============================================================================
// Smart Add Route Handlers
// ============================================================================

async fn smart_add_lookup(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    axum::Json(body): axum::Json<SmartAddRequest>,
) -> Response {
    // Always return JSON responses for consistency
    if !is_logged_in(&jar) {
        let result = SmartAddResult {
            input_type: "error".to_string(),
            local_match: None,
            external_result: None,
            error: Some("Not logged in".to_string()),
        };
        return axum::Json(result).into_response();
    }

    let input = body.input.trim().to_string();
    if input.is_empty() {
        let result = SmartAddResult {
            input_type: "empty".to_string(),
            local_match: None,
            external_result: None,
            error: Some("Input is empty".to_string()),
        };
        return axum::Json(result).into_response();
    }

    let input_type = detect_input_type(&input);
    let notes = state.load_notes();

    // Check for local match first
    let local_match = search_local_for_match(&notes, &input, &input_type);

    // Query external APIs based on input type (with error handling)
    let external_result = match &input_type {
        InputType::ArxivUrl { arxiv_id } => {
            let arxiv_id = arxiv_id.clone();
            // Try arXiv API, fallback to Claude, then None
            match query_arxiv_api(&arxiv_id).await {
                Some(r) => Some(r),
                None => {
                    // Claude fallback is optional - don't fail if it's not available
                    query_claude_for_url(&format!("https://arxiv.org/abs/{}", arxiv_id)).await
                }
            }
        }
        InputType::DoiUrl { doi } => {
            let doi = doi.clone();
            // Try CrossRef API, fallback to Claude
            match query_crossref_api(&doi).await {
                Some(r) => Some(r),
                None => {
                    query_claude_for_url(&format!("https://doi.org/{}", doi)).await
                }
            }
        }
        InputType::GenericUrl { url } => {
            // Try to fetch and extract metadata from the page
            match fetch_and_extract_metadata(url).await {
                Some(r) => Some(r),
                None => query_claude_for_url(url).await,
            }
        }
        InputType::PlainText { text } => {
            // Try CrossRef title search
            query_crossref_by_title(text).await
        }
    };

    let input_type_str = match &input_type {
        InputType::ArxivUrl { .. } => "arxiv",
        InputType::DoiUrl { .. } => "doi",
        InputType::GenericUrl { .. } => "url",
        InputType::PlainText { .. } => "text",
    };

    let result = SmartAddResult {
        input_type: input_type_str.to_string(),
        local_match,
        external_result,
        error: None,
    };

    axum::Json(result).into_response()
}

#[derive(Serialize)]
struct SmartAddCreateResponse {
    key: Option<String>,
    error: Option<String>,
}

async fn smart_add_create(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    axum::Json(body): axum::Json<SmartAddCreateRequest>,
) -> Response {
    if !is_logged_in(&jar) {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some("Not logged in".to_string()),
        }).into_response();
    }

    // Validate required fields
    if body.title.trim().is_empty() || body.filename.trim().is_empty() || body.bib_key.trim().is_empty() {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some("Title, filename, and bib_key are required".to_string()),
        }).into_response();
    }

    let filename = body.filename.trim();

    // Validate filename
    if !filename.ends_with(".md") {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some("Filename must end with .md".to_string()),
        }).into_response();
    }

    // Check for path traversal
    if filename.contains("..") {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some("Invalid filename".to_string()),
        }).into_response();
    }

    let file_path = state.notes_dir.join(filename);

    // Check if file exists
    if file_path.exists() {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some(format!("A note with filename '{}' already exists", filename)),
        }).into_response();
    }

    // Create parent directories if needed
    if let Some(parent) = file_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return axum::Json(SmartAddCreateResponse {
                key: None,
                error: Some(format!("Failed to create directory: {}", e)),
            }).into_response();
        }
    }

    // Build frontmatter
    let today = Utc::now().format("%Y-%m-%d");
    let mut frontmatter = format!(
        "---\ntitle: {}\ndate: {}\ntype: paper\nbib_key: {}\n",
        body.title, today, body.bib_key
    );

    if let Some(ref authors) = body.authors {
        if !authors.is_empty() {
            frontmatter.push_str(&format!("authors: {}\n", authors));
        }
    }
    if let Some(year) = body.year {
        frontmatter.push_str(&format!("year: {}\n", year));
    }
    if let Some(ref venue) = body.venue {
        if !venue.is_empty() {
            frontmatter.push_str(&format!("venue: {}\n", venue));
        }
    }
    if let Some(ref bibtex) = body.bibtex {
        if !bibtex.is_empty() {
            frontmatter.push_str("bibtex: |\n");
            for line in bibtex.lines() {
                frontmatter.push_str(&format!("  {}\n", line));
            }
        }
    }
    if let Some(ref arxiv_id) = body.arxiv_id {
        if !arxiv_id.is_empty() {
            frontmatter.push_str(&format!("arxiv: {}\n", arxiv_id));
        }
    }
    if let Some(ref doi) = body.doi {
        if !doi.is_empty() {
            frontmatter.push_str(&format!("doi: {}\n", doi));
        }
    }

    frontmatter.push_str("---\n\n## Summary\n\n## Key Contributions\n\n## Notes\n\n");

    // Write the file
    if let Err(e) = fs::write(&file_path, &frontmatter) {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some(format!("Failed to create note: {}", e)),
        }).into_response();
    }

    // Generate key for the new note
    let relative_path = PathBuf::from(filename);
    let key = generate_key(&relative_path);

    axum::Json(SmartAddCreateResponse {
        key: Some(key),
        error: None,
    }).into_response()
}

#[derive(Debug, Clone, Deserialize)]
pub struct AttachSourceRequest {
    pub note_key: String,
    pub source_type: String,
    pub identifier: String,
}

#[derive(Serialize)]
struct AttachSourceResponse {
    success: bool,
    error: Option<String>,
}

async fn smart_add_attach(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    axum::Json(body): axum::Json<AttachSourceRequest>,
) -> Response {
    if !is_logged_in(&jar) {
        return axum::Json(AttachSourceResponse {
            success: false,
            error: Some("Not logged in".to_string()),
        }).into_response();
    }

    let notes_map = state.notes_map();
    let note = match notes_map.get(&body.note_key) {
        Some(n) => n,
        None => return axum::Json(AttachSourceResponse {
            success: false,
            error: Some("Note not found".to_string()),
        }).into_response(),
    };

    // Read the current file content
    let full_path = state.notes_dir.join(&note.path);
    let content = match fs::read_to_string(&full_path) {
        Ok(c) => c,
        Err(e) => return axum::Json(AttachSourceResponse {
            success: false,
            error: Some(format!("Failed to read note: {}", e)),
        }).into_response(),
    };

    // Add the new source to frontmatter
    let source_line = match body.source_type.as_str() {
        "arxiv" => format!("arxiv: {}", body.identifier),
        "doi" => format!("doi: {}", body.identifier),
        _ => format!("url: {}", body.identifier),
    };

    // Find the end of frontmatter and insert before ---
    let new_content = if let Some(second_dash) = content.find("---").and_then(|first| {
        content[first + 3..].find("---").map(|second| first + 3 + second)
    }) {
        // Insert the source line before the closing ---
        let mut new = content[..second_dash].to_string();
        // Make sure there's a newline
        if !new.ends_with('\n') {
            new.push('\n');
        }
        new.push_str(&source_line);
        new.push('\n');
        new.push_str(&content[second_dash..]);
        new
    } else {
        return axum::Json(AttachSourceResponse {
            success: false,
            error: Some("Could not find frontmatter".to_string()),
        }).into_response();
    };

    // Write the updated content
    if let Err(e) = fs::write(&full_path, &new_content) {
        return axum::Json(AttachSourceResponse {
            success: false,
            error: Some(format!("Failed to write note: {}", e)),
        }).into_response();
    }

    axum::Json(AttachSourceResponse {
        success: true,
        error: None,
    }).into_response()
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState::new());

    let app = Router::new()
        .route("/", get(index))
        .route("/search", get(search))
        .route("/new", get(new_note_page).post(create_note))
        .route("/login", get(login_page).post(login_submit))
        .route("/logout", get(logout))
        .route("/note/{key}", get(view_note))
        .route("/api/note/{key}", axum::routing::post(save_note))
        .route("/note/{key}/history/{commit}", get(view_note_history))
        .route("/papers", get(papers))
        .route("/time", get(time_tracking))
        .route("/graph", get(graph_page))
        .route("/api/graph", get(graph_api))
        .route("/api/smart-add/lookup", axum::routing::post(smart_add_lookup))
        .route("/api/smart-add/create", axum::routing::post(smart_add_create))
        .route("/api/smart-add/attach", axum::routing::post(smart_add_attach))
        .route("/bibliography.bib", get(bibliography))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("Failed to bind to port 3000");

    println!("Notes server running at http://127.0.0.1:3000");
    println!("Notes directory: {}", NOTES_DIR);

    if is_auth_enabled() {
        println!("Authentication: ENABLED (NOTES_PASSWORD set)");
    } else {
        println!("Authentication: DISABLED (set NOTES_PASSWORD env var to enable editing)");
    }

    axum::serve(listener, app).await.expect("Server error");
}
