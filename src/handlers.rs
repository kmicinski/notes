//! HTTP route handlers for the web application.
//!
//! This module contains all the route handlers for the notes application,
//! including index, search, note viewing/editing, authentication, and more.

use crate::auth::{create_session, is_logged_in, SESSION_COOKIE, SESSION_TTL_HOURS};
use crate::models::{Note, NoteType, TimeCategory};
use crate::notes::{
    generate_bibliography, generate_key, get_file_at_commit, get_git_history, html_escape,
    parse_frontmatter, process_crosslinks, render_markdown, search_notes,
};
use crate::templates::{base_html, render_editor, render_viewer};
use crate::AppState;
use axum::{
    extract::{Multipart, Path, Query, State},
    http::{header::SET_COOKIE, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_extra::extract::CookieJar;
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use reqwest;
use subtle::ConstantTimeEq;
use crate::url_validator::validate_url;
use crate::validate_path_within;

// ============================================================================
// Index Handler
// ============================================================================

pub async fn index(State(state): State<Arc<AppState>>, jar: CookieJar) -> Html<String> {
    let logged_in = is_logged_in(&jar);
    let notes = state.load_notes();

    let mut list_html = String::from("<ul class=\"note-list\">");

    for note in &notes {
        let is_paper = matches!(note.note_type, NoteType::Paper(_));
        let class = if is_paper {
            "note-item paper"
        } else {
            "note-item"
        };
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

// ============================================================================
// Search Handler
// ============================================================================

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

pub async fn search(
    Query(query): Query<SearchQuery>,
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Html<String> {
    let logged_in = is_logged_in(&jar);
    let q = query.q.unwrap_or_default();

    if q.is_empty() {
        return Html(base_html(
            "Search",
            "<p>Enter a search term.</p>",
            Some(&q),
            logged_in,
        ));
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

    Html(base_html(
        &format!("Search: {}", q),
        &html,
        Some(&q),
        logged_in,
    ))
}

// ============================================================================
// Note View Handler
// ============================================================================

#[derive(Deserialize)]
pub struct NoteQuery {
    pub edit: Option<bool>,
}

pub async fn view_note(
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
        return Html(render_editor(note, &notes_map, logged_in)).into_response();
    }

    render_view(note, &notes_map, &state.notes_dir, logged_in).into_response()
}

fn render_view(
    note: &Note,
    notes_map: &HashMap<String, Note>,
    notes_dir: &PathBuf,
    logged_in: bool,
) -> Html<String> {
    let mut meta_html = String::from("<div class=\"meta-block\">");

    // Helper macro for rows
    fn meta_row(label: &str, value: &str) -> String {
        format!(
            r#"<div class="meta-row"><span class="meta-label">{}</span><span class="meta-value">{}</span></div>"#,
            label, value
        )
    }

    meta_html.push_str(&meta_row("Key", &format!("<code>[@{}]</code>", note.key)));

    if let Some(date) = note.date {
        meta_html.push_str(&meta_row("Date", &date.format("%Y-%m-%d").to_string()));
    }

    if let NoteType::Paper(ref paper) = note.note_type {
        let effective = paper.effective_metadata(&note.title);

        meta_html.push_str(&meta_row(
            "Cite",
            &format!("<code>{}</code>", html_escape(&effective.bib_key)),
        ));
        if let Some(ref authors) = effective.authors {
            meta_html.push_str(&meta_row("Authors", &html_escape(authors)));
        }
        if let Some(year) = effective.year {
            meta_html.push_str(&meta_row("Year", &year.to_string()));
        }
        if let Some(ref venue) = effective.venue {
            meta_html.push_str(&meta_row("Venue", &html_escape(venue)));
        }
        if !paper.sources.is_empty() {
            let mut sources_html = String::new();
            for (i, source) in paper.sources.iter().enumerate() {
                if i > 0 {
                    sources_html.push_str(" · ");
                }
                let link = match source.source_type.as_str() {
                    "arxiv" => format!(
                        "<a href=\"https://arxiv.org/abs/{}\" target=\"_blank\">arXiv</a>",
                        html_escape(&source.identifier)
                    ),
                    "doi" => format!(
                        "<a href=\"https://doi.org/{}\" target=\"_blank\">DOI</a>",
                        html_escape(&source.identifier)
                    ),
                    _ => format!(
                        "<a href=\"{}\" target=\"_blank\">Link</a>",
                        html_escape(&source.identifier)
                    ),
                };
                sources_html.push_str(&link);
            }
            meta_html.push_str(&meta_row("Sources", &sources_html));
        }
    }

    if let Some(ref parent_key) = note.parent_key {
        if let Some(parent) = notes_map.get(parent_key) {
            meta_html.push_str(&meta_row(
                "Parent",
                &format!(
                    "<a href=\"/note/{}\">{}</a>",
                    parent_key,
                    html_escape(&parent.title)
                ),
            ));
        }
    }

    meta_html.push_str("</div>");

    // BibTeX block (separate from meta)
    if let NoteType::Paper(ref paper) = note.note_type {
        if let Some(bibtex) = paper.canonical_bibtex() {
            let bibtex_id = format!("bibtex-{}", note.key);
            meta_html.push_str(&format!(
                r#"<div class="bibtex-block" onclick="copyBibtex('{}')">
                    <div class="bibtex-header">
                        <span>BibTeX</span>
                        <span class="bibtex-copy-hint" id="{}-hint">Click to copy</span>
                    </div>
                    <pre id="{}">{}</pre>
                </div>"#,
                bibtex_id, bibtex_id, bibtex_id, html_escape(bibtex)
            ));
        }
    }

    let content_with_links = process_crosslinks(&note.raw_content, notes_map);
    let rendered_content = render_markdown(&content_with_links);

    let mut time_html = String::new();
    if !note.time_entries.is_empty() {
        time_html.push_str("<h2>Time Log</h2><table class=\"time-table\">");
        time_html
            .push_str("<tr><th>Date</th><th>Minutes</th><th>Category</th><th>Description</th></tr>");

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
                <button class="delete-btn" onclick="confirmDelete('{}', '{}')">Delete</button>
            </div>"#,
            note.key,
            note.key,
            html_escape(&note.title).replace('\'', "\\'")
        )
    } else {
        String::new()
    };

    // Use full-page viewer layout if note has a PDF (for split view)
    if note.pdf.is_some() {
        return Html(render_viewer(
            note,
            &rendered_content,
            &meta_html,
            &time_html,
            &sub_notes_html,
            &history_html,
            logged_in,
        ));
    }

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

// ============================================================================
// Note Save Handler
// ============================================================================

#[derive(Deserialize)]
pub struct SaveNoteBody {
    pub content: String,
    #[serde(default)]
    pub auto_commit: bool,
}

pub async fn save_note(
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
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to save: {}", e),
        )
            .into_response();
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

// ============================================================================
// Note Delete Handler
// ============================================================================

#[derive(Deserialize)]
pub struct DeleteNoteBody {
    pub confirm: bool,
}

pub async fn delete_note(
    Path(key): Path<String>,
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    axum::Json(body): axum::Json<DeleteNoteBody>,
) -> Response {
    if !is_logged_in(&jar) {
        return (StatusCode::UNAUTHORIZED, "Not logged in").into_response();
    }

    if !body.confirm {
        return (StatusCode::BAD_REQUEST, "Deletion not confirmed").into_response();
    }

    let notes_map = state.notes_map();

    let note = match notes_map.get(&key) {
        Some(n) => n,
        None => return (StatusCode::NOT_FOUND, "Note not found").into_response(),
    };

    let full_path = state.notes_dir.join(&note.path);
    let note_path = note.path.clone();
    let note_title = note.title.clone();

    // Delete the file
    if let Err(e) = fs::remove_file(&full_path) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to delete: {}", e),
        )
            .into_response();
    }

    // Git commit the deletion
    let notes_dir = state.notes_dir.clone();
    tokio::task::spawn_blocking(move || {
        let now = chrono::Local::now();
        let commit_msg = format!(
            "deleted note '{}': {}",
            note_title,
            now.format("%a %b %d, %-I:%M%p")
        );

        // Stage the deletion
        let _ = Command::new("git")
            .args(["rm", "--cached", &note_path.to_string_lossy()])
            .current_dir(&notes_dir)
            .output();

        // Also stage the actual file removal
        let _ = Command::new("git")
            .args(["add", "-A"])
            .current_dir(&notes_dir)
            .output();

        // Commit
        let _ = Command::new("git")
            .args(["commit", "-m", &commit_msg])
            .current_dir(&notes_dir)
            .output();
    });

    (StatusCode::OK, "Deleted").into_response()
}

// ============================================================================
// Note History Handler
// ============================================================================

pub async fn view_note_history(
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
        html_escape(&key),
        html_escape(&note.title),
        html_escape(&commit),
        rendered
    );

    Html(base_html(
        &format!("{} (history)", note.title),
        &html,
        None,
        logged_in,
    ))
    .into_response()
}

// ============================================================================
// Authentication Handlers
// ============================================================================

pub async fn login_page(jar: CookieJar) -> Response {
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
pub struct LoginForm {
    pub password: String,
}

pub async fn login_submit(axum::Form(form): axum::Form<LoginForm>) -> Response {
    if !crate::auth::is_auth_enabled() {
        let html = r#"<div class="message error">Authentication not configured.</div>"#;
        return Html(base_html("Error", html, None, false)).into_response();
    }

    let password = std::env::var("NOTES_PASSWORD").unwrap_or_default();
    let input_bytes = form.password.as_bytes();
    let expected_bytes = password.as_bytes();
    let password_matches = input_bytes.len() == expected_bytes.len()
        && input_bytes.ct_eq(expected_bytes).unwrap_u8() == 1;
    if !password_matches {
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
        "{}={}; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age={}",
        SESSION_COOKIE,
        session_token,
        SESSION_TTL_HOURS * 3600
    );

    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, cookie.parse().unwrap());

    (headers, Redirect::to("/")).into_response()
}

pub async fn logout() -> Response {
    let cookie = format!("{}=; Path=/; HttpOnly; Secure; Max-Age=0", SESSION_COOKIE);

    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, cookie.parse().unwrap());

    (headers, Redirect::to("/")).into_response()
}

// ============================================================================
// New Note Handlers
// ============================================================================

pub async fn new_note_page(jar: CookieJar) -> Response {
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
pub struct NewNoteForm {
    pub title: String,
    pub filename: String,
    pub note_type: String,
    pub date: Option<String>,
    pub bib_key: Option<String>,
    pub authors: Option<String>,
    pub year: Option<String>,
    pub venue: Option<String>,
}

pub async fn create_note(
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

    // Check for path traversal: reject .., absolute paths, and null bytes
    if filename.contains("..") || filename.starts_with('/') || filename.contains('\0') {
        let html = r#"<div class="message error">Invalid filename.</div>"#;
        return Html(base_html("Error", html, None, true)).into_response();
    }

    let file_path = state.notes_dir.join(filename);

    // Validate the path stays within notes_dir
    if let Err(_) = validate_path_within(&state.notes_dir, &file_path) {
        let html = r#"<div class="message error">Invalid filename.</div>"#;
        return Html(base_html("Error", html, None, true)).into_response();
    }

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
            let html = format!(
                r#"<div class="message error">Failed to create directory: {}</div>"#,
                e
            );
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
        let html = format!(
            r#"<div class="message error">Failed to create note: {}</div>"#,
            e
        );
        return Html(base_html("Error", &html, None, true)).into_response();
    }

    // Get the key of the new note
    let relative_path = PathBuf::from(filename);
    let key = generate_key(&relative_path);

    // Redirect to edit the new note
    Redirect::to(&format!("/note/{}?edit=true", key)).into_response()
}

// ============================================================================
// Papers Handler
// ============================================================================

pub async fn papers(State(state): State<Arc<AppState>>, jar: CookieJar) -> Html<String> {
    let logged_in = is_logged_in(&jar);
    let notes = state.load_notes();
    let papers: Vec<_> = notes
        .iter()
        .filter(|n| matches!(n.note_type, NoteType::Paper(_)))
        .collect();

    let mut html = String::from("<h1>Papers</h1><ul class=\"note-list\">");

    for note in papers {
        if let NoteType::Paper(ref paper) = note.note_type {
            let meta = paper.effective_metadata(&note.title);
            let authors = meta.authors.as_deref().unwrap_or("Unknown");
            let year = meta.year.map(|y| y.to_string()).unwrap_or_default();

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
                meta.bib_key
            ));
        }
    }

    html.push_str("</ul>");

    Html(base_html("Papers", &html, None, logged_in))
}

