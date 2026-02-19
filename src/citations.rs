//! Citation scanning: extract references from PDFs, match against note pool,
//! cache results in sled, and write `[@key]` links into managed markdown sections.

use crate::auth::is_logged_in;
use crate::models::{
    CitationMatch, CitationScanAllResult, CitationScanRequest, CitationScanResult,
    CitationWriteRequest, ExtractedReference, Note, NoteType,
};
use crate::notes::{normalize_title, parse_bibtex};
use crate::smart_add::{extract_arxiv_id, extract_doi};
use crate::AppState;

use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use chrono::Utc;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

const CITATIONS_TREE: &str = "citations";
const BEGIN_MARKER: &str = "<!-- BEGIN AUTO-CITATIONS -->";
const END_MARKER: &str = "<!-- END AUTO-CITATIONS -->";

// ============================================================================
// PDF Text Extraction
// ============================================================================

/// Run `pdftotext <path> -` and return stdout as a String.
/// Note: we intentionally omit `-layout` because it preserves column positioning
/// which merges left/right columns on the same line, mangling reference sections
/// in multi-column papers.
fn extract_pdf_text(path: &Path) -> Result<String, String> {
    let output = Command::new("pdftotext")
        .arg(path.as_os_str())
        .arg("-")
        .output()
        .map_err(|e| format!("Failed to run pdftotext: {}. Is poppler installed?", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pdftotext failed: {}", stderr));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| format!("pdftotext output not valid UTF-8: {}", e))
}

// ============================================================================
// Reference Section Parsing
// ============================================================================

/// Find the reference/bibliography section at the end of extracted PDF text
/// and split it into individual reference entries.
fn extract_references_from_text(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();

    // Search backwards for a heading line that signals references.
    // Matches optional section numbering: "7. REFERENCES", "VII. REFERENCES", "REFERENCES", etc.
    let section_prefix = r"(?:\d+[\.\)]\s*|[IVXLC]+[\.\)]\s*)?";
    // Strict match: heading alone on the line.
    let heading_strict = Regex::new(&format!(
        r"(?i)^\s*{section_prefix}(references|bibliography|works cited|cited references)\s*$"
    ))
    .unwrap();
    // Lenient match: heading at start of line (for multi-column PDFs where
    // column text may follow on the same line).
    let heading_lenient = Regex::new(&format!(
        r"(?i)^\s*{section_prefix}(references|bibliography|works cited|cited references)\s"
    ))
    .unwrap();

    let mut ref_start = None;
    // Search from the last 40% of the document
    let search_from = lines.len().saturating_sub(lines.len() * 2 / 5);
    for i in (search_from..lines.len()).rev() {
        if heading_strict.is_match(lines[i]) {
            ref_start = Some(i + 1);
            break;
        }
    }
    // Fallback: try lenient match
    if ref_start.is_none() {
        for i in (search_from..lines.len()).rev() {
            if heading_lenient.is_match(lines[i]) {
                ref_start = Some(i + 1);
                break;
            }
        }
    }

    let ref_start = match ref_start {
        Some(s) => s,
        None => return Vec::new(),
    };

    let ref_lines = &lines[ref_start..];
    let ref_text: String = ref_lines.join("\n");

    // Try numbered references: [1], [2], ... or 1. 2. ...
    let numbered_bracket = Regex::new(r"(?m)^\s*\[(\d+)\]").unwrap();
    let numbered_dot = Regex::new(r"(?m)^\s*(\d+)\.\s+[A-Z]").unwrap();

    if numbered_bracket.find_count(&ref_text) >= 3 {
        return split_by_pattern(&ref_text, &numbered_bracket);
    }

    if numbered_dot.find_count(&ref_text) >= 3 {
        return split_by_pattern(&ref_text, &numbered_dot);
    }

    // Fallback: split by blank-line separated blocks
    split_by_blank_lines(&ref_text)
}

/// Split reference text at each match of `pattern`, collecting everything between
/// successive matches as one entry.
fn split_by_pattern(text: &str, pattern: &Regex) -> Vec<String> {
    let mut entries = Vec::new();
    let match_positions: Vec<usize> = pattern.find_iter(text).map(|m| m.start()).collect();

    for (i, &start) in match_positions.iter().enumerate() {
        let end = if i + 1 < match_positions.len() {
            match_positions[i + 1]
        } else {
            text.len()
        };
        let entry = text[start..end].trim().to_string();
        if !entry.is_empty() {
            entries.push(entry);
        }
    }
    entries
}

/// Split by blank lines (two or more consecutive newlines).
fn split_by_blank_lines(text: &str) -> Vec<String> {
    let splitter = Regex::new(r"\n\s*\n").unwrap();
    splitter
        .split(text)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s.len() > 20) // Skip very short fragments
        .collect()
}

/// Helper trait for counting regex matches without allocating a Vec.
trait FindCount {
    fn find_count(&self, text: &str) -> usize;
}
impl FindCount for Regex {
    fn find_count(&self, text: &str) -> usize {
        self.find_iter(text).count()
    }
}

// ============================================================================
// Reference Text Parsing
// ============================================================================

/// Parse a single reference entry text to extract structured identifiers.
fn parse_reference_text(text: &str, index: usize) -> ExtractedReference {
    let doi = extract_doi(text);
    let arxiv_id = extract_arxiv_id(text);

    // Extract year: look for 4-digit years between 1900-2099
    let year_re = Regex::new(r"\b((?:19|20)\d{2})\b").unwrap();
    let year = year_re
        .captures_iter(text)
        .filter_map(|c| c.get(1)?.as_str().parse::<i32>().ok())
        .last(); // Prefer later occurrences (often the publication year)

    // Extract author last names: typically at the start before the year
    let authors = extract_author_lastnames(text);

    // Extract title: heuristic — text between first period and second period,
    // or between first comma group and the venue/year.
    let title = extract_title_from_ref(text);

    ExtractedReference {
        raw_text: text.to_string(),
        index,
        doi,
        arxiv_id,
        title,
        authors,
        year,
    }
}

/// Extract author last names from the beginning of a reference entry.
fn extract_author_lastnames(text: &str) -> Vec<String> {
    // Strip leading [N] or N. numbering
    let stripped = Regex::new(r"^\s*(?:\[\d+\]\s*|\d+\.\s*)")
        .unwrap()
        .replace(text, "");

    // Take text up to first year or title delimiter
    let cutoff_re = Regex::new(r#"(?:19|20)\d{2}|["\u{201c}\u{201d}]"#).unwrap();
    let author_part = match cutoff_re.find(&stripped) {
        Some(m) => &stripped[..m.start()],
        None => {
            // Take up to first period followed by a space and uppercase
            let period_re = Regex::new(r"\.\s+[A-Z]").unwrap();
            match period_re.find(&stripped) {
                Some(m) => &stripped[..m.start() + 1],
                None => {
                    let len = stripped.len().min(200);
                    &stripped[..len]
                }
            }
        }
    };

    // Extract capitalized words that look like last names
    let name_re = Regex::new(r"\b([A-Z][a-z]{1,20})\b").unwrap();
    name_re
        .captures_iter(author_part)
        .filter_map(|c| {
            let name = c.get(1)?.as_str();
            // Skip common non-name words
            let skip = [
                "The", "And", "For", "With", "From", "This", "That", "Into", "Over", "Under",
                "Vol", "Proc", "IEEE", "ACM", "Int", "Conf",
            ];
            if skip.contains(&name) {
                None
            } else {
                Some(name.to_lowercase())
            }
        })
        .collect()
}

/// Heuristic title extraction from a reference entry.
fn extract_title_from_ref(text: &str) -> Option<String> {
    // Strip leading numbering
    let stripped = Regex::new(r"^\s*(?:\[\d+\]\s*|\d+\.\s*)")
        .unwrap()
        .replace(text, "");

    // Look for quoted title
    let quoted_re = Regex::new(r#"["\u{201c}]([^"\u{201d}]{10,}?)["\u{201d}]"#).unwrap();
    if let Some(caps) = quoted_re.captures(&stripped) {
        if let Some(m) = caps.get(1) {
            return Some(m.as_str().trim().to_string());
        }
    }

    // Heuristic: after authors (first period or year), the title runs to the next period
    // that's followed by a space and an identifier (venue, "In ", year, etc.)
    let after_author_re = Regex::new(r"(?:19|20)\d{2}[.)]*\s*(.+?)(?:\.\s|$)").unwrap();
    if let Some(caps) = after_author_re.captures(&stripped) {
        if let Some(m) = caps.get(1) {
            let candidate = m.as_str().trim();
            if candidate.len() >= 10 {
                return Some(candidate.to_string());
            }
        }
    }

    None
}

// ============================================================================
// Note Pool Index for Matching
// ============================================================================

pub struct NotePoolIndex {
    doi_to_key: HashMap<String, String>,
    arxiv_to_key: HashMap<String, String>,
    title_to_key: HashMap<String, String>,
    /// author_lastname + year -> list of keys (may be ambiguous)
    author_year_to_keys: HashMap<String, Vec<String>>,
}

impl NotePoolIndex {
    /// Build lookup maps from all notes in the pool.
    pub fn build(notes: &[Note]) -> Self {
        let mut doi_to_key = HashMap::new();
        let mut arxiv_to_key = HashMap::new();
        let mut title_to_key = HashMap::new();
        let mut author_year_to_keys: HashMap<String, Vec<String>> = HashMap::new();

        for note in notes {
            let key = &note.key;

            if let NoteType::Paper(ref meta) = note.note_type {
                // Index DOIs and arXiv IDs from sources
                for source in &meta.sources {
                    match source.source_type.as_str() {
                        "doi" => {
                            doi_to_key
                                .insert(source.identifier.to_lowercase(), key.clone());
                        }
                        "arxiv" => {
                            arxiv_to_key.insert(source.identifier.clone(), key.clone());
                        }
                        _ => {}
                    }
                }

                // Also extract DOI/arXiv from bibtex entries
                for bib_entry in &meta.bibtex_entries {
                    if let Some(parsed) = parse_bibtex(bib_entry) {
                        if let Some(ref doi) = parsed.doi {
                            doi_to_key
                                .entry(doi.to_lowercase())
                                .or_insert_with(|| key.clone());
                        }
                        if let Some(ref eprint) = parsed.eprint {
                            arxiv_to_key
                                .entry(eprint.clone())
                                .or_insert_with(|| key.clone());
                        }

                        // Index by author last names + year
                        if let (Some(ref authors), Some(year)) = (&parsed.author, parsed.year) {
                            let name_re = Regex::new(r"\b([A-Z][a-z]{1,20})\b").unwrap();
                            for cap in name_re.captures_iter(authors) {
                                if let Some(name) = cap.get(1) {
                                    let lookup =
                                        format!("{}_{}", name.as_str().to_lowercase(), year);
                                    author_year_to_keys
                                        .entry(lookup)
                                        .or_default()
                                        .push(key.clone());
                                }
                            }
                        }
                    }
                }
            }

            // Index by normalized title
            let norm = normalize_title(&note.title);
            if norm.len() >= 5 {
                title_to_key.insert(norm, key.clone());
            }
        }

        // Deduplicate author_year entries
        for keys in author_year_to_keys.values_mut() {
            keys.sort();
            keys.dedup();
        }

        NotePoolIndex {
            doi_to_key,
            arxiv_to_key,
            title_to_key,
            author_year_to_keys,
        }
    }

    /// Match a parsed reference against the note pool using tiered matching.
    pub fn match_reference(&self, reference: &ExtractedReference) -> Option<CitationMatch> {
        // Tier 1: DOI exact match (confidence 1.0)
        if let Some(ref doi) = reference.doi {
            if let Some(key) = self.doi_to_key.get(&doi.to_lowercase()) {
                return Some(CitationMatch {
                    target_key: key.clone(),
                    match_type: "doi".to_string(),
                    confidence: 1.0,
                    raw_text: reference.raw_text.clone(),
                });
            }
        }

        // Tier 2: arXiv exact match (confidence 1.0)
        if let Some(ref arxiv_id) = reference.arxiv_id {
            if let Some(key) = self.arxiv_to_key.get(arxiv_id) {
                return Some(CitationMatch {
                    target_key: key.clone(),
                    match_type: "arxiv".to_string(),
                    confidence: 1.0,
                    raw_text: reference.raw_text.clone(),
                });
            }
        }

        // Tier 3: Title match (confidence 0.85)
        if let Some(ref title) = reference.title {
            let norm = normalize_title(title);
            if norm.len() >= 5 {
                if let Some(key) = self.title_to_key.get(&norm) {
                    return Some(CitationMatch {
                        target_key: key.clone(),
                        match_type: "title".to_string(),
                        confidence: 0.85,
                        raw_text: reference.raw_text.clone(),
                    });
                }
            }
        }

        // Tier 4: Author last name + year (confidence 0.5, only if unambiguous)
        if let Some(year) = reference.year {
            for author in &reference.authors {
                let lookup = format!("{}_{}", author, year);
                if let Some(keys) = self.author_year_to_keys.get(&lookup) {
                    if keys.len() == 1 {
                        return Some(CitationMatch {
                            target_key: keys[0].clone(),
                            match_type: "author_year".to_string(),
                            confidence: 0.5,
                            raw_text: reference.raw_text.clone(),
                        });
                    }
                }
            }
        }

        None
    }
}

// ============================================================================
// PDF Hashing for Cache Validation
// ============================================================================

/// SHA256 of the first 64KB of a file, sufficient for change detection.
fn hash_pdf(path: &Path) -> Result<String, String> {
    let mut file =
        std::fs::File::open(path).map_err(|e| format!("Cannot open PDF: {}", e))?;
    let mut buf = vec![0u8; 65536];
    let n = file
        .read(&mut buf)
        .map_err(|e| format!("Cannot read PDF: {}", e))?;
    let mut hasher = Sha256::new();
    hasher.update(&buf[..n]);
    Ok(format!("{:x}", hasher.finalize()))
}

// ============================================================================
// Sled Cache Operations
// ============================================================================

fn load_cached_result(db: &sled::Db, key: &str) -> Option<CitationScanResult> {
    let tree = db.open_tree(CITATIONS_TREE).ok()?;
    let data = tree.get(key.as_bytes()).ok()??;
    serde_json::from_slice(&data).ok()
}

fn save_cached_result(db: &sled::Db, result: &CitationScanResult) -> Result<(), String> {
    let tree = db
        .open_tree(CITATIONS_TREE)
        .map_err(|e| format!("Cannot open citations tree: {}", e))?;
    let json = serde_json::to_vec(result).map_err(|e| format!("JSON serialize error: {}", e))?;
    tree.insert(result.source_key.as_bytes(), json)
        .map_err(|e| format!("Sled insert error: {}", e))?;
    Ok(())
}

// ============================================================================
// Core Scan Logic
// ============================================================================

fn scan_note_pdf(
    note: &Note,
    notes: &[Note],
    pdfs_dir: &Path,
    db: &sled::Db,
    force: bool,
) -> Result<CitationScanResult, String> {
    let pdf_filename = note
        .pdf
        .as_deref()
        .ok_or_else(|| "Note has no attached PDF".to_string())?;

    let pdf_path = pdfs_dir.join(pdf_filename);
    if !pdf_path.exists() {
        return Err(format!("PDF file not found: {}", pdf_filename));
    }

    let current_hash = hash_pdf(&pdf_path)?;

    // Check cache
    if !force {
        if let Some(cached) = load_cached_result(db, &note.key) {
            if cached.pdf_hash == current_hash {
                return Ok(cached);
            }
        }
    }

    // Extract text
    let text = extract_pdf_text(&pdf_path)?;

    // Parse references
    let raw_refs = extract_references_from_text(&text);
    let parsed_refs: Vec<ExtractedReference> = raw_refs
        .iter()
        .enumerate()
        .map(|(i, r)| parse_reference_text(r, i))
        .collect();

    // Build index and match
    let index = NotePoolIndex::build(notes);
    let mut matches = Vec::new();
    let mut matched_keys = std::collections::HashSet::new();
    let mut unmatched = 0;

    for parsed in &parsed_refs {
        if let Some(m) = index.match_reference(parsed) {
            // Don't match self and avoid duplicates
            if m.target_key != note.key && !matched_keys.contains(&m.target_key) {
                matched_keys.insert(m.target_key.clone());
                matches.push(m);
            }
        } else {
            unmatched += 1;
        }
    }

    let result = CitationScanResult {
        source_key: note.key.clone(),
        matches,
        unmatched_count: unmatched,
        timestamp: Utc::now().to_rfc3339(),
        pdf_hash: current_hash,
    };

    save_cached_result(db, &result)?;

    Ok(result)
}

// ============================================================================
// Markdown Writing
// ============================================================================

/// Write citation matches as a `## References` section with `[@key]` links,
/// wrapped in managed markers for idempotent updates.
fn write_citations_to_markdown(
    note: &Note,
    result: &CitationScanResult,
    notes_map: &HashMap<String, Note>,
    notes_dir: &Path,
) -> Result<(), String> {
    let content = &note.full_file_content;

    // Build the new auto-citations block
    let mut block = String::new();
    block.push_str(BEGIN_MARKER);
    block.push('\n');
    block.push_str("## References\n\n");

    let mut sorted_matches = result.matches.clone();
    sorted_matches.sort_by(|a, b| a.target_key.cmp(&b.target_key));

    for m in &sorted_matches {
        let title = notes_map
            .get(&m.target_key)
            .map(|n| n.title.as_str())
            .unwrap_or("Unknown");
        block.push_str(&format!("- [@{}] {}\n", m.target_key, title));
    }
    block.push_str(END_MARKER);

    // Replace existing block or append
    let new_content = if let Some(begin_pos) = content.find(BEGIN_MARKER) {
        if let Some(end_pos) = content.find(END_MARKER) {
            let end = end_pos + END_MARKER.len();
            format!("{}{}{}", &content[..begin_pos], block, &content[end..])
        } else {
            // Malformed: replace from begin marker to end of file
            format!("{}{}", &content[..begin_pos], block)
        }
    } else {
        // Append with blank line
        let trimmed = content.trim_end();
        format!("{}\n\n{}\n", trimmed, block)
    };

    let full_path = notes_dir.join(&note.path);
    std::fs::write(&full_path, &new_content)
        .map_err(|e| format!("Failed to write note to {}: {}", full_path.display(), e))?;

    Ok(())
}

// ============================================================================
// API Handlers
// ============================================================================

/// POST /api/citations/scan — scan one paper's PDF for citations
pub async fn citation_scan(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    axum::Json(req): axum::Json<CitationScanRequest>,
) -> Response {
    if !is_logged_in(&jar, &state.db) {
        return (axum::http::StatusCode::UNAUTHORIZED, "Not logged in").into_response();
    }

    let notes = state.load_notes();
    let notes_map: HashMap<String, Note> = notes.iter().map(|n| (n.key.clone(), n.clone())).collect();

    let note = match notes_map.get(&req.note_key) {
        Some(n) => n.clone(),
        None => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                format!("Note not found: {}", req.note_key),
            )
                .into_response();
        }
    };

    let pdfs_dir = state.pdfs_dir.clone();
    let db = state.db.clone();
    let force = req.force;

    // Run pdftotext on spawn_blocking to avoid blocking async runtime
    let result = tokio::task::spawn_blocking(move || {
        scan_note_pdf(&note, &notes, &pdfs_dir, &db, force)
    })
    .await;

    match result {
        Ok(Ok(scan_result)) => axum::Json(scan_result).into_response(),
        Ok(Err(e)) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Task join error: {}", e),
        )
            .into_response(),
    }
}

