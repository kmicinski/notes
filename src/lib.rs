//! Notes library - re-exports for testing and external use.
//!
//! This module provides public access to all the application's modules
//! for testing purposes and potential library use.

use sled::Db;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};

pub mod auth;
pub mod citations;
pub mod graph;
pub mod handlers;
pub mod models;
pub mod notes;
pub mod smart_add;
pub mod templates;
pub mod url_validator;

// ============================================================================
// Configuration
// ============================================================================

pub const NOTES_DIR: &str = "content";
pub const PDFS_DIR: &str = "pdfs";
pub const DB_PATH: &str = ".notes_db";

// ============================================================================
// Rate Limiting
// ============================================================================

/// Tracks login failures for rate limiting with exponential backoff.
pub struct LoginRateLimit {
    pub failures: u32,
    pub locked_until: Option<DateTime<Utc>>,
}

impl LoginRateLimit {
    pub fn new() -> Self {
        Self {
            failures: 0,
            locked_until: None,
        }
    }

    /// Check if login attempts are currently locked out.
    pub fn is_locked(&self) -> bool {
        if let Some(until) = self.locked_until {
            Utc::now() < until
        } else {
            false
        }
    }

    /// Record a failed login attempt. After 5 failures, apply exponential backoff capped at 64s.
    pub fn record_failure(&mut self) {
        self.failures += 1;
        if self.failures >= 5 {
            let delay_secs = std::cmp::min(1i64 << (self.failures - 5), 64);
            self.locked_until =
                Some(Utc::now() + chrono::Duration::seconds(delay_secs));
        }
    }

    /// Reset on successful login.
    pub fn reset(&mut self) {
        self.failures = 0;
        self.locked_until = None;
    }
}

// ============================================================================
// Application State
// ============================================================================

#[derive(Clone)]
pub struct AppState {
    pub notes_dir: PathBuf,
    pub pdfs_dir: PathBuf,
    pub db: Db,
    pub password_hash: Option<String>,
    pub login_rate_limit: Arc<Mutex<LoginRateLimit>>,
}

impl AppState {
    pub fn new() -> Self {
        let notes_dir = PathBuf::from(NOTES_DIR);
        fs::create_dir_all(&notes_dir).ok();

        let pdfs_dir = PathBuf::from(PDFS_DIR);
        fs::create_dir_all(&pdfs_dir).ok();

        let db = sled::open(DB_PATH).expect("Failed to open database");

        // Purge expired sessions/CSRF tokens from previous runs
        auth::purge_expired_sessions(&db);

        // Hash password at startup (Argon2id — ~100ms, done once)
        let password_hash = auth::hash_password_at_startup();

        Self {
            notes_dir,
            pdfs_dir,
            db,
            password_hash,
            login_rate_limit: Arc::new(Mutex::new(LoginRateLimit::new())),
        }
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

/// Validate that a constructed path stays within the given base directory.
/// Returns the validated path on success, or an error message on failure.
/// For new files (that don't yet exist), validates the parent directory.
pub fn validate_path_within(base: &PathBuf, target: &PathBuf) -> Result<PathBuf, String> {
    let canonical_base = fs::canonicalize(base)
        .map_err(|e| format!("Cannot resolve base directory: {}", e))?;

    if target.exists() {
        let canonical = fs::canonicalize(target)
            .map_err(|e| format!("Cannot resolve path: {}", e))?;
        if canonical.starts_with(&canonical_base) {
            Ok(canonical)
        } else {
            Err("Path escapes base directory".to_string())
        }
    } else {
        // For new files, ensure the parent is within base
        let parent = target.parent().ok_or("No parent directory")?;
        fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create directory: {}", e))?;
        let canonical_parent = fs::canonicalize(parent)
            .map_err(|e| format!("Cannot resolve parent: {}", e))?;
        if canonical_parent.starts_with(&canonical_base) {
            Ok(target.clone())
        } else {
            Err("Path escapes base directory".to_string())
        }
    }
}

// Re-export commonly used types
pub use models::{
    AttachSourceRequest, CitationMatch, CitationScanAllResult, CitationScanRequest,
    CitationScanResult, CitationWriteRequest, ExternalResult, ExtractedReference, GitCommit,
    GraphEdge, GraphNode, GraphQuery, GraphStats, InputType, KnowledgeGraph, LocalMatch, Note,
    NoteType, PaperMeta, PaperSource, SearchMatch, SearchResult, SmartAddCreateRequest,
    SmartAddRequest, SmartAddResult, TimeCategory, TimeEntry,
};

pub use notes::{
    extract_references, generate_bibliography, generate_key, get_file_at_commit, get_git_history,
    html_escape, load_all_notes, load_note, normalize_bibtex, normalize_title, parse_bibtex,
    parse_frontmatter, process_crosslinks, render_markdown, search_notes, split_bib_file,
    Frontmatter, ParsedBibtex,
};

pub use auth::{
    create_csrf_token, create_session, delete_session, hash_password_at_startup, is_auth_enabled,
    is_logged_in, purge_expired_sessions, verify_and_consume_csrf_token, verify_password,
    verify_session, SESSION_COOKIE, SESSION_TTL_HOURS,
};

pub use graph::{build_knowledge_graph, find_reachable, find_shortest_path};

pub use smart_add::{
    bib_import_analyze, bib_import_execute, detect_input_type, extract_arxiv_id, extract_doi,
    fetch_and_extract_metadata, generate_bib_key, generate_suggested_filename, query_arxiv_api,
    query_claude_for_url, query_crossref_api, query_crossref_by_title, search_local_for_match,
};

pub use templates::{base_html, nav_bar, render_editor, render_viewer, smart_add_html, STYLE};

pub use url_validator::{validate_api_url, validate_url, UrlValidationError};