// ============================================================================
// Time Tracking Handler
// ============================================================================

pub async fn time_tracking(State(state): State<Arc<AppState>>, jar: CookieJar) -> Html<String> {
    let logged_in = is_logged_in(&jar);
    let notes = state.load_notes();

    let mut totals: HashMap<TimeCategory, u32> = HashMap::new();
    let mut entries_by_date: HashMap<chrono::NaiveDate, Vec<(&Note, &crate::models::TimeEntry)>> =
        HashMap::new();

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
    entries_html.push_str(
        "<tr><th>Date</th><th>Note</th><th>Category</th><th>Minutes</th><th>Description</th></tr>",
    );

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

// ============================================================================
// Bibliography Handler
// ============================================================================

pub async fn bibliography(State(state): State<Arc<AppState>>) -> Response {
    let notes = state.load_notes();
    let bib = generate_bibliography(&notes);

    ([("content-type", "text/plain; charset=utf-8")], bib).into_response()
}

// ============================================================================
// PDF Handlers
// ============================================================================

#[derive(Deserialize)]
pub struct UploadPdfQuery {
    pub note_key: String,
}

pub async fn upload_pdf(
    Query(query): Query<UploadPdfQuery>,
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    mut multipart: Multipart,
) -> Response {
    if !is_logged_in(&jar) {
        return (StatusCode::UNAUTHORIZED, "Not logged in").into_response();
    }

    let notes_map = state.notes_map();
    let note = match notes_map.get(&query.note_key) {
        Some(n) => n.clone(),
        None => return (StatusCode::NOT_FOUND, "Note not found").into_response(),
    };

    // Get the file from multipart
    let mut filename = String::new();
    let mut file_data = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            filename = field
                .file_name()
                .unwrap_or("document.pdf")
                .to_string();

            match field.bytes().await {
                Ok(bytes) => file_data = bytes.to_vec(),
                Err(e) => return (StatusCode::BAD_REQUEST, format!("Failed to read file: {}", e)).into_response(),
            }
            break;
        }
    }

    if file_data.is_empty() {
        return (StatusCode::BAD_REQUEST, "No file uploaded").into_response();
    }

    // Sanitize filename
    let safe_filename = sanitize_pdf_filename(&filename);
    let pdf_path = state.pdfs_dir.join(&safe_filename);

    // Validate path stays within pdfs_dir
    if let Err(_) = validate_path_within(&state.pdfs_dir, &pdf_path) {
        return (StatusCode::BAD_REQUEST, "Invalid filename").into_response();
    }

    // Save file
    if let Err(e) = fs::write(&pdf_path, &file_data) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save PDF: {}", e)).into_response();
    }

    // Update note frontmatter
    if let Err(e) = update_note_pdf_frontmatter(&state.notes_dir, &note.path, &safe_filename) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to update note: {}", e)).into_response();
    }

    axum::Json(serde_json::json!({
        "success": true,
        "filename": safe_filename
    })).into_response()
}

