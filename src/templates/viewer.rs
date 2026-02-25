//! Viewer template for the notes application.
//!
//! Contains the view mode with PDF viewing support.

use crate::models::Note;
use crate::notes::html_escape;
use super::graph_js::{render_graph_js, graph_css, GraphRendererConfig, GraphDataSource};

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
    is_paper: bool,
) -> String {
    let pdf_filename = note.pdf.as_deref().unwrap_or("");
    let pdf_filename_json = serde_json::to_string(pdf_filename)
        .unwrap_or_else(|_| "\"\"".to_string());
    let has_pdf = note.pdf.is_some();

    let pdf_status_html = if let Some(ref pdf) = note.pdf {
        let unlink_btn = if logged_in {
            r#" <button class="pdf-toggle-btn" onclick="unlinkPdf()" title="Remove PDF link from this note">Unlink</button>"#
        } else {
            ""
        };
        let scan_btn = if is_paper && logged_in {
            r#" <button class="pdf-toggle-btn" onclick="scanReferences()" title="Scan PDF for references to other papers">Scan Refs</button>"#
        } else {
            ""
        };
        format!(
            r#"<a href="/pdfs/{}" target="_blank">📄 {}</a>
               <button class="pdf-toggle-btn" id="pdf-toggle-btn" onclick="togglePdfViewer()">View PDF</button>{}{}"#,
            html_escape(pdf),
            html_escape(pdf),
            unlink_btn,
            scan_btn
        )
    } else if is_paper && logged_in {
        r#"<button class="pdf-toggle-btn" id="pdf-toggle-btn" onclick="togglePdfViewer()">Find PDF</button>"#.to_string()
    } else {
        String::new()
    };

    let mode_toggle = if logged_in {
        format!(
            r#"<div class="mode-toggle">
                <button class="active">View</button>
                <button onclick="window.location.href='/note/{}?edit=true'">Edit</button>
                <button onclick="openSharePanel('{}')" title="Create collaborative copy">Share</button>
                <button class="delete-btn" onclick="confirmDelete('{}', '{}')">Delete</button>
            </div>"#,
            note.key,
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
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
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

        /* Font Size Controls */
        .font-size-controls {{
            display: flex;
            align-items: baseline;
            gap: 0.15rem;
            font-family: Georgia, 'Times New Roman', serif;
            color: var(--muted);
        }}
        .font-size-controls label {{
            cursor: pointer;
            padding: 0.2rem 0.35rem;
            border-radius: 3px;
            transition: all 0.15s ease;
            line-height: 1;
        }}
        .font-size-controls label:hover {{
            color: var(--fg);
        }}
        .font-size-controls input[type="radio"] {{
            display: none;
        }}
        .font-size-controls input[type="radio"]:checked + span {{
            color: var(--link);
        }}
        .font-size-controls .size-tiny {{ font-size: 0.7rem; }}
        .font-size-controls .size-small {{ font-size: 0.85rem; }}
        .font-size-controls .size-normal {{ font-size: 1rem; font-weight: 500; }}
        .font-size-controls .size-large {{ font-size: 1.2rem; font-weight: 500; }}

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

        /* Resizable Split Divider */
        #split-divider {{
            width: 6px;
            background: var(--border);
            cursor: col-resize;
            flex-shrink: 0;
            display: none;
            transition: background 0.15s;
        }}
        #split-divider:hover,
        #split-divider.dragging {{
            background: var(--link);
        }}
        #split-divider.active {{
            display: block;
        }}

        /* PDF Viewer Pane */
        #pdf-viewer-pane {{
            width: 50%;
            display: none;
            background: #586e75;
            flex-direction: column;
            flex-shrink: 0;
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

        /* PDF Dropzone */
        #pdf-dropzone-viewer {{
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            height: 100%;
            padding: 2rem;
            text-align: center;
            color: var(--base1);
        }}
        #pdf-dropzone-viewer.drag-over {{
            background: rgba(38, 139, 210, 0.1);
            border: 2px dashed var(--link);
        }}
        .pdf-drop-icon {{
            font-size: 1.2rem;
            margin-bottom: 0.75rem;
            color: var(--base1);
        }}
        .pdf-drop-or {{
            font-size: 0.85rem;
            color: var(--base01);
            margin: 0.75rem 0;
        }}
        #smart-find-btn {{
            padding: 0.5rem 1.25rem;
            border: 1px solid var(--link);
            border-radius: 4px;
            background: var(--link);
            color: white;
            cursor: pointer;
            font-size: 0.9rem;
            font-family: inherit;
        }}
        #smart-find-btn:hover {{
            background: var(--cyan);
            border-color: var(--cyan);
        }}
        #smart-find-btn:disabled {{
            opacity: 0.6;
            cursor: default;
        }}
        #smart-find-status {{
            margin-top: 1rem;
            font-size: 0.85rem;
            color: var(--base1);
            min-height: 1.5em;
        }}
        .smart-find-spinner {{
            display: inline-block;
            width: 16px;
            height: 16px;
            border: 2px solid #93a1a1;
            border-top-color: #fdf6e3;
            border-radius: 50%;
            animation: spin 1s linear infinite;
            vertical-align: middle;
            margin-right: 0.4rem;
        }}
        .smart-find-result {{
            background: rgba(133, 153, 0, 0.15);
            border: 1px solid var(--green);
            border-radius: 6px;
            padding: 1rem;
            margin-top: 1rem;
            text-align: left;
            max-width: 400px;
        }}
        .smart-find-result .source-badge {{
            display: inline-block;
            font-size: 0.7rem;
            padding: 0.15rem 0.4rem;
            border-radius: 3px;
            background: var(--green);
            color: white;
            text-transform: uppercase;
            font-weight: 600;
            margin-bottom: 0.5rem;
        }}
        .smart-find-result .found-url {{
            font-size: 0.8rem;
            color: var(--base1);
            word-break: break-all;
            margin: 0.5rem 0;
        }}
        .smart-find-actions {{
            display: flex;
            gap: 0.5rem;
            margin-top: 0.75rem;
        }}
        .smart-find-actions button {{
            padding: 0.4rem 0.8rem;
            border: 1px solid var(--border);
            border-radius: 4px;
            cursor: pointer;
            font-size: 0.85rem;
            font-family: inherit;
        }}
        .btn-accept {{
            background: var(--green) !important;
            color: white !important;
            border-color: var(--green) !important;
        }}
        .btn-cancel {{
            background: var(--bg);
            color: var(--fg);
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

        /* Mini knowledge graph panel */
        .mini-graph-panel {{
            position: fixed;
            bottom: 20px;
            right: 20px;
            width: 960px;
            height: 800px;
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 8px;
            box-shadow: 0 4px 20px rgba(0,0,0,0.2);
            z-index: 999;
            display: none;
            flex-direction: column;
            overflow: hidden;
            resize: both;
        }}
        .mini-graph-panel.active {{ display: flex; }}
        .mini-graph-panel-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 0.5rem 0.75rem;
            background: var(--accent);
            border-bottom: 1px solid var(--border);
            font-size: 0.8rem;
            font-weight: 600;
            cursor: move;
        }}
        .mini-graph-panel-header a {{
            font-weight: normal;
            font-size: 0.75rem;
        }}
        .mini-graph-panel-close {{
            background: none;
            border: none;
            font-size: 1.1rem;
            cursor: pointer;
            color: var(--muted);
            padding: 0 0.25rem;
        }}
        .mini-graph-body {{
            flex: 1;
            position: relative;
            overflow: hidden;
        }}
        .mini-graph-body svg {{ width: 100%; height: 100%; }}
        {mini_graph_css}

        /* Citation scan results panel */
        .citation-panel {{
            position: fixed;
            top: 60px;
            right: 20px;
            width: 380px;
            max-height: 70vh;
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 8px;
            box-shadow: 0 4px 16px rgba(0,0,0,0.15);
            z-index: 1000;
            display: none;
            flex-direction: column;
            overflow: hidden;
        }}
        .citation-panel.active {{ display: flex; }}
        .citation-panel-header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 0.75rem 1rem;
            background: var(--accent);
            border-bottom: 1px solid var(--border);
            font-weight: 600;
            font-size: 0.9rem;
        }}
        .citation-panel-close {{
            background: none;
            border: none;
            font-size: 1.1rem;
            cursor: pointer;
            color: var(--muted);
            padding: 0 0.25rem;
        }}
        .citation-panel-body {{
            overflow-y: auto;
            padding: 0.75rem 1rem;
            flex: 1;
        }}
        .citation-match {{
            padding: 0.5rem 0;
            border-bottom: 1px solid var(--border);
            font-size: 0.85rem;
        }}
        .citation-match:last-child {{ border-bottom: none; }}
        .citation-match .cm-key {{
            font-family: monospace;
            color: var(--link);
            font-size: 0.8rem;
        }}
        .citation-match .cm-type {{
            display: inline-block;
            font-size: 0.7rem;
            padding: 0.1rem 0.3rem;
            border-radius: 3px;
            background: var(--accent);
            color: var(--muted);
            margin-left: 0.3rem;
        }}
        .citation-match .cm-title {{
            display: block;
            margin-top: 0.2rem;
            color: var(--fg);
        }}
        .citation-panel-footer {{
            padding: 0.75rem 1rem;
            border-top: 1px solid var(--border);
            display: flex;
            gap: 0.5rem;
            align-items: center;
        }}
        /* Share panel - reuses citation panel styles */
        .share-panel {{
            position: fixed;
            top: 60px;
            right: 20px;
            width: 380px;
            max-height: 70vh;
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 8px;
            box-shadow: 0 4px 16px rgba(0,0,0,0.15);
            z-index: 1000;
            display: none;
            flex-direction: column;
            overflow: hidden;
        }}
        .share-panel.active {{ display: flex; }}

        .share-link-item {{
            padding: 0.5rem 0;
            border-bottom: 1px solid var(--border);
            font-size: 0.8rem;
        }}
        .share-link-item:last-child {{ border-bottom: none; }}
        .share-link-url {{
            font-family: monospace;
            color: var(--link);
            font-size: 0.75rem;
            word-break: break-all;
            cursor: pointer;
        }}
        .share-link-url:hover {{ text-decoration: underline; }}
        .share-link-meta {{
            color: var(--muted);
            font-size: 0.7rem;
            margin-top: 0.2rem;
        }}
        .share-link-actions {{
            display: flex;
            gap: 0.3rem;
            margin-top: 0.3rem;
        }}
        .share-active-badge {{
            display: inline-block;
            font-size: 0.65rem;
            padding: 0.1rem 0.3rem;
            border-radius: 3px;
            font-weight: 600;
        }}
        .share-active-badge.active {{ background: var(--green); color: white; }}
        .share-active-badge.inactive {{ background: var(--red); color: white; }}

        .citation-panel-footer .citation-stats {{
            flex: 1;
            font-size: 0.8rem;
            color: var(--muted);
        }}
    </style>
