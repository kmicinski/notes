//! Data models for the notes application.
//!
//! This module contains all the core data structures used throughout the application,
//! including notes, papers, time tracking, graph visualization, and smart add features.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ============================================================================
// Core Note Types
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
    pub pdf: Option<String>,
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
    pub source_type: String, // "arxiv", "doi", "url"
    pub identifier: String,  // The arxiv ID, DOI, or URL
}

// ============================================================================
// Effective Metadata (derived from BibTeX)
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct EffectivePaperMeta {
    pub bib_key: String,
    pub title: Option<String>,
    pub authors: Option<String>,
    pub year: Option<i32>,
    pub venue: Option<String>,
}

impl PaperMeta {
    /// Returns effective metadata by preferring BibTeX-derived values over explicit fields.
    /// This makes BibTeX the single source of truth when present.
    pub fn effective_metadata(&self, note_title: &str) -> EffectivePaperMeta {
        use crate::notes::parse_bibtex;

        let mut effective = EffectivePaperMeta {
            bib_key: self.bib_key.clone(),
            title: Some(note_title.to_string()),
            authors: self.authors.clone(),
            year: self.year,
            venue: self.venue.clone(),
        };

        // If we have bibtex, parse it and use those values as primary source
        if let Some(ref bibtex) = self.bibtex {
            if let Some(parsed) = parse_bibtex(bibtex) {
                // Use parsed cite_key if different from stored bib_key
                if !parsed.cite_key.is_empty() {
                    effective.bib_key = parsed.cite_key;
                }
                // Prefer BibTeX title, fall back to note title
                if parsed.title.is_some() {
                    effective.title = parsed.title;
                }
                // Prefer BibTeX author
                if parsed.author.is_some() {
                    effective.authors = parsed.author;
                }
                // Prefer BibTeX year
                if parsed.year.is_some() {
                    effective.year = parsed.year;
                }
                // Prefer BibTeX venue
                if parsed.venue.is_some() {
                    effective.venue = parsed.venue;
                }
            }
        }

        effective
    }
}

// ============================================================================
// Time Tracking
// ============================================================================

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

// ============================================================================
// Git and Search
// ============================================================================

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
    pub node_type: String,           // "note" or "paper"
    pub date: Option<String>,
    pub time_total: u32,             // Total minutes tracked
    pub primary_category: Option<String>,
    pub in_degree: usize,            // Incoming links
    pub out_degree: usize,           // Outgoing links
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub weight: usize, // Number of times referenced
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
    pub orphan_count: usize,     // Nodes with no connections
    pub hub_threshold: usize,    // Degree considered "hub"
    pub hub_count: usize,
    pub avg_degree: f64,
    pub max_degree: usize,
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
                if gq.depth == 99 {
                    gq.depth = 2;
                } // Default depth when centered
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
            parts.push(format!(
                "path {}->{}",
                self.path_start.as_ref().unwrap(),
                self.path_end.as_ref().unwrap()
            ));
        }

        if parts.is_empty() {
            "Full graph".to_string()
        } else {
            parts.join(", ")
        }
    }
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
    pub match_type: String, // "exact", "title", "arxiv_id"
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
    pub source: String, // "arxiv", "crossref", "claude"
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

#[derive(Debug, Clone, Deserialize)]
pub struct AttachSourceRequest {
    pub note_key: String,
    pub source_type: String,
    pub identifier: String,
}