#[derive(Deserialize)]
pub struct DownloadPdfRequest {
    pub note_key: String,
    pub url: String,
}

pub async fn download_pdf_from_url(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    axum::Json(body): axum::Json<DownloadPdfRequest>,
) -> Response {
    if !is_logged_in(&jar) {
        return (StatusCode::UNAUTHORIZED, "Not logged in").into_response();
    }

    // Validate URL for SSRF protection
    if let Err(e) = validate_url(&body.url) {
        return (StatusCode::BAD_REQUEST, format!("Invalid URL: {}", e)).into_response();
    }

    let notes_map = state.notes_map();
    let note = match notes_map.get(&body.note_key) {
        Some(n) => n.clone(),
        None => return (StatusCode::NOT_FOUND, "Note not found").into_response(),
    };

    // Download the PDF
    let client = reqwest::Client::new();
    let response = match client.get(&body.url).send().await {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("Failed to download: {}", e)).into_response(),
    };

    if !response.status().is_success() {
        return (StatusCode::BAD_REQUEST, format!("Download failed with status: {}", response.status())).into_response();
    }

    let bytes = match response.bytes().await {
        Ok(b) => b,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("Failed to read response: {}", e)).into_response(),
    };

    // Generate filename from URL or use bib_key
    let filename = if let crate::models::NoteType::Paper(ref paper) = note.note_type {
        let meta = paper.effective_metadata(&note.title);
        format!("{}.pdf", meta.bib_key)
    } else {
        let url_path = body.url.split('/').last().unwrap_or("document");
        if url_path.ends_with(".pdf") {
            url_path.to_string()
        } else {
            format!("{}.pdf", note.key)
        }
    };

    let safe_filename = sanitize_pdf_filename(&filename);
    let pdf_path = state.pdfs_dir.join(&safe_filename);

    // Validate path stays within pdfs_dir
    if let Err(_) = validate_path_within(&state.pdfs_dir, &pdf_path) {
        return (StatusCode::BAD_REQUEST, "Invalid filename").into_response();
    }

    // Save file
    if let Err(e) = fs::write(&pdf_path, &bytes) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save PDF: {}", e)).into_response();
    }

    // Update note frontmatter
    if let Err(e) = update_note_pdf_frontmatter(&state.notes_dir, &note.path, &safe_filename) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to update note: {}", e)).into_response();
    }

    axum::Json(serde_json::json!({
        "success": true,
        "filename": safe_filename
    })).into_response()
}