</head>
<body>
    <div class="viewer-container">
        <div class="viewer-header">
            <a href="/" class="back-link">&larr; All Notes</a>
            <h1>{title}</h1>
            <div class="font-size-controls" title="Font size">
                <label><input type="radio" name="font-size" value="14" onchange="setFontSize(14)"><span class="size-tiny">A</span></label>
                <label><input type="radio" name="font-size" value="16" onchange="setFontSize(16)"><span class="size-small">A</span></label>
                <label><input type="radio" name="font-size" value="18" onchange="setFontSize(18)"><span class="size-normal">A</span></label>
                <label><input type="radio" name="font-size" value="22" onchange="setFontSize(22)"><span class="size-large">A</span></label>
            </div>
            <div class="pdf-status" id="pdf-status">
                <button class="pdf-toggle-btn" id="mini-graph-btn" onclick="toggleMiniGraph()" title="Show local knowledge graph">Graph</button>
                {pdf_status_html}
            </div>
            {mode_toggle}
        </div>
        <div class="viewer-main">
            <div class="content-pane" id="content-pane">
                <div class="content-wrapper">
                    {meta_html}
                    <div class="note-content">{rendered_content}</div>
                    {time_html}
                    {sub_notes_html}
                    {history_html}
                </div>
            </div>
            <div id="split-divider"></div>
            <div id="pdf-viewer-pane">
                <div class="pdf-toolbar" id="pdf-toolbar">
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
                <div id="pdf-dropzone-viewer" style="display:none;">
                    <div class="pdf-drop-icon">Drop PDF here</div>
                    <div class="pdf-drop-or">or</div>
                    <button id="smart-find-btn" onclick="startSmartFind()">Find PDF automatically</button>
                    <div id="smart-find-status"></div>
                </div>
            </div>
        </div>
        <div class="mini-graph-panel" id="mini-graph-panel">
            <div class="mini-graph-panel-header" id="mini-graph-header">
                <span>Local Graph</span>
                <span>
                    <a href="/graph?q=from:{key}+depth:3" title="Open full graph view">Full Graph</a>
                    <button class="mini-graph-panel-close" onclick="closeMiniGraph()">&times;</button>
                </span>
            </div>
            <div class="mini-graph-body" id="mini-graph-body"></div>
        </div>
        <div class="share-panel" id="share-panel">
            <div class="citation-panel-header">
                <span>Share Note</span>
                <button class="citation-panel-close" onclick="closeSharePanel()">&times;</button>
            </div>
            <div class="citation-panel-body" id="share-panel-body">
                <div style="margin-bottom:1rem;">
                    <div style="font-size:0.85rem;font-weight:600;margin-bottom:0.5rem;">Create New Shared Copy</div>
                    <div style="font-size:0.8rem;color:var(--muted);margin-bottom:0.5rem;">
                        Add contributor names (optional):
                    </div>
                    <div id="contributor-fields">
                        <div style="display:flex;gap:0.3rem;margin-bottom:0.3rem;">
                            <input type="text" class="contrib-name-input" placeholder="Name (e.g., Yihao)" style="flex:1;padding:0.3rem 0.5rem;border:1px solid var(--border);border-radius:3px;font-size:0.8rem;background:var(--bg);color:var(--fg);">
                            <button class="pdf-toggle-btn" onclick="addContributorField()">+</button>
                        </div>
                    </div>
                    <button class="pdf-toggle-btn" style="margin-top:0.5rem;background:var(--link);color:white;border-color:var(--link);" onclick="createSharedNote()" id="create-share-btn">Create Shared Link</button>
                    <div id="create-share-status" style="font-size:0.8rem;margin-top:0.5rem;"></div>
                </div>
                <hr style="border:none;border-top:1px solid var(--border);margin:1rem 0;">
                <div style="font-size:0.85rem;font-weight:600;margin-bottom:0.5rem;">Existing Shared Links</div>
                <div id="share-list" style="font-size:0.8rem;color:var(--muted);">Loading...</div>
            </div>
        </div>
        <div class="citation-panel" id="citation-panel">
            <div class="citation-panel-header">
                <span>Citation Scan Results</span>
                <button class="citation-panel-close" onclick="closeCitationPanel()">&times;</button>
            </div>
            <div class="citation-panel-body" id="citation-panel-body"></div>
            <div class="citation-panel-footer" id="citation-panel-footer" style="display:none;">
                <span class="citation-stats" id="citation-stats"></span>
                <button class="pdf-toggle-btn" onclick="writeCitations()" id="write-citations-btn">Write to note</button>
            </div>
        </div>
    </div>

    <script src="https://d3js.org/d3.v7.min.js"></script>
    <script src="https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.min.js"></script>
    <script>
        // Set pdf.js worker
        pdfjsLib.GlobalWorkerOptions.workerSrc = 'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/pdf.worker.min.js';

        const noteKey = "{key}";
        const pdfFilename = {pdf_filename_json};
        const hasPdf = {has_pdf_json};
        const isPaper = {is_paper_json};
        const loggedIn = {logged_in_json};

        // Font size handling - shared with editor via localStorage
        // Editor uses Monaco sizes (11, 13, 15, 18), viewer scales up by 1.2x for readability
        const fontSizeMap = {{ 11: 14, 13: 16, 15: 18, 18: 22 }};
        const reverseFontSizeMap = {{ 14: 11, 16: 13, 18: 15, 22: 18 }};
        let editorFontSize = parseInt(localStorage.getItem('editorFontSize')) || 15;
        let currentFontSize = fontSizeMap[editorFontSize] || 18;

        function setFontSize(size) {{
            currentFontSize = size;
            // Save as editor font size for sharing
            const editorSize = reverseFontSizeMap[size] || 15;
            localStorage.setItem('editorFontSize', editorSize);
            applyFontSize();
        }}

        function applyFontSize() {{
            const content = document.querySelector('.note-content');
            if (content) {{
                content.style.fontSize = currentFontSize + 'px';
            }}
        }}

        function initFontSizeControls() {{
            const radios = document.querySelectorAll('input[name="font-size"]');
            radios.forEach(radio => {{
                if (parseInt(radio.value) === currentFontSize) {{
                    radio.checked = true;
                }}
            }});
            applyFontSize();
        }}

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
            const pane = document.getElementById('pdf-viewer-pane');
            const divider = document.getElementById('split-divider');
            const btn = document.getElementById('pdf-toggle-btn');

            pane.classList.add('active');
            if (divider) divider.classList.add('active');
            if (btn) btn.classList.add('active');

            // Apply saved split position
            applySplitPosition();

            if (pdfFilename) {{
                // Show PDF viewer, hide dropzone
                document.getElementById('pdf-canvas-container').style.display = '';
                document.getElementById('pdf-toolbar').style.display = '';
                const dropzone = document.getElementById('pdf-dropzone-viewer');
                if (dropzone) dropzone.style.display = 'none';

                // Restore state
                const savedState = restorePdfState();

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
            }} else if (isPaper && loggedIn) {{
                // Show dropzone, hide PDF viewer
                document.getElementById('pdf-canvas-container').style.display = 'none';
                document.getElementById('pdf-toolbar').style.display = 'none';
                const dropzone = document.getElementById('pdf-dropzone-viewer');
                if (dropzone) dropzone.style.display = 'flex';
            }}
        }}

        function hidePdfViewer() {{
            savePdfState();

            const pane = document.getElementById('pdf-viewer-pane');
            const divider = document.getElementById('split-divider');
            const btn = document.getElementById('pdf-toggle-btn');

            pane.classList.remove('active');
            if (divider) divider.classList.remove('active');
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
        const RENDER_BUFFER = 1;  // Render 1 page before/after visible

        // Setup placeholders for all pages (fast, no rendering)
        async function setupPagePlaceholders() {{
            if (!pdfDoc) return;

            const zoomWrapper = document.getElementById('pdf-zoom-wrapper');
            zoomWrapper.innerHTML = '';
            renderedPages = [];
            renderedPageNums.clear();

            // Get first page to determine dimensions
            const firstPage = await pdfDoc.getPage(1);
            const defaultViewport = firstPage.getViewport({{ scale: PDF_SCALE }});

            for (let pageNum = 1; pageNum <= totalPdfPages; pageNum++) {{
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

            zoomWrapper.style.transform = 'scale(' + currentZoom + ')';
            updatePageInfo();
            updateNavButtons();
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

            wrapper.style.width = Math.floor(viewport.width) + 'px';
            wrapper.style.height = Math.floor(viewport.height) + 'px';

            const dpr = window.devicePixelRatio || 1;

            const canvas = document.createElement('canvas');
            const ctx = canvas.getContext('2d');
            canvas.width = Math.floor(viewport.width * dpr);
            canvas.height = Math.floor(viewport.height * dpr);
            canvas.style.width = Math.floor(viewport.width) + 'px';
            canvas.style.height = Math.floor(viewport.height) + 'px';
            ctx.scale(dpr, dpr);
            wrapper.appendChild(canvas);

            const textLayerDiv = document.createElement('div');
            textLayerDiv.className = 'textLayer';
            wrapper.appendChild(textLayerDiv);

            // Set scale factor CSS variable for pdf.js text layer
            wrapper.style.setProperty('--scale-factor', viewport.scale);

            await page.render({{
                canvasContext: ctx,
                viewport: viewport
            }}).promise;

            const textContent = await page.getTextContent();
            pdfjsLib.renderTextLayer({{
                textContentSource: textContent,
                container: textLayerDiv,
                viewport: viewport,
                textDivs: []
            }});

            const idx = pageNum - 1;
            renderedPages[idx] = {{ wrapper, canvas, page, viewport }};
        }}

        // Render pages in/near the viewport
        async function renderVisiblePages() {{
            const container = document.getElementById('pdf-canvas-container');
            if (!container || !pdfDoc) return;

            const containerRect = container.getBoundingClientRect();
            const visiblePages = [];

            renderedPages.forEach(({{ wrapper }}, index) => {{
                const pageNum = index + 1;
                const rect = wrapper.getBoundingClientRect();
                const buffer = containerRect.height;
                if (rect.bottom > containerRect.top - buffer &&
                    rect.top < containerRect.bottom + buffer) {{
                    visiblePages.push(pageNum);
                }}
            }});

            const minPage = Math.max(1, currentPdfPage - RENDER_BUFFER);
            const maxPage = Math.min(totalPdfPages, currentPdfPage + RENDER_BUFFER);
            for (let p = minPage; p <= maxPage; p++) {{
                if (!visiblePages.includes(p)) visiblePages.push(p);
            }}

            for (const pageNum of visiblePages) {{
                await renderPage(pageNum);
            }}
        }}

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

                // Debounced lazy rendering
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

            const containerWidth = container.clientWidth - 32;
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

        let splitPosition = parseFloat(localStorage.getItem('pdfSplitPosition')) || 50; // percentage for content pane

        function setupSplitDivider() {{
            const divider = document.getElementById('split-divider');
            const contentPane = document.getElementById('content-pane');
            const pdfPane = document.getElementById('pdf-viewer-pane');
            const main = document.querySelector('.viewer-main');

            if (!divider || !contentPane || !pdfPane || !main) return;

            let isDragging = false;

            function updateSplitPosition(percent) {{
                splitPosition = Math.min(Math.max(percent, 20), 80); // Clamp between 20-80%
                const pdfWidth = 100 - splitPosition;

                contentPane.style.flex = '0 0 ' + splitPosition + '%';
                pdfPane.style.width = pdfWidth + '%';
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
            }});
        }}

        function applySplitPosition() {{
            const contentPane = document.getElementById('content-pane');
            const pdfPane = document.getElementById('pdf-viewer-pane');

            if (!contentPane || !pdfPane) return;

            const pdfWidth = 100 - splitPosition;
            contentPane.style.flex = '0 0 ' + splitPosition + '%';
            pdfPane.style.width = pdfWidth + '%';
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

        // =====================================================================
        // Unlink PDF
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

        // =====================================================================
        // Drag-and-Drop PDF Upload
        // =====================================================================

        function setupDropzone() {{
            const dropzone = document.getElementById('pdf-dropzone-viewer');
            if (!dropzone) return;

            dropzone.addEventListener('dragover', function(e) {{
                e.preventDefault();
                dropzone.classList.add('drag-over');
            }});
            dropzone.addEventListener('dragleave', function(e) {{
                e.preventDefault();
                dropzone.classList.remove('drag-over');
            }});
            dropzone.addEventListener('drop', function(e) {{
                e.preventDefault();
                dropzone.classList.remove('drag-over');
                const files = e.dataTransfer.files;
                if (files.length === 0) return;
                const file = files[0];
                if (!file.name.toLowerCase().endsWith('.pdf')) {{
                    alert('Please drop a PDF file.');
                    return;
                }}
                uploadPdfFile(file);
            }});
        }}

        async function uploadPdfFile(file) {{
            const status = document.getElementById('smart-find-status');
            if (status) status.innerHTML = '<span class="smart-find-spinner"></span> Uploading...';

            const formData = new FormData();
            formData.append('file', file);

            try {{
                const resp = await fetch('/api/pdf/upload?note_key=' + encodeURIComponent(noteKey), {{
                    method: 'POST',
                    body: formData
                }});
                if (resp.ok) {{
                    window.location.reload();
                }} else {{
                    const err = await resp.text();
                    if (status) status.textContent = 'Upload failed: ' + err;
                }}
            }} catch (e) {{
                if (status) status.textContent = 'Upload error: ' + e.message;
            }}
        }}

        // =====================================================================
        // Smart Find PDF
        // =====================================================================

        let smartFindInterval = null;

        async function startSmartFind() {{
            const btn = document.getElementById('smart-find-btn');
            const status = document.getElementById('smart-find-status');
            if (btn) btn.disabled = true;

            // Animated progress: cycle through source names so the UI feels alive
            const sources = ['arXiv', 'Semantic Scholar', 'Unpaywall'];
            let srcIdx = 0;
            if (status) status.innerHTML = '<span class="smart-find-spinner"></span> Checking ' + sources[0] + '...';
            smartFindInterval = setInterval(() => {{
                srcIdx = (srcIdx + 1) % sources.length;
                if (status) status.innerHTML = '<span class="smart-find-spinner"></span> Checking ' + sources[srcIdx] + '...';
            }}, 1500);

            try {{
                const resp = await fetch('/api/pdf/smart-find', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ note_key: noteKey }})
                }});

                clearInterval(smartFindInterval);
                const data = await resp.json();

                if (data.status === 'found') {{
                    const truncUrl = data.url.length > 60 ? data.url.substring(0, 60) + '...' : data.url;
                    if (status) {{
                        status.innerHTML = '<div class="smart-find-result">' +
                            '<span class="source-badge">' + (data.source || 'found') + '</span>' +
                            '<div class="found-url">' + truncUrl + '</div>' +
                            '<div class="smart-find-actions">' +
                            '<button class="btn-accept" onclick="acceptSmartFind(\'' + data.url.replace(/'/g, "\\'") + '\')">Download &amp; Attach</button>' +
                            '<button class="btn-cancel" onclick="cancelSmartFind()">Cancel</button>' +
                            '</div></div>';
                    }}
                }} else {{
                    if (status) status.textContent = data.error || 'Could not find PDF';
                    setTimeout(() => {{
                        if (status) status.textContent = '';
                        if (btn) btn.disabled = false;
                    }}, 3000);
                }}
            }} catch (e) {{
                clearInterval(smartFindInterval);
                if (status) status.textContent = 'Error: ' + e.message;
                if (btn) btn.disabled = false;
            }}
        }}

        async function acceptSmartFind(url) {{
            const status = document.getElementById('smart-find-status');
            if (status) status.innerHTML = '<span class="smart-find-spinner"></span> Downloading PDF...';

            try {{
                const resp = await fetch('/api/pdf/download-url', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ note_key: noteKey, url: url }})
                }});

                if (resp.ok) {{
                    window.location.reload();
                }} else {{
                    const err = await resp.text();
                    if (status) status.innerHTML = '<div style="color:var(--red);">Download failed: ' + err + '</div>' +
                        '<div class="smart-find-actions" style="margin-top:0.5rem;">' +
                        '<button class="btn-accept" onclick="startSmartFind()">Try Again</button>' +
                        '<button class="btn-cancel" onclick="cancelSmartFind()">Cancel</button>' +
                        '</div>';
                }}
            }} catch (e) {{
                if (status) status.innerHTML = '<div style="color:var(--red);">Error: ' + e.message + '</div>' +
                    '<div class="smart-find-actions" style="margin-top:0.5rem;">' +
                    '<button class="btn-accept" onclick="startSmartFind()">Try Again</button>' +
                    '<button class="btn-cancel" onclick="cancelSmartFind()">Cancel</button>' +
                    '</div>';
            }}
        }}

        function cancelSmartFind() {{
            const status = document.getElementById('smart-find-status');
            const btn = document.getElementById('smart-find-btn');
            if (status) status.innerHTML = '';
            if (btn) btn.disabled = false;
        }}

        // =====================================================================
        // Citation Scanning
        // =====================================================================

        async function scanReferences() {{
            const panel = document.getElementById('citation-panel');
            const body = document.getElementById('citation-panel-body');
            const footer = document.getElementById('citation-panel-footer');
            const stats = document.getElementById('citation-stats');

            panel.classList.add('active');
            body.innerHTML = '<div style="text-align:center;padding:1rem;color:var(--muted);"><span class="smart-find-spinner"></span> Scanning PDF references...</div>';
            footer.style.display = 'none';

            try {{
                const resp = await fetch('/api/citations/scan', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ note_key: noteKey, force: false }})
                }});

                if (!resp.ok) {{
                    const err = await resp.text();
                    body.innerHTML = '<div style="color:var(--red);padding:0.5rem;">' + err + '</div>';
                    return;
                }}

                const data = await resp.json();

                if (data.matches.length === 0) {{
                    body.innerHTML = '<div style="padding:0.5rem;color:var(--muted);">No matches found among ' + data.unmatched_count + ' references.</div>';
                    return;
                }}

                let html = '';
                for (const m of data.matches) {{
                    html += '<div class="citation-match">' +
                        '<span class="cm-key">[@' + m.target_key + ']</span>' +
                        '<span class="cm-type">' + m.match_type + ' ' + Math.round(m.confidence * 100) + '%</span>' +
                        '<span class="cm-title">' + (m.raw_text.substring(0, 120)) + '</span>' +
                        '</div>';
                }}
                body.innerHTML = html;

                stats.textContent = data.matches.length + ' match(es), ' + data.unmatched_count + ' unmatched';
                footer.style.display = 'flex';
            }} catch (e) {{
                body.innerHTML = '<div style="color:var(--red);padding:0.5rem;">Error: ' + e.message + '</div>';
            }}
        }}

        async function writeCitations() {{
            const btn = document.getElementById('write-citations-btn');
            btn.disabled = true;
            btn.textContent = 'Writing...';

            try {{
                const resp = await fetch('/api/citations/write', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ note_key: noteKey }})
                }});

                if (resp.ok) {{
                    window.location.reload();
                }} else {{
                    const err = await resp.text();
                    alert('Failed to write citations: ' + err);
                    btn.disabled = false;
                    btn.textContent = 'Write to note';
                }}
            }} catch (e) {{
                alert('Error: ' + e.message);
                btn.disabled = false;
                btn.textContent = 'Write to note';
            }}
        }}

        function closeCitationPanel() {{
            document.getElementById('citation-panel').classList.remove('active');
        }}

        // =====================================================================
        // Mini Knowledge Graph
        // =====================================================================

        let miniGraphLoaded = false;

        function toggleMiniGraph() {{
            const panel = document.getElementById('mini-graph-panel');
            const btn = document.getElementById('mini-graph-btn');
            if (panel.classList.contains('active')) {{
                closeMiniGraph();
            }} else {{
                panel.classList.add('active');
                btn.classList.add('active');
                if (!miniGraphLoaded) {{
                    loadMiniGraph();
                }}
            }}
        }}

        function closeMiniGraph() {{
            document.getElementById('mini-graph-panel').classList.remove('active');
            document.getElementById('mini-graph-btn').classList.remove('active');
            if (window._kgMiniSim) {{ window._kgMiniSim.stop(); }}
        }}

        function loadMiniGraph() {{
            const body = document.getElementById('mini-graph-body');
            body.innerHTML = '<div style="display:flex;align-items:center;justify-content:center;height:100%;color:var(--muted);font-size:0.85rem;"><span class="smart-find-spinner"></span> Loading graph...</div>';
            miniGraphLoaded = true;
            // The unified graph engine runs as an async IIFE injected below
            _kgMiniInit();
        }}

        // Draggable mini-graph panel
        (function() {{
            const panel = document.getElementById('mini-graph-panel');
            const header = document.getElementById('mini-graph-header');
            if (!panel || !header) return;
            let dragging = false, startX, startY, origLeft, origTop;
            header.addEventListener('mousedown', function(e) {{
                if (e.target.tagName === 'A' || e.target.tagName === 'BUTTON') return;
                dragging = true;
                const r = panel.getBoundingClientRect();
                startX = e.clientX; startY = e.clientY;
                origLeft = r.left; origTop = r.top;
                document.body.style.userSelect = 'none';
            }});
            document.addEventListener('mousemove', function(e) {{
                if (!dragging) return;
                panel.style.left = (origLeft + e.clientX - startX) + 'px';
                panel.style.top = (origTop + e.clientY - startY) + 'px';
                panel.style.right = 'auto';
                panel.style.bottom = 'auto';
            }});
            document.addEventListener('mouseup', function() {{
                dragging = false;
                document.body.style.userSelect = '';
            }});
        }})();

        // =====================================================================
        // Share Panel
        // =====================================================================

        function openSharePanel(noteKeyParam) {{
            const panel = document.getElementById('share-panel');
            panel.classList.add('active');
            loadShareList();
        }}

        function closeSharePanel() {{
            document.getElementById('share-panel').classList.remove('active');
        }}

        function addContributorField() {{
            const container = document.getElementById('contributor-fields');
            const div = document.createElement('div');
            div.style = 'display:flex;gap:0.3rem;margin-bottom:0.3rem;';
            div.innerHTML = '<input type="text" class="contrib-name-input" placeholder="Name" style="flex:1;padding:0.3rem 0.5rem;border:1px solid var(--border);border-radius:3px;font-size:0.8rem;background:var(--bg);color:var(--fg);">' +
                '<button class="pdf-toggle-btn" onclick="this.parentElement.remove()">-</button>';
            container.appendChild(div);
        }}

        async function createSharedNote() {{
            const btn = document.getElementById('create-share-btn');
            const status = document.getElementById('create-share-status');
            btn.disabled = true;
            status.textContent = 'Creating...';
            status.style.color = 'var(--muted)';

            const nameInputs = document.querySelectorAll('.contrib-name-input');
            const contributors = [];
            nameInputs.forEach(input => {{
                const name = input.value.trim();
                if (name) contributors.push({{ name: name }});
            }});

            try {{
                const resp = await fetch('/api/shared/create', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ note_key: noteKey, contributors: contributors }})
                }});

                if (resp.ok) {{
                    const data = await resp.json();
                    const fullUrl = window.location.origin + data.url;
                    status.innerHTML = '<span style="color:var(--green);">Created!</span> <a href="' + data.url + '" target="_blank" style="color:var(--link);">' + fullUrl + '</a>';
                    navigator.clipboard.writeText(fullUrl).then(() => {{
                        status.innerHTML += ' <span style="color:var(--muted);">(copied)</span>';
                    }});
                    loadShareList();
                }} else {{
                    const err = await resp.text();
                    status.textContent = 'Error: ' + err;
                    status.style.color = 'var(--red)';
                }}
            }} catch (e) {{
                status.textContent = 'Error: ' + e.message;
                status.style.color = 'var(--red)';
            }}

            btn.disabled = false;
        }}

        async function loadShareList() {{
            const container = document.getElementById('share-list');

            try {{
                const resp = await fetch('/api/shared/list/' + encodeURIComponent(noteKey));
                if (!resp.ok) {{
                    container.textContent = 'Failed to load';
                    return;
                }}

                const shares = await resp.json();
                if (shares.length === 0) {{
                    container.textContent = 'No shared links yet.';
                    return;
                }}

                let html = '';
                for (const share of shares) {{
                    const fullUrl = window.location.origin + '/shared/' + share.share_token;
                    const truncToken = share.share_token.substring(0, 8) + '...';
                    const activeBadge = share.active
                        ? '<span class="share-active-badge active">Active</span>'
                        : '<span class="share-active-badge inactive">Inactive</span>';
                    const contribs = share.contributors.map(c => c.name).join(', ') || 'No contributors';
                    const created = new Date(share.created_at).toLocaleDateString();

                    html += '<div class="share-link-item">' +
                        '<div>' + activeBadge + ' <span class="share-link-url" onclick="navigator.clipboard.writeText(\'' + fullUrl + '\');this.style.color=\'var(--green)\';setTimeout(()=>this.style.color=\'\',1000);" title="Click to copy">' + truncToken + '</span></div>' +
                        '<div class="share-link-meta">' + contribs + ' &middot; ' + created + '</div>' +
                        '<div class="share-link-actions">' +
                        '<a href="/shared/' + share.share_token + '" target="_blank" class="pdf-toggle-btn" style="text-decoration:none;font-size:0.7rem;">Open</a>' +
                        '<button class="pdf-toggle-btn" style="font-size:0.7rem;" onclick="toggleShareActive(\'' + share.share_token + '\')">' + (share.active ? 'Deactivate' : 'Activate') + '</button>' +
                        '</div></div>';
                }}
                container.innerHTML = html;
            }} catch (e) {{
                container.textContent = 'Error: ' + e.message;
            }}
        }}

        async function toggleShareActive(token) {{
            try {{
                const resp = await fetch('/api/shared/' + token + '/deactivate', {{ method: 'POST' }});
                if (resp.ok) {{
                    loadShareList();
                }}
            }} catch (e) {{
                console.error('Failed to toggle share:', e);
            }}
        }}

        // Initialize on page load
        document.addEventListener('DOMContentLoaded', function() {{
            setupScrollTracking();
            setupSplitDivider();
            initFontSizeControls();
            setupDropzone();

            if (pdfFilename) {{
                // Auto-open PDF pane (unless user explicitly closed it)
                const savedState = restorePdfState();
                if (!savedState || savedState.visible !== false) {{
                    showPdfViewer();
                }}
            }} else if (isPaper && loggedIn) {{
                // Auto-open dropzone pane for paper notes without PDF
                showPdfViewer();
            }}
        }});

        // Save state before leaving
        window.addEventListener('beforeunload', function() {{
            savePdfState();
        }});
    </script>
    {mini_graph_script}
</body>
</html>"##,
        title = html_escape(&note.title),
        key = note.key,
        pdf_filename_json = pdf_filename_json,
        has_pdf_json = if has_pdf { "true" } else { "false" },
        is_paper_json = if is_paper { "true" } else { "false" },
        logged_in_json = if logged_in { "true" } else { "false" },
        pdf_status_html = pdf_status_html,
        mode_toggle = mode_toggle,
        meta_html = meta_html,
        rendered_content = rendered_content,
        time_html = time_html,
        sub_notes_html = sub_notes_html,
        history_html = history_html,
        mini_graph_css = graph_css(),
        mini_graph_script = render_graph_js(&GraphRendererConfig {
            container_selector: "#mini-graph-body".into(),
            center_key: Some(note.key.clone()),
            is_mini: true,
            logged_in,
            show_arrows: false,
            show_edge_tooltips: true,
            auto_fit: true,
            max_nodes: 30,
            data_source: GraphDataSource::FetchUrl {
                url: format!("/api/graph?q=from:{}+depth:3", note.key),
            },
            notes_json: None,
        }),
    )
}
