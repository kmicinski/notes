//! HTML templates and styling.
//!
//! This module contains all the CSS styles, JavaScript code, and HTML
//! generation functions for the web interface.

use crate::auth::is_auth_enabled;
use crate::models::Note;
use crate::notes::html_escape;
use serde::Serialize;
use std::collections::HashMap;

// ============================================================================
// CSS Styles
// ============================================================================

pub const STYLE: &str = r#"
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
    padding: 0.5rem 0.75rem;
    margin-bottom: 1rem;
    border-radius: 4px;
    font-size: 0.8rem;
    line-height: 1.4;
}
.meta-block .meta-row {
    display: flex;
    gap: 0.5rem;
}
.meta-block .meta-label {
    font-weight: 600;
    color: var(--base01);
    min-width: 60px;
}
.meta-block .meta-value {
    color: var(--fg);
}
.meta-block code {
    font-size: 0.75rem;
    background: var(--bg);
    padding: 0.1rem 0.3rem;
    border-radius: 2px;
}

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

/* BibTeX Block */
.bibtex-block {
    background: var(--code-bg);
    border: 1px solid var(--border);
    border-radius: 4px;
    margin: 1rem 0;
    cursor: pointer;
    transition: border-color 0.2s;
}
.bibtex-block:hover {
    border-color: var(--link);
}
.bibtex-header {
    display: flex;
    justify-content: space-between;
    padding: 0.5rem 1rem;
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--muted);
}
.bibtex-copy-hint {
    font-weight: normal;
    font-style: italic;
}
.bibtex-block pre {
    margin: 0;
    padding: 0 1rem;
    font-family: "SF Mono", "Consolas", "Liberation Mono", monospace;
    font-size: 0.8rem;
    white-space: pre-wrap;
    word-wrap: break-word;
    background: transparent;
    border-radius: 0;
    max-height: 0;
    overflow: hidden;
    transition: max-height 0.3s ease, padding 0.3s ease;
}
.bibtex-block:hover pre {
    max-height: 500px;
    padding: 1rem;
    border-top: 1px solid var(--border);
}