#[derive(Deserialize)]
pub struct RenamePdfRequest {
    pub note_key: String,
    pub new_name: String,
}

pub async fn rename_pdf(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    axum::Json(body): axum::Json<RenamePdfRequest>,
) -> Response {
    if !is_logged_in(&jar) {
        return (StatusCode::UNAUTHORIZED, "Not logged in").into_response();
    }

    let notes_map = state.notes_map();
    let note = match notes_map.get(&body.note_key) {
        Some(n) => n.clone(),
        None => return (StatusCode::NOT_FOUND, "Note not found").into_response(),
    };

    let old_filename = match &note.pdf {
        Some(f) => f.clone(),
        None => return (StatusCode::BAD_REQUEST, "Note has no PDF attached").into_response(),
    };

    // Sanitize both old (from frontmatter, could be tampered) and new filenames
    let old_filename_safe = sanitize_pdf_filename(&old_filename);
    let new_filename = sanitize_pdf_filename(&body.new_name);
    let old_path = state.pdfs_dir.join(&old_filename_safe);
    let new_path = state.pdfs_dir.join(&new_filename);

    // Validate both paths stay within pdfs_dir
    if let Err(_) = validate_path_within(&state.pdfs_dir, &old_path) {
        return (StatusCode::BAD_REQUEST, "Invalid source filename").into_response();
    }
    if let Err(_) = validate_path_within(&state.pdfs_dir, &new_path) {
        return (StatusCode::BAD_REQUEST, "Invalid target filename").into_response();
    }

    if !old_path.exists() {
        return (StatusCode::NOT_FOUND, "PDF file not found").into_response();
    }

    if let Err(e) = fs::rename(&old_path, &new_path) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to rename PDF: {}", e)).into_response();
    }

    // Update note frontmatter
    if let Err(e) = update_note_pdf_frontmatter(&state.notes_dir, &note.path, &new_filename) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to update note: {}", e)).into_response();
    }

    axum::Json(serde_json::json!({
        "success": true,
        "filename": new_filename
    })).into_response()
}

