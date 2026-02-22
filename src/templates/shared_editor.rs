//! Shared editor overlay scripts.
//!
//! These functions generate `<script>` blocks that are injected into the
//! existing editor and viewer templates to add collaborative features
//! (WebSocket sync, attribution decorations, peer count, etc.)
//! This way the shared pages reuse the exact same editor/viewer templates.

/// Generate the overlay `<script>` block for the shared EDITOR page.
/// Injected before `</body>` in the output of `render_editor`.
///
/// This script:
/// - Connects to the WebSocket for real-time sync
/// - Intercepts Monaco edits and sends them to the server
/// - Applies remote edits from other clients
/// - Adds attribution line decorations (colored left borders + hover)
/// - Modifies the header: hides save/git/emacs/done, adds shared badge/peer count/name input
/// - Disables the normal auto-save-to-disk mechanism
pub fn render_shared_overlay(token: &str, contributors_json: &str) -> String {
    format!(
        r##"<script>
(function() {{
    'use strict';

    const SHARE_TOKEN = "{token}";
    const CONTRIBUTORS = {contributors_json};
    const contributorColors = {{}};
    const contributorNames = {{}};
    CONTRIBUTORS.forEach(c => {{ contributorColors[c.id] = c.color; contributorNames[c.id] = c.name; }});

    // ---- Solarized color palette for picker ----
    const PALETTE = ['#268bd2','#d33682','#859900','#cb4b16','#6c71c4','#2aa198','#b58900','#dc322f'];

    // ---- Cookie-based identity ----
    function readIdentityCookie() {{
        const m = document.cookie.match(/(?:^|; )sharedIdentity=([^;]*)/);
        if (!m) return null;
        try {{ return JSON.parse(decodeURIComponent(m[1])); }} catch(e) {{ return null; }}
    }}

    function writeIdentityCookie(identity) {{
        const val = encodeURIComponent(JSON.stringify(identity));
        document.cookie = 'sharedIdentity=' + val + ';path=/;max-age=31536000;SameSite=Lax';
    }}

    function getOrCreateIdentity() {{
        let identity = readIdentityCookie();
        if (!identity || !identity.id) {{
            const id = Array.from(crypto.getRandomValues(new Uint8Array(4))).map(b => b.toString(16).padStart(2,'0')).join('');
            identity = {{ id: id, name: '', color: PALETTE[0] }};
            writeIdentityCookie(identity);
        }}
        return identity;
    }}

    let currentIdentity = getOrCreateIdentity();

    function sendIdentify() {{
        if (ws && ws.readyState === WebSocket.OPEN) {{
            ws.send(JSON.stringify({{ type: 'identify', id: currentIdentity.id, name: currentIdentity.name, color: currentIdentity.color }}));
        }}
    }}

    // ---- Header modifications ----
    // Wait for DOM, then modify the header
    function modifyHeader() {{
        const header = document.querySelector('.editor-header');
        if (!header) return;

        // Hide elements not relevant for shared mode
        const backLink = header.querySelector('.back-link');
        if (backLink) backLink.style.display = 'none';

        const emacsBadge = document.getElementById('emacs-badge');
        if (emacsBadge) emacsBadge.style.display = 'none';

        const gitToggle = header.querySelector('.git-mode-toggle');
        if (gitToggle) gitToggle.style.display = 'none';

        // Hide knowledge graph button and panel
        const graphBtn = document.getElementById('mini-graph-btn');
        if (graphBtn) graphBtn.style.display = 'none';
        const graphPanel = document.getElementById('mini-graph-panel');
        if (graphPanel) graphPanel.style.display = 'none';

        // Keep the Save button visible — it will use sharedSaveHandler

        // Change "Done" button to link to shared view mode
        const doneBtn = header.querySelector('a.btn');
        if (doneBtn) {{
            doneBtn.href = '/shared/' + SHARE_TOKEN;
            doneBtn.textContent = 'View';
            doneBtn.onclick = null;
        }}

        // Add shared badge
        const badge = document.createElement('span');
        badge.style.cssText = 'font-size:0.65rem;font-weight:600;padding:0.2rem 0.5rem;background:#268bd2;color:#fdf6e3;border-radius:3px;letter-spacing:0.05em;';
        badge.textContent = 'SHARED';
        header.insertBefore(badge, header.children[0]);

        // ---- Identity bar: color picker + name input ----
        const identityBar = document.createElement('div');
        identityBar.style.cssText = 'display:flex;align-items:center;gap:0.4rem;';

        // Color picker: 8 small circles
        const colorPicker = document.createElement('div');
        colorPicker.style.cssText = 'display:flex;align-items:center;gap:0.25rem;';
        PALETTE.forEach(color => {{
            const swatch = document.createElement('span');
            swatch.style.cssText = 'width:16px;height:16px;border-radius:50%;cursor:pointer;display:inline-block;background:' + color + ';border:2px solid transparent;transition:border-color 0.15s;';
            if (color === currentIdentity.color) {{
                swatch.style.borderColor = '#fdf6e3';
                swatch.style.boxShadow = '0 0 0 2px ' + color;
            }}
            swatch.onclick = function() {{
                currentIdentity.color = color;
                writeIdentityCookie(currentIdentity);
                // Update all swatches
                colorPicker.querySelectorAll('span').forEach(s => {{ s.style.borderColor = 'transparent'; s.style.boxShadow = 'none'; }});
                swatch.style.borderColor = '#fdf6e3';
                swatch.style.boxShadow = '0 0 0 2px ' + color;
                sendIdentify();
            }};
            colorPicker.appendChild(swatch);
        }});
        identityBar.appendChild(colorPicker);

        // Name input
        const nameInput = document.createElement('input');
        nameInput.type = 'text';
        nameInput.placeholder = 'Your name';
        nameInput.title = 'Enter your name for attribution';
        nameInput.style.cssText = 'padding:0.3rem 0.6rem;border:1px solid #93a1a1;border-radius:4px;background:#fdf6e3;color:#657b83;font-size:0.8rem;font-family:inherit;width:120px;';
        nameInput.value = currentIdentity.name || '';
        nameInput.addEventListener('blur', function() {{
            const name = this.value.trim();
            currentIdentity.name = name;
            writeIdentityCookie(currentIdentity);
            sendIdentify();
        }});
        nameInput.addEventListener('keydown', function(e) {{
            if (e.key === 'Enter') this.blur();
        }});
        identityBar.appendChild(nameInput);

        // Add peer count indicator
        const peerDiv = document.createElement('div');
        peerDiv.style.cssText = 'font-size:0.8rem;color:#93a1a1;display:flex;align-items:center;gap:0.3rem;';
        peerDiv.innerHTML = '<span style="width:8px;height:8px;border-radius:50%;background:#859900;display:inline-block;"></span><span id="shared-peer-count">1</span>';

        // Add connection status
        const connStatus = document.createElement('span');
        connStatus.id = 'shared-conn-status';
        connStatus.style.cssText = 'font-size:0.75rem;color:#b58900;';
        connStatus.textContent = 'Connecting...';

        // Add copy link button
        const copyBtn = document.createElement('button');
        copyBtn.className = 'btn';
        copyBtn.textContent = 'Copy Link';
        copyBtn.onclick = function() {{
            navigator.clipboard.writeText(window.location.origin + '/shared/' + SHARE_TOKEN).then(() => {{
                copyBtn.textContent = 'Copied!';
                setTimeout(() => copyBtn.textContent = 'Copy Link', 2000);
            }});
        }};

        // Insert before the status area
        const statusEl = document.getElementById('editor-status');
        if (statusEl) {{
            statusEl.parentNode.insertBefore(identityBar, statusEl);
            statusEl.parentNode.insertBefore(peerDiv, statusEl);
            statusEl.parentNode.insertBefore(connStatus, statusEl);
            statusEl.parentNode.insertBefore(copyBtn, statusEl);
            // Repurpose status area for sync feedback
            const textEl = document.getElementById('status-text');
            if (textEl) textEl.textContent = 'Synced';
            statusEl.className = 'editor-status saved';
        }}
    }}

    // ---- WebSocket connection ----
    let ws;
    let isRemoteChange = false;
    let reconnectTimer = null;
    let reconnectDelay = 1000;
    let decorationIds = [];
    const attribStyles = new Set();

    function setConnStatus(status, text) {{
        const el = document.getElementById('shared-conn-status');
        if (!el) return;
        el.textContent = text;
        el.style.color = status === 'connected' ? '#859900' : status === 'disconnected' ? '#dc322f' : '#b58900';
    }}

    function connectWebSocket() {{
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = protocol + '//' + window.location.host + '/shared/' + SHARE_TOKEN + '/ws';
        setConnStatus('connecting', 'Connecting...');

        ws = new WebSocket(wsUrl);

        ws.onopen = function() {{
            setConnStatus('connected', 'Connected');
            reconnectDelay = 1000;
            sendIdentify();
        }};

        ws.onmessage = function(event) {{
            try {{
                const msg = JSON.parse(event.data);
                switch (msg.type) {{
                    case 'init':
                        // Set editor content from server
                        if (window.editor) {{
                            isRemoteChange = true;
                            window.editor.setValue(msg.text || '');
                            isRemoteChange = false;
                            window.hasUnsavedChanges = false;
                        }}
                        // Set initial peer count
                        if (msg.peers !== undefined) {{
                            const el = document.getElementById('shared-peer-count');
                            if (el) el.textContent = msg.peers;
                        }}
                        // Parse initial attribution
                        if (msg.attribution) {{
                            try {{
                                const attrib = typeof msg.attribution === 'string' ? JSON.parse(msg.attribution) : msg.attribution;
                                updateAttributionDecorations(attrib);
                            }} catch(e) {{}}
                        }}
                        break;

                    case 'text_update':
                        // Remote edit from another client
                        if (window.editor && msg.text !== undefined) {{
                            applyRemoteText(msg.text);
                        }}
                        break;

                    case 'peers':
                        try {{
                            const data = typeof msg.data === 'string' ? JSON.parse(msg.data) : msg.data;
                            const el = document.getElementById('shared-peer-count');
                            if (el && data.count !== undefined) el.textContent = data.count;
                        }} catch(e) {{}}
                        break;

                    case 'attribution':
                        try {{
                            const data = typeof msg.data === 'string' ? JSON.parse(msg.data) : msg.data;
                            updateAttributionDecorations(data);
                        }} catch(e) {{}}
                        break;

                    case 'saved':
                        {{
                            const statusEl = document.getElementById('editor-status');
                            const textEl = document.getElementById('status-text');
                            if (statusEl) statusEl.className = 'editor-status saved';
                            if (textEl) {{
                                const now = new Date();
                                const timeStr = now.toLocaleTimeString('en-US', {{ hour: 'numeric', minute: '2-digit' }});
                                textEl.textContent = 'Saved at ' + timeStr;
                            }}
                        }}
                        break;

                    case 'contributors':
                        try {{
                            const data = typeof msg.data === 'string' ? JSON.parse(msg.data) : msg.data;
                            if (Array.isArray(data)) {{
                                data.forEach(c => {{ contributorColors[c.id] = c.color; contributorNames[c.id] = c.name; }});
                            }}
                            // Re-run attribution decorations with updated colors/names
                            if (window._lastAttribution) updateAttributionDecorations(window._lastAttribution);
                        }} catch(e) {{}}
                        break;
                }}
            }} catch(e) {{
                console.error('WS parse error:', e);
            }}
        }};

        ws.onclose = function() {{
            setConnStatus('disconnected', 'Disconnected');
            reconnectTimer = setTimeout(() => {{
                reconnectDelay = Math.min(reconnectDelay * 2, 30000);
                connectWebSocket();
            }}, reconnectDelay);
        }};

        ws.onerror = function() {{
            console.error('WebSocket error');
        }};
    }}

    // ---- Remote text application (minimal diff) ----
    function applyRemoteText(newText) {{
        if (!window.editor) return;
        const model = window.editor.getModel();
        const currentText = model.getValue();
        if (currentText === newText) return;

        isRemoteChange = true;

        // Find first difference
        let start = 0;
        while (start < currentText.length && start < newText.length && currentText[start] === newText[start]) start++;

        // Find last difference
        let endCur = currentText.length;
        let endNew = newText.length;
        while (endCur > start && endNew > start && currentText[endCur - 1] === newText[endNew - 1]) {{ endCur--; endNew--; }}

        const startPos = model.getPositionAt(start);
        const endPos = model.getPositionAt(endCur);

        window.editor.executeEdits('remote', [{{
            range: new monaco.Range(startPos.lineNumber, startPos.column, endPos.lineNumber, endPos.column),
            text: newText.substring(start, endNew),
            forceMoveMarkers: false
        }}]);

        isRemoteChange = false;
    }}

    // ---- Send local edits ----
    function sendEdit(ops) {{
        if (ws && ws.readyState === WebSocket.OPEN) {{
            ws.send(JSON.stringify({{ type: 'edit', ops: ops }}));
        }}
    }}

    // ---- Attribution decorations ----
    function ensureAttribStyle(className, color) {{
        if (attribStyles.has(className)) return;
        attribStyles.add(className);
        const style = document.createElement('style');
        style.textContent = '.' + className + ' {{ border-left: 3px solid ' + color + ' !important; }}';
        document.head.appendChild(style);
    }}

    function updateAttributionDecorations(attrib) {{
        if (!window.editor || !attrib || !attrib.lines) return;
        window._lastAttribution = attrib;
        const model = window.editor.getModel();
        const lineCount = model.getLineCount();
        const newDecorations = [];

        for (let i = 0; i < Math.min(attrib.lines.length, lineCount); i++) {{
            const la = attrib.lines[i];
            if (!la || !la.contributor_id) continue;
            const color = contributorColors[la.contributor_id] || '#93a1a1';
            const name = contributorNames[la.contributor_id] || la.contributor_id;
            const cls = 'attrib-' + la.contributor_id.replace(/[^a-zA-Z0-9]/g, '_');
            ensureAttribStyle(cls, color);
            newDecorations.push({{
                range: new monaco.Range(i + 1, 1, i + 1, 1),
                options: {{
                    isWholeLine: true,
                    className: cls,
                    hoverMessage: {{ value: 'Last edited by **' + name + '**' }},
                }}
            }});
        }}

        decorationIds = window.editor.deltaDecorations(decorationIds, newDecorations);
    }}

    // ---- Hook into Monaco once it's ready ----
    function hookEditor() {{
        if (!window.editor) {{
            setTimeout(hookEditor, 100);
            return;
        }}

        // Disable disk saves - shared notes sync via WebSocket only
        window.sharedMode = true;

        // Set up shared save handler — sends snapshot over WS
        window.sharedSaveHandler = function() {{
            if (ws && ws.readyState === WebSocket.OPEN) {{
                const statusEl = document.getElementById('editor-status');
                const textEl = document.getElementById('status-text');
                if (statusEl) statusEl.className = 'editor-status saving';
                if (textEl) textEl.textContent = 'Saving...';
                ws.send(JSON.stringify({{ type: 'snapshot' }}));
            }}
        }};

        // Hook into content changes to send edits via WS
        window.editor.onDidChangeModelContent(function(event) {{
            if (isRemoteChange) return;

            const ops = [];
            for (const change of event.changes) {{
                const offset = change.rangeOffset;
                if (change.rangeLength > 0) {{
                    ops.push({{ type: 'delete', pos: offset, len: change.rangeLength }});
                }}
                if (change.text.length > 0) {{
                    ops.push({{ type: 'insert', pos: offset, text: change.text }});
                }}
            }}
            if (ops.length > 0) sendEdit(ops);
        }});

        // Connect WebSocket
        connectWebSocket();
    }}

    // ---- Init ----
    document.addEventListener('DOMContentLoaded', function() {{
        modifyHeader();
        hookEditor();
    }});

    window.addEventListener('beforeunload', function() {{
        if (ws) ws.close();
        if (reconnectTimer) clearTimeout(reconnectTimer);
    }});
}})();
</script>"##,
        token = token,
        contributors_json = contributors_json,
    )
}