/// POST /api/citations/write — write cached scan results into note's markdown
pub async fn citation_write(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    axum::Json(req): axum::Json<CitationWriteRequest>,
) -> Response {
    if !is_logged_in(&jar, &state.db) {
        return (axum::http::StatusCode::UNAUTHORIZED, "Not logged in").into_response();
    }

    let notes_map = state.notes_map();

    let note = match notes_map.get(&req.note_key) {
        Some(n) => n.clone(),
        None => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                format!("Note not found: {}", req.note_key),
            )
                .into_response();
        }
    };

    let cached = match load_cached_result(&state.db, &req.note_key) {
        Some(r) => r,
        None => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                "No cached scan results. Run scan first.",
            )
                .into_response();
        }
    };

    if cached.matches.is_empty() {
        return (axum::http::StatusCode::OK, "No matches to write").into_response();
    }

    match write_citations_to_markdown(&note, &cached, &notes_map, &state.notes_dir) {
        Ok(()) => {
            let msg = format!("Wrote {} citation(s) to {}", cached.matches.len(), req.note_key);
            (axum::http::StatusCode::OK, msg).into_response()
        }
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

/// POST /api/citations/scan-all — bulk scan all papers with PDFs
pub async fn citation_scan_all(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Response {
    if !is_logged_in(&jar, &state.db) {
        return (axum::http::StatusCode::UNAUTHORIZED, "Not logged in").into_response();
    }

    let notes = state.load_notes();
    let pdfs_dir = state.pdfs_dir.clone();
    let db = state.db.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut scanned = 0;
        let mut skipped_cached = 0;
        let mut skipped_no_pdf = 0;
        let mut total_matches = 0;
        let mut errors = Vec::new();

        for note in &notes {
            if note.pdf.is_none() {
                skipped_no_pdf += 1;
                continue;
            }

            // Check cache first
            if let Some(cached) = load_cached_result(&db, &note.key) {
                let pdf_path = pdfs_dir.join(note.pdf.as_deref().unwrap());
                if let Ok(hash) = hash_pdf(&pdf_path) {
                    if cached.pdf_hash == hash {
                        skipped_cached += 1;
                        total_matches += cached.matches.len();
                        continue;
                    }
                }
            }

            match scan_note_pdf(note, &notes, &pdfs_dir, &db, false) {
                Ok(result) => {
                    total_matches += result.matches.len();
                    scanned += 1;
                }
                Err(e) => {
                    errors.push(format!("{}: {}", note.key, e));
                }
            }
        }

        CitationScanAllResult {
            scanned,
            skipped_cached,
            skipped_no_pdf,
            total_matches,
            errors,
        }
    })
    .await;

    match result {
        Ok(scan_all_result) => axum::Json(scan_all_result).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Task join error: {}", e),
        )
            .into_response(),
    }
}
