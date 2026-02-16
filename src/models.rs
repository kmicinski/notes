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
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NoteType {
    Note,
    Paper(PaperMeta),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaperMeta {
    /// One or more BibTeX entries. The bibtex is the sole source of truth for
    /// citation metadata (bib_key, authors, year, venue, title).
    pub bibtex_entries: Vec<String>,
    /// When multiple bibtex entries exist, this specifies which cite key is canonical.
    /// If None, the first entry is used.
    pub canonical_key: Option<String>,
    /// External sources (arxiv, doi, url) for the paper
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
    /// Returns effective metadata from the canonical BibTeX entry.
    /// BibTeX is the single source of truth for all citation metadata.
    pub fn effective_metadata(&self, note_title: &str) -> EffectivePaperMeta {
        use crate::notes::parse_bibtex;

        let mut effective = EffectivePaperMeta {
            bib_key: String::new(),
            title: Some(note_title.to_string()),
            authors: None,
            year: None,
            venue: None,
        };

        if self.bibtex_entries.is_empty() {
            return effective;
        }

        // Find the canonical entry: either by canonical_key or use first entry
        let canonical_bibtex = if let Some(ref key) = self.canonical_key {
            // Find entry matching the canonical key
            self.bibtex_entries.iter().find(|entry| {
                if let Some(parsed) = parse_bibtex(entry) {
                    parsed.cite_key == *key
                } else {
                    false
                }
            }).unwrap_or(&self.bibtex_entries[0])
        } else {
            &self.bibtex_entries[0]
        };

        if let Some(parsed) = parse_bibtex(canonical_bibtex) {
            effective.bib_key = parsed.cite_key;
            if parsed.title.is_some() {
                effective.title = parsed.title;
            }
            effective.authors = parsed.author;
            effective.year = parsed.year;
            effective.venue = parsed.venue;
        }

        effective
    }

    /// Returns the canonical BibTeX entry as a string
    pub fn canonical_bibtex(&self) -> Option<&String> {
        use crate::notes::parse_bibtex;

        if self.bibtex_entries.is_empty() {
            return None;
        }

        if let Some(ref key) = self.canonical_key {
            self.bibtex_entries.iter().find(|entry| {
                if let Some(parsed) = parse_bibtex(entry) {
                    parsed.cite_key == *key
                } else {
                    false
                }
            }).or(Some(&self.bibtex_entries[0]))
        } else {
            Some(&self.bibtex_entries[0])
        }
    }

    /// Returns all parsed BibTeX entries
    pub fn all_bibtex_parsed(&self) -> Vec<crate::notes::ParsedBibtex> {
        use crate::notes::parse_bibtex;
        self.bibtex_entries.iter()
            .filter_map(|entry| parse_bibtex(entry))
            .collect()
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
    pub bibtex: String,
    pub filename: String,
    pub arxiv_id: Option<String>,
    pub doi: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuickNoteRequest {
    pub title: String,
    pub date: Option<String>,
    pub subdirectory: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AttachSourceRequest {
    pub note_key: String,
    pub source_type: String,
    pub identifier: String,
}

// ============================================================================
// BibTeX Import Data Structures
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct BibImportAnalysis {
    pub new_entries: Vec<BibImportEntry>,
    pub existing_entries: Vec<BibImportExisting>,
    pub conflicts: Vec<BibImportConflict>,
    pub parse_errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BibImportEntry {
    pub index: usize,
    pub bibtex: String,
    pub cite_key: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub year: Option<i32>,
    pub suggested_filename: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BibImportExisting {
    pub index: usize,
    pub cite_key: String,
    pub note_key: String,
    pub note_title: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BibImportConflict {
    pub index: usize,
    pub bibtex: String,
    pub cite_key: String,
    pub title: Option<String>,
    pub match_type: String,
    pub matched_note_key: String,
    pub matched_note_title: String,
    pub existing_bibtex: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BibImportExecuteRequest {
    pub create: Vec<BibImportCreateItem>,
    pub add_secondary: Vec<BibImportSecondaryItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BibImportCreateItem {
    pub bibtex: String,
    pub filename: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BibImportSecondaryItem {
    pub note_key: String,
    pub bibtex: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BibImportExecuteResult {
    pub created: Vec<BibImportCreatedNote>,
    pub updated: Vec<BibImportUpdatedNote>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BibImportCreatedNote {
    pub key: String,
    pub filename: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BibImportUpdatedNote {
    pub key: String,
    pub title: String,
}
