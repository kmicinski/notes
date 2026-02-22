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
               <button class="pdf-toggle-btn" onclick="unlinkPdf()" title="Remove PDF link from this note">Unlink</button>
               <div class="note-type-dropdown" id="note-type-dropdown">
                   <button class="dropdown-btn" onclick="toggleNoteDropdown(event)">+ Note</button>
                   <div class="dropdown-content">
                       <button class="dropdown-item" onclick="addTypedNote('Definition')">Definition</button>
                       <button class="dropdown-item" onclick="addTypedNote('Question')">Question</button>
                       <button class="dropdown-item" onclick="addTypedNote('Highlight')">Highlight</button>
                       <button class="dropdown-item" onclick="addTypedNote('Begin study')">Begin study</button>
                       <button class="dropdown-item" onclick="addTypedNote('End study')">End study</button>
                   </div>
               </div>"#,
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
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
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
        }}

        .back-link {{
            color: #268bd2;
            text-decoration: none;
            font-size: 0.85rem;
            white-space: nowrap;
        }}
        .back-link:hover {{
            text-decoration: underline;
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
            font-family: inherit;
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
        }}

        #monaco-editor.with-pdf {{
            right: 50%;
        }}

        /* Resizable Split Divider */
        #split-divider {{
            position: absolute;
            top: 0;
            bottom: 0;
            width: 6px;
            background: #93a1a1;
            cursor: col-resize;
            z-index: 100;
            display: none;
            transition: background 0.15s;
        }}
        #split-divider:hover,
        #split-divider.dragging {{
            background: #268bd2;
        }}
        #split-divider.active {{
            display: block;
        }}

        /* PDF Viewer Pane */
        #pdf-viewer-pane {{
            position: absolute;
            top: 0;
            right: 0;
            width: 50%;
            bottom: 0;
            display: none;
            border-left: none;
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
            margin-left: auto;
        }}

        /* PDF Canvas Container */
        .pdf-canvas-container {{
            flex: 1;
            overflow: auto;
            background: #586e75;
            touch-action: pan-x pan-y; /* Allow scroll, capture pinch for custom zoom */
            position: relative;
            -webkit-overflow-scrolling: touch;
            overscroll-behavior: contain;
        }}

        /* Inner wrapper for zoom transform */
        .pdf-zoom-wrapper {{
            display: flex;
            flex-direction: column;
            align-items: center;
            padding: 1rem;
            gap: 1rem;
            transform-origin: center top;
            min-width: 100%;
            will-change: transform;
        }}

        /* PDF Page wrapper for canvas + text layer */
        .pdf-page-wrapper {{
            position: relative;
            background: white;
            box-shadow: 0 2px 8px rgba(0,0,0,0.3);
            contain: layout style;
        }}

        .pdf-page-wrapper canvas {{
            display: block;
        }}

        /* Text layer for selection */
        .textLayer {{
            position: absolute;
            left: 0;
            top: 0;
            right: 0;
            bottom: 0;
            overflow: hidden;
            opacity: 0.2;
            line-height: 1.0;
        }}

        .textLayer > span {{
            color: transparent;
            position: absolute;
            white-space: pre;
            cursor: text;
            transform-origin: 0% 0%;
        }}

        .textLayer .highlight {{
            margin: -1px;
            padding: 1px;
            background-color: rgba(180, 0, 170, 0.2);
            border-radius: 4px;
        }}

        .textLayer .highlight.selected {{
            background-color: rgba(0, 100, 0, 0.2);
        }}


        .textLayer ::selection {{
            background: rgba(0, 0, 255, 0.3);
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

        /* Note Type Dropdown */
        .note-type-dropdown {{
            position: relative;
            display: inline-block;
        }}
        .note-type-dropdown .dropdown-btn {{
            padding: 0.2rem 0.5rem;
            border: 1px solid #93a1a1;
            border-radius: 3px;
            background: transparent;
            color: #93a1a1;
            cursor: pointer;
            font-size: 0.75rem;
        }}
        .note-type-dropdown .dropdown-btn:hover {{
            background: #eee8d5;
        }}
        .note-type-dropdown .dropdown-content {{
            display: none;
            position: absolute;
            top: 100%;
            left: 0;
            background: #fdf6e3;
            border: 1px solid #93a1a1;
            border-radius: 4px;
            box-shadow: 0 2px 8px rgba(0,0,0,0.15);
            z-index: 100;
            min-width: 120px;
            margin-top: 2px;
        }}
        .note-type-dropdown.open .dropdown-content {{
            display: block;
        }}
        .note-type-dropdown .dropdown-item {{
            display: block;
            width: 100%;
            padding: 0.4rem 0.6rem;
            border: none;
            background: none;
            color: #657b83;
            font-size: 0.75rem;
            text-align: left;
            cursor: pointer;
        }}
        .note-type-dropdown .dropdown-item:hover {{
            background: #eee8d5;
        }}
        .note-type-dropdown .dropdown-item:first-child {{
            border-radius: 3px 3px 0 0;
        }}
        .note-type-dropdown .dropdown-item:last-child {{
            border-radius: 0 0 3px 3px;
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
            <a href="#" onclick="goBack('/')" class="back-link">&larr; All Notes</a>
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
            <a href="#" onclick="goBack('/note/{key}')" class="btn">Done</a>
        </div>
        <div class="editor-main">
            <div id="monaco-editor"></div>
            <div id="split-divider"></div>
            <div id="pdf-viewer-pane">
                <div class="pdf-toolbar">
                    <button onclick="pdfPrevPage()" id="pdf-prev-btn" disabled>&larr; Prev</button>
                    <button onclick="pdfNextPage()" id="pdf-next-btn" disabled>Next &rarr;</button>
                    <button onclick="pdfFitToWidth()" title="Fit to width">Fit</button>
                    <span class="pdf-page-info" id="pdf-page-info">Page 1 of 1</span>
                </div>
                <div class="pdf-canvas-container" id="pdf-canvas-container">
                    <div class="pdf-zoom-wrapper" id="pdf-zoom-wrapper">
                        <div class="pdf-loading" id="pdf-loading">
                            <div class="spinner"></div>
                            <span>Loading PDF...</span>
                        </div>
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
        const PDF_SCALE = 1.5; // Fixed scale for good readability
        let renderedPages = [];

        // Pinch zoom state
        let currentZoom = 1.0;
        let initialPinchDistance = 0;
        let initialZoom = 1.0;


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
        // PDF Viewer Functions (pdf.js based)
        // =====================================================================

        function savePdfState() {{
            if (!pdfFilename) return;
            const container = document.getElementById('pdf-canvas-container');
            const state = {{
                page: currentPdfPage,
                scrollTop: container ? container.scrollTop : 0,
                scrollLeft: container ? container.scrollLeft : 0,
                zoom: currentZoom,
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
            const divider = document.getElementById('split-divider');
            const btn = document.getElementById('pdf-toggle-btn');

            pane.classList.add('active');
            divider.classList.add('active');
            if (monacoDiv) monacoDiv.classList.add('with-pdf');
            if (btn) btn.classList.add('active');

            // Apply saved split position
            applySplitPosition();

            // Restore state
            const savedState = restorePdfState();

            // Trigger Monaco layout update
            setTimeout(function() {{ if (editor) editor.layout(); }}, 50);

            // Load PDF if not already loaded
            if (!pdfDoc) {{
                await loadPdf();
                // After loading, restore page, zoom, and scroll position
                if (savedState) {{
                    if (savedState.page) {{
                        currentPdfPage = Math.min(savedState.page, totalPdfPages);
                    }}
                    if (savedState.zoom) {{
                        applyZoom(savedState.zoom);
                    }}
                    await renderAllPages();
                    // Restore scroll position after a delay to allow layout
                    const container = document.getElementById('pdf-canvas-container');
                    if (savedState.scrollTop || savedState.scrollLeft) {{
                        setTimeout(() => {{
                            if (savedState.scrollTop) container.scrollTop = savedState.scrollTop;
                            if (savedState.scrollLeft) container.scrollLeft = savedState.scrollLeft;
                        }}, 150);
                    }}
                }}
                // Setup pinch-to-zoom handlers
                setupPinchZoom();
            }}

            savePdfState();
        }}

        function hidePdfViewer() {{
            savePdfState();

            const pane = document.getElementById('pdf-viewer-pane');
            const monacoDiv = document.getElementById('monaco-editor');
            const divider = document.getElementById('split-divider');
            const btn = document.getElementById('pdf-toggle-btn');

            pane.classList.remove('active');
            divider.classList.remove('active');
            if (monacoDiv) {{
                monacoDiv.classList.remove('with-pdf');
                monacoDiv.style.right = '0'; // Reset to full width
            }}
            if (btn) btn.classList.remove('active');

            // Trigger Monaco layout update
            setTimeout(function() {{ if (editor) editor.layout(); }}, 50);
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
            const zoomWrapper = document.getElementById('pdf-zoom-wrapper');
            const loading = document.getElementById('pdf-loading');

            // Show loading - clear zoom wrapper but keep it
            zoomWrapper.innerHTML = '';
            zoomWrapper.appendChild(loading);
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
                zoomWrapper.innerHTML = '<div class="pdf-error">Failed to load PDF: ' + error.message + '</div>';
                console.error('PDF load error:', error);
            }}
        }}

        // Track which pages have been rendered
        let renderedPageNums = new Set();
        let pageViewports = {{}};  // pageNum -> viewport
        const RENDER_BUFFER = 1;  // Render 1 page before/after visible

        // Setup placeholders for all pages (fast, no rendering)
        async function setupPagePlaceholders() {{
            if (!pdfDoc) return;

            const zoomWrapper = document.getElementById('pdf-zoom-wrapper');
            zoomWrapper.innerHTML = '';
            renderedPages = [];
            renderedPageNums.clear();
            pageViewports = {{}};

            // Get first page to determine dimensions (most PDFs have uniform size)
            const firstPage = await pdfDoc.getPage(1);
            const defaultViewport = firstPage.getViewport({{ scale: PDF_SCALE }});

            for (let pageNum = 1; pageNum <= totalPdfPages; pageNum++) {{
                // Create placeholder wrapper with estimated size
                const wrapper = document.createElement('div');
                wrapper.className = 'pdf-page-wrapper pdf-page-placeholder';
                wrapper.id = 'pdf-page-' + pageNum;
                wrapper.dataset.page = pageNum;
                wrapper.style.width = Math.floor(defaultViewport.width) + 'px';
                wrapper.style.height = Math.floor(defaultViewport.height) + 'px';
                wrapper.style.background = '#f5f5f5';

                zoomWrapper.appendChild(wrapper);
                renderedPages.push({{ wrapper, pageNum }});
            }}

            // Apply current zoom
            zoomWrapper.style.transform = 'scale(' + currentZoom + ')';

            updatePageInfo();
            updateNavButtons();

            // Render visible pages
            await renderVisiblePages();
        }}

        // Render a single page on demand
        async function renderPage(pageNum) {{
            if (renderedPageNums.has(pageNum)) return;
            if (pageNum < 1 || pageNum > totalPdfPages) return;

            const wrapper = document.getElementById('pdf-page-' + pageNum);
            if (!wrapper) return;

            renderedPageNums.add(pageNum);
            wrapper.classList.remove('pdf-page-placeholder');
            wrapper.style.background = 'white';

            const page = await pdfDoc.getPage(pageNum);
            const viewport = page.getViewport({{ scale: PDF_SCALE }});
            pageViewports[pageNum] = viewport;

            // Update wrapper size to actual page size
            wrapper.style.width = Math.floor(viewport.width) + 'px';
            wrapper.style.height = Math.floor(viewport.height) + 'px';

            const dpr = window.devicePixelRatio || 1;

            // Create canvas
            const canvas = document.createElement('canvas');
            const ctx = canvas.getContext('2d');
            canvas.width = Math.floor(viewport.width * dpr);
            canvas.height = Math.floor(viewport.height * dpr);
            canvas.style.width = Math.floor(viewport.width) + 'px';
            canvas.style.height = Math.floor(viewport.height) + 'px';
            ctx.scale(dpr, dpr);
            wrapper.appendChild(canvas);

            // Create text layer
            const textLayerDiv = document.createElement('div');
            textLayerDiv.className = 'textLayer';
            wrapper.appendChild(textLayerDiv);

            // Render canvas
            await page.render({{
                canvasContext: ctx,
                viewport: viewport
            }}).promise;

            // Render text layer
            const textContent = await page.getTextContent();
            await pdfjsLib.renderTextLayer({{
                textContent: textContent,
                container: textLayerDiv,
                viewport: viewport,
                textDivs: []
            }}).promise;

            // Update renderedPages entry
            const idx = pageNum - 1;
            renderedPages[idx] = {{ wrapper, canvas, page, viewport }};
        }}

        // Determine which pages are visible and render them
        async function renderVisiblePages() {{
            const container = document.getElementById('pdf-canvas-container');
            if (!container || !pdfDoc) return;

            const containerRect = container.getBoundingClientRect();
            const visiblePages = [];

            // Find pages that are in or near the viewport
            renderedPages.forEach(({{ wrapper }}, index) => {{
                const pageNum = index + 1;
                const rect = wrapper.getBoundingClientRect();

                // Check if page is visible (with some buffer)
                const buffer = containerRect.height;
                if (rect.bottom > containerRect.top - buffer &&
                    rect.top < containerRect.bottom + buffer) {{
                    visiblePages.push(pageNum);
                }}
            }});

            // Also include buffer pages around current page
            const minPage = Math.max(1, currentPdfPage - RENDER_BUFFER);
            const maxPage = Math.min(totalPdfPages, currentPdfPage + RENDER_BUFFER);
            for (let p = minPage; p <= maxPage; p++) {{
                if (!visiblePages.includes(p)) {{
                    visiblePages.push(p);
                }}
            }}

            // Render all visible pages
            for (const pageNum of visiblePages) {{
                await renderPage(pageNum);
            }}
        }}

        // Legacy function name for compatibility
        async function renderAllPages() {{
            await setupPagePlaceholders();
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

        async function pdfPrevPage() {{
            if (currentPdfPage <= 1) return;
            currentPdfPage--;
            await scrollToPage(currentPdfPage);
            updatePageInfo();
            updateNavButtons();
            savePdfState();
        }}

        async function pdfNextPage() {{
            if (currentPdfPage >= totalPdfPages) return;
            currentPdfPage++;
            await scrollToPage(currentPdfPage);
            updatePageInfo();
            updateNavButtons();
            savePdfState();
        }}

        async function scrollToPage(pageNum) {{
            // Render the page first if not already rendered
            await renderPage(pageNum);
            const wrapper = document.getElementById('pdf-page-' + pageNum);
            if (wrapper) {{
                wrapper.scrollIntoView({{ behavior: 'smooth', block: 'start' }});
            }}
        }}

        // Track current page based on scroll position
        let scrollSaveTimeout = null;
        let lazyRenderTimeout = null;
        function setupScrollTracking() {{
            const container = document.getElementById('pdf-canvas-container');
            if (!container) return;

            container.addEventListener('scroll', function() {{
                if (renderedPages.length === 0) return;

                const containerRect = container.getBoundingClientRect();
                const containerTop = containerRect.top;

                let closestPage = 1;
                let closestDistance = Infinity;

                renderedPages.forEach(({{ wrapper }}, index) => {{
                    const rect = wrapper.getBoundingClientRect();
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

                // Debounced lazy rendering of visible pages
                if (lazyRenderTimeout) clearTimeout(lazyRenderTimeout);
                lazyRenderTimeout = setTimeout(renderVisiblePages, 100);

                // Debounced save of scroll position
                if (scrollSaveTimeout) clearTimeout(scrollSaveTimeout);
                scrollSaveTimeout = setTimeout(savePdfState, 300);
            }});
        }}

        // Pinch-to-zoom handling - direct response during gesture
        function setupPinchZoom() {{
            const container = document.getElementById('pdf-canvas-container');
            const zoomWrapper = document.getElementById('pdf-zoom-wrapper');
            if (!container || !zoomWrapper) return;

            let isPinching = false;

            function getDistance(touches) {{
                const dx = touches[0].clientX - touches[1].clientX;
                const dy = touches[0].clientY - touches[1].clientY;
                return Math.sqrt(dx * dx + dy * dy);
            }}

            function applyZoomDirect(zoom) {{
                currentZoom = Math.min(Math.max(zoom, 0.5), 4.0);
                zoomWrapper.style.transform = 'scale(' + currentZoom + ')';
            }}

            container.addEventListener('touchstart', function(e) {{
                if (e.touches.length === 2) {{
                    e.preventDefault();
                    initialPinchDistance = getDistance(e.touches);
                    initialZoom = currentZoom;
                    isPinching = true;
                }}
            }}, {{ passive: false }});

            container.addEventListener('touchmove', function(e) {{
                if (e.touches.length === 2 && isPinching) {{
                    e.preventDefault();
                    const currentDistance = getDistance(e.touches);
                    const rawScale = currentDistance / initialPinchDistance;
                    // Amplify the scale change for faster zooming
                    const amplifiedScale = 1 + (rawScale - 1) * 2.0;
                    applyZoomDirect(initialZoom * amplifiedScale);
                }}
            }}, {{ passive: false }});

            container.addEventListener('touchend', function(e) {{
                if (e.touches.length < 2) {{
                    isPinching = false;
                    initialPinchDistance = 0;
                    savePdfState();
                }}
            }});

            // Also support mouse wheel zoom with ctrl/cmd
            let wheelZoomTimeout = null;
            container.addEventListener('wheel', function(e) {{
                if (e.ctrlKey || e.metaKey) {{
                    e.preventDefault();
                    const delta = e.deltaY > 0 ? 0.95 : 1.05;
                    applyZoomDirect(currentZoom * delta);
                    // Debounced save of zoom state
                    if (wheelZoomTimeout) clearTimeout(wheelZoomTimeout);
                    wheelZoomTimeout = setTimeout(savePdfState, 300);
                }}
            }}, {{ passive: false }});
        }}

        function applyZoom(zoom) {{
            currentZoom = zoom;
            const zoomWrapper = document.getElementById('pdf-zoom-wrapper');
            if (zoomWrapper) {{
                zoomWrapper.style.transform = 'scale(' + currentZoom + ')';
            }}
        }}

        // Fit PDF to container width
        function pdfFitToWidth() {{
            const container = document.getElementById('pdf-canvas-container');
            const firstPage = document.getElementById('pdf-page-1');
            if (!container || !firstPage) return;

            const containerWidth = container.clientWidth - 32; // Account for padding
            const pageWidth = firstPage.offsetWidth;

            if (pageWidth > 0) {{
                const newZoom = containerWidth / pageWidth;
                currentZoom = Math.min(Math.max(newZoom, 0.5), 4.0);
                const zoomWrapper = document.getElementById('pdf-zoom-wrapper');
                if (zoomWrapper) {{
                    zoomWrapper.style.transform = 'scale(' + currentZoom + ')';
                }}
                savePdfState();
            }}
        }}

        // =====================================================================
        // Resizable Split Pane
        // =====================================================================

        let splitPosition = parseFloat(localStorage.getItem('pdfSplitPosition')) || 50; // percentage

        function setupSplitDivider() {{
            const divider = document.getElementById('split-divider');
            const monacoDiv = document.getElementById('monaco-editor');
            const pdfPane = document.getElementById('pdf-viewer-pane');
            const main = document.querySelector('.editor-main');

            if (!divider || !monacoDiv || !pdfPane || !main) return;

            let isDragging = false;

            function updateSplitPosition(percent) {{
                splitPosition = Math.min(Math.max(percent, 20), 80); // Clamp between 20-80%
                const pdfWidth = 100 - splitPosition;

                monacoDiv.style.right = pdfWidth + '%';
                pdfPane.style.width = pdfWidth + '%';
                divider.style.right = 'calc(' + pdfWidth + '% - 3px)';

                // Trigger Monaco layout update
                if (editor) editor.layout();
            }}

            function onMouseDown(e) {{
                e.preventDefault();
                isDragging = true;
                divider.classList.add('dragging');
                document.body.style.cursor = 'col-resize';
                document.body.style.userSelect = 'none';
            }}

            function onMouseMove(e) {{
                if (!isDragging) return;
                const rect = main.getBoundingClientRect();
                const percent = ((e.clientX - rect.left) / rect.width) * 100;
                updateSplitPosition(percent);
            }}

            function onMouseUp() {{
                if (!isDragging) return;
                isDragging = false;
                divider.classList.remove('dragging');
                document.body.style.cursor = '';
                document.body.style.userSelect = '';
                localStorage.setItem('pdfSplitPosition', splitPosition.toString());
                // Final layout update
                if (editor) editor.layout();
            }}

            divider.addEventListener('mousedown', onMouseDown);
            document.addEventListener('mousemove', onMouseMove);
            document.addEventListener('mouseup', onMouseUp);

            // Touch support
            divider.addEventListener('touchstart', function(e) {{
                e.preventDefault();
                isDragging = true;
                divider.classList.add('dragging');
            }}, {{ passive: false }});

            document.addEventListener('touchmove', function(e) {{
                if (!isDragging) return;
                const touch = e.touches[0];
                const rect = main.getBoundingClientRect();
                const percent = ((touch.clientX - rect.left) / rect.width) * 100;
                updateSplitPosition(percent);
            }}, {{ passive: true }});

            document.addEventListener('touchend', function() {{
                if (!isDragging) return;
                isDragging = false;
                divider.classList.remove('dragging');
                localStorage.setItem('pdfSplitPosition', splitPosition.toString());
                if (editor) editor.layout();
            }});
        }}

        function applySplitPosition() {{
            const monacoDiv = document.getElementById('monaco-editor');
            const pdfPane = document.getElementById('pdf-viewer-pane');
            const divider = document.getElementById('split-divider');

            if (!monacoDiv || !pdfPane || !divider) return;

            const pdfWidth = 100 - splitPosition;
            monacoDiv.style.right = pdfWidth + '%';
            pdfPane.style.width = pdfWidth + '%';
            divider.style.right = 'calc(' + pdfWidth + '% - 3px)';
        }}

        // =====================================================================
        // Typed Note Annotations
        // =====================================================================

        function getCurrentPdfPage() {{
            return currentPdfPage;
        }}

        // Toggle note type dropdown
        function toggleNoteDropdown(event) {{
            event.stopPropagation();
            const dropdown = document.getElementById('note-type-dropdown');
            dropdown.classList.toggle('open');
        }}

        // Close dropdown when clicking outside
        document.addEventListener('click', function(e) {{
            const dropdown = document.getElementById('note-type-dropdown');
            if (dropdown && !dropdown.contains(e.target)) {{
                dropdown.classList.remove('open');
            }}
        }});

        // Format timestamp as "1/28/26 14:12"
        function formatTimestamp() {{
            const now = new Date();
            const month = now.getMonth() + 1;
            const day = now.getDate();
            const year = now.getFullYear() % 100;
            const hours = now.getHours();
            const minutes = now.getMinutes().toString().padStart(2, '0');
            return month + '/' + day + '/' + year + ' ' + hours + ':' + minutes;
        }}

        // Add a typed note at current page
        function addTypedNote(noteType) {{
            // Close dropdown
            const dropdown = document.getElementById('note-type-dropdown');
            if (dropdown) dropdown.classList.remove('open');

            const page = getCurrentPdfPage();
            if (!page || page < 1) {{
                alert('Could not detect PDF page. Make sure the PDF viewer is open.');
                return;
            }}

            const timestamp = formatTimestamp();
            let noteText;

            // Begin study and End study don't have "" or page number
            if (noteType === 'Begin study' || noteType === 'End study') {{
                noteText = '[(' + timestamp + ') ' + noteType + ']';
            }} else {{
                // Definition, Question, Highlight include page number
                noteText = '[(' + timestamp + ') ' + noteType + ' (pg. ' + page + '), ""]';
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

                // Insert the note
                const annotation = nl + '- ' + noteText;
                insertAnnotation(insertAt, annotation, noteType);
            }} else {{
                // Need to create page section - find or create Paper Notes first
                const paperNotesHeading = h2 + ' Paper Notes';
                let paperIdx = content.indexOf(paperNotesHeading);

                if (paperIdx === -1) {{
                    // Create Paper Notes section at end
                    const newSection = nl + nl + paperNotesHeading + nl + nl + pageHeading + nl + nl + '- ' + noteText;
                    insertAnnotation(content.length, newSection, noteType);
                }} else {{
                    // Find where to insert the new page section (keep pages sorted)
                    const afterPaperNotes = content.substring(paperIdx + paperNotesHeading.length);

                    let insertPos = paperIdx + paperNotesHeading.length;
                    let foundSpot = false;

                    const nextH2 = afterPaperNotes.indexOf(nl + h2 + ' ');
                    const searchEnd = nextH2 !== -1 ? nextH2 : afterPaperNotes.length;

                    const pageRegex = new RegExp(h3.replace(/\x23/g, '\\\\x23') + ' Page (\\\\d+)', 'g');
                    let match;
                    const searchArea = afterPaperNotes.substring(0, searchEnd);

                    while ((match = pageRegex.exec(searchArea)) !== null) {{
                        const existingPage = parseInt(match[1], 10);
                        if (existingPage > page) {{
                            insertPos = paperIdx + paperNotesHeading.length + match.index;
                            foundSpot = true;
                            break;
                        }}
                    }}

                    if (!foundSpot) {{
                        if (nextH2 !== -1) {{
                            insertPos = paperIdx + paperNotesHeading.length + nextH2;
                        }} else {{
                            insertPos = content.length;
                        }}
                    }}

                    const annotation = nl + nl + pageHeading + nl + nl + '- ' + noteText;
                    insertAnnotation(insertPos, annotation, noteType);
                }}
            }}
        }}

        function insertAnnotation(position, text, noteType) {{
            const model = editor.getModel();
            const pos = model.getPositionAt(position);

            editor.executeEdits('annotation', [{{
                range: new monaco.Range(pos.lineNumber, pos.column, pos.lineNumber, pos.column),
                text: text,
                forceMoveMarkers: true
            }}]);

            // Position cursor appropriately
            let cursorOffset;
            if (noteType && noteType !== 'Begin study' && noteType !== 'End study') {{
                // For types with "", position cursor between the quotes
                cursorOffset = position + text.length - 2; // Before the closing "]
            }} else {{
                // For Begin/End study or no type, position at end
                cursorOffset = position + text.length;
            }}

            const newPos = model.getPositionAt(cursorOffset);
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

        async function unlinkPdf() {{
            if (!confirm('Unlink PDF from this note? The PDF file will remain in the pdfs folder.')) return;
            try {{
                const resp = await fetch('/api/pdf/unlink', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ note_key: noteKey }})
                }});
                if (resp.ok) {{
                    window.location.reload();
                }} else {{
                    const err = await resp.text();
                    alert('Failed to unlink PDF: ' + err);
                }}
            }} catch (e) {{
                alert('Error unlinking PDF: ' + e.message);
            }}
        }}

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

                if (!response.ok) {{
                    const errText = await response.text();
                    showUploadStatus('error', errText || 'Upload failed');
                    return;
                }}

                const result = await response.json();
                showUploadStatus('success', 'Uploaded: ' + result.filename);
                pdfFilename = result.filename;
                pdfDoc = null; // Reset to force reload

                // Update the PDF status display
                const pdfStatus = document.getElementById('pdf-status');
                pdfStatus.innerHTML = `
                    <a href="/pdfs/${{encodeURIComponent(result.filename)}}" target="_blank" class="pdf-link">📄 ${{result.filename}}</a>
                    <button class="pdf-toggle-btn" id="pdf-toggle-btn" onclick="togglePdfViewer()">View</button>
                    <div class="note-type-dropdown" id="note-type-dropdown">
                        <button class="dropdown-btn" onclick="toggleNoteDropdown(event)">+ Note</button>
                        <div class="dropdown-content">
                            <button class="dropdown-item" onclick="addTypedNote('Definition')">Definition</button>
                            <button class="dropdown-item" onclick="addTypedNote('Question')">Question</button>
                            <button class="dropdown-item" onclick="addTypedNote('Highlight')">Highlight</button>
                            <button class="dropdown-item" onclick="addTypedNote('Begin study')">Begin study</button>
                            <button class="dropdown-item" onclick="addTypedNote('End study')">End study</button>
                        </div>
                    </div>
                `;

                // Close modal and show PDF
                setTimeout(() => {{
                    closePdfUpload();
                    showPdfViewer();
                }}, 1000);
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

                if (!response.ok) {{
                    const errText = await response.text();
                    showUploadStatus('error', errText || 'Download failed');
                    return;
                }}

                const result = await response.json();
                showUploadStatus('success', 'Downloaded: ' + result.filename);
                pdfFilename = result.filename;
                pdfDoc = null; // Reset to force reload

                // Update the PDF status display
                const pdfStatus = document.getElementById('pdf-status');
                pdfStatus.innerHTML = `
                    <a href="/pdfs/${{encodeURIComponent(result.filename)}}" target="_blank" class="pdf-link">📄 ${{result.filename}}</a>
                    <button class="pdf-toggle-btn" id="pdf-toggle-btn" onclick="togglePdfViewer()">View</button>
                    <div class="note-type-dropdown" id="note-type-dropdown">
                        <button class="dropdown-btn" onclick="toggleNoteDropdown(event)">+ Note</button>
                        <div class="dropdown-content">
                            <button class="dropdown-item" onclick="addTypedNote('Definition')">Definition</button>
                            <button class="dropdown-item" onclick="addTypedNote('Question')">Question</button>
                            <button class="dropdown-item" onclick="addTypedNote('Highlight')">Highlight</button>
                            <button class="dropdown-item" onclick="addTypedNote('Begin study')">Begin study</button>
                            <button class="dropdown-item" onclick="addTypedNote('End study')">End study</button>
                        </div>
                    </div>
                `;

                // Close modal and show PDF
                setTimeout(() => {{
                    closePdfUpload();
                    showPdfViewer();
                }}, 1000);
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

            // Setup resizable split pane
            setupSplitDivider();

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
                if (window.sharedMode) return; // Shared mode: edits sync via WS
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

            // Expose to window for shared mode overlay
            window.editor = editor;
            window.hasUnsavedChanges = hasUnsavedChanges;
        }});

        window.scheduleAutoSave = scheduleAutoSave;
        function scheduleAutoSave() {{
            if (window.sharedMode) return; // Shared mode: no disk auto-save
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

        window.saveNote = saveNote;
        async function saveNote(isAutoSave, shouldCommit) {{
            if (window.sharedSaveHandler) {{ window.sharedSaveHandler(); return; }}
            if (window.sharedMode) return;
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

        // Navigate back, auto-saving if needed
        async function goBack(url) {{
            if (hasUnsavedChanges) {{
                await saveNote(true, true); // Auto-save with commit
            }}
            savePdfState();
            window.location.href = url;
        }}

        // Warn before leaving with unsaved changes and save PDF state
        window.addEventListener('beforeunload', (e) => {{
            savePdfState();
            if (window.sharedMode) return; // Shared mode: edits sync via WS
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