/* Delete Button */
.delete-btn {
    background: var(--red) !important;
    color: white !important;
    border-color: var(--red) !important;
}
.delete-btn:hover {
    background: #b02020 !important;
    border-color: #b02020 !important;
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

// ============================================================================
// Navigation Bar
// ============================================================================

pub fn nav_bar(search_query: Option<&str>, logged_in: bool) -> String {
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

// ============================================================================
// Smart Add HTML
// ============================================================================

pub fn smart_add_html() -> &'static str {
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
            const match = input.match(/(\d{4}\.\d{4,5})/);
            return match ? match[1] : input;
        } else if (data.input_type === 'doi') {
            // Extract DOI
            const match = input.match(/(10\.\d{4,}\/[^\s]+)/);
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

// ============================================================================
// Base HTML Template
// ============================================================================

pub fn base_html(title: &str, content: &str, search_query: Option<&str>, logged_in: bool) -> String {
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
    <script>
    // Copy BibTeX to clipboard
    function copyBibtex(elementId) {{
        const pre = document.getElementById(elementId);
        const hint = document.getElementById(elementId + '-hint');
        if (!pre) return;

        const text = pre.textContent;
        navigator.clipboard.writeText(text).then(() => {{
            if (hint) {{
                hint.textContent = 'Copied!';
                setTimeout(() => {{
                    hint.textContent = 'Click to copy';
                }}, 2000);
            }}
        }}).catch(err => {{
            console.error('Failed to copy:', err);
            if (hint) {{
                hint.textContent = 'Copy failed';
                setTimeout(() => {{
                    hint.textContent = 'Click to copy';
                }}, 2000);
            }}
        }});
    }}

    // Confirm and delete note
    async function confirmDelete(key, title) {{
        const confirmed = confirm('Delete "' + title + '"?\n\nThis will remove the note file and create a git commit. You can recover it from git history if needed.');
        if (!confirmed) return;

        try {{
            const response = await fetch('/api/note/' + key, {{
                method: 'DELETE',
                headers: {{ 'Content-Type': 'application/json' }},
                body: JSON.stringify({{ confirm: true }})
            }});

            if (response.ok) {{
                window.location.href = '/';
            }} else {{
                const err = await response.text();
                alert('Failed to delete: ' + err);
            }}
        }} catch (e) {{
            alert('Error deleting note: ' + e.message);
        }}
    }}
    </script>
</body>
</html>"#,
        title = html_escape(title),
        nav = nav_bar(search_query, logged_in),
        fab = fab_html,
    )
}

// ============================================================================
// Editor Template
// ============================================================================

#[derive(Serialize)]
struct NoteSuggestion {
    key: String,
    title: String,
}

pub fn render_editor(note: &Note, notes_map: &HashMap<String, Note>, _logged_in: bool) -> String {
    // Use serde_json for proper escaping
    let content_json = serde_json::to_string(&note.full_file_content)
        .unwrap_or_else(|_| "\"\"".to_string());

    // PDF handling
    let pdf_filename = note.pdf.as_deref().unwrap_or("");
    let pdf_filename_json = serde_json::to_string(pdf_filename)
        .unwrap_or_else(|_| "\"\"".to_string());

    let pdf_status_html = if let Some(ref pdf) = note.pdf {
        format!(
            r#"<a href="/pdfs/{}" target="_blank" class="pdf-link" title="Open PDF in new tab">📄 {}</a>
               <button class="pdf-toggle-btn" id="pdf-toggle-btn" onclick="togglePdfViewer()" title="Toggle PDF viewer">View</button>
               <button class="pdf-toggle-btn" onclick="addPageNote()" title="Add note for current PDF page">+ Page Note</button>"#,
            html_escape(pdf),
            html_escape(pdf)
        )
    } else {
        r#"<button class="pdf-toggle-btn" onclick="openPdfUpload()">Upload PDF</button>"#.to_string()
    };

    // Build note suggestions for autocomplete using serde_json
    let suggestions: Vec<NoteSuggestion> = notes_map
        .iter()
        .map(|(key, n)| NoteSuggestion {
            key: key.clone(),
            title: n.title.clone(),
        })
        .collect();
    let notes_json = serde_json::to_string(&suggestions)
        .unwrap_or_else(|_| "[]".to_string());

    format!(
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
            background: #6c71c4;
            color: #fdf6e3;
            border-radius: 3px;
            font-family: monospace;
            letter-spacing: 0.05em;
        }}

        /* Git Mode Toggle */
        .git-mode-toggle {{
            display: flex;
            align-items: center;
            gap: 0.5rem;
            font-size: 0.8rem;
            color: #93a1a1;
        }}
        .toggle-switch {{
            position: relative;
            display: inline-block;
            width: 36px;
            height: 20px;
        }}
        .toggle-switch input {{
            opacity: 0;
            width: 0;
            height: 0;
        }}
        .toggle-slider {{
            position: absolute;
            cursor: pointer;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            background-color: #93a1a1;
            transition: 0.3s;
            border-radius: 20px;
        }}
        .toggle-slider:before {{
            position: absolute;
            content: "";
            height: 14px;
            width: 14px;
            left: 3px;
            bottom: 3px;
            background-color: #fdf6e3;
            transition: 0.3s;
            border-radius: 50%;
        }}
        input:checked + .toggle-slider {{
            background-color: #859900;
        }}
        input:checked + .toggle-slider:before {{
            transform: translateX(16px);
        }}
        #git-mode-label {{
            min-width: 100px;
        }}

        /* Font Size Controls - Stylized A icons */
        .font-size-controls {{
            display: flex;
            align-items: baseline;
            gap: 0.15rem;
            font-family: Georgia, 'Times New Roman', serif;
            color: #93a1a1;
        }}
        .font-size-controls label {{
            cursor: pointer;
            padding: 0.2rem 0.35rem;
            border-radius: 3px;
            transition: all 0.15s ease;
            line-height: 1;
        }}
        .font-size-controls label:hover {{
            color: #657b83;
        }}
        .font-size-controls input[type="radio"] {{
            display: none;
        }}
        .font-size-controls input[type="radio"]:checked + span {{
            color: #268bd2;
        }}
        .font-size-controls .size-tiny {{ font-size: 0.7rem; }}
        .font-size-controls .size-small {{ font-size: 0.85rem; }}
        .font-size-controls .size-normal {{ font-size: 1rem; font-weight: 500; }}
        .font-size-controls .size-large {{ font-size: 1.2rem; font-weight: 500; }}

        .editor-status-dot {{
            width: 8px;
            height: 8px;
            border-radius: 50%;
            background: currentColor;
            flex-shrink: 0;
        }}

        /* Split Layout */
        .editor-main {{
            position: absolute;
            top: 48px;
            left: 0;
            right: 0;
            bottom: 0;
        }}

        #monaco-editor {{
            position: absolute;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            transition: right 0.2s ease;
        }}

        #monaco-editor.with-pdf {{
            right: 50%;
        }}

        /* PDF Viewer - Simple iframe, no toolbar */
        #pdf-viewer-pane {{
            position: absolute;
            top: 0;
            right: 0;
            width: 50%;
            bottom: 0;
            display: none;
            border-left: 1px solid #93a1a1;
            background: #586e75;
        }}
        #pdf-viewer-pane.active {{
            display: block;
        }}

        #pdf-iframe {{
            width: 100%;
            height: 100%;
            border: none;
            background: white;
        }}

        /* PDF Status in header */
        .pdf-status {{
            font-size: 0.75rem;
            color: #93a1a1;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}
        .pdf-status .pdf-link {{
            color: #268bd2;
            text-decoration: none;
            font-size: 0.8rem;
            padding: 0.2rem 0.4rem;
            border-radius: 3px;
            transition: background 0.15s;
        }}
        .pdf-status .pdf-link:hover {{
            background: #eee8d5;
            text-decoration: none;
        }}
        .pdf-status .pdf-toggle-btn {{
            padding: 0.2rem 0.5rem;
            border: 1px solid #93a1a1;
            border-radius: 3px;
            background: transparent;
            color: #93a1a1;
            cursor: pointer;
            font-size: 0.75rem;
        }}
        .pdf-status .pdf-toggle-btn:hover {{
            background: #eee8d5;
        }}
        .pdf-status .pdf-toggle-btn.active {{
            background: #268bd2;
            color: #fdf6e3;
            border-color: #268bd2;
        }}

        /* PDF Upload Modal */
        .pdf-upload-overlay {{
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            background: rgba(0,0,0,0.6);
            z-index: 2000;
            display: none;
            align-items: center;
            justify-content: center;
        }}
        .pdf-upload-overlay.active {{
            display: flex;
        }}
        .pdf-upload-modal {{
            background: #fdf6e3;
            border-radius: 8px;
            width: 90%;
            max-width: 500px;
            box-shadow: 0 8px 32px rgba(0,0,0,0.4);
        }}
        .pdf-upload-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 1rem 1.5rem;
            border-bottom: 1px solid #eee8d5;
        }}
        .pdf-upload-header h3 {{
            margin: 0;
            font-size: 1.1rem;
            color: #657b83;
        }}
        .pdf-upload-close {{
            background: none;
            border: none;
            font-size: 1.5rem;
            cursor: pointer;
            color: #93a1a1;
            padding: 0;
            line-height: 1;
        }}
        .pdf-upload-close:hover {{
            color: #657b83;
        }}
        .pdf-upload-body {{
            padding: 1.5rem;
        }}
        .pdf-dropzone {{
            border: 2px dashed #93a1a1;
            border-radius: 8px;
            padding: 2rem;
            text-align: center;
            cursor: pointer;
            transition: border-color 0.2s, background 0.2s;
            margin-bottom: 1rem;
        }}
        .pdf-dropzone:hover,
        .pdf-dropzone.dragover {{
            border-color: #268bd2;
            background: #eee8d5;
        }}
        .pdf-dropzone-icon {{
            font-size: 2rem;
            margin-bottom: 0.5rem;
        }}
        .pdf-dropzone-text {{
            font-size: 0.9rem;
            color: #657b83;
        }}
        .pdf-dropzone-hint {{
            font-size: 0.8rem;
            color: #93a1a1;
            margin-top: 0.5rem;
        }}
        .pdf-upload-divider {{
            display: flex;
            align-items: center;
            gap: 1rem;
            margin: 1rem 0;
            color: #93a1a1;
            font-size: 0.8rem;
        }}
        .pdf-upload-divider::before,
        .pdf-upload-divider::after {{
            content: '';
            flex: 1;
            border-top: 1px solid #eee8d5;
        }}
        .pdf-url-input {{
            display: flex;
            gap: 0.5rem;
        }}
        .pdf-url-input input {{
            flex: 1;
            padding: 0.6rem 0.8rem;
            border: 1px solid #93a1a1;
            border-radius: 4px;
            background: #fdf6e3;
            color: #657b83;
            font-size: 0.9rem;
        }}
        .pdf-url-input input::placeholder {{
            color: #93a1a1;
        }}
        .pdf-upload-status {{
            margin-top: 1rem;
            padding: 0.75rem;
            border-radius: 4px;
            font-size: 0.85rem;
            display: none;
        }}
        .pdf-upload-status.active {{
            display: block;
        }}
        .pdf-upload-status.loading {{
            background: #eee8d5;
            color: #657b83;
        }}
        .pdf-upload-status.success {{
            background: #d5e8d5;
            color: #2d6a2d;
        }}
        .pdf-upload-status.error {{
            background: #f5d5d5;
            color: #a02020;
        }}
    </style>
