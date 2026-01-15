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
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sled::Db;
use std::{
    collections::HashMap,
    env,
    fs,
    path::PathBuf,
    process::Command,
    sync::Arc,
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
:root {
    --bg: #fafafa;
    --fg: #1a1a1a;
    --muted: #666;
    --border: #e0e0e0;
    --link: #0066cc;
    --link-hover: #004499;
    --accent: #f0f0f0;
    --paper-bg: #fff8e7;
}

@media (prefers-color-scheme: dark) {
    :root {
        --bg: #1e1e1e;
        --fg: #d4d4d4;
        --muted: #888;
        --border: #333;
        --link: #6699ff;
        --link-hover: #99bbff;
        --accent: #252526;
        --paper-bg: #2a2518;
    }
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

.cat-programming { background: #4a90d9; }
.cat-teaching { background: #50c878; }
.cat-reading { background: #f4a460; }
.cat-writing { background: #dda0dd; }
.cat-service { background: #778899; }
.cat-other { background: #999; }

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
    height: calc(100vh - 200px);
    min-height: 400px;
    border: 1px solid var(--border);
    border-radius: 4px;
    overflow: hidden;
}

#monaco-editor {
    width: 100%;
    height: 100%;
}

.editor-actions {
    margin-top: 1rem;
    display: flex;
    gap: 1rem;
    align-items: center;
}

.btn {
    padding: 0.5rem 1rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--link);
    color: white;
    cursor: pointer;
    font-size: 0.9rem;
    font-family: inherit;
}

.btn:hover { background: var(--link-hover); }
.btn.secondary { background: var(--accent); color: var(--fg); }
.btn.secondary:hover { background: var(--border); }

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
.message.error { background: #422; color: #faa; border: 1px solid #633; }
.message.success { background: #242; color: #afa; border: 1px solid #363; }

.back-link {
    display: inline-block;
    margin-bottom: 1rem;
    font-size: 0.9rem;
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

fn base_html(title: &str, content: &str, search_query: Option<&str>, logged_in: bool) -> String {
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
</body>
</html>"#,
        title = html_escape(title),
        nav = nav_bar(search_query, logged_in),
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

fn render_editor(note: &Note, _notes_map: &HashMap<String, Note>, logged_in: bool) -> Html<String> {
    let content_escaped = js_escape(&note.full_file_content);

    let html = format!(
        r##"<div class="note-header">
            <h1>Editing: {title}</h1>
            <div class="mode-toggle">
                <button onclick="window.location.href='/note/{key}'">View</button>
                <button class="active">Edit</button>
            </div>
        </div>
        <div class="editor-container">
            <div id="monaco-editor"></div>
        </div>
        <div class="editor-actions">
            <button class="btn" onclick="saveNote()">Save</button>
            <button class="btn secondary" onclick="window.location.href='/note/{key}'">Cancel</button>
            <span id="save-status" class="save-status"></span>
        </div>

        <script src="https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.45.0/min/vs/loader.min.js"></script>
        <script>
            let editor;
            const noteKey = "{key}";

            require.config({{ paths: {{ vs: 'https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.45.0/min/vs' }} }});

            require(['vs/editor/editor.main'], function() {{
                const isDark = window.matchMedia('(prefers-color-scheme: dark)').matches;

                editor = monaco.editor.create(document.getElementById('monaco-editor'), {{
                    value: `{content}`,
                    language: 'markdown',
                    theme: isDark ? 'vs-dark' : 'vs',
                    fontSize: 14,
                    lineNumbers: 'on',
                    wordWrap: 'on',
                    minimap: {{ enabled: false }},
                    scrollBeyondLastLine: false,
                    automaticLayout: true,
                    tabSize: 2,
                    insertSpaces: true,
                    renderWhitespace: 'selection',
                    lineHeight: 1.6
                }});

                // Ctrl/Cmd+S to save
                editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, function() {{
                    saveNote();
                }});

                // Listen for theme changes
                window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', e => {{
                    monaco.editor.setTheme(e.matches ? 'vs-dark' : 'vs');
                }});
            }});

            async function saveNote() {{
                const status = document.getElementById('save-status');
                status.textContent = 'Saving...';
                status.className = 'save-status saving';

                try {{
                    const response = await fetch('/api/note/' + noteKey, {{
                        method: 'POST',
                        headers: {{ 'Content-Type': 'application/json' }},
                        body: JSON.stringify({{ content: editor.getValue() }})
                    }});

                    if (response.ok) {{
                        status.textContent = 'Saved!';
                        status.className = 'save-status saved';
                        setTimeout(() => {{ status.textContent = ''; }}, 2000);
                    }} else {{
                        const err = await response.text();
                        status.textContent = 'Error: ' + err;
                        status.className = 'save-status error';
                    }}
                }} catch (e) {{
                    status.textContent = 'Error: ' + e.message;
                    status.className = 'save-status error';
                }}
            }}
        </script>"##,
        title = html_escape(&note.title),
        key = note.key,
        content = content_escaped,
    );

    Html(base_html(&format!("Edit: {}", note.title), &html, None, logged_in))
}

#[derive(Deserialize)]
struct SaveNoteBody {
    content: String,
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

    if let Err(e) = fs::write(&full_path, &body.content) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save: {}", e)).into_response();
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
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState::new());

    let app = Router::new()
        .route("/", get(index))
        .route("/search", get(search))
        .route("/login", get(login_page).post(login_submit))
        .route("/logout", get(logout))
        .route("/note/{key}", get(view_note))
        .route("/api/note/{key}", axum::routing::post(save_note))
        .route("/note/{key}/history/{commit}", get(view_note_history))
        .route("/papers", get(papers))
        .route("/time", get(time_tracking))
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
