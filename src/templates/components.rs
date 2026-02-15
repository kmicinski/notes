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

            <!-- Tabs -->
            <div class="smart-tabs">
                <button class="smart-tab active" onclick="switchTab('paper')" id="tab-paper">Add Paper</button>
                <button class="smart-tab" onclick="switchTab('note')" id="tab-note">New Note</button>
            </div>

            <!-- Paper Tab -->
            <div class="smart-modal-body" id="panel-paper">
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

                    <div class="smart-input-group">
                        <label for="smart-bibtex">BibTeX <span class="required">*</span></label>
                        <textarea id="smart-bibtex" rows="8" placeholder="@article{authorYYYYkeyword,
  title = {Paper Title},
  author = {First Author and Second Author},
  year = {2024},
  journal = {Journal Name},
}"></textarea>
                    </div>

                    <div id="bibtex-validation" class="bibtex-validation"></div>

                    <div id="bibtex-preview" class="bibtex-preview"></div>

                    <div class="smart-form-row">
                        <div class="smart-input-group">
                            <label for="smart-filename">Filename</label>
                            <input type="text" id="smart-filename" placeholder="authorYYYYkeyword.md">
                            <small>Auto-generated from bib key; editable</small>
                        </div>
                    </div>

                    <div class="smart-result-actions">
                        <button class="btn" onclick="createFromSmartAdd()" id="btn-create-paper">Create Note</button>
                        <button class="btn secondary" onclick="closeSmartAdd()">Cancel</button>
                    </div>
                </div>
            </div>

            <!-- Note Tab -->
            <div class="smart-modal-body" id="panel-note" style="display:none">
                <div class="smart-input-group">
                    <label for="note-title">Title <span class="required">*</span></label>
                    <input type="text" id="note-title" placeholder="My new note">
                </div>
                <div class="smart-form-row">
                    <div class="smart-input-group">
                        <label for="note-date">Date</label>
                        <input type="date" id="note-date">
                        <small>Defaults to today</small>
                    </div>
                    <div class="smart-input-group">
                        <label for="note-subdir">Subdirectory</label>
                        <input type="text" id="note-subdir" placeholder="projects/">
                        <small>Optional subfolder</small>
                    </div>
                </div>
                <div class="smart-result-actions">
                    <button class="btn" onclick="createQuickNote()">Create Note</button>
                    <button class="btn secondary" onclick="closeSmartAdd()">Cancel</button>
                </div>
            </div>
        </div>
    </div>

    <script>
    let smartDebounceTimer = null;

    function switchTab(tab) {
        document.getElementById('tab-paper').classList.toggle('active', tab === 'paper');
        document.getElementById('tab-note').classList.toggle('active', tab === 'note');
        document.getElementById('panel-paper').style.display = tab === 'paper' ? '' : 'none';
        document.getElementById('panel-note').style.display = tab === 'note' ? '' : 'none';
        if (tab === 'note') document.getElementById('note-title').focus();
        if (tab === 'paper') document.getElementById('smart-input').focus();
    }

    function openSmartAdd() {
        document.getElementById('smart-modal-overlay').classList.add('active');
        switchTab('paper');
        document.getElementById('smart-input').value = '';
        document.getElementById('smart-bibtex').value = '';
        document.getElementById('smart-filename').value = '';
        document.getElementById('bibtex-validation').innerHTML = '';
        document.getElementById('bibtex-preview').innerHTML = '';
        document.getElementById('smart-result').classList.remove('active');
        document.getElementById('smart-form').classList.remove('active');
        document.getElementById('smart-input').focus();
        document.getElementById('note-title').value = '';
        document.getElementById('note-date').value = '';
        document.getElementById('note-subdir').value = '';
        window.detectedArxivId = null;
        window.detectedDoi = null;
    }

    function closeSmartAdd() {
        document.getElementById('smart-modal-overlay').classList.remove('active');
        document.getElementById('smart-loading').classList.remove('active');
    }

    document.getElementById('smart-input').addEventListener('input', function() {
        clearTimeout(smartDebounceTimer);
        smartDebounceTimer = setTimeout(performSmartLookup, 800);
    });

    // ---- Client-side BibTeX parser ----
    function parseBibtex(text) {
        text = text.trim();
        if (!text) return null;
        const entryMatch = text.match(/@(\w+)\s*\{\s*([^,\s]+)/);
        if (!entryMatch) return null;
        const entryType = entryMatch[1].toLowerCase();
        const citeKey = entryMatch[2];

        function extractField(name) {
            const re = new RegExp(name + '\\s*=\\s*(?:\\{([^}]*)\\}|"([^"]*)"|([0-9]+))', 'i');
            const m = text.match(re);
            if (!m) return null;
            return (m[1] || m[2] || m[3] || '').trim() || null;
        }

        return {
            entryType,
            citeKey,
            title: extractField('title'),
            author: extractField('author'),
            year: extractField('year'),
            venue: extractField('journal') || extractField('booktitle') || extractField('howpublished')
        };
    }

    document.getElementById('smart-bibtex').addEventListener('input', function() {
        const parsed = parseBibtex(this.value);
        const valEl = document.getElementById('bibtex-validation');
        const prevEl = document.getElementById('bibtex-preview');

        if (!this.value.trim()) {
            valEl.innerHTML = '';
            prevEl.innerHTML = '';
            document.getElementById('smart-filename').value = '';
            return;
        }

        if (!parsed) {
            valEl.innerHTML = '<span class="bib-invalid">Invalid BibTeX</span>';
            prevEl.innerHTML = '';
            document.getElementById('smart-filename').value = '';
            return;
        }

        valEl.innerHTML = '<span class="bib-valid">Valid BibTeX: <code>' + escapeHtml(parsed.citeKey) + '</code></span>';

        let preview = '<div class="bib-preview-fields">';
        if (parsed.title) preview += '<div class="bib-field"><span class="bib-label">Title</span> ' + escapeHtml(parsed.title) + '</div>';
        if (parsed.author) preview += '<div class="bib-field"><span class="bib-label">Authors</span> ' + escapeHtml(parsed.author) + '</div>';
        if (parsed.year) preview += '<div class="bib-field"><span class="bib-label">Year</span> ' + escapeHtml(parsed.year) + '</div>';
        if (parsed.venue) preview += '<div class="bib-field"><span class="bib-label">Venue</span> ' + escapeHtml(parsed.venue) + '</div>';
        preview += '</div>';
        prevEl.innerHTML = preview;

        // Auto-fill filename from cite key
        document.getElementById('smart-filename').value = parsed.citeKey + '.md';
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

            if (!response.ok) {
                loading.classList.remove('active');
                showManualEntryOption(result, 'Server error: ' + response.status);
                return;
            }

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

                window.currentMatch = data.local_match;
                window.currentInputType = data.input_type;
                window.currentInput = document.getElementById('smart-input').value.trim();

                if (data.external_result) {
                    populateForm(data.external_result);
                }
                return;
            }

            if (data.external_result) {
                populateForm(data.external_result);

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

            result.innerHTML = `
                <h3>No paper metadata found</h3>
                <p>External APIs didn't return results. You can enter BibTeX manually below.</p>
                <div class="smart-result-actions">
                    <button class="btn" onclick="showFormForManualEntry()">Enter BibTeX Manually</button>
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
            <p>You can still enter BibTeX manually:</p>
            <div class="smart-result-actions">
                <button class="btn" onclick="showFormForManualEntry()">Enter BibTeX Manually</button>
            </div>
        `;
        result.className = 'smart-result active';
    }

    function getSourceIdentifier(data) {
        const input = document.getElementById('smart-input').value.trim();
        if (data.input_type === 'arxiv') {
            const match = input.match(/(\d{4}\.\d{4,5})/);
            return match ? match[1] : input;
        } else if (data.input_type === 'doi') {
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
        document.getElementById('smart-bibtex').focus();
    }

    function populateForm(ext) {
        // Populate BibTeX textarea — this is the primary input now
        document.getElementById('smart-bibtex').value = ext.bibtex || '';
        // Trigger parsing to update preview and filename
        document.getElementById('smart-bibtex').dispatchEvent(new Event('input'));
    }

    async function createFromSmartAdd() {
        const bibtex = document.getElementById('smart-bibtex').value.trim();
        const filename = document.getElementById('smart-filename').value.trim();

        if (!bibtex) {
            alert('BibTeX is required');
            return;
        }

        const parsed = parseBibtex(bibtex);
        if (!parsed) {
            alert('Could not parse BibTeX entry');
            return;
        }

        if (!filename) {
            alert('Filename is required');
            return;
        }

        const data = {
            bibtex: bibtex,
            filename: filename,
            arxiv_id: window.detectedArxivId || null,
            doi: window.detectedDoi || null
        };

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

    async function createQuickNote() {
        const title = document.getElementById('note-title').value.trim();
        if (!title) {
            alert('Title is required');
            return;
        }

        const data = {
            title: title,
            date: document.getElementById('note-date').value || null,
            subdirectory: document.getElementById('note-subdir').value || null
        };

        try {
            const response = await fetch('/api/smart-add/quick-note', {
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