</head>
<body>
    <div class="editor-container">
        <div class="editor-header">
            <h1>{title}</h1>
            <span class="emacs-badge" id="emacs-badge" style="display:none;">EMACS</span>
            <div class="font-size-controls" title="Font size">
                <label><input type="radio" name="font-size" value="11" onchange="setFontSize(11)"><span class="size-tiny">A</span></label>
                <label><input type="radio" name="font-size" value="13" onchange="setFontSize(13)"><span class="size-small">A</span></label>
                <label><input type="radio" name="font-size" value="15" onchange="setFontSize(15)"><span class="size-normal">A</span></label>
                <label><input type="radio" name="font-size" value="18" onchange="setFontSize(18)"><span class="size-large">A</span></label>
            </div>
            <div class="git-mode-toggle">
                <label class="toggle-switch">
                    <input type="checkbox" id="commit-on-save" onchange="toggleGitMode()">
                    <span class="toggle-slider"></span>
                </label>
                <span id="git-mode-label">Commit on type</span>
            </div>
            <div class="editor-status" id="editor-status">
                <span class="editor-status-dot"></span>
                <span id="status-text">Ready</span>
            </div>
            <button class="btn primary" onclick="saveNote(false)">Save</button>
            <div class="pdf-status" id="pdf-status">{pdf_status_html}</div>
            <a href="/note/{key}" class="btn">Done</a>
        </div>
        <div class="editor-main">
            <div id="monaco-editor"></div>
            <div id="pdf-viewer-pane">
                <iframe id="pdf-iframe" src=""></iframe>
            </div>
        </div>

        <!-- PDF Upload Modal -->
        <div class="pdf-upload-overlay" id="pdf-upload-overlay" onclick="if(event.target===this)closePdfUpload()">
            <div class="pdf-upload-modal">
                <div class="pdf-upload-header">
                    <h3>Upload PDF</h3>
                    <button class="pdf-upload-close" onclick="closePdfUpload()">&times;</button>
                </div>
                <div class="pdf-upload-body">
                    <div class="pdf-dropzone" id="pdf-dropzone" onclick="document.getElementById('pdf-file-input').click()">
                        <div class="pdf-dropzone-icon">📄</div>
                        <div class="pdf-dropzone-text">Click or drag PDF here</div>
                        <div class="pdf-dropzone-hint">PDF files only, max 50MB</div>
                        <input type="file" id="pdf-file-input" accept=".pdf" style="display:none" onchange="handlePdfFileSelect(event)">
                    </div>
                    <div class="pdf-upload-divider">or enter URL</div>
                    <div class="pdf-url-input">
                        <input type="text" id="pdf-url-input" placeholder="https://arxiv.org/pdf/...">
                        <button class="btn" onclick="downloadPdfFromUrl()">Download</button>
                    </div>
                    <div class="pdf-upload-status" id="pdf-upload-status"></div>
                </div>
            </div>
        </div>
    </div>

    <script src="https://cdnjs.cloudflare.com/ajax/libs/monaco-editor/0.45.0/min/vs/loader.min.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/monaco-emacs@0.3.0/dist/monaco-emacs.min.js"></script>
    <script>
        let editor;
        let emacsMode;
        let lastSavedContent = {content_json};
        let autoSaveTimer = null;
        let hasUnsavedChanges = false;
        const noteKey = "{key}";
        const AUTO_SAVE_DELAY = 90000; // 90 seconds

        // Git mode: 'type' = commit on auto-save, 'save' = only commit on explicit save
        let gitMode = localStorage.getItem('gitMode') || 'type';

        // PDF filename
        let pdfFilename = {pdf_filename_json};

        // Notes for autocomplete
        const allNotes = {notes_json};

        // Font size from localStorage
        let currentFontSize = parseInt(localStorage.getItem('editorFontSize')) || 15;

        function setFontSize(size) {{
            currentFontSize = size;
            localStorage.setItem('editorFontSize', size);
            if (editor) {{
                editor.updateOptions({{ fontSize: size }});
            }}
        }}

        function initFontSizeControls() {{
            const radios = document.querySelectorAll('input[name="font-size"]');
            radios.forEach(radio => {{
                if (parseInt(radio.value) === currentFontSize) {{
                    radio.checked = true;
                }}
            }});
        }}

        // =====================================================================
        // PDF Viewer Functions (Simple iframe-based)
        // =====================================================================

        function showPdfViewer() {{
            if (!pdfFilename) return;
            const pane = document.getElementById('pdf-viewer-pane');
            const monacoDiv = document.getElementById('monaco-editor');
            const iframe = document.getElementById('pdf-iframe');
            iframe.src = '/pdfs/' + encodeURIComponent(pdfFilename);
            pane.classList.add('active');
            if (monacoDiv) monacoDiv.classList.add('with-pdf');
            const btn = document.getElementById('pdf-toggle-btn');
            if (btn) btn.classList.add('active');
            sessionStorage.setItem('pdf-visible-' + noteKey, 'true');
            // Trigger Monaco layout update after transition
            setTimeout(function() {{ if (editor) editor.layout(); }}, 250);
        }}

        function hidePdfViewer() {{
            const pane = document.getElementById('pdf-viewer-pane');
            const monacoDiv = document.getElementById('monaco-editor');
            const iframe = document.getElementById('pdf-iframe');
            iframe.src = '';
            pane.classList.remove('active');
            if (monacoDiv) monacoDiv.classList.remove('with-pdf');
            const btn = document.getElementById('pdf-toggle-btn');
            if (btn) btn.classList.remove('active');
            sessionStorage.setItem('pdf-visible-' + noteKey, 'false');
            // Trigger Monaco layout update after transition
            setTimeout(function() {{ if (editor) editor.layout(); }}, 250);
        }}

        function togglePdfViewer() {{
            const pane = document.getElementById('pdf-viewer-pane');
            if (pane.classList.contains('active')) {{
                hidePdfViewer();
            }} else {{
                showPdfViewer();
            }}
        }}

        function openPdfInNewTab() {{
            if (pdfFilename) {{
                window.open('/pdfs/' + encodeURIComponent(pdfFilename), '_blank');
            }}
        }}

        // =====================================================================
        // Page-Based Annotation
        // =====================================================================

        function getCurrentPdfPage() {{
            try {{
                const iframe = document.getElementById('pdf-iframe');
                if (!iframe || !iframe.contentWindow) return null;
                const hash = iframe.contentWindow.location.hash;
                const match = hash.match(/page=(\d+)/);
                return match ? parseInt(match[1], 10) : 1;
            }} catch (e) {{
                // Cross-origin or other error
                return null;
            }}
        }}

        function addPageNote() {{
            const page = getCurrentPdfPage();
            if (page === null) {{
                alert('Could not detect PDF page. Make sure the PDF viewer is open.');
                return;
            }}

            const content = editor.getValue();
            const h2 = String.fromCharCode(35, 35);
            const h3 = String.fromCharCode(35, 35, 35);
            const nl = String.fromCharCode(10);

            // Look for existing page section
            const pageHeading = h3 + ' Page ' + page;
            const pageIdx = content.indexOf(pageHeading);

            if (pageIdx !== -1) {{
                // Find end of this page section (next h3 or h2, or end)
                const afterPage = content.substring(pageIdx + pageHeading.length);
                const nextH3 = afterPage.indexOf(nl + h3 + ' ');
                const nextH2 = afterPage.indexOf(nl + h2 + ' ');

                let insertAt;
                if (nextH3 !== -1 && (nextH2 === -1 || nextH3 < nextH2)) {{
                    insertAt = pageIdx + pageHeading.length + nextH3;
                }} else if (nextH2 !== -1) {{
                    insertAt = pageIdx + pageHeading.length + nextH2;
                }} else {{
                    insertAt = content.length;
                }}

                // Insert a new bullet point
                const annotation = nl + '- ';
                insertAnnotation(insertAt, annotation);
            }} else {{
                // Need to create page section - find or create Paper Notes first
                const paperNotesHeading = h2 + ' Paper Notes';
                let paperIdx = content.indexOf(paperNotesHeading);

                if (paperIdx === -1) {{
                    // Create Paper Notes section at end
                    const newSection = nl + nl + paperNotesHeading + nl + nl + pageHeading + nl + nl + '- ';
                    insertAnnotation(content.length, newSection);
                }} else {{
                    // Find where to insert the new page section (keep pages sorted)
                    const afterPaperNotes = content.substring(paperIdx + paperNotesHeading.length);

                    // Find all existing page headings and their positions
                    let insertPos = paperIdx + paperNotesHeading.length;
                    let foundSpot = false;

                    // Look for next h2 section (end of Paper Notes)
                    const nextH2 = afterPaperNotes.indexOf(nl + h2 + ' ');
                    const searchEnd = nextH2 !== -1 ? nextH2 : afterPaperNotes.length;

                    // Find existing page sections
                    const pageRegex = new RegExp(h3.replace(/\x23/g, '\\\\x23') + ' Page (\\\\d+)', 'g');
                    let match;
                    const searchArea = afterPaperNotes.substring(0, searchEnd);

                    while ((match = pageRegex.exec(searchArea)) !== null) {{
                        const existingPage = parseInt(match[1], 10);
                        if (existingPage > page) {{
                            // Insert before this page
                            insertPos = paperIdx + paperNotesHeading.length + match.index;
                            foundSpot = true;
                            break;
                        }}
                    }}

                    if (!foundSpot) {{
                        // Insert at end of Paper Notes section
                        if (nextH2 !== -1) {{
                            insertPos = paperIdx + paperNotesHeading.length + nextH2;
                        }} else {{
                            insertPos = content.length;
                        }}
                    }}

                    const annotation = nl + nl + pageHeading + nl + nl + '- ';
                    insertAnnotation(insertPos, annotation);
                }}
            }}
        }}

        function insertAnnotation(position, text) {{
            const model = editor.getModel();
            const pos = model.getPositionAt(position);

            editor.executeEdits('annotation', [{{
                range: new monaco.Range(pos.lineNumber, pos.column, pos.lineNumber, pos.column),
                text: text,
                forceMoveMarkers: true
            }}]);

            // Move cursor to end of inserted annotation
            const newPos = model.getPositionAt(position + text.length);
            editor.setPosition(newPos);
            editor.focus();

            // Mark as unsaved
            hasUnsavedChanges = true;
            updateStatus('pending', 'Unsaved changes');
            scheduleAutoSave();
        }}

        // =====================================================================
        // PDF Upload Functions
        // =====================================================================

        function openPdfUpload() {{
            document.getElementById('pdf-upload-overlay').classList.add('active');
            document.getElementById('pdf-url-input').value = '';
            document.getElementById('pdf-upload-status').classList.remove('active');
        }}

        function closePdfUpload() {{
            document.getElementById('pdf-upload-overlay').classList.remove('active');
        }}

        // Drag and drop handling
        document.addEventListener('DOMContentLoaded', function() {{
            const dropzone = document.getElementById('pdf-dropzone');
            if (dropzone) {{
                dropzone.addEventListener('dragover', (e) => {{
                    e.preventDefault();
                    dropzone.classList.add('dragover');
                }});
                dropzone.addEventListener('dragleave', () => {{
                    dropzone.classList.remove('dragover');
                }});
                dropzone.addEventListener('drop', (e) => {{
                    e.preventDefault();
                    dropzone.classList.remove('dragover');
                    const files = e.dataTransfer.files;
                    if (files.length > 0) {{
                        uploadPdfFile(files[0]);
                    }}
                }});
            }}
        }});

        function handlePdfFileSelect(event) {{
            const files = event.target.files;
            if (files.length > 0) {{
                uploadPdfFile(files[0]);
            }}
        }}

        async function uploadPdfFile(file) {{
            if (!file.name.toLowerCase().endsWith('.pdf')) {{
                showUploadStatus('error', 'Please select a PDF file');
                return;
            }}

            if (file.size > 50 * 1024 * 1024) {{
                showUploadStatus('error', 'File too large (max 50MB)');
                return;
            }}

            showUploadStatus('loading', 'Uploading...');

            const formData = new FormData();
            formData.append('file', file);

            try {{
                const response = await fetch('/api/pdf/upload?note_key=' + noteKey, {{
                    method: 'POST',
                    body: formData
                }});

                const result = await response.json();

                if (result.success) {{
                    showUploadStatus('success', 'Uploaded: ' + result.filename);
                    pdfFilename = result.filename;

                    // Update the PDF status display
                    const pdfStatus = document.getElementById('pdf-status');
                    pdfStatus.innerHTML = `
                        <a href="/pdfs/${{encodeURIComponent(result.filename)}}" target="_blank">${{result.filename}}</a>
                        <button class="pdf-toggle-btn" id="pdf-toggle-btn" onclick="togglePdfViewer()">View</button>
                    `;

                    // Close modal and show PDF
                    setTimeout(() => {{
                        closePdfUpload();
                        showPdfViewer();
                    }}, 1000);
                }} else {{
                    showUploadStatus('error', result.error || 'Upload failed');
                }}
            }} catch (e) {{
                showUploadStatus('error', 'Upload failed: ' + e.message);
            }}
        }}

        async function downloadPdfFromUrl() {{
            const url = document.getElementById('pdf-url-input').value.trim();
            if (!url) {{
                showUploadStatus('error', 'Please enter a URL');
                return;
            }}

            showUploadStatus('loading', 'Downloading...');

            try {{
                const response = await fetch('/api/pdf/download-url', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ note_key: noteKey, url: url }})
                }});

                const result = await response.json();

                if (result.success) {{
                    showUploadStatus('success', 'Downloaded: ' + result.filename);
                    pdfFilename = result.filename;

                    // Update the PDF status display
                    const pdfStatus = document.getElementById('pdf-status');
                    pdfStatus.innerHTML = `
                        <a href="/pdfs/${{encodeURIComponent(result.filename)}}" target="_blank">${{result.filename}}</a>
                        <button class="pdf-toggle-btn" id="pdf-toggle-btn" onclick="togglePdfViewer()">View</button>
                    `;

                    // Close modal and show PDF
                    setTimeout(() => {{
                        closePdfUpload();
                        showPdfViewer();
                    }}, 1000);
                }} else {{
                    showUploadStatus('error', result.error || 'Download failed');
                }}
            }} catch (e) {{
                showUploadStatus('error', 'Download failed: ' + e.message);
            }}
        }}

        function showUploadStatus(type, message) {{
            const status = document.getElementById('pdf-upload-status');
            status.className = 'pdf-upload-status active ' + type;
            status.textContent = message;
        }}

        // Initialize controls on page load
        document.addEventListener('DOMContentLoaded', function() {{
            // Font size controls
            initFontSizeControls();

            // Git mode toggle
            const toggle = document.getElementById('commit-on-save');
            const label = document.getElementById('git-mode-label');
            if (gitMode === 'save') {{
                toggle.checked = true;
                label.textContent = 'Commit on save';
            }} else {{
                toggle.checked = false;
                label.textContent = 'Commit on type';
            }}

            // Restore PDF visibility from session storage
            if (pdfFilename && sessionStorage.getItem('pdf-visible-' + noteKey) === 'true') {{
                showPdfViewer();
            }}
        }});

        function toggleGitMode() {{
            const toggle = document.getElementById('commit-on-save');
            const label = document.getElementById('git-mode-label');
            if (toggle.checked) {{
                gitMode = 'save';
                label.textContent = 'Commit on save';
            }} else {{
                gitMode = 'type';
                label.textContent = 'Commit on type';
            }}
            localStorage.setItem('gitMode', gitMode);
        }}

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
                value: {content_json},
                language: 'markdown',
                theme: 'solarized-light',
                fontSize: currentFontSize,
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
                quickSuggestions: {{ other: true, comments: false, strings: true }},
                suggestOnTriggerCharacters: true,
            }});

            // Register note reference completion provider
            monaco.languages.registerCompletionItemProvider('markdown', {{
                triggerCharacters: ['@', '['],
                provideCompletionItems: function(model, position) {{
                    const textUntilPosition = model.getValueInRange({{
                        startLineNumber: position.lineNumber,
                        startColumn: 1,
                        endLineNumber: position.lineNumber,
                        endColumn: position.column
                    }});

                    // Check if we're in a [@...] context
                    const match = textUntilPosition.match(/\[@([^\]]*)$/);
                    if (!match) {{
                        return {{ suggestions: [] }};
                    }}

                    const prefix = match[1].toLowerCase();
                    const wordRange = {{
                        startLineNumber: position.lineNumber,
                        startColumn: position.column - match[1].length,
                        endLineNumber: position.lineNumber,
                        endColumn: position.column
                    }};

                    const suggestions = allNotes
                        .filter(note => {{
                            const titleLower = note.title.toLowerCase();
                            const keyLower = note.key.toLowerCase();
                            return titleLower.includes(prefix) || keyLower.includes(prefix);
                        }})
                        .map(note => ({{
                            label: note.title,
                            kind: monaco.languages.CompletionItemKind.Reference,
                            detail: '[@' + note.key + ']',
                            insertText: note.key + ']',
                            range: wordRange,
                            sortText: note.title.toLowerCase(),
                            filterText: note.title + ' ' + note.key
                        }}));

                    return {{ suggestions: suggestions }};
                }}
            }});

            // Register hover provider to show note titles for [@key] references
            monaco.languages.registerHoverProvider('markdown', {{
                provideHover: function(model, position) {{
                    const line = model.getLineContent(position.lineNumber);
                    // Find all [@...] references on this line
                    const regex = /\[@([^\]]+)\]/g;
                    let match;
                    while ((match = regex.exec(line)) !== null) {{
                        const startCol = match.index + 1;
                        const endCol = match.index + match[0].length + 1;
                        if (position.column >= startCol && position.column <= endCol) {{
                            const key = match[1];
                            const note = allNotes.find(n => n.key === key);
                            if (note) {{
                                return {{
                                    range: new monaco.Range(
                                        position.lineNumber, startCol,
                                        position.lineNumber, endCol
                                    ),
                                    contents: [
                                        {{ value: '**' + note.title + '**' }},
                                        {{ value: '_Click to open: [' + note.key + '](/note/' + note.key + ')_' }}
                                    ]
                                }};
                            }} else {{
                                return {{
                                    range: new monaco.Range(
                                        position.lineNumber, startCol,
                                        position.lineNumber, endCol
                                    ),
                                    contents: [{{ value: '_Unknown reference_' }}]
                                }};
                            }}
                        }}
                    }}
                    return null;
                }}
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
                    // Auto-save: commit only if gitMode is 'type'
                    saveNote(true, gitMode === 'type');
                }}
            }}, AUTO_SAVE_DELAY);
        }}

        function updateStatus(state, text) {{
            const statusEl = document.getElementById('editor-status');
            const textEl = document.getElementById('status-text');
            statusEl.className = 'editor-status ' + state;
            textEl.textContent = text;
        }}

        async function saveNote(isAutoSave, shouldCommit) {{
            if (!editor) return;

            // If called with only one arg (explicit save button), always commit
            if (shouldCommit === undefined) {{
                shouldCommit = true; // Explicit save always commits
            }}

            const currentContent = editor.getValue();
            if (currentContent === lastSavedContent) {{
                updateStatus('saved', 'No changes');
                return;
            }}

            const statusMsg = isAutoSave
                ? (shouldCommit ? 'Auto-saving + committing...' : 'Auto-saving...')
                : 'Saving + committing...';
            updateStatus('saving', statusMsg);

            try {{
                const response = await fetch('/api/note/' + noteKey, {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{
                        content: currentContent,
                        auto_commit: shouldCommit
                    }})
                }});

                if (response.ok) {{
                    lastSavedContent = currentContent;
                    hasUnsavedChanges = false;
                    const now = new Date();
                    const timeStr = now.toLocaleTimeString('en-US', {{ hour: 'numeric', minute: '2-digit' }});
                    const commitNote = shouldCommit ? ' (committed)' : '';
                    updateStatus('saved', 'Saved at ' + timeStr + commitNote);
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
        content_json = content_json,
        pdf_filename_json = pdf_filename_json,
        pdf_status_html = pdf_status_html,
        notes_json = notes_json,
    )
}

// ============================================================================
// Viewer Template (View mode with PDF support)
// ============================================================================

pub fn render_viewer(
    note: &Note,
    rendered_content: &str,
    meta_html: &str,
    time_html: &str,
    sub_notes_html: &str,
    history_html: &str,
    logged_in: bool,
) -> String {
    let pdf_filename = note.pdf.as_deref().unwrap_or("");
    let pdf_filename_json = serde_json::to_string(pdf_filename)
        .unwrap_or_else(|_| "\"\"".to_string());

    let pdf_status_html = if let Some(ref pdf) = note.pdf {
        format!(
            r#"<a href="/pdfs/{}" target="_blank">{}</a>
               <button class="pdf-toggle-btn" id="pdf-toggle-btn" onclick="togglePdfViewer()">View PDF</button>"#,
            html_escape(pdf),
            html_escape(pdf)
        )
    } else {
        String::new()
    };

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

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <style>
        :root {{
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
            --code-bg: var(--base2);
        }}

        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            line-height: 1.6;
            color: var(--fg);
            background: var(--bg);
            overflow: hidden;
        }}

        .viewer-container {{
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            display: flex;
            flex-direction: column;
        }}

        .viewer-header {{
            background: var(--accent);
            border-bottom: 1px solid var(--border);
            padding: 0.75rem 1rem;
            display: flex;
            align-items: center;
            gap: 1rem;
            flex-wrap: wrap;
        }}

        .viewer-header h1 {{
            margin: 0;
            font-size: 1.25rem;
            font-weight: 600;
            color: var(--fg);
            flex: 1;
        }}

        .viewer-header a, .viewer-header button {{
            font-size: 0.9rem;
        }}

        .mode-toggle {{
            display: flex;
            gap: 0;
            border: 1px solid var(--border);
            border-radius: 4px;
            overflow: hidden;
        }}
        .mode-toggle button {{
            padding: 0.4rem 1rem;
            border: none;
            background: var(--accent);
            color: var(--fg);
            cursor: pointer;
            font-size: 0.85rem;
            font-family: inherit;
        }}
        .mode-toggle button.active {{
            background: var(--link);
            color: white;
        }}
        .mode-toggle button:hover:not(.active) {{
            background: var(--border);
        }}
        .delete-btn {{
            background: var(--red) !important;
            color: white !important;
        }}
        .delete-btn:hover {{
            background: #b02020 !important;
        }}

        .pdf-status {{
            font-size: 0.8rem;
            color: var(--muted);
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}
        .pdf-status a {{
            color: var(--link);
            text-decoration: none;
        }}
        .pdf-status a:hover {{
            text-decoration: underline;
        }}
        .pdf-toggle-btn {{
            padding: 0.3rem 0.6rem;
            border: 1px solid var(--border);
            border-radius: 4px;
            background: var(--bg);
            color: var(--fg);
            cursor: pointer;
            font-size: 0.8rem;
        }}
        .pdf-toggle-btn:hover {{
            background: var(--accent);
        }}
        .pdf-toggle-btn.active {{
            background: var(--link);
            color: white;
            border-color: var(--link);
        }}

        .back-link {{
            color: var(--link);
            text-decoration: none;
            font-size: 0.9rem;
        }}
        .back-link:hover {{
            text-decoration: underline;
        }}

        .viewer-main {{
            flex: 1;
            display: flex;
            overflow: hidden;
        }}

        .content-pane {{
            flex: 1;
            overflow-y: auto;
            padding: 2rem;
            min-width: 0;
        }}

        .content-wrapper {{
            max-width: 900px;
            margin: 0 auto;
        }}

        /* PDF Viewer Pane - no toolbar, just iframe */
        #pdf-viewer-pane {{
            width: 50%;
            display: none;
            border-left: 1px solid var(--border);
            background: #586e75;
        }}
        #pdf-viewer-pane.active {{
            display: block;
        }}

        #pdf-iframe {{
            width: 100%;
            height: 100%;
            border: none;
            background: white;
        }}

        /* Content Styling */
        a {{ color: var(--link); text-decoration: none; }}
        a:hover {{ color: var(--link-hover); text-decoration: underline; }}

        h1, h2, h3 {{ font-weight: 600; margin-top: 1.5em; margin-bottom: 0.5em; }}
        h2 {{ font-size: 1.3rem; }}
        h3 {{ font-size: 1.1rem; }}

        .meta-block {{
            background: var(--accent);
            padding: 0.5rem 0.75rem;
            margin-bottom: 1rem;
            border-radius: 4px;
            font-size: 0.8rem;
            line-height: 1.4;
        }}
        .meta-block .meta-row {{
            display: flex;
            gap: 0.5rem;
        }}
        .meta-block .meta-label {{
            font-weight: 600;
            color: var(--base01);
            min-width: 60px;
        }}
        .meta-block .meta-value {{
            color: var(--fg);
        }}
        .meta-block code {{
            font-size: 0.75rem;
            background: var(--bg);
            padding: 0.1rem 0.3rem;
            border-radius: 2px;
        }}

        .bibtex-block {{
            background: var(--code-bg);
            border: 1px solid var(--border);
            border-radius: 4px;
            margin: 1rem 0;
            cursor: pointer;
            transition: border-color 0.2s;
        }}
        .bibtex-block:hover {{
            border-color: var(--link);
        }}
        .bibtex-header {{
            display: flex;
            justify-content: space-between;
            padding: 0.5rem 1rem;
            font-size: 0.8rem;
            font-weight: 600;
            color: var(--muted);
        }}
        .bibtex-copy-hint {{
            font-weight: normal;
            font-style: italic;
        }}
        .bibtex-block pre {{
            margin: 0;
            padding: 0 1rem;
            font-family: "SF Mono", "Consolas", "Liberation Mono", monospace;
            font-size: 0.8rem;
            white-space: pre-wrap;
            word-wrap: break-word;
            background: transparent;
            border-radius: 0;
            max-height: 0;
            overflow: hidden;
            transition: max-height 0.3s ease, padding 0.3s ease;
        }}
        .bibtex-block:hover pre {{
            max-height: 500px;
            padding: 1rem;
            border-top: 1px solid var(--border);
        }}

        .note-content pre {{
            background: var(--accent);
            padding: 1rem;
            overflow-x: auto;
            border-radius: 4px;
            margin: 1rem 0;
        }}
        .note-content code {{
            font-family: "SF Mono", "Consolas", "Liberation Mono", monospace;
            font-size: 0.9em;
        }}
        .note-content p code {{
            background: var(--accent);
            padding: 0.1rem 0.3rem;
            border-radius: 3px;
        }}
        .note-content blockquote {{
            border-left: 3px solid var(--border);
            margin: 1rem 0;
            padding-left: 1rem;
            color: var(--muted);
        }}
        .note-content ul, .note-content ol {{
            margin: 1rem 0;
            padding-left: 1.5rem;
        }}
        .note-content p {{ margin: 1rem 0; }}

        .crosslink {{
            background: var(--accent);
            padding: 0.1rem 0.3rem;
            border-radius: 3px;
            font-size: 0.9em;
        }}

        .time-table {{ width: 100%; border-collapse: collapse; font-size: 0.85rem; margin-top: 1rem; }}
        .time-table th, .time-table td {{ padding: 0.5rem; text-align: left; border-bottom: 1px solid var(--border); }}
        .time-table th {{ font-weight: 600; }}

        .history-list {{ font-size: 0.85rem; }}
        .history-item {{ padding: 0.5rem 0; border-bottom: 1px solid var(--border); }}
        .history-item:last-child {{ border-bottom: none; }}
        .history-hash {{ font-family: monospace; color: var(--muted); }}

        .sub-notes {{ margin-top: 1rem; padding-top: 1rem; border-top: 1px solid var(--border); }}
        .sub-notes h3 {{ font-size: 1rem; margin-top: 0; }}
    </style>
