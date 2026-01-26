//! Note loading, parsing, search, and content processing.
//!
//! This module handles all operations related to note files including:
//! - Frontmatter parsing (YAML-like format)
//! - File system operations for loading notes
//! - Full-text search
//! - Cross-link processing
//! - Markdown rendering
//! - Git integration for version history
//! - Bibliography generation

use crate::auth::hex_encode;
use crate::models::{
    GitCommit, Note, NoteType, PaperMeta, PaperSource, SearchMatch, SearchResult, TimeCategory,
    TimeEntry,
};
use chrono::{DateTime, NaiveDate, Utc};
use pulldown_cmark::Parser;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use walkdir::WalkDir;

// ============================================================================
// Frontmatter Parsing
// ============================================================================

#[derive(Debug, Default)]
pub struct Frontmatter {
    pub title: Option<String>,
    pub date: Option<NaiveDate>,
    pub note_type: Option<String>,
    pub parent: Option<String>,
    /// One or more BibTeX entries (sole source of truth for paper metadata)
    pub bibtex_entries: Vec<String>,
    /// When multiple bibtex entries exist, this specifies which cite key is canonical
    pub canonical_key: Option<String>,
    pub time: Vec<TimeEntry>,
    pub sources: Vec<PaperSource>,
    pub pdf: Option<String>,
}

pub fn parse_frontmatter(content: &str) -> (Frontmatter, String) {
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
                    t.3 = Some(
                        trimmed
                            .strip_prefix("description:")
                            .unwrap()
                            .trim()
                            .to_string(),
                    );
                }
                continue;
            } else if !trimmed.is_empty()
                && !trimmed.starts_with('-')
                && !line.starts_with("  ")
                && !line.starts_with("\t")
            {
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
                // Add this bibtex entry to the list
                fm.bibtex_entries.push(multiline_value.trim().to_string());
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
                "canonical_key" | "canonical" => fm.canonical_key = Some(value.to_string()),
                "bibtex" => {
                    // Single-line bibtex (rare but supported)
                    if !value.starts_with('|') && !value.is_empty() {
                        fm.bibtex_entries.push(value.to_string());
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
                "pdf" => {
                    if !value.is_empty() {
                        fm.pdf = Some(value.to_string());
                    }
                }
                // Legacy fields - ignore (bibtex is now the source of truth)
                "bib_key" | "bibkey" | "authors" | "venue" | "year" => {}
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
            fm.bibtex_entries.push(multiline_value.trim().to_string());
        }
    }

    let body = lines[end_idx + 1..].join("\n");
    (fm, body)
}

// ============================================================================
// Key Generation
// ============================================================================

pub fn generate_key(path: &PathBuf) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(path.to_string_lossy().as_bytes());
    let result = hasher.finalize();
    hex_encode(&result[..3])
}

// ============================================================================
// Note Loading
// ============================================================================

pub fn load_note(path: &PathBuf, notes_dir: &PathBuf) -> Option<Note> {
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

    let note_type = if fm.note_type.as_deref() == Some("paper") || !fm.bibtex_entries.is_empty() {
        NoteType::Paper(PaperMeta {
            bibtex_entries: fm.bibtex_entries,
            canonical_key: fm.canonical_key,
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
        pdf: fm.pdf,
    })
}

pub fn load_all_notes(notes_dir: &PathBuf) -> Vec<Note> {
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

pub fn search_notes(notes: &[Note], query: &str) -> Vec<SearchResult> {
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

pub fn process_crosslinks(content: &str, notes: &HashMap<String, Note>) -> String {
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

// ============================================================================
// Text Escaping
// ============================================================================

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

// ============================================================================
// Markdown Rendering
// ============================================================================

pub fn render_markdown(content: &str) -> String {
    let parser = Parser::new(content);
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);
    html_output
}

// ============================================================================
// Git Integration
// ============================================================================

pub fn get_git_history(file_path: &PathBuf, notes_dir: &PathBuf) -> Vec<GitCommit> {
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

pub fn get_file_at_commit(
    file_path: &PathBuf,
    commit_hash: &str,
    notes_dir: &PathBuf,
) -> Option<String> {
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
// BibTeX Parsing
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct ParsedBibtex {
    pub entry_type: String,
    pub cite_key: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub year: Option<i32>,
    pub venue: Option<String>,
    pub doi: Option<String>,
}

/// Parse a BibTeX entry string and extract structured fields.
/// Uses regex-based parsing to avoid additional dependencies.
pub fn parse_bibtex(bibtex: &str) -> Option<ParsedBibtex> {
    let bibtex = bibtex.trim();
    if bibtex.is_empty() {
        return None;
    }

    let mut result = ParsedBibtex::default();

    // Parse entry type and cite key: @type{key,
    let entry_re = regex::Regex::new(r"@(\w+)\s*\{\s*([^,\s]+)").ok()?;
    if let Some(caps) = entry_re.captures(bibtex) {
        result.entry_type = caps.get(1)?.as_str().to_lowercase();
        result.cite_key = caps.get(2)?.as_str().to_string();
    } else {
        return None;
    }

    // Helper to extract a field value
    fn extract_field(bibtex: &str, field: &str) -> Option<String> {
        // Match: field = {value} or field = "value" or field = number
        // Need to escape braces for format! macro: {{ becomes {, }} becomes }
        let pattern = format!(
            r#"(?i){}\s*=\s*(?:\{{([^}}]*)\}}|"([^"]*)"|(\d+))"#,
            regex::escape(field)
        );
        let re = regex::Regex::new(&pattern).ok()?;
        if let Some(caps) = re.captures(bibtex) {
            // Return first non-None capture group
            caps.get(1)
                .or_else(|| caps.get(2))
                .or_else(|| caps.get(3))
                .map(|m| m.as_str().trim().to_string())
        } else {
            None
        }
    }

    result.title = extract_field(bibtex, "title");
    result.author = extract_field(bibtex, "author");
    result.doi = extract_field(bibtex, "doi");

    // Parse year
    if let Some(year_str) = extract_field(bibtex, "year") {
        result.year = year_str.parse().ok();
    }

    // Derive venue from journal, booktitle, or howpublished
    result.venue = extract_field(bibtex, "journal")
        .or_else(|| extract_field(bibtex, "booktitle"))
        .or_else(|| extract_field(bibtex, "howpublished"));

    Some(result)
}

// ============================================================================
// Bibliography Export
// ============================================================================

pub fn generate_bibliography(notes: &[Note]) -> String {
    let mut bib = String::new();

    for note in notes {
        if let NoteType::Paper(ref paper) = note.note_type {
            // Include all bibtex entries for this paper
            for bibtex_entry in &paper.bibtex_entries {
                bib.push_str(bibtex_entry);
                bib.push_str("\n\n");
            }
        }
    }

    bib
}

// ============================================================================
// Reference Extraction (for graph building)
// ============================================================================

pub fn extract_references(content: &str) -> Vec<String> {
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
