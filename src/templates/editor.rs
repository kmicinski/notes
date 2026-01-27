//! Editor template for the notes application.
//!
//! Contains the Monaco-based editor with PDF viewing support.

use crate::models::Note;
use crate::notes::html_escape;
use serde::Serialize;
use std::collections::HashMap;

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

        /* PDF Viewer Pane */
        #pdf-viewer-pane {{
            position: absolute;
            top: 0;
            right: 0;
            width: 50%;
            bottom: 0;
            display: none;
            border-left: 1px solid #93a1a1;
            background: #586e75;
            flex-direction: column;
        }}
        #pdf-viewer-pane.active {{
            display: flex;
        }}

        /* PDF Toolbar */
        .pdf-toolbar {{
            height: 40px;
            background: #eee8d5;
            border-bottom: 1px solid #93a1a1;
            display: flex;
            align-items: center;
            padding: 0 0.5rem;
            gap: 0.5rem;
            flex-shrink: 0;
        }}

        .pdf-toolbar button {{
            padding: 0.3rem 0.5rem;
            border: 1px solid #93a1a1;
            border-radius: 3px;
            background: #fdf6e3;
            color: #657b83;
            cursor: pointer;
            font-size: 0.8rem;
            font-family: inherit;
        }}
        .pdf-toolbar button:hover {{
            background: #eee8d5;
        }}
        .pdf-toolbar button:disabled {{
            opacity: 0.5;
            cursor: default;
        }}

        .pdf-page-info {{
            font-size: 0.8rem;
            color: #657b83;
            margin: 0 0.5rem;
        }}

        .pdf-scale-controls {{
            display: flex;
            align-items: baseline;
            gap: 0.1rem;
            margin-left: auto;
            font-family: Georgia, 'Times New Roman', serif;
            color: #93a1a1;
        }}
        .pdf-scale-controls label {{
            cursor: pointer;
            padding: 0.2rem 0.3rem;
            border-radius: 3px;
            transition: all 0.15s ease;
            line-height: 1;
        }}
        .pdf-scale-controls label:hover {{
            color: #657b83;
        }}
        .pdf-scale-controls input[type="radio"] {{
            display: none;
        }}
        .pdf-scale-controls input[type="radio"]:checked + span {{
            color: #268bd2;
        }}
        .pdf-scale-controls .scale-75 {{ font-size: 0.7rem; }}
        .pdf-scale-controls .scale-100 {{ font-size: 0.85rem; }}
        .pdf-scale-controls .scale-125 {{ font-size: 1rem; font-weight: 500; }}
        .pdf-scale-controls .scale-150 {{ font-size: 1.15rem; font-weight: 500; }}

        /* PDF Canvas Container */
        .pdf-canvas-container {{
            flex: 1;
            overflow: auto;
            background: #586e75;
            display: flex;
            flex-direction: column;
            align-items: center;
            padding: 1rem;
            gap: 1rem;
        }}

        .pdf-canvas-container canvas {{
            background: white;
            box-shadow: 0 2px 8px rgba(0,0,0,0.3);
        }}

        .pdf-loading {{
            display: flex;
            align-items: center;
            justify-content: center;
            height: 100%;
            color: #fdf6e3;
            font-size: 0.9rem;
        }}

        .pdf-loading .spinner {{
            width: 24px;
            height: 24px;
            border: 3px solid #93a1a1;
            border-top-color: #fdf6e3;
            border-radius: 50%;
            animation: spin 1s linear infinite;
            margin-right: 0.5rem;
        }}

        @keyframes spin {{
            to {{ transform: rotate(360deg); }}
        }}

        .pdf-error {{
            display: flex;
            align-items: center;
            justify-content: center;
            height: 100%;
            color: #dc322f;
            font-size: 0.9rem;
            padding: 1rem;
            text-align: center;
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
                <div class="pdf-toolbar">
                    <button onclick="pdfPrevPage()" id="pdf-prev-btn" disabled>&larr; Prev</button>
                    <button onclick="pdfNextPage()" id="pdf-next-btn" disabled>Next &rarr;</button>
                    <span class="pdf-page-info" id="pdf-page-info">Page 1 of 1</span>
                    <div class="pdf-scale-controls" title="PDF Scale">
                        <label><input type="radio" name="pdf-scale" value="0.75" onchange="setPdfScale(0.75)"><span class="scale-75">A</span></label>
                        <label><input type="radio" name="pdf-scale" value="1.0" onchange="setPdfScale(1.0)"><span class="scale-100">A</span></label>
                        <label><input type="radio" name="pdf-scale" value="1.25" onchange="setPdfScale(1.25)"><span class="scale-125">A</span></label>
                        <label><input type="radio" name="pdf-scale" value="1.5" onchange="setPdfScale(1.5)"><span class="scale-150">A</span></label>
                    </div>
                </div>
                <div class="pdf-canvas-container" id="pdf-canvas-container">
                    <div class="pdf-loading" id="pdf-loading">
                        <div class="spinner"></div>
                        <span>Loading PDF...</span>
                    </div>
                </div>
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
    <script src="https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.min.js"></script>
    <script>
        // Set pdf.js worker
        pdfjsLib.GlobalWorkerOptions.workerSrc = 'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.worker.min.js';

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

        // PDF state
        let pdfDoc = null;
        let currentPdfPage = 1;
        let totalPdfPages = 0;
        let currentPdfScale = parseFloat(localStorage.getItem('pdfScale')) || 1.0;
        let renderedPages = [];

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

        function initPdfScaleControls() {{
            const radios = document.querySelectorAll('input[name="pdf-scale"]');
            radios.forEach(radio => {{
                if (parseFloat(radio.value) === currentPdfScale) {{
                    radio.checked = true;
                }}
            }});
        }}

        // =====================================================================
        // PDF Viewer Functions (pdf.js based)
        // =====================================================================

        function savePdfState() {{
            if (!pdfFilename) return;
            const container = document.getElementById('pdf-canvas-container');
            const state = {{
                page: currentPdfPage,
                scale: currentPdfScale,
                scrollTop: container ? container.scrollTop : 0,
                visible: document.getElementById('pdf-viewer-pane').classList.contains('active'),
                timestamp: Date.now()
            }};
            localStorage.setItem('pdfState-' + noteKey, JSON.stringify(state));
        }}

        function restorePdfState() {{
            if (!pdfFilename) return null;
            try {{
                const stored = localStorage.getItem('pdfState-' + noteKey);
                if (!stored) return null;
                const state = JSON.parse(stored);
                // Check 24h expiry
                if (Date.now() - state.timestamp > 24 * 60 * 60 * 1000) {{
                    localStorage.removeItem('pdfState-' + noteKey);
                    return null;
                }}
                return state;
            }} catch (e) {{
                return null;
            }}
        }}

        async function showPdfViewer() {{
            if (!pdfFilename) return;

            const pane = document.getElementById('pdf-viewer-pane');
            const monacoDiv = document.getElementById('monaco-editor');
            const btn = document.getElementById('pdf-toggle-btn');

            pane.classList.add('active');
            if (monacoDiv) monacoDiv.classList.add('with-pdf');
            if (btn) btn.classList.add('active');

            // Restore state
            const savedState = restorePdfState();
            if (savedState) {{
                currentPdfScale = savedState.scale;
                initPdfScaleControls();
            }}

            // Trigger Monaco layout update after transition
            setTimeout(function() {{ if (editor) editor.layout(); }}, 250);

            // Load PDF if not already loaded
            if (!pdfDoc) {{
                await loadPdf();
                // After loading, restore page and scroll position
                if (savedState && savedState.page) {{
                    currentPdfPage = Math.min(savedState.page, totalPdfPages);
                    await renderAllPages();
                    // Restore scroll position
                    if (savedState.scrollTop) {{
                        const container = document.getElementById('pdf-canvas-container');
                        setTimeout(() => {{
                            container.scrollTop = savedState.scrollTop;
                        }}, 100);
                    }}
                }}
            }}

            savePdfState();
        }}

        function hidePdfViewer() {{
            savePdfState();

            const pane = document.getElementById('pdf-viewer-pane');
            const monacoDiv = document.getElementById('monaco-editor');
            const btn = document.getElementById('pdf-toggle-btn');

            pane.classList.remove('active');
            if (monacoDiv) monacoDiv.classList.remove('with-pdf');
            if (btn) btn.classList.remove('active');

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

        async function loadPdf() {{
            const container = document.getElementById('pdf-canvas-container');
            const loading = document.getElementById('pdf-loading');

            // Show loading
            container.innerHTML = '';
            container.appendChild(loading);
            loading.style.display = 'flex';

            try {{
                const loadingTask = pdfjsLib.getDocument('/pdfs/' + encodeURIComponent(pdfFilename));
                pdfDoc = await loadingTask.promise;
                totalPdfPages = pdfDoc.numPages;

                updatePageInfo();
                loading.style.display = 'none';

                await renderAllPages();
            }} catch (error) {{
                loading.style.display = 'none';
                container.innerHTML = '<div class="pdf-error">Failed to load PDF: ' + error.message + '</div>';
                console.error('PDF load error:', error);
            }}
        }}

        async function renderAllPages() {{
            if (!pdfDoc) return;

            const container = document.getElementById('pdf-canvas-container');
            container.innerHTML = '';
            renderedPages = [];

            const dpr = window.devicePixelRatio || 1;

            for (let pageNum = 1; pageNum <= totalPdfPages; pageNum++) {{
                const page = await pdfDoc.getPage(pageNum);
                const viewport = page.getViewport({{ scale: currentPdfScale }});

                const canvas = document.createElement('canvas');
                canvas.id = 'pdf-page-' + pageNum;
                canvas.dataset.page = pageNum;
                const ctx = canvas.getContext('2d');

                // High DPI support
                canvas.width = Math.floor(viewport.width * dpr);
                canvas.height = Math.floor(viewport.height * dpr);
                canvas.style.width = Math.floor(viewport.width) + 'px';
                canvas.style.height = Math.floor(viewport.height) + 'px';

                ctx.scale(dpr, dpr);

                container.appendChild(canvas);
                renderedPages.push({{ canvas, page, viewport }});

                await page.render({{
                    canvasContext: ctx,
                    viewport: viewport
                }}).promise;
            }}

            updatePageInfo();
            updateNavButtons();
        }}

        function updatePageInfo() {{
            const info = document.getElementById('pdf-page-info');
            if (info) {{
                info.textContent = 'Page ' + currentPdfPage + ' of ' + totalPdfPages;
            }}
        }}

        function updateNavButtons() {{
            const prevBtn = document.getElementById('pdf-prev-btn');
            const nextBtn = document.getElementById('pdf-next-btn');
            if (prevBtn) prevBtn.disabled = currentPdfPage <= 1;
            if (nextBtn) nextBtn.disabled = currentPdfPage >= totalPdfPages;
        }}

        function pdfPrevPage() {{
            if (currentPdfPage <= 1) return;
            currentPdfPage--;
            scrollToPage(currentPdfPage);
            updatePageInfo();
            updateNavButtons();
            savePdfState();
        }}

        function pdfNextPage() {{
            if (currentPdfPage >= totalPdfPages) return;
            currentPdfPage++;
            scrollToPage(currentPdfPage);
            updatePageInfo();
            updateNavButtons();
            savePdfState();
        }}

        function scrollToPage(pageNum) {{
            const canvas = document.getElementById('pdf-page-' + pageNum);
            if (canvas) {{
                canvas.scrollIntoView({{ behavior: 'smooth', block: 'start' }});
            }}
        }}

        function setPdfScale(scale) {{
            currentPdfScale = scale;
            localStorage.setItem('pdfScale', scale);
            if (pdfDoc) {{
                renderAllPages();
            }}
            savePdfState();
        }}

        // Track current page based on scroll position
        function setupScrollTracking() {{
            const container = document.getElementById('pdf-canvas-container');
            if (!container) return;

            container.addEventListener('scroll', function() {{
                if (renderedPages.length === 0) return;

                const containerRect = container.getBoundingClientRect();
                const containerTop = containerRect.top;

                let closestPage = 1;
                let closestDistance = Infinity;

                renderedPages.forEach(({{ canvas }}, index) => {{
                    const rect = canvas.getBoundingClientRect();
                    const distance = Math.abs(rect.top - containerTop);
                    if (distance < closestDistance) {{
                        closestDistance = distance;
                        closestPage = index + 1;
                    }}
                }});

                if (closestPage !== currentPdfPage) {{
                    currentPdfPage = closestPage;
                    updatePageInfo();
                    updateNavButtons();
                }}
            }});
        }}

        // =====================================================================
        // Page-Based Annotation
        // =====================================================================

        function getCurrentPdfPage() {{
            return currentPdfPage;
        }}

        function addPageNote() {{
            const page = getCurrentPdfPage();
            if (!page || page < 1) {{
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

            // Setup scroll tracking for PDF
            setupScrollTracking();
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
                    pdfDoc = null; // Reset to force reload

                    // Update the PDF status display
                    const pdfStatus = document.getElementById('pdf-status');
                    pdfStatus.innerHTML = `
                        <a href="/pdfs/${{encodeURIComponent(result.filename)}}" target="_blank" class="pdf-link">📄 ${{result.filename}}</a>
                        <button class="pdf-toggle-btn" id="pdf-toggle-btn" onclick="togglePdfViewer()">View</button>
                        <button class="pdf-toggle-btn" onclick="addPageNote()">+ Page Note</button>
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
                    pdfDoc = null; // Reset to force reload

                    // Update the PDF status display
                    const pdfStatus = document.getElementById('pdf-status');
                    pdfStatus.innerHTML = `
                        <a href="/pdfs/${{encodeURIComponent(result.filename)}}" target="_blank" class="pdf-link">📄 ${{result.filename}}</a>
                        <button class="pdf-toggle-btn" id="pdf-toggle-btn" onclick="togglePdfViewer()">View</button>
                        <button class="pdf-toggle-btn" onclick="addPageNote()">+ Page Note</button>
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
            initPdfScaleControls();

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

            // Restore PDF visibility from saved state
            const savedState = restorePdfState();
            if (pdfFilename && savedState && savedState.visible) {{
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

        // Warn before leaving with unsaved changes and save PDF state
        window.addEventListener('beforeunload', (e) => {{
            savePdfState();
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