</head>
<body>
    <div class="viewer-container">
        <div class="viewer-header">
            <a href="/" class="back-link">&larr; All Notes</a>
            <h1>{title}</h1>
            <div class="pdf-status" id="pdf-status">{pdf_status_html}</div>
            {mode_toggle}
        </div>
        <div class="viewer-main">
            <div class="content-pane">
                <div class="content-wrapper">
                    {meta_html}
                    <div class="note-content">{rendered_content}</div>
                    {time_html}
                    {sub_notes_html}
                    {history_html}
                </div>
            </div>
            <div id="pdf-viewer-pane">
                <iframe id="pdf-iframe" src=""></iframe>
            </div>
        </div>
    </div>

    <script>
        const noteKey = "{key}";
        const pdfFilename = {pdf_filename_json};

        function showPdfViewer() {{
            if (!pdfFilename) return;
            const pane = document.getElementById('pdf-viewer-pane');
            const iframe = document.getElementById('pdf-iframe');
            iframe.src = '/pdfs/' + encodeURIComponent(pdfFilename);
            pane.classList.add('active');
            const btn = document.getElementById('pdf-toggle-btn');
            if (btn) btn.classList.add('active');
            sessionStorage.setItem('pdf-visible-' + noteKey, 'true');
        }}

        function hidePdfViewer() {{
            const pane = document.getElementById('pdf-viewer-pane');
            const iframe = document.getElementById('pdf-iframe');
            iframe.src = '';
            pane.classList.remove('active');
            const btn = document.getElementById('pdf-toggle-btn');
            if (btn) btn.classList.remove('active');
            sessionStorage.setItem('pdf-visible-' + noteKey, 'false');
        }}

        function togglePdfViewer() {{
            const pane = document.getElementById('pdf-viewer-pane');
            if (pane.classList.contains('active')) {{
                hidePdfViewer();
            }} else {{
                showPdfViewer();
            }}
        }}

        function openPdfInNewTab() {{
            if (pdfFilename) {{
                window.open('/pdfs/' + encodeURIComponent(pdfFilename), '_blank');
            }}
        }}

        // Copy BibTeX to clipboard
        function copyBibtex(elementId) {{
            const pre = document.getElementById(elementId);
            const hint = document.getElementById(elementId + '-hint');
            if (!pre) return;

            const text = pre.textContent;
            navigator.clipboard.writeText(text).then(() => {{
                if (hint) {{
                    hint.textContent = 'Copied!';
                    setTimeout(() => {{
                        hint.textContent = 'Click to copy';
                    }}, 2000);
                }}
            }}).catch(err => {{
                console.error('Failed to copy:', err);
            }});
        }}

        // Confirm and delete note
        async function confirmDelete(key, title) {{
            const confirmed = confirm('Delete "' + title + '"?\\n\\nThis will remove the note file and create a git commit.');
            if (!confirmed) return;

            try {{
                const response = await fetch('/api/note/' + key, {{
                    method: 'DELETE',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ confirm: true }})
                }});

                if (response.ok) {{
                    window.location.href = '/';
                }} else {{
                    const err = await response.text();
                    alert('Failed to delete: ' + err);
                }}
            }} catch (e) {{
                alert('Error deleting note: ' + e.message);
            }}
        }}

        // Restore PDF visibility from session storage
        document.addEventListener('DOMContentLoaded', function() {{
            if (pdfFilename && sessionStorage.getItem('pdf-visible-' + noteKey) === 'true') {{
                showPdfViewer();
            }}
        }});
    </script>
</body>
</html>"##,
        title = html_escape(&note.title),
        key = note.key,
        pdf_filename_json = pdf_filename_json,
        pdf_status_html = pdf_status_html,
        mode_toggle = mode_toggle,
        meta_html = meta_html,
        rendered_content = rendered_content,
        time_html = time_html,
        sub_notes_html = sub_notes_html,
        history_html = history_html,
    )
}
