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
                <button class="smart-tab" onclick="switchTab('bibimport')" id="tab-bibimport">Import .bib</button>
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

            <!-- Import .bib Tab -->
            <div class="smart-modal-body" id="panel-bibimport" style="display:none">
                <div class="smart-input-group">
                    <label for="bib-file-input">Select .bib file</label>
                    <input type="file" id="bib-file-input" accept=".bib" onchange="handleBibFile(this.files[0])">
                    <small>Or drag-and-drop a .bib file anywhere on the page</small>
                </div>
                <div class="smart-loading" id="bib-loading">
                    <div class="smart-spinner"></div>
                    <span>Analyzing entries...</span>
                </div>
                <div id="bib-review"></div>
            </div>
        </div>
    </div>

    <!-- Drag-drop overlay -->
    <div class="bib-drop-overlay" id="bib-drop-overlay">
        <div class="bib-drop-message">Drop .bib file to import</div>
    </div>

    <script>
    let smartDebounceTimer = null;

    function switchTab(tab) {
        document.getElementById('tab-paper').classList.toggle('active', tab === 'paper');
        document.getElementById('tab-note').classList.toggle('active', tab === 'note');
        document.getElementById('tab-bibimport').classList.toggle('active', tab === 'bibimport');
        document.getElementById('panel-paper').style.display = tab === 'paper' ? '' : 'none';
        document.getElementById('panel-note').style.display = tab === 'note' ? '' : 'none';
        document.getElementById('panel-bibimport').style.display = tab === 'bibimport' ? '' : 'none';
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
            // Find field = position
            const fieldRe = new RegExp(name + '\\s*=\\s*', 'i');
            const fm = fieldRe.exec(text);
            if (!fm) return null;
            const rest = text.slice(fm.index + fm[0].length);
            let val = null;
            if (rest[0] === '{') {
                // Brace-delimited: track depth
                let depth = 0, end = 0;
                for (let i = 0; i < rest.length; i++) {
                    if (rest[i] === '{') depth++;
                    else if (rest[i] === '}') { depth--; if (depth === 0) { end = i; break; } }
                }
                if (end > 1) val = rest.slice(1, end);
            } else if (rest[0] === '"') {
                const qi = rest.indexOf('"', 1);
                if (qi > 0) val = rest.slice(1, qi);
            } else {
                const ei = rest.search(/[,}\n]/);
                val = rest.slice(0, ei > 0 ? ei : undefined).trim();
            }
            if (val === null) return null;
            // Strip BibTeX protection braces
            val = val.replace(/[{}]/g, '').trim();
            return val || null;
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

    // =========================================================================
    // BibTeX Import
    // =========================================================================

    let bibDragCounter = 0;

    document.addEventListener('dragenter', function(e) {
        e.preventDefault();
        bibDragCounter++;
        if (bibDragCounter === 1) {
            const items = e.dataTransfer && e.dataTransfer.items;
            if (items && items.length > 0) {
                document.getElementById('bib-drop-overlay').classList.add('active');
            }
        }
    });

    document.addEventListener('dragleave', function(e) {
        e.preventDefault();
        bibDragCounter--;
        if (bibDragCounter === 0) {
            document.getElementById('bib-drop-overlay').classList.remove('active');
        }
    });

    document.addEventListener('dragover', function(e) {
        e.preventDefault();
    });

    document.addEventListener('drop', function(e) {
        e.preventDefault();
        bibDragCounter = 0;
        document.getElementById('bib-drop-overlay').classList.remove('active');

        const files = e.dataTransfer && e.dataTransfer.files;
        if (files && files.length > 0) {
            const file = files[0];
            if (file.name.endsWith('.bib')) {
                openSmartAdd();
                switchTab('bibimport');
                handleBibFile(file);
            }
        }
    });

    async function handleBibFile(file) {
        if (!file) return;
        const loading = document.getElementById('bib-loading');
        const review = document.getElementById('bib-review');

        loading.classList.add('active');
        review.innerHTML = '';

        const formData = new FormData();
        formData.append('file', file);

        try {
            const response = await fetch('/api/bib-import/analyze', {
                method: 'POST',
                body: formData
            });

            const data = await response.json();
            loading.classList.remove('active');
            renderBibReview(data);
        } catch (e) {
            loading.classList.remove('active');
            review.innerHTML = '<p class="message error">Failed to analyze file: ' + escapeHtml(e.message) + '</p>';
        }
    }

    let bibAnalysisData = null;

    function renderBibReview(data) {
        bibAnalysisData = data;
        const review = document.getElementById('bib-review');

        const nNew = data.new_entries.length;
        const nExisting = data.existing_entries.length;
        const nConflict = data.conflicts.length;
        const nErrors = data.parse_errors.length;

        let html = '<div class="bib-badges">';
        if (nNew > 0) html += '<span class="bib-badge new">' + nNew + ' new</span>';
        if (nExisting > 0) html += '<span class="bib-badge existing">' + nExisting + ' already exist</span>';
        if (nConflict > 0) html += '<span class="bib-badge conflict">' + nConflict + ' conflicts</span>';
        if (nErrors > 0) html += '<span class="bib-badge error">' + nErrors + ' parse errors</span>';
        html += '</div>';

        // New entries
        if (nNew > 0) {
            html += '<h3>New Entries</h3>';
            for (const entry of data.new_entries) {
                html += '<div class="bib-import-item new">';
                html += '<label><input type="checkbox" checked data-index="' + entry.index + '" class="bib-new-check"> ';
                html += '<code>' + escapeHtml(entry.cite_key) + '</code></label>';
                if (entry.title) html += ' &mdash; ' + escapeHtml(entry.title);
                if (entry.author) html += '<br><small>' + escapeHtml(entry.author) + (entry.year ? ' (' + entry.year + ')' : '') + '</small>';
                html += '<div class="bib-filename-row"><label>Filename: <input type="text" class="bib-filename" data-index="' + entry.index + '" value="' + escapeHtml(entry.suggested_filename) + '"></label></div>';
                html += '</div>';
            }
        }

        // Existing entries
        if (nExisting > 0) {
            html += '<details class="bib-existing-section"><summary>' + nExisting + ' entries already in your notes (skip)</summary>';
            for (const entry of data.existing_entries) {
                html += '<div class="bib-import-item existing">';
                html += '<code>' + escapeHtml(entry.cite_key) + '</code> &rarr; ';
                html += '<a href="/note/' + escapeHtml(entry.note_key) + '">' + escapeHtml(entry.note_title) + '</a>';
                html += '</div>';
            }
            html += '</details>';
        }

        // Conflicts
        if (nConflict > 0) {
            html += '<h3>Conflicts</h3>';
            for (const entry of data.conflicts) {
                html += '<div class="bib-import-item conflict">';
                html += '<code>' + escapeHtml(entry.cite_key) + '</code>';
                if (entry.title) html += ' &mdash; ' + escapeHtml(entry.title);
                html += '<br><small>Matches <a href="/note/' + escapeHtml(entry.matched_note_key) + '">' + escapeHtml(entry.matched_note_title) + '</a> (by ' + escapeHtml(entry.match_type) + ')</small>';
                html += '<div class="bib-conflict-actions">';
                html += '<select class="bib-conflict-action" data-index="' + entry.index + '" onchange="bibConflictChanged(this)">';
                html += '<option value="skip">Skip</option>';
                html += '<option value="secondary">Add as secondary key</option>';
                html += '<option value="create">Create as new note</option>';
                html += '</select>';
                html += '<input type="text" class="bib-conflict-filename" data-index="' + entry.index + '" value="' + escapeHtml(entry.cite_key) + '.md" style="display:none">';
                html += '</div>';
                html += '</div>';
            }
        }

        // Parse errors
        if (nErrors > 0) {
            html += '<details class="bib-existing-section"><summary>' + nErrors + ' parse errors</summary>';
            for (const err of data.parse_errors) {
                html += '<div class="bib-import-item error-item"><small>' + escapeHtml(err) + '</small></div>';
            }
            html += '</details>';
        }

        if (nNew > 0 || nConflict > 0) {
            html += '<div class="smart-result-actions" style="margin-top:1rem">';
            html += '<button class="btn" onclick="executeBibImport()">Import Selected</button>';
            html += '<button class="btn secondary" onclick="closeSmartAdd()">Cancel</button>';
            html += '</div>';
        } else if (nExisting > 0 && nNew === 0 && nConflict === 0) {
            html += '<p style="margin-top:1rem">All entries already exist in your notes.</p>';
        }

        review.innerHTML = html;
    }

    function bibConflictChanged(select) {
        const idx = select.dataset.index;
        const filenameInput = document.querySelector('.bib-conflict-filename[data-index="' + idx + '"]');
        if (select.value === 'create') {
            filenameInput.style.display = '';
        } else {
            filenameInput.style.display = 'none';
        }
    }

    async function executeBibImport() {
        if (!bibAnalysisData) return;

        const createItems = [];
        const secondaryItems = [];

        // Gather checked new entries
        document.querySelectorAll('.bib-new-check:checked').forEach(function(cb) {
            const idx = parseInt(cb.dataset.index);
            const entry = bibAnalysisData.new_entries.find(function(e) { return e.index === idx; });
            if (!entry) return;
            const filenameInput = document.querySelector('.bib-filename[data-index="' + idx + '"]');
            const filename = filenameInput ? filenameInput.value.trim() : entry.suggested_filename;
            createItems.push({ bibtex: entry.bibtex, filename: filename });
        });

        // Gather conflict decisions
        document.querySelectorAll('.bib-conflict-action').forEach(function(select) {
            const idx = parseInt(select.dataset.index);
            const entry = bibAnalysisData.conflicts.find(function(e) { return e.index === idx; });
            if (!entry) return;

            if (select.value === 'secondary') {
                secondaryItems.push({ note_key: entry.matched_note_key, bibtex: entry.bibtex });
            } else if (select.value === 'create') {
                const filenameInput = document.querySelector('.bib-conflict-filename[data-index="' + idx + '"]');
                const filename = filenameInput ? filenameInput.value.trim() : (entry.cite_key + '.md');
                createItems.push({ bibtex: entry.bibtex, filename: filename });
            }
        });

        if (createItems.length === 0 && secondaryItems.length === 0) {
            alert('No entries selected for import.');
            return;
        }

        const review = document.getElementById('bib-review');
        review.innerHTML = '<div class="smart-loading active"><div class="smart-spinner"></div><span>Importing...</span></div>';

        try {
            const response = await fetch('/api/bib-import/execute', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ create: createItems, add_secondary: secondaryItems })
            });

            const result = await response.json();
            let html = '<h3>Import Complete</h3>';

            if (result.created.length > 0) {
                html += '<p>' + result.created.length + ' notes created:</p><ul>';
                for (const n of result.created) {
                    html += '<li><a href="/note/' + escapeHtml(n.key) + '">' + escapeHtml(n.title) + '</a></li>';
                }
                html += '</ul>';
            }

            if (result.updated.length > 0) {
                html += '<p>' + result.updated.length + ' notes updated:</p><ul>';
                for (const n of result.updated) {
                    html += '<li><a href="/note/' + escapeHtml(n.key) + '">' + escapeHtml(n.title) + '</a></li>';
                }
                html += '</ul>';
            }

            if (result.errors.length > 0) {
                html += '<p class="message error">' + result.errors.length + ' errors:</p><ul>';
                for (const e of result.errors) {
                    html += '<li>' + escapeHtml(e) + '</li>';
                }
                html += '</ul>';
            }

            html += '<div class="smart-result-actions" style="margin-top:1rem">';
            html += '<button class="btn secondary" onclick="closeSmartAdd(); location.reload();">Close</button>';
            html += '</div>';

            review.innerHTML = html;
        } catch (e) {
            review.innerHTML = '<p class="message error">Import failed: ' + escapeHtml(e.message) + '</p>';
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

    // Toggle hidden state on a note
    async function toggleHidden(key, btn) {{
        try {{
            const response = await fetch('/api/note/' + key + '/toggle-hidden', {{
                method: 'POST',
                headers: {{ 'Content-Type': 'application/json' }}
            }});
            if (!response.ok) {{
                const err = await response.text();
                alert('Failed to toggle: ' + err);
                return;
            }}
            const data = await response.json();
            const li = btn.closest('.note-item');
            if (data.hidden) {{
                li.classList.add('hidden-note');
                if (li.querySelector('.title')) li.querySelector('.title').style.textDecoration = 'line-through';
                btn.textContent = 'unhide';
                btn.title = 'unhide';
                // If not showing hidden, fade out and remove
                if (!window.location.search.includes('hidden=true')) {{
                    li.style.transition = 'opacity 0.3s';
                    li.style.opacity = '0';
                    setTimeout(() => li.remove(), 300);
                }}
            }} else {{
                li.classList.remove('hidden-note');
                if (li.querySelector('.title')) li.querySelector('.title').style.textDecoration = '';
                li.style.opacity = '';
                btn.textContent = 'hide';
                btn.title = 'hide';
            }}
        }} catch (e) {{
            alert('Error toggling hidden: ' + e.message);
        }}
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
