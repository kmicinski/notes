//! Viewer template for the notes application.
//!
//! Contains the view mode with PDF viewing support.

use crate::models::Note;
use crate::notes::html_escape;

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
            r#"<a href="/pdfs/{}" target="_blank">📄 {}</a>
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

        /* PDF Viewer Pane */
        #pdf-viewer-pane {{
            width: 50%;
            display: none;
            border-left: 1px solid var(--border);
            background: #586e75;
            flex-direction: column;
        }}
        #pdf-viewer-pane.active {{
            display: flex;
        }}

        /* PDF Toolbar */
        .pdf-toolbar {{
            height: 40px;
            background: var(--accent);
            border-bottom: 1px solid var(--border);
            display: flex;
            align-items: center;
            padding: 0 0.5rem;
            gap: 0.5rem;
            flex-shrink: 0;
        }}

        .pdf-toolbar button {{
            padding: 0.3rem 0.5rem;
            border: 1px solid var(--border);
            border-radius: 3px;
            background: var(--bg);
            color: var(--fg);
            cursor: pointer;
            font-size: 0.8rem;
            font-family: inherit;
        }}
        .pdf-toolbar button:hover {{
            background: var(--accent);
        }}
        .pdf-toolbar button:disabled {{
            opacity: 0.5;
            cursor: default;
        }}

        .pdf-page-info {{
            font-size: 0.8rem;
            color: var(--fg);
            margin: 0 0.5rem;
        }}

        .pdf-scale-controls {{
            display: flex;
            align-items: baseline;
            gap: 0.1rem;
            margin-left: auto;
            font-family: Georgia, 'Times New Roman', serif;
            color: var(--muted);
        }}
        .pdf-scale-controls label {{
            cursor: pointer;
            padding: 0.2rem 0.3rem;
            border-radius: 3px;
            transition: all 0.15s ease;
            line-height: 1;
        }}
        .pdf-scale-controls label:hover {{
            color: var(--fg);
        }}
        .pdf-scale-controls input[type="radio"] {{
            display: none;
        }}
        .pdf-scale-controls input[type="radio"]:checked + span {{
            color: var(--link);
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
    </div>

    <script src="https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.min.js"></script>
    <script>
        // Set pdf.js worker
        pdfjsLib.GlobalWorkerOptions.workerSrc = 'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.worker.min.js';

        const noteKey = "{key}";
        const pdfFilename = {pdf_filename_json};

        // PDF state
        let pdfDoc = null;
        let currentPdfPage = 1;
        let totalPdfPages = 0;
        let currentPdfScale = parseFloat(localStorage.getItem('pdfScale')) || 1.0;
        let renderedPages = [];

        function initPdfScaleControls() {{
            const radios = document.querySelectorAll('input[name="pdf-scale"]');
            radios.forEach(radio => {{
                if (parseFloat(radio.value) === currentPdfScale) {{
                    radio.checked = true;
                }}
            }});
        }}

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
            const btn = document.getElementById('pdf-toggle-btn');

            pane.classList.add('active');
            if (btn) btn.classList.add('active');

            // Restore state
            const savedState = restorePdfState();
            if (savedState) {{
                currentPdfScale = savedState.scale;
                initPdfScaleControls();
            }}

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
            const btn = document.getElementById('pdf-toggle-btn');

            pane.classList.remove('active');
            if (btn) btn.classList.remove('active');
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

        // Initialize on page load
        document.addEventListener('DOMContentLoaded', function() {{
            initPdfScaleControls();
            setupScrollTracking();

            // Restore PDF visibility from saved state
            const savedState = restorePdfState();
            if (pdfFilename && savedState && savedState.visible) {{
                showPdfViewer();
            }}
        }});

        // Save state before leaving
        window.addEventListener('beforeunload', function() {{
            savePdfState();
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