/// Generate the overlay `<script>` block for the shared VIEW page.
/// Injected before `</body>` in the output of `render_viewer`.
///
/// This script:
/// - Modifies the header: removes owner-only buttons, adds shared badge/edit toggle/copy link
/// - Fetches attribution data and displays colored left borders + tooltips on content blocks
/// - Shows a contributor legend
pub fn render_shared_view_overlay(token: &str, contributors_json: &str) -> String {
    format!(
        r##"<script>
(function() {{
    'use strict';

    const SHARE_TOKEN = "{token}";
    const CONTRIBUTORS = {contributors_json};
    const contributorColors = {{}};
    const contributorNames = {{}};
    CONTRIBUTORS.forEach(c => {{ contributorColors[c.id] = c.color; contributorNames[c.id] = c.name; }});

    function modifyHeader() {{
        const header = document.querySelector('.viewer-header');
        if (!header) return;

        // Hide owner-only elements
        const backLink = header.querySelector('.back-link');
        if (backLink) backLink.style.display = 'none';

        const modeToggle = header.querySelector('.mode-toggle');
        if (modeToggle) modeToggle.style.display = 'none';

        // Hide knowledge graph button and panel
        const graphBtn = document.getElementById('mini-graph-btn');
        if (graphBtn) graphBtn.style.display = 'none';
        const graphPanel = document.getElementById('mini-graph-panel');
        if (graphPanel) graphPanel.style.display = 'none';

        // Keep pdf-status visible — owner-only buttons (Unlink, Scan Refs)
        // are already hidden since logged_in=false in shared view

        // Add shared badge
        const badge = document.createElement('span');
        badge.style.cssText = 'font-size:0.65rem;font-weight:600;padding:0.2rem 0.5rem;background:#268bd2;color:#fdf6e3;border-radius:3px;letter-spacing:0.05em;margin-right:0.5rem;';
        badge.textContent = 'SHARED';
        header.insertBefore(badge, header.children[0]);

        // Add mode toggle (View active, Edit links to edit mode)
        const toggle = document.createElement('div');
        toggle.className = 'mode-toggle';
        toggle.innerHTML = '<button class="active">View</button><button onclick="window.location.href=\'/shared/' + SHARE_TOKEN + '?edit=true\'">Edit</button>';
        header.appendChild(toggle);

        // Add copy link button
        const copyBtn = document.createElement('button');
        copyBtn.className = 'pdf-toggle-btn';
        copyBtn.textContent = 'Copy Link';
        copyBtn.style.marginLeft = '0.5rem';
        copyBtn.onclick = function() {{
            navigator.clipboard.writeText(window.location.origin + '/shared/' + SHARE_TOKEN).then(() => {{
                copyBtn.textContent = 'Copied!';
                setTimeout(() => copyBtn.textContent = 'Copy Link', 2000);
            }});
        }};
        header.appendChild(copyBtn);
    }}

    function addAttributionStyles() {{
        const style = document.createElement('style');
        style.textContent = `
            .note-content > p,
            .note-content > h1,
            .note-content > h2,
            .note-content > h3,
            .note-content > h4,
            .note-content > h5,
            .note-content > h6,
            .note-content > ul > li,
            .note-content > ol > li,
            .note-content > pre,
            .note-content > blockquote,
            .note-content > table {{
                border-left: 3px solid transparent;
                padding-left: 0.75rem;
                transition: border-color 0.15s;
            }}
            .contributor-legend {{
                display: flex;
                flex-wrap: wrap;
                gap: 0.75rem;
                margin-top: 1.5rem;
                padding-top: 1rem;
                border-top: 1px solid var(--base2, #eee8d5);
                font-size: 0.8rem;
                color: var(--muted, #93a1a1);
            }}
            .contributor-legend-item {{
                display: flex;
                align-items: center;
                gap: 0.3rem;
            }}
            .contributor-legend-swatch {{
                width: 12px;
                height: 12px;
                border-radius: 2px;
                flex-shrink: 0;
            }}
        `;
        document.head.appendChild(style);
    }}

    function relativeTime(dateStr) {{
        const now = new Date();
        const then = new Date(dateStr);
        const diffMs = now - then;
        const diffMin = Math.floor(diffMs / 60000);
        if (diffMin < 1) return 'just now';
        if (diffMin < 60) return diffMin + 'm ago';
        const diffHr = Math.floor(diffMin / 60);
        if (diffHr < 24) return diffHr + 'h ago';
        const diffDay = Math.floor(diffHr / 24);
        return diffDay + 'd ago';
    }}

    async function fetchAndApplyAttribution() {{
        try {{
            const resp = await fetch('/api/shared/' + SHARE_TOKEN + '/attribution');
            if (!resp.ok) return;
            const data = await resp.json();
            if (!data.attribution || !data.attribution.lines) return;

            // Build contributor lookup from response
            if (data.contributors) {{
                data.contributors.forEach(c => {{
                    contributorColors[c.id] = c.color;
                    contributorNames[c.id] = c.name;
                }});
            }}

            const content = document.querySelector('.note-content');
            if (!content) return;

            // Get block-level children
            const blockSelectors = 'p, h1, h2, h3, h4, h5, h6, pre, blockquote, table';
            const blocks = [];
            // Direct children that are block elements
            content.querySelectorAll(':scope > ' + blockSelectors).forEach(el => blocks.push(el));
            // Also list items
            content.querySelectorAll(':scope > ul > li, :scope > ol > li').forEach(el => blocks.push(el));

            const lines = data.attribution.lines;
            // Map blocks to lines approximately (each block ~ corresponds to some lines)
            // Simple heuristic: distribute lines proportionally across blocks
            if (blocks.length === 0 || lines.length === 0) return;

            const linesPerBlock = Math.max(1, Math.ceil(lines.length / blocks.length));

            blocks.forEach((block, idx) => {{
                const lineIdx = Math.min(idx * linesPerBlock, lines.length - 1);
                const la = lines[lineIdx];
                if (!la || !la.contributor_id) return;

                const color = contributorColors[la.contributor_id] || '#93a1a1';
                const name = contributorNames[la.contributor_id] || la.contributor_id;
                const time = relativeTime(la.timestamp);

                block.style.borderLeftColor = color;
                block.title = name + ' \u00B7 ' + time;
            }});

            // Add contributor legend
            const seenContributors = new Set();
            lines.forEach(la => {{ if (la && la.contributor_id) seenContributors.add(la.contributor_id); }});

            if (seenContributors.size > 0) {{
                const legend = document.createElement('div');
                legend.className = 'contributor-legend';
                seenContributors.forEach(cid => {{
                    const color = contributorColors[cid] || '#93a1a1';
                    const name = contributorNames[cid] || cid;
                    const item = document.createElement('span');
                    item.className = 'contributor-legend-item';
                    item.innerHTML = '<span class="contributor-legend-swatch" style="background:' + color + '"></span>' + name;
                    legend.appendChild(item);
                }});
                content.appendChild(legend);
            }}
        }} catch(e) {{
            console.error('Attribution fetch error:', e);
        }}
    }}

    document.addEventListener('DOMContentLoaded', function() {{
        modifyHeader();
        addAttributionStyles();
        fetchAndApplyAttribution();
    }});
}})();
</script>"##,
        token = token,
        contributors_json = contributors_json,
    )
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_contributors_json() -> String {
        serde_json::to_string(&serde_json::json!([
            {"id": "c0", "name": "Alice", "color": "#268bd2"},
            {"id": "c1", "name": "Bob", "color": "#d33682"}
        ])).unwrap()
    }

    // ---- render_shared_overlay (editor mode) tests ----

    #[test]
    fn test_editor_overlay_contains_token() {
        let html = render_shared_overlay("abc123", &sample_contributors_json());
        assert!(html.contains("abc123"));
    }

    #[test]
    fn test_editor_overlay_contains_contributors() {
        let html = render_shared_overlay("tok", &sample_contributors_json());
        assert!(html.contains("Alice"));
        assert!(html.contains("Bob"));
    }

    #[test]
    fn test_editor_overlay_sets_shared_mode() {
        let html = render_shared_overlay("tok", "[]");
        assert!(html.contains("window.sharedMode = true"));
    }

    #[test]
    fn test_editor_overlay_sets_shared_save_handler() {
        let html = render_shared_overlay("tok", "[]");
        assert!(html.contains("window.sharedSaveHandler"));
    }

    #[test]
    fn test_editor_overlay_sends_snapshot() {
        let html = render_shared_overlay("tok", "[]");
        assert!(html.contains("snapshot"));
    }

    #[test]
    fn test_editor_overlay_has_websocket_connection() {
        let html = render_shared_overlay("tok", "[]");
        assert!(html.contains("WebSocket"));
        assert!(html.contains("/ws"));
    }

    #[test]
    fn test_editor_overlay_has_attribution_decorations() {
        let html = render_shared_overlay("tok", "[]");
        assert!(html.contains("updateAttributionDecorations"));
    }

    #[test]
    fn test_editor_overlay_handles_saved_message() {
        let html = render_shared_overlay("tok", "[]");
        assert!(html.contains("case 'saved'"));
    }

    #[test]
    fn test_editor_overlay_has_name_input() {
        let html = render_shared_overlay("tok", "[]");
        assert!(html.contains("Your name"));
        assert!(html.contains("sharedIdentity"));
    }

    #[test]
    fn test_editor_overlay_has_peer_count() {
        let html = render_shared_overlay("tok", "[]");
        assert!(html.contains("shared-peer-count"));
    }

    #[test]
    fn test_editor_overlay_has_copy_link() {
        let html = render_shared_overlay("tok", "[]");
        assert!(html.contains("Copy Link"));
    }

    #[test]
    fn test_editor_overlay_has_view_link() {
        let html = render_shared_overlay("mytoken", "[]");
        // Token is embedded as SHARE_TOKEN variable, used in JS concatenation
        assert!(html.contains("\"mytoken\""));
        assert!(html.contains("'/shared/' + SHARE_TOKEN"));
        assert!(html.contains("View"));
    }

    #[test]
    fn test_editor_overlay_is_script_tag() {
        let html = render_shared_overlay("tok", "[]");
        assert!(html.starts_with("<script>"));
        assert!(html.ends_with("</script>"));
    }

    #[test]
    fn test_editor_overlay_shows_status_area() {
        let html = render_shared_overlay("tok", "[]");
        // Should repurpose status area, not hide it
        assert!(html.contains("Synced"));
        assert!(!html.contains("statusEl.style.display = 'none'"));
    }

    // ---- render_shared_view_overlay tests ----

    #[test]
    fn test_view_overlay_contains_token() {
        let html = render_shared_view_overlay("abc123", &sample_contributors_json());
        assert!(html.contains("abc123"));
    }

    #[test]
    fn test_view_overlay_contains_contributors() {
        let html = render_shared_view_overlay("tok", &sample_contributors_json());
        assert!(html.contains("Alice"));
        assert!(html.contains("Bob"));
    }

    #[test]
    fn test_view_overlay_has_shared_badge() {
        let html = render_shared_view_overlay("tok", "[]");
        assert!(html.contains("SHARED"));
    }

    #[test]
    fn test_view_overlay_has_edit_link() {
        let html = render_shared_view_overlay("mytoken", "[]");
        // Token is embedded as SHARE_TOKEN, used in JS concatenation
        assert!(html.contains("\"mytoken\""));
        assert!(html.contains("?edit=true"));
    }

    #[test]
    fn test_view_overlay_has_copy_link() {
        let html = render_shared_view_overlay("tok", "[]");
        assert!(html.contains("Copy Link"));
    }

    #[test]
    fn test_view_overlay_fetches_attribution() {
        let html = render_shared_view_overlay("tok", "[]");
        // Token is embedded as SHARE_TOKEN, used in JS concatenation
        assert!(html.contains("/api/shared/' + SHARE_TOKEN + '/attribution"));
    }

    #[test]
    fn test_view_overlay_has_attribution_styles() {
        let html = render_shared_view_overlay("tok", "[]");
        assert!(html.contains("border-left"));
        assert!(html.contains("contributor-legend"));
    }

    #[test]
    fn test_view_overlay_has_relative_time() {
        let html = render_shared_view_overlay("tok", "[]");
        assert!(html.contains("relativeTime"));
    }

    #[test]
    fn test_view_overlay_does_not_hide_pdf() {
        let html = render_shared_view_overlay("tok", "[]");
        // Should NOT hide the pdf-status element
        assert!(!html.contains("pdfStatus"));
    }

    #[test]
    fn test_view_overlay_hides_owner_elements() {
        let html = render_shared_view_overlay("tok", "[]");
        // Should hide back link and mode toggle (owner-only)
        assert!(html.contains("backLink"));
        assert!(html.contains("display = 'none'"));
    }

    #[test]
    fn test_view_overlay_is_script_tag() {
        let html = render_shared_view_overlay("tok", "[]");
        assert!(html.starts_with("<script>"));
        assert!(html.ends_with("</script>"));
    }

    #[test]
    fn test_view_overlay_mode_toggle() {
        let html = render_shared_view_overlay("tok", "[]");
        // Should add its own mode toggle with View (active) and Edit buttons
        assert!(html.contains("mode-toggle"));
    }
}
