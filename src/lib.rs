//! Notes library - re-exports for testing and external use.
//!
//! This module provides public access to all the application's modules
//! for testing purposes and potential library use.

use sled::Db;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub mod auth;
pub mod graph;
pub mod handlers;
pub mod models;
pub mod notes;
pub mod smart_add;
pub mod templates;

// ============================================================================
// Configuration
// ============================================================================

pub const NOTES_DIR: &str = "content";
pub const DB_PATH: &str = ".notes_db";

// ============================================================================
// Application State
// ============================================================================

#[derive(Clone)]
pub struct AppState {
    pub notes_dir: PathBuf,
    #[allow(dead_code)]
    db: Db,
}

impl AppState {
    pub fn new() -> Self {
        let notes_dir = PathBuf::from(NOTES_DIR);
        fs::create_dir_all(&notes_dir).ok();

        let db = sled::open(DB_PATH).expect("Failed to open database");

        Self { notes_dir, db }
    }

    pub fn load_notes(&self) -> Vec<models::Note> {
        notes::load_all_notes(&self.notes_dir)
    }

    pub fn notes_map(&self) -> HashMap<String, models::Note> {
        self.load_notes()
            .into_iter()
            .map(|n| (n.key.clone(), n))
            .collect()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export commonly used types
pub use models::{
    AttachSourceRequest, ExternalResult, GitCommit, GraphEdge, GraphNode, GraphQuery, GraphStats,
    InputType, KnowledgeGraph, LocalMatch, Note, NoteType, PaperMeta, PaperSource, SearchMatch,
    SearchResult, SmartAddCreateRequest, SmartAddRequest, SmartAddResult, TimeCategory, TimeEntry,
};

pub use notes::{
    extract_references, generate_bibliography, generate_key, get_file_at_commit, get_git_history,
    html_escape, load_all_notes, load_note, parse_bibtex, parse_frontmatter,
    process_crosslinks, render_markdown, search_notes, Frontmatter, ParsedBibtex,
};

pub use auth::{
    base64_decode, base64_encode, create_session, get_secret_key, hex_encode, is_auth_enabled,
    is_logged_in, verify_session, SESSION_COOKIE, SESSION_TTL_HOURS,
};

pub use graph::{build_knowledge_graph, find_reachable, find_shortest_path};

pub use smart_add::{
    detect_input_type, extract_arxiv_id, extract_doi, fetch_and_extract_metadata, generate_bib_key,
    generate_suggested_filename, query_arxiv_api, query_claude_for_url, query_crossref_api,
    query_crossref_by_title, search_local_for_match,
};

pub use templates::{base_html, nav_bar, render_editor, smart_add_html, STYLE};
