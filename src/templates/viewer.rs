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
        .mini-graph-body .mg-link {{ stroke: #93a1a1; stroke-opacity: 0.3; }}
        .mini-graph-body .mg-link.deg1 {{ stroke: #073642; stroke-opacity: 0.7; stroke-width: 1.5px; }}
        .mini-graph-body .mg-link.citation {{ stroke-dasharray: 4,2; stroke: #b58900; stroke-opacity: 0.5; }}
        .mini-graph-body .mg-link.citation.deg1 {{ stroke-opacity: 0.8; stroke-width: 1.5px; }}
        .mini-graph-body .mg-link.highlighted {{ stroke: var(--fg); stroke-opacity: 0.9; stroke-width: 2.5px; }}
        .mini-graph-body .mg-node circle {{ cursor: pointer; stroke: var(--bg); stroke-width: 1.5px; }}
        .mini-graph-body .mg-node .mg-label {{
            font-size: 8px;
            fill: var(--fg);
            pointer-events: none;
            text-anchor: middle;
            opacity: 0.7;
        }}
        .mini-graph-body .mg-node.center .mg-label {{ opacity: 1; font-size: 10px; font-weight: 600; }}
        .mini-graph-body .mg-node:hover .mg-label {{ opacity: 1; font-size: 10px; }}
        .mg-tooltip {{
            position: absolute;
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 4px;
            padding: 0.4rem 0.6rem;
            font-size: 0.8rem;
            pointer-events: none;
            z-index: 1001;
            box-shadow: 0 2px 6px rgba(0,0,0,0.15);
            max-width: 250px;
        }}
        .mg-tooltip .mgt-title {{ font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }}
        .mg-tooltip .mgt-meta {{ font-size: 0.75rem; color: var(--muted); margin-top: 0.15rem; }}
        .mg-legend {{
            position: absolute;
            bottom: 6px;
            left: 6px;
            font-size: 0.7rem;
            color: var(--muted);
            display: flex;
            gap: 0.6rem;
            align-items: center;
        }}
        .mg-legend-item {{
            display: flex;
            align-items: center;
            gap: 0.2rem;
        }}
        .mg-legend-dot {{
            width: 8px;
            height: 8px;
            border-radius: 50%;
            display: inline-block;
        }}

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

            await page.render({{
                canvasContext: ctx,
                viewport: viewport
            }}).promise;

            const textContent = await page.getTextContent();
            pdfjsLib.renderTextLayer({{
                textContent: textContent,
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
                    if (status) status.textContent = 'Download failed: ' + err;
                }}
            }} catch (e) {{
                if (status) status.textContent = 'Error: ' + e.message;
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
        let miniGraphSim = null;

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
            if (miniGraphSim) {{ miniGraphSim.stop(); }}
        }}

        async function loadMiniGraph() {{
            const body = document.getElementById('mini-graph-body');
            body.innerHTML = '<div style="display:flex;align-items:center;justify-content:center;height:100%;color:var(--muted);font-size:0.85rem;"><span class="smart-find-spinner"></span> Loading graph...</div>';

            try {{
                const resp = await fetch('/api/graph?q=from:' + encodeURIComponent(noteKey) + '+depth:3');
                if (!resp.ok) {{ body.innerHTML = '<div style="padding:1rem;color:var(--red);">Failed to load graph</div>'; return; }}

                const data = await resp.json();
                miniGraphLoaded = true;
                renderMiniGraph(data);
            }} catch (e) {{
                body.innerHTML = '<div style="padding:1rem;color:var(--red);">Error: ' + e.message + '</div>';
            }}
        }}

        function renderMiniGraph(data) {{
            const container = document.getElementById('mini-graph-body');
            container.innerHTML = '';

            const rect = container.getBoundingClientRect();
            const width = rect.width || 460;
            const height = rect.height || 360;

            // --- BFS to compute distance from center node ---
            const adj = {{}};
            data.nodes.forEach(n => {{ adj[n.id] = []; }});
            data.edges.forEach(e => {{
                const sid = typeof e.source === 'object' ? e.source.id : e.source;
                const tid = typeof e.target === 'object' ? e.target.id : e.target;
                if (adj[sid]) adj[sid].push(tid);
                if (adj[tid]) adj[tid].push(sid);
            }});
            const dist = {{}};
            dist[noteKey] = 0;
            const queue = [noteKey];
            let qi = 0;
            while (qi < queue.length) {{
                const cur = queue[qi++];
                (adj[cur] || []).forEach(nb => {{
                    if (dist[nb] === undefined) {{
                        dist[nb] = dist[cur] + 1;
                        queue.push(nb);
                    }}
                }});
            }}
            data.nodes.forEach(n => {{
                n._dist = dist[n.id] !== undefined ? dist[n.id] : 99;
            }});

            // --- Prune distant nodes when graph is too large ---
            // Always keep center + all 1st degree. If > 30 nodes, trim from furthest distance inward.
            const MAX_NODES = 30;
            const firstDegreeNodes = data.nodes.filter(n => n._dist <= 1);
            let keepNodes;
            if (data.nodes.length <= MAX_NODES) {{
                keepNodes = data.nodes;
            }} else {{
                // Always keep dist 0 and 1. Fill remaining budget from dist 2, then 3, etc.
                const budget = Math.max(MAX_NODES, firstDegreeNodes.length);
                const byDist = {{}};
                data.nodes.forEach(n => {{
                    if (n._dist > 1) {{
                        if (!byDist[n._dist]) byDist[n._dist] = [];
                        byDist[n._dist].push(n);
                    }}
                }});
                keepNodes = [...firstDegreeNodes];
                const distances = Object.keys(byDist).map(Number).sort((a, b) => a - b);
                for (const d of distances) {{
                    if (keepNodes.length >= budget) break;
                    const remaining = budget - keepNodes.length;
                    const candidates = byDist[d];
                    if (candidates.length <= remaining) {{
                        keepNodes.push(...candidates);
                    }} else {{
                        // Take a random sample to avoid bias
                        candidates.sort(() => Math.random() - 0.5);
                        keepNodes.push(...candidates.slice(0, remaining));
                    }}
                }}
            }}
            const keepIds = new Set(keepNodes.map(n => n.id));
            data.nodes = keepNodes;
            data.edges = data.edges.filter(e => {{
                const sid = typeof e.source === 'object' ? e.source.id : e.source;
                const tid = typeof e.target === 'object' ? e.target.id : e.target;
                return keepIds.has(sid) && keepIds.has(tid);
            }});

            // --- Color palette by distance ---
            const distColors = ['#dc322f', '#cb4b16', '#268bd2', '#93a1a1'];
            function nodeColor(d) {{
                return distColors[Math.min(d._dist, distColors.length - 1)];
            }}
            function nodeRadius(d) {{
                if (d._dist === 0) return 16;
                if (d._dist === 1) return 10;
                if (d._dist === 2) return 7;
                return 5;
            }}
            function nodeOpacity(d) {{
                if (d._dist === 0) return 1;
                if (d._dist === 1) return 0.95;
                if (d._dist === 2) return 0.7;
                return 0.45;
            }}

            const svg = d3.select(container).append('svg');
            const g = svg.append('g');

            // Tooltip
            const tip = d3.select(container).append('div')
                .attr('class', 'mg-tooltip')
                .style('display', 'none');

            // Pin center node to middle
            const centerNode = data.nodes.find(n => n.id === noteKey);
            if (centerNode) {{
                centerNode.fx = width / 2;
                centerNode.fy = height / 2;
            }}

            // Count 1st degree neighbors for link distance tuning
            const deg1Count = firstDegreeNodes.length - 1; // exclude center
            // Spread 1st degree nodes in a ring that fits the viewport
            const ringRadius = Math.min(width, height) * 0.3;
            const linkDist1 = Math.max(60, ringRadius);

            // Simulation tuned so 1st degree fills viewport nicely
            const sim = d3.forceSimulation(data.nodes)
                .force('link', d3.forceLink(data.edges).id(d => d.id).distance(d => {{
                    const s = d.source._dist !== undefined ? d.source._dist : 1;
                    const t = d.target._dist !== undefined ? d.target._dist : 1;
                    const maxDist = Math.max(s, t);
                    if (maxDist <= 1) return linkDist1;
                    return linkDist1 * 0.6 + maxDist * 20;
                }}).strength(d => {{
                    const s = d.source._dist !== undefined ? d.source._dist : 1;
                    const t = d.target._dist !== undefined ? d.target._dist : 1;
                    return Math.max(s, t) <= 1 ? 1.0 : 0.3;
                }}))
                .force('charge', d3.forceManyBody().strength(d => {{
                    if (d._dist === 0) return -400;
                    if (d._dist === 1) return -200;
                    return -60;
                }}))
                .force('center', d3.forceCenter(width / 2, height / 2).strength(0.05))
                .force('collision', d3.forceCollide().radius(d => nodeRadius(d) + 6))
                .force('radial', d3.forceRadial(d => {{
                    if (d._dist === 0) return 0;
                    if (d._dist === 1) return ringRadius;
                    return ringRadius + d._dist * 60;
                }}, width / 2, height / 2).strength(d => d._dist <= 1 ? 0.6 : 0.2));
            miniGraphSim = sim;

            // Links — 1st degree edges (touching center) get 'deg1' class for visibility
            const link = g.append('g').selectAll('line')
                .data(data.edges).join('line')
                .attr('class', d => {{
                    const sid = typeof d.source === 'object' ? d.source.id : d.source;
                    const tid = typeof d.target === 'object' ? d.target.id : d.target;
                    const isDeg1 = sid === noteKey || tid === noteKey;
                    let cls = 'mg-link';
                    if (isDeg1) cls += ' deg1';
                    if (d.edge_type === 'citation') cls += ' citation';
                    return cls;
                }});

            // Nodes — sorted so center renders on top
            const sortedNodes = [...data.nodes].sort((a, b) => b._dist - a._dist);
            const node = g.append('g').selectAll('g')
                .data(sortedNodes, d => d.id).join('g')
                .attr('class', d => 'mg-node' + (d._dist === 0 ? ' center' : ''))
                .style('opacity', d => nodeOpacity(d))
                .call(d3.drag()
                    .on('start', (e, d) => {{ if (!e.active) sim.alphaTarget(0.3).restart(); d.fx = d.x; d.fy = d.y; }})
                    .on('drag', (e, d) => {{ d.fx = e.x; d.fy = e.y; }})
                    .on('end', (e, d) => {{
                        if (!e.active) sim.alphaTarget(0);
                        if (d.id !== noteKey) {{ d.fx = null; d.fy = null; }}
                    }}));

            node.append('circle')
                .attr('r', d => nodeRadius(d))
                .attr('fill', d => nodeColor(d));

            // Labels — "Smith et al." style, hide for 3rd+ degree to reduce clutter
            node.append('text')
                .attr('class', 'mg-label')
                .text(d => {{
                    if (d._dist >= 3) return '';
                    return d.short_label || d.title.substring(0, 14);
                }})
                .attr('dy', d => -(nodeRadius(d) + 3));

            // Hover
            node.on('mouseover', function(event, d) {{
                d3.select(this).raise().select('circle').attr('stroke', 'var(--fg)').attr('stroke-width', 2.5);
                // Show label on hover for nodes without one
                if (d._dist >= 3) {{
                    d3.select(this).select('text').text(d.short_label || d.title.substring(0, 20));
                }}
                link.classed('highlighted', l => l.source.id === d.id || l.target.id === d.id);
                const distLabel = d._dist === 0 ? 'center' : d._dist + (d._dist === 1 ? 'st' : d._dist === 2 ? 'nd' : d._dist === 3 ? 'rd' : 'th') + ' degree';
                tip.style('display', 'block')
                    .html('<div class="mgt-title">' + d.title + '</div><div class="mgt-meta">' + d.node_type + ' \u00b7 ' + distLabel + '</div>')
                    .style('left', (event.offsetX + 14) + 'px')
                    .style('top', (event.offsetY - 10) + 'px');
            }})
            .on('mouseout', function(event, d) {{
                d3.select(this).select('circle').attr('stroke', 'var(--bg)').attr('stroke-width', 1.5);
                if (d._dist >= 3) {{
                    d3.select(this).select('text').text('');
                }}
                link.classed('highlighted', false);
                tip.style('display', 'none');
            }})
            .on('click', function(event, d) {{
                if (d.id !== noteKey) {{
                    window.location.href = '/note/' + d.id;
                }}
            }});

            sim.on('tick', () => {{
                link.attr('x1', d => d.source.x).attr('y1', d => d.source.y)
                    .attr('x2', d => d.target.x).attr('y2', d => d.target.y);
                node.attr('transform', d => 'translate(' + d.x + ',' + d.y + ')');
            }});

            // After simulation stabilizes, auto-fit so 1st degree nodes fill viewport
            sim.on('end', () => {{
                // Compute bounding box of 1st degree nodes
                const deg1Nodes = data.nodes.filter(n => n._dist <= 1);
                if (deg1Nodes.length < 2) return;
                let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
                deg1Nodes.forEach(n => {{
                    minX = Math.min(minX, n.x);
                    maxX = Math.max(maxX, n.x);
                    minY = Math.min(minY, n.y);
                    maxY = Math.max(maxY, n.y);
                }});
                const pad = 60;
                const bw = (maxX - minX) + pad * 2;
                const bh = (maxY - minY) + pad * 2;
                const scale = Math.min(width / bw, height / bh, 2.0);
                const cx = (minX + maxX) / 2;
                const cy = (minY + maxY) / 2;
                const tx = width / 2 - cx * scale;
                const ty = height / 2 - cy * scale;
                const transform = d3.zoomIdentity.translate(tx, ty).scale(scale);
                svg.transition().duration(500).call(
                    d3.zoom().scaleExtent([0.2, 5]).on('zoom', e => {{
                        g.attr('transform', e.transform);
                    }}).transform, transform
                );
            }});

            // Zoom — apply to single group
            svg.call(d3.zoom().scaleExtent([0.2, 5]).on('zoom', e => {{
                g.attr('transform', e.transform);
            }}));

            // Legend
            const legend = d3.select(container).append('div').attr('class', 'mg-legend');
            [['Center', distColors[0]], ['1st', distColors[1]], ['2nd', distColors[2]], ['3rd+', distColors[3]]].forEach(([label, color]) => {{
                const item = legend.append('span').attr('class', 'mg-legend-item');
                item.append('span').attr('class', 'mg-legend-dot').style('background', color);
                item.append('span').text(label);
            }});
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
    )
}
