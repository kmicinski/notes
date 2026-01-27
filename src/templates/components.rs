//! Shared HTML components for the notes application.
//!
//! Contains navigation bar, Smart Add modal, and base HTML template.

use crate::auth::is_auth_enabled;
use crate::notes::html_escape;

use super::styles::STYLE;

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