fn sanitize_pdf_filename(filename: &str) -> String {
    // Allow only safe characters: alphanumeric, hyphen, underscore, dot
    let safe: String = filename
        .trim()
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .take(200) // Limit filename length
        .collect();

    let safe = if safe.is_empty() {
        "document".to_string()
    } else {
        safe
    };

    if safe.to_lowercase().ends_with(".pdf") {
        safe
    } else {
        format!("{}.pdf", safe)
    }
}

fn update_note_pdf_frontmatter(notes_dir: &PathBuf, note_path: &PathBuf, pdf_filename: &str) -> Result<(), String> {
    let full_path = notes_dir.join(note_path);
    let content = fs::read_to_string(&full_path)
        .map_err(|e| format!("Failed to read note: {}", e))?;

    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() || lines[0].trim() != "---" {
        return Err("Note has no frontmatter".to_string());
    }

    // Find end of frontmatter
    let mut end_idx = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            end_idx = Some(i);
            break;
        }
    }

    let end_idx = end_idx.ok_or("Invalid frontmatter")?;

    // Check if pdf: already exists in frontmatter
    let mut has_pdf = false;
    let mut new_lines: Vec<String> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if i > 0 && i < end_idx && line.trim().starts_with("pdf:") {
            has_pdf = true;
            new_lines.push(format!("pdf: {}", pdf_filename));
        } else {
            new_lines.push(line.to_string());
        }
    }

    // If pdf: didn't exist, add it before the closing ---
    if !has_pdf {
        new_lines.insert(end_idx, format!("pdf: {}", pdf_filename));
    }

    let new_content = new_lines.join("\n");
    fs::write(&full_path, new_content)
        .map_err(|e| format!("Failed to write note: {}", e))?;

    Ok(())
}
