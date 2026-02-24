//! CSS styles for the notes application.
//!
//! Contains the main STYLE constant with all CSS for the web interface.

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
.note-item .key { font-family: "SF Mono", "Consolas", "Liberation Mono", monospace; font-size: 0.7rem; color: var(--muted); margin-left: 0.5rem; }
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
.history-hash { font-family: "SF Mono", "Consolas", "Liberation Mono", monospace; color: var(--muted); }

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
    font-family: "SF Mono", "Consolas", "Liberation Mono", monospace;
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
    font-family: "SF Mono", "Consolas", "Liberation Mono", monospace;
    font-size: 0.85rem;
    min-height: 100px;
    resize: vertical;
}

/* Smart Add Tabs */
.smart-tabs {
    display: flex;
    border-bottom: 1px solid var(--border);
    padding: 0 1.5rem;
}
.smart-tab {
    padding: 0.6rem 1.2rem;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    cursor: pointer;
    font-size: 0.9rem;
    font-family: inherit;
    color: var(--muted);
    transition: color 0.2s, border-color 0.2s;
}
.smart-tab:hover { color: var(--fg); }
.smart-tab.active {
    color: var(--link);
    border-bottom-color: var(--link);
    font-weight: 600;
}

/* BibTeX validation & preview */
.bibtex-validation {
    margin-bottom: 0.75rem;
    font-size: 0.85rem;
}
.bib-valid { color: var(--green); }
.bib-valid code {
    font-size: 0.8rem;
    background: var(--accent);
    padding: 0.1rem 0.3rem;
    border-radius: 2px;
}
.bib-invalid { color: var(--red); }

.bibtex-preview { margin-bottom: 0.75rem; }
.bib-preview-fields {
    background: var(--accent);
    border-radius: 4px;
    padding: 0.5rem 0.75rem;
    font-size: 0.85rem;
}
.bib-field {
    padding: 0.15rem 0;
}
.bib-label {
    font-weight: 600;
    color: var(--base01);
    margin-right: 0.5rem;
    display: inline-block;
    min-width: 55px;
}

.required { color: var(--red); }

/* BibTeX Import - Drop Overlay */
.bib-drop-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(38, 139, 210, 0.15);
    z-index: 2000;
    display: none;
    align-items: center;
    justify-content: center;
}
.bib-drop-overlay.active {
    display: flex;
}
.bib-drop-message {
    padding: 2rem 3rem;
    background: var(--bg);
    border: 3px dashed var(--link);
    border-radius: 12px;
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--link);
}

/* BibTeX Import - Badges */
.bib-badges {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    margin-bottom: 1rem;
}
.bib-badge {
    display: inline-block;
    padding: 0.2rem 0.6rem;
    border-radius: 12px;
    font-size: 0.8rem;
    font-weight: 600;
}
.bib-badge.new { background: #d4edda; color: #155724; }
.bib-badge.existing { background: var(--accent); color: var(--muted); }
.bib-badge.conflict { background: #fff3cd; color: #856404; }
.bib-badge.error { background: #f8d7da; color: #721c24; }

/* BibTeX Import - Items */
.bib-import-item {
    padding: 0.5rem 0.75rem;
    margin-bottom: 0.5rem;
    border-radius: 4px;
    background: var(--accent);
    border-left: 3px solid transparent;
    font-size: 0.9rem;
}
.bib-import-item.new { border-left-color: var(--green); }
.bib-import-item.conflict { border-left-color: var(--orange); }
.bib-import-item.existing { border-left-color: var(--muted); }
.bib-import-item.error-item { border-left-color: var(--red); }

.bib-import-item code {
    font-size: 0.8rem;
    background: var(--bg);
    padding: 0.1rem 0.3rem;
    border-radius: 2px;
}

.bib-filename-row {
    margin-top: 0.4rem;
    font-size: 0.85rem;
}
.bib-filename-row input {
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg);
    color: var(--fg);
    font-size: 0.8rem;
    font-family: "SF Mono", "Consolas", "Liberation Mono", monospace;
    width: 250px;
}

/* BibTeX Import - Conflict Actions */
.bib-conflict-actions {
    margin-top: 0.4rem;
    display: flex;
    gap: 0.5rem;
    align-items: center;
}
.bib-conflict-actions select {
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg);
    color: var(--fg);
    font-size: 0.8rem;
}
.bib-conflict-actions input {
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg);
    color: var(--fg);
    font-size: 0.8rem;
    font-family: "SF Mono", "Consolas", "Liberation Mono", monospace;
    width: 200px;
}

.bib-existing-section {
    margin: 0.75rem 0;
    font-size: 0.9rem;
}
.bib-existing-section summary {
    cursor: pointer;
    color: var(--muted);
    font-size: 0.85rem;
}

/* Hidden Notes */
.note-hide-btn {
    background: none;
    border: none;
    color: var(--muted);
    cursor: pointer;
    font-size: 0.7rem;
    font-family: inherit;
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
    opacity: 0;
    transition: opacity 0.15s;
    margin-right: 0.5rem;
    vertical-align: middle;
}
.note-item:hover .note-hide-btn {
    opacity: 1;
}
.note-hide-btn:hover {
    background: var(--accent);
    color: var(--fg);
}

.note-item.hidden-note {
    opacity: 0.45;
}
.note-item.hidden-note .title {
    text-decoration: line-through;
}

.hidden-toggle {
    font-size: 0.8rem;
    margin-bottom: 0.5rem;
}
.hidden-toggle a {
    color: var(--muted);
    background: var(--accent);
    padding: 0.2rem 0.6rem;
    border-radius: 10px;
    font-size: 0.75rem;
}
.hidden-toggle a:hover {
    color: var(--fg);
    text-decoration: none;
}
"#;
