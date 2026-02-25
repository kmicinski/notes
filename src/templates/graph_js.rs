//! Unified D3.js graph rendering engine.
//!
//! Generates parameterized `<script>` and `<style>` blocks used by both
//! the full-page graph (`/graph`) and the per-note mini graph panel.

/// How graph data is provided to the JS engine.
pub enum GraphDataSource {
    /// Data embedded directly in the page as a JS literal.
    Inline { graph_json: String },
    /// Data fetched from a URL when the graph is first shown.
    FetchUrl { url: String },
}

/// Configuration for the unified graph renderer.
pub struct GraphRendererConfig {
    /// CSS selector for the container element (e.g. "#graph-container").
    pub container_selector: String,
    /// When set, enables radial ring layout centred on this node key.
    pub center_key: Option<String>,
    /// True for the floating mini-graph panel (affects sizing/interactions).
    pub is_mini: bool,
    /// Enables drag-to-link handles and link creation.
    pub logged_in: bool,
    /// Show arrowhead markers on directed edges.
    pub show_arrows: bool,
    /// Show tooltips when hovering edges.
    pub show_edge_tooltips: bool,
    /// Auto zoom-to-fit after simulation settles.
    pub auto_fit: bool,
    /// BFS pruning: 0 = no limit, >0 = keep at most this many nodes.
    pub max_nodes: usize,
    /// Where the graph data comes from.
    pub data_source: GraphDataSource,
    /// Inline notes list for autocomplete. None = lazy-fetch from /api/notes/list.
    pub notes_json: Option<String>,
}

/// Returns the unified `<style>` block with `.kg-` prefixed classes.
pub fn graph_css() -> String {
    r#"
        .kg-link { stroke: var(--border); stroke-opacity: 0.6; }
        .kg-link.citation { stroke-dasharray: 5,3; stroke: #b58900; stroke-opacity: 0.5; }
        .kg-link.manual { stroke: #859900; stroke-opacity: 0.7; }
        .kg-link.deg1 { stroke: #073642; stroke-opacity: 0.7; stroke-width: 1.5px; }
        .kg-link.citation.deg1 { stroke-opacity: 0.8; stroke-width: 1.5px; }
        .kg-link.highlighted { stroke: var(--link); stroke-opacity: 1; stroke-width: 2.5px; }
        .kg-link.kg-edge-annotated { stroke-width: 2.5px; stroke-opacity: 0.9; }

        .kg-link-handle {
            fill: #859900;
            stroke: none;
            cursor: crosshair;
            opacity: 0;
            transition: opacity 0.15s;
        }
        .kg-node:hover .kg-link-handle { opacity: 0.7; }
        .kg-node.link-target circle { stroke: #859900 !important; stroke-width: 3px !important; }
        .kg-temp-link-line { stroke: #859900; stroke-width: 2; stroke-dasharray: 6,4; pointer-events: none; }

        .kg-node circle { cursor: pointer; stroke: var(--bg); stroke-width: 1.5px; }
        .kg-node.selected circle { stroke: #fff; stroke-width: 3px; }
        .kg-node text {
            font-size: 10px;
            fill: var(--fg);
            pointer-events: none;
            text-anchor: middle;
            dominant-baseline: middle;
        }
        .kg-node.center text { opacity: 1; font-size: 10px; font-weight: 600; }
        .kg-node:hover text { opacity: 1; font-size: 10px; }

        .kg-tooltip {
            position: absolute;
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 4px;
            padding: 0.5rem 0.75rem;
            font-size: 0.85rem;
            pointer-events: none;
            z-index: 1001;
            box-shadow: 0 2px 8px rgba(0,0,0,0.15);
            max-width: 300px;
        }
        .kg-tooltip .title { font-weight: 600; margin-bottom: 0.25rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
        .kg-tooltip .meta { color: var(--muted); font-size: 0.8rem; }

        .kg-legend {
            position: absolute;
            bottom: 8px;
            left: 8px;
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 4px;
            padding: 0.4rem 0.6rem;
            font-size: 0.75rem;
            display: flex;
            gap: 0.6rem;
            align-items: center;
        }
        .kg-legend-item { display: flex; align-items: center; gap: 0.3rem; }
        .kg-legend-dot { width: 9px; height: 9px; border-radius: 50%; display: inline-block; }

        .kg-autocomplete-wrap { position: relative; }
        .kg-autocomplete-dropdown {
            position: absolute; top: 100%; left: 0; right: 0;
            background: var(--bg); border: 1px solid var(--border);
            border-radius: 0 0 4px 4px; max-height: 200px; overflow-y: auto;
            z-index: 3001; box-shadow: 0 4px 8px rgba(0,0,0,0.15);
        }
        .kg-autocomplete-item {
            padding: 0.4rem 0.5rem; cursor: pointer; font-size: 0.85rem;
            display: flex; justify-content: space-between; align-items: center;
        }
        .kg-autocomplete-item:hover, .kg-autocomplete-item.active { background: var(--accent); }
        .kg-autocomplete-badge {
            font-size: 0.7rem; padding: 0.1rem 0.3rem; border-radius: 3px;
            background: var(--border); color: var(--muted);
        }

        .kg-search-panel {
            position: absolute;
            top: 12px; left: 50%; transform: translateX(-50%);
            z-index: 2000;
            background: var(--bg);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 0.75rem 1rem;
            box-shadow: 0 4px 16px rgba(0,0,0,0.2);
            width: 340px;
        }
        .kg-search-panel .kg-search-label {
            font-size: 0.8rem; color: var(--muted); margin-bottom: 0.4rem;
            white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
        }
        .kg-search-panel input {
            width: 100%; box-sizing: border-box;
            padding: 0.5rem; border: 1px solid var(--border);
            border-radius: 4px; background: var(--accent); color: var(--fg);
            font-size: 0.9rem; font-family: inherit;
        }
        .kg-search-panel input:focus { outline: none; border-color: var(--link); }

        .kg-annotation-editor {
            position: absolute; z-index: 2500;
            background: var(--bg); border: 1px solid var(--border);
            border-radius: 8px; padding: 0.75rem;
            box-shadow: 0 4px 16px rgba(0,0,0,0.2);
            width: 280px;
        }
        .kg-annotation-editor .kg-ann-label {
            font-size: 0.8rem; color: var(--muted); margin-bottom: 0.4rem;
            white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
        }
        .kg-annotation-editor textarea {
            width: 100%; box-sizing: border-box;
            padding: 0.5rem; border: 1px solid var(--border);
            border-radius: 4px; background: var(--accent); color: var(--fg);
            font-size: 0.85rem; font-family: inherit;
            height: 60px; resize: vertical;
        }
        .kg-annotation-editor textarea:focus { outline: none; border-color: var(--link); }
        .kg-annotation-editor .kg-ann-actions {
            display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 0.5rem;
        }

        .kg-annotation-dot {
            fill: #859900;
            stroke: var(--bg);
            stroke-width: 1;
            pointer-events: none;
        }
    "#.to_string()
}

/// Returns the `<script src="d3">` tag + `<script>` IIFE for the graph engine.
pub fn render_graph_js(config: &GraphRendererConfig) -> String {
    let center_key_js = match &config.center_key {
        Some(k) => format!("\"{}\"", k),
        None => "null".to_string(),
    };
    let container_sel = &config.container_selector;
    let is_mini = config.is_mini;
    let logged_in = config.logged_in;
    let show_arrows = config.show_arrows;
    let show_edge_tooltips = config.show_edge_tooltips;
    let auto_fit = config.auto_fit;
    let max_nodes = config.max_nodes;

    let data_loader_js = match &config.data_source {
        GraphDataSource::Inline { graph_json } => {
            format!("const _kgData = {};", graph_json)
        }
        GraphDataSource::FetchUrl { url } => {
            format!(
                r#"const _kgResp = await fetch('{}');
                if (!_kgResp.ok) {{ _kgContainer.innerHTML = '<div style="padding:1rem;color:var(--red);">Failed to load graph</div>'; return; }}
                const _kgData = await _kgResp.json();"#,
                url
            )
        }
    };

    let notes_data_js = match &config.notes_json {
        Some(json) => format!("let _kgAllNotes = {};", json),
        None => "let _kgAllNotes = null;".to_string(),
    };

    let d3_tag = if is_mini {
        "" // viewer.rs already loads d3
    } else {
        r#"<script src="https://d3js.org/d3.v7.min.js"></script>"#
    };

    let fn_open = if is_mini { "async function _kgMiniInit() {" } else { "(async function() {" };
    let fn_close = if is_mini { "}" } else { "})();" };

    format!(
        r##"{d3_tag}
        <script>
        {fn_open}
            const _kgContainer = document.querySelector('{container_sel}');
            if (!_kgContainer) return;

            const centerKey = {center_key_js};
            const isMini = {is_mini};
            const isLoggedIn = {logged_in};
            const showArrows = {show_arrows};
            const showEdgeTooltips = {show_edge_tooltips};
            const autoFit = {auto_fit};
            const maxNodes = {max_nodes};

            {notes_data_js}

            // --- Data loading ---
            {data_loader_js}

            const nodes = _kgData.nodes;
            const edges = _kgData.edges;

            // --- BFS distances from center ---
            if (centerKey) {{
                const adj = {{}};
                nodes.forEach(n => {{ adj[n.id] = []; }});
                edges.forEach(e => {{
                    const sid = typeof e.source === 'object' ? e.source.id : e.source;
                    const tid = typeof e.target === 'object' ? e.target.id : e.target;
                    if (adj[sid]) adj[sid].push(tid);
                    if (adj[tid]) adj[tid].push(sid);
                }});
                const dist = {{}};
                dist[centerKey] = 0;
                const queue = [centerKey];
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
                nodes.forEach(n => {{ n._dist = dist[n.id] !== undefined ? dist[n.id] : 99; }});
            }}

            // --- BFS pruning ---
            let activeNodes = nodes;
            let activeEdges = edges;
            if (centerKey && maxNodes > 0 && nodes.length > maxNodes) {{
                const firstDeg = nodes.filter(n => n._dist <= 1);
                const budget = Math.max(maxNodes, firstDeg.length);
                const byDist = {{}};
                nodes.forEach(n => {{
                    if (n._dist > 1) {{
                        if (!byDist[n._dist]) byDist[n._dist] = [];
                        byDist[n._dist].push(n);
                    }}
                }});
                const keep = [...firstDeg];
                const dists = Object.keys(byDist).map(Number).sort((a, b) => a - b);
                for (const d of dists) {{
                    if (keep.length >= budget) break;
                    const rem = budget - keep.length;
                    const cands = byDist[d];
                    if (cands.length <= rem) {{
                        keep.push(...cands);
                    }} else {{
                        cands.sort(() => Math.random() - 0.5);
                        keep.push(...cands.slice(0, rem));
                    }}
                }}
                const keepIds = new Set(keep.map(n => n.id));
                activeNodes = keep;
                activeEdges = edges.filter(e => {{
                    const sid = typeof e.source === 'object' ? e.source.id : e.source;
                    const tid = typeof e.target === 'object' ? e.target.id : e.target;
                    return keepIds.has(sid) && keepIds.has(tid);
                }});
            }}

            // --- Node map for tooltips ---
            const nodeMap = {{}};
            activeNodes.forEach(n => {{ nodeMap[n.id] = n; }});

            // --- Layout functions ---
            const distColors = ['#dc322f', '#cb4b16', '#268bd2', '#93a1a1'];

            function nodeRadius(d) {{
                if (centerKey) {{
                    if (d._dist === 0) return 16;
                    if (d._dist === 1) return 10;
                    if (d._dist === 2) return 7;
                    return 5;
                }}
                const deg = (d.in_degree || 0) + (d.out_degree || 0);
                return 8 + Math.sqrt(deg) * 3;
            }}

            function nodeColor(d) {{
                if (centerKey) {{
                    return distColors[Math.min(d._dist, distColors.length - 1)];
                }}
                return d.node_type === 'paper' ? '#f4a460' : 'var(--link)';
            }}

            function nodeOpacity(d) {{
                if (centerKey) {{
                    if (d._dist === 0) return 1;
                    if (d._dist === 1) return 0.95;
                    if (d._dist === 2) return 0.7;
                    return 0.45;
                }}
                const deg = (d.in_degree || 0) + (d.out_degree || 0);
                return deg === 0 ? 0.6 : 1;
            }}

            // --- SVG setup (clear any spinner / previous render) ---
            _kgContainer.innerHTML = '';
            const rect = _kgContainer.getBoundingClientRect();
            const width = rect.width || 600;
            const height = rect.height || 400;
            const svg = d3.select(_kgContainer).append('svg')
                .attr('width', '100%').attr('height', '100%');
            const g = svg.append('g');

            // --- Named groups for rebinding ---
            const linkGroup = g.append('g').attr('class', 'kg-links');
            const nodeGroup = g.append('g').attr('class', 'kg-nodes');

            // --- Arrowhead markers ---
            if (showArrows) {{
                const defs = svg.append('defs');
                function addMarker(id, color) {{
                    defs.append('marker')
                        .attr('id', id)
                        .attr('viewBox', '0 -7 14 14')
                        .attr('refX', 14).attr('refY', 0)
                        .attr('markerWidth', 10).attr('markerHeight', 10)
                        .attr('markerUnits', 'userSpaceOnUse')
                        .attr('orient', 'auto')
                        .append('path')
                        .attr('d', 'M0,-5L12,0L0,5')
                        .attr('fill', color);
                }}
                addMarker('kg-arrow-default', '#93a1a1');
                addMarker('kg-arrow-citation', '#b58900');
                addMarker('kg-arrow-manual', '#859900');
            }}

            // --- Tooltip ---
            const tooltip = d3.select(_kgContainer).append('div')
                .attr('class', 'kg-tooltip')
                .style('display', 'none');

            // --- Helper: edge key for D3 data joins ---
            function edgeKey(d) {{
                const sid = typeof d.source === 'object' ? d.source.id : d.source;
                const tid = typeof d.target === 'object' ? d.target.id : d.target;
                return sid + '|' + tid;
            }}

            // --- Helper: apply link attributes ---
            function applyLinkAttrs(sel) {{
                return sel
                    .attr('class', d => {{
                        const sid = typeof d.source === 'object' ? d.source.id : d.source;
                        const tid = typeof d.target === 'object' ? d.target.id : d.target;
                        let cls = 'kg-link';
                        if (d.edge_type === 'citation') cls += ' citation';
                        if (d.edge_type === 'manual') cls += ' manual';
                        if (centerKey && (sid === centerKey || tid === centerKey)) cls += ' deg1';
                        if (d.edge_type === 'manual' && d.annotation) cls += ' kg-edge-annotated';
                        return cls;
                    }})
                    .attr('stroke-width', d => {{
                        if (d.edge_type === 'manual' && d.annotation) return 2.5;
                        if (centerKey) return 1;
                        return Math.sqrt(d.weight || 1) * 1.5;
                    }})
                    .attr('marker-end', d => {{
                        if (!showArrows) return null;
                        if (d.edge_type === 'citation') return 'url(#kg-arrow-citation)';
                        if (d.edge_type === 'manual') return 'url(#kg-arrow-manual)';
                        return 'url(#kg-arrow-default)';
                    }});
            }}

            // --- Helper: create entering node groups ---
            function createNodeEnter(enter) {{
                const g = enter.append('g')
                    .attr('class', d => {{
                        let cls = 'kg-node';
                        if (centerKey && d._dist === 0) cls += ' center';
                        if (!centerKey) {{
                            cls += ' ' + d.node_type;
                            const deg = (d.in_degree || 0) + (d.out_degree || 0);
                            if (deg === 0) cls += ' orphan';
                            if (deg >= 5) cls += ' hub';
                        }}
                        return cls;
                    }})
                    .style('opacity', d => nodeOpacity(d))
                    .call(d3.drag()
                        .filter(event => !event.button)
                        .on('start', (e, d) => {{
                            if (!e.active) sim.alphaTarget(0.3).restart();
                            d.fx = d.x; d.fy = d.y;
                        }})
                        .on('drag', (e, d) => {{ d.fx = e.x; d.fy = e.y; }})
                        .on('end', (e, d) => {{
                            if (!e.active) sim.alphaTarget(0);
                            if (centerKey && d.id === centerKey) return;
                            d.fx = null; d.fy = null;
                        }}));

                g.append('circle')
                    .attr('r', d => nodeRadius(d))
                    .attr('fill', d => nodeColor(d));

                g.append('text')
                    .text(d => {{
                        if (centerKey && d._dist >= 3) return '';
                        const label = d.short_label || d.title;
                        return label.length > 15 ? label.substring(0, 15) + '...' : label;
                    }})
                    .attr('dy', d => -(nodeRadius(d) + 3))
                    .style('opacity', () => centerKey ? 0.7 : 1);

                if (isLoggedIn) {{
                    g.append('circle')
                        .attr('class', 'kg-link-handle')
                        .attr('r', 6)
                        .attr('cx', d => nodeRadius(d) + 4)
                        .attr('cy', 0)
                        .each(function(d) {{
                            this.addEventListener('pointerdown', function(e) {{
                                e.stopPropagation();
                                e.stopImmediatePropagation();
                                e.preventDefault();
                                startLinkDrag(d, e);
                            }});
                        }});
                }}

                return g;
            }}

            // --- Helper: bind node hover/click events ---
            function bindNodeEvents(sel) {{
                sel.on('mouseover', function(event, d) {{
                    d3.select(this).raise().classed('selected', true);
                    if (centerKey) {{
                        d3.select(this).select('circle')
                            .attr('stroke', 'var(--fg)').attr('stroke-width', 2.5);
                        if (d._dist >= 3) {{
                            d3.select(this).select('text')
                                .text(d.short_label || d.title.substring(0, 20));
                        }}
                    }}
                    link.classed('highlighted', l => l.source.id === d.id || l.target.id === d.id);

                    const distLabel = centerKey
                        ? (d._dist === 0 ? 'center' : d._dist + (d._dist === 1 ? 'st' : d._dist === 2 ? 'nd' : d._dist === 3 ? 'rd' : 'th') + ' degree')
                        : null;
                    let tipHtml = '<div class="title">' + d.title + '</div><div class="meta">';
                    tipHtml += 'Type: ' + d.node_type;
                    if (distLabel) tipHtml += ' \u00b7 ' + distLabel;
                    tipHtml += '<br>Links: ' + (d.in_degree || 0) + ' in, ' + (d.out_degree || 0) + ' out';
                    if (d.time_total > 0) tipHtml += '<br>Time: ' + Math.floor(d.time_total/60) + 'h ' + (d.time_total%60) + 'm';
                    if (d.primary_category) tipHtml += '<br>Category: ' + d.primary_category;
                    tipHtml += '</div>';

                    tooltip.style('display', 'block').html(tipHtml)
                        .style('left', (event.offsetX + 14) + 'px')
                        .style('top', (event.offsetY - 10) + 'px');
                }})
                .on('mouseout', function(event, d) {{
                    d3.select(this).classed('selected', false);
                    if (centerKey) {{
                        d3.select(this).select('circle')
                            .attr('stroke', 'var(--bg)').attr('stroke-width', 1.5);
                        if (d._dist >= 3) {{
                            d3.select(this).select('text').text('');
                        }}
                    }}
                    link.classed('highlighted', false);
                    tooltip.style('display', 'none');
                }})
                .on('click', function(event, d) {{
                    if (centerKey && d.id === centerKey) return;
                    window.location.href = '/note/' + d.id;
                }})
                .on('dblclick', function(event, d) {{
                    window.location.href = '/graph?q=from:' + d.id + ' depth:2';
                }});
            }}

            // --- Helper: bind edge hover/click events ---
            function bindEdgeEvents(sel) {{
                if (showEdgeTooltips) {{
                    sel.on('mouseover', function(event, d) {{
                        const srcNode = (typeof d.source === 'object') ? d.source : nodeMap[d.source];
                        const tgtNode = (typeof d.target === 'object') ? d.target : nodeMap[d.target];
                        const srcTitle = srcNode ? srcNode.title : String(d.source);
                        const tgtTitle = tgtNode ? tgtNode.title : String(d.target);
                        const typeLabels = {{
                            crosslink: 'cites ([@ref])',
                            parent: 'child of',
                            citation: 'PDF citation',
                            manual: 'linked by user'
                        }};
                        const typeLabel = typeLabels[d.edge_type] || d.edge_type;
                        let html = '<div class="title">' + srcTitle + ' &rarr; ' + tgtTitle + '</div>';
                        html += '<div class="meta">Type: ' + typeLabel;
                        if (d.annotation) html += '<br>' + d.annotation;
                        html += '</div>';
                        d3.select(this).attr('stroke-width', 4).attr('stroke-opacity', 1);
                        tooltip.style('display', 'block').html(html)
                            .style('left', (event.offsetX + 14) + 'px')
                            .style('top', (event.offsetY - 10) + 'px');
                    }})
                    .on('mouseout', function(event, d) {{
                        const w = (d.edge_type === 'manual' && d.annotation) ? 2.5
                            : centerKey ? 1 : Math.sqrt(d.weight || 1) * 1.5;
                        d3.select(this).attr('stroke-width', w).attr('stroke-opacity', null);
                        tooltip.style('display', 'none');
                    }});
                }}

                if (isLoggedIn) {{
                    sel.style('cursor', d => d.edge_type === 'manual' ? 'pointer' : null)
                        .on('click', function(event, d) {{
                            if (d.edge_type !== 'manual') return;
                            event.stopPropagation();
                            openAnnotationEditor(d, event);
                        }});
                }}
            }}

            // --- Helper: lazy-load notes list ---
            async function ensureNotesLoaded() {{
                if (!_kgAllNotes) {{
                    try {{
                        const r = await fetch('/api/notes/list');
                        if (r.ok) _kgAllNotes = await r.json();
                        else _kgAllNotes = [];
                    }} catch (_) {{ _kgAllNotes = []; }}
                }}
            }}

            // --- Annotation visuals ---
            let annotationDots;

            function updateAnnotationVisuals() {{
                link.classed('kg-edge-annotated', d => d.edge_type === 'manual' && d.annotation);
                link.attr('stroke-width', d => {{
                    if (d.edge_type === 'manual' && d.annotation) return 2.5;
                    if (centerKey) return 1;
                    return Math.sqrt(d.weight || 1) * 1.5;
                }});
                annotationDots = linkGroup.selectAll('.kg-annotation-dot')
                    .data(activeEdges.filter(d => d.annotation), edgeKey)
                    .join('circle')
                    .attr('class', 'kg-annotation-dot').attr('r', 3);
            }}

            // --- Force simulation ---
            let sim;
            if (centerKey) {{
                const centerNode = activeNodes.find(n => n.id === centerKey);
                if (centerNode) {{
                    centerNode.fx = width / 2;
                    centerNode.fy = height / 2;
                }}
                const ringRadius = Math.min(width, height) * 0.3;
                const linkDist1 = Math.max(60, ringRadius);

                sim = d3.forceSimulation(activeNodes)
                    .force('link', d3.forceLink(activeEdges).id(d => d.id).distance(d => {{
                        const s = d.source._dist !== undefined ? d.source._dist : 1;
                        const t = d.target._dist !== undefined ? d.target._dist : 1;
                        const mx = Math.max(s, t);
                        if (mx <= 1) return linkDist1;
                        return linkDist1 * 0.6 + mx * 20;
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
            }} else {{
                sim = d3.forceSimulation(activeNodes)
                    .force('link', d3.forceLink(activeEdges).id(d => d.id).distance(80))
                    .force('charge', d3.forceManyBody().strength(-200))
                    .force('center', d3.forceCenter(width / 2, height / 2))
                    .force('collision', d3.forceCollide().radius(30));
            }}

            if (isMini) {{ window._kgMiniSim = sim; }}

            // --- Initial render using named groups ---
            let link = linkGroup.selectAll('line')
                .data(activeEdges, edgeKey).join(
                    enter => applyLinkAttrs(enter.append('line')),
                    update => update,
                    exit => exit.remove()
                );
            bindEdgeEvents(link);

            const sortedNodes = centerKey
                ? [...activeNodes].sort((a, b) => b._dist - a._dist)
                : activeNodes;

            let node = nodeGroup.selectAll('g')
                .data(sortedNodes, d => d.id).join(
                    enter => createNodeEnter(enter),
                    update => update,
                    exit => exit.remove()
                );
            bindNodeEvents(node);

            updateAnnotationVisuals();

            // --- rebindGraph: re-join data after in-place changes ---
            function rebindGraph() {{
                link = linkGroup.selectAll('line')
                    .data(activeEdges, edgeKey).join(
                        enter => applyLinkAttrs(enter.append('line')),
                        update => update,
                        exit => exit.remove()
                    );
                bindEdgeEvents(link);

                node = nodeGroup.selectAll('g')
                    .data(activeNodes, d => d.id).join(
                        enter => createNodeEnter(enter),
                        update => update,
                        exit => exit.remove()
                    );
                bindNodeEvents(node);

                sim.nodes(activeNodes);
                sim.force('link').links(activeEdges);
                sim.alpha(0.5).restart();
                updateAnnotationVisuals();
            }}

            // --- addEdgeInPlace: add edge to data arrays and rebind ---
            function addEdgeInPlace(sourceKey, targetKey, acEntry) {{
                // Check for duplicate
                const dup = activeEdges.some(e => {{
                    const sid = typeof e.source === 'object' ? e.source.id : e.source;
                    const tid = typeof e.target === 'object' ? e.target.id : e.target;
                    return sid === sourceKey && tid === targetKey && e.edge_type === 'manual';
                }});
                if (dup) return;

                // If target not in graph, add synthetic node near source
                if (!nodeMap[targetKey]) {{
                    const srcNode = nodeMap[sourceKey];
                    const newNode = {{
                        id: targetKey,
                        title: acEntry ? acEntry.title : targetKey,
                        short_label: acEntry ? (acEntry.title.length > 16 ? acEntry.title.substring(0, 16) + '\u2026' : acEntry.title) : targetKey,
                        node_type: acEntry ? acEntry.node_type : 'note',
                        in_degree: 0,
                        out_degree: 0,
                        time_total: 0,
                        primary_category: null,
                        _dist: srcNode ? (srcNode._dist !== undefined ? srcNode._dist + 1 : 99) : 99,
                        x: srcNode ? srcNode.x + (Math.random() - 0.5) * 80 : width / 2,
                        y: srcNode ? srcNode.y + (Math.random() - 0.5) * 80 : height / 2,
                    }};
                    activeNodes.push(newNode);
                    nodeMap[targetKey] = newNode;
                }}

                activeEdges.push({{
                    source: sourceKey,
                    target: targetKey,
                    edge_type: 'manual',
                    weight: 1,
                    annotation: null,
                }});

                rebindGraph();
            }}

            // --- removeEdgeInPlace: remove edge from data arrays and rebind ---
            function removeEdgeInPlace(sourceKey, targetKey) {{
                const idx = activeEdges.findIndex(e => {{
                    const sid = typeof e.source === 'object' ? e.source.id : e.source;
                    const tid = typeof e.target === 'object' ? e.target.id : e.target;
                    return sid === sourceKey && tid === targetKey && e.edge_type === 'manual';
                }});
                if (idx >= 0) activeEdges.splice(idx, 1);
                rebindGraph();
            }}

            // --- deleteEdge: DELETE from API then update in-place ---
            async function deleteEdge(sourceKey, targetKey) {{
                const r = await fetch('/api/graph/edge', {{
                    method: 'DELETE',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ source: sourceKey, target: targetKey }})
                }});
                if (r.ok) {{
                    removeEdgeInPlace(sourceKey, targetKey);
                }} else {{
                    const t = await r.text();
                    console.error('Failed to delete edge:', t);
                }}
            }}

            // --- createEdge: POST to API then update in-place ---
            async function createEdge(sourceKey, targetKey, acEntry) {{
                const r = await fetch('/api/graph/edge', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ source: sourceKey, target: targetKey, annotation: null }})
                }});
                if (r.ok) {{
                    addEdgeInPlace(sourceKey, targetKey, acEntry);
                }} else {{
                    const t = await r.text();
                    console.error('Failed to create edge:', t);
                }}
            }}

            // --- Tick ---
            sim.on('tick', () => {{
                link
                    .attr('x1', d => d.source.x).attr('y1', d => d.source.y)
                    .attr('x2', d => {{
                        if (!showArrows) return d.target.x;
                        const dx = d.target.x - d.source.x;
                        const dy = d.target.y - d.source.y;
                        const dist = Math.sqrt(dx*dx + dy*dy) || 1;
                        const r = nodeRadius(d.target);
                        return d.target.x - dx * (r / dist);
                    }})
                    .attr('y2', d => {{
                        if (!showArrows) return d.target.y;
                        const dx = d.target.x - d.source.x;
                        const dy = d.target.y - d.source.y;
                        const dist = Math.sqrt(dx*dx + dy*dy) || 1;
                        const r = nodeRadius(d.target);
                        return d.target.y - dy * (r / dist);
                    }});
                node.attr('transform', d => 'translate(' + d.x + ',' + d.y + ')');

                // Update annotation dot positions to edge midpoints
                if (annotationDots) {{
                    annotationDots
                        .attr('cx', d => (d.source.x + d.target.x) / 2)
                        .attr('cy', d => (d.source.y + d.target.y) / 2);
                }}
            }});

            // --- Auto-fit ---
            if (autoFit) {{
                sim.on('end', () => {{
                    const fitNodes = centerKey
                        ? activeNodes.filter(n => n._dist <= 1)
                        : activeNodes;
                    if (fitNodes.length < 2) return;
                    let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
                    fitNodes.forEach(n => {{
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
            }}

            // --- Zoom ---
            svg.call(d3.zoom().scaleExtent([0.2, 5]).on('zoom', e => {{
                g.attr('transform', e.transform);
            }}));

            // --- Resize (full page only) ---
            if (!isMini) {{
                window.addEventListener('resize', () => {{
                    const nw = _kgContainer.clientWidth;
                    const nh = _kgContainer.clientHeight;
                    if (!centerKey) {{
                        sim.force('center', d3.forceCenter(nw / 2, nh / 2));
                    }}
                    sim.alpha(0.3).restart();
                }});
            }}

            // --- Legend ---
            const legend = d3.select(_kgContainer).append('div').attr('class', 'kg-legend');
            if (centerKey) {{
                [['Center', distColors[0]], ['1st', distColors[1]], ['2nd', distColors[2]], ['3rd+', distColors[3]]].forEach(([label, color]) => {{
                    const item = legend.append('span').attr('class', 'kg-legend-item');
                    item.append('span').attr('class', 'kg-legend-dot').style('background', color);
                    item.append('span').text(label);
                }});
            }} else {{
                [['Note', 'var(--link)'], ['Paper', '#f4a460']].forEach(([label, color]) => {{
                    const item = legend.append('span').attr('class', 'kg-legend-item');
                    item.append('span').attr('class', 'kg-legend-dot').style('background', color);
                    item.append('span').text(label);
                }});
                const citItem = legend.append('span').attr('class', 'kg-legend-item');
                citItem.append('span').html('<svg width="24" height="10"><line x1="0" y1="5" x2="24" y2="5" stroke="#b58900" stroke-dasharray="4,3" stroke-width="1.5"/></svg>');
                citItem.append('span').text('Citation');
                const manItem = legend.append('span').attr('class', 'kg-legend-item');
                manItem.append('span').html('<svg width="24" height="10"><line x1="0" y1="5" x2="24" y2="5" stroke="#859900" stroke-width="1.5"/></svg>');
                manItem.append('span').text('Manual');
            }}

            // --- Drag-to-link with type-to-search ---
            function startLinkDrag(sourceNode, startEvent) {{
                const svgEl = svg.node();
                const tempLine = svg.append('line').attr('class', 'kg-temp-link-line');
                let targetNode = null;
                let searchMode = false;
                let searchPanel = null;

                function svgCoords(cx, cy) {{
                    const pt = svgEl.createSVGPoint();
                    pt.x = cx; pt.y = cy;
                    return pt.matrixTransform(svgEl.getScreenCTM().inverse());
                }}
                function simToSvg(sx, sy) {{
                    const t = d3.zoomTransform(svgEl);
                    return [t.k * sx + t.x, t.k * sy + t.y];
                }}
                function getSimCoords(cx, cy) {{
                    const svgPt = svgCoords(cx, cy);
                    const t = d3.zoomTransform(svgEl);
                    return [(svgPt.x - t.x) / t.k, (svgPt.y - t.y) / t.k];
                }}
                function updateLine(cx, cy) {{
                    const [sx, sy] = simToSvg(sourceNode.x, sourceNode.y);
                    const ep = svgCoords(cx, cy);
                    tempLine.attr('x1', sx).attr('y1', sy).attr('x2', ep.x).attr('y2', ep.y);
                }}
                // Point temp line to top-center of container when search panel is open
                function updateLineToPanel() {{
                    const [sx, sy] = simToSvg(sourceNode.x, sourceNode.y);
                    const containerRect = _kgContainer.getBoundingClientRect();
                    const svgRect = svgEl.getBoundingClientRect();
                    const panelX = containerRect.width / 2;
                    const panelY = 50;
                    const pt = svgEl.createSVGPoint();
                    pt.x = panelX + (containerRect.left - svgRect.left);
                    pt.y = panelY + (containerRect.top - svgRect.top);
                    tempLine.attr('x1', sx).attr('y1', sy).attr('x2', pt.x).attr('y2', pt.y);
                }}
                updateLine(startEvent.clientX, startEvent.clientY);

                function cleanup() {{
                    window.removeEventListener('pointermove', onMove);
                    window.removeEventListener('pointerup', onUp);
                    window.removeEventListener('keydown', onKeyDown);
                    tempLine.remove();
                    node.classed('link-target', false);
                    if (searchPanel) {{ searchPanel.remove(); searchPanel = null; }}
                }}

                function onMove(e) {{
                    if (searchMode) return;
                    updateLine(e.clientX, e.clientY);
                    const [mx, my] = getSimCoords(e.clientX, e.clientY);
                    let closest = null, minDist = Infinity;
                    activeNodes.forEach(n => {{
                        if (n === sourceNode) return;
                        const dx = n.x - mx, dy = n.y - my;
                        const d = Math.sqrt(dx*dx + dy*dy);
                        const thr = nodeRadius(n) + 15;
                        if (d < thr && d < minDist) {{ minDist = d; closest = n; }}
                    }});
                    targetNode = closest;
                    node.classed('link-target', d => d === closest);
                }}

                function onUp() {{
                    if (searchMode) return; // search panel handles completion
                    cleanup();
                    if (targetNode) {{
                        // Instant link creation — no modal
                        createEdge(sourceNode.id, targetNode.id, null);
                    }}
                }}

                function onKeyDown(e) {{
                    if (searchMode) return;
                    if (e.key === 'Escape') {{ cleanup(); return; }}
                    if (e.key.length === 1 && !e.ctrlKey && !e.metaKey) {{
                        searchMode = true;
                        // Stop pointer tracking
                        window.removeEventListener('pointermove', onMove);
                        window.removeEventListener('pointerup', onUp);
                        node.classed('link-target', false);
                        targetNode = null;
                        openSearchPanel(e.key);
                    }}
                }}

                async function openSearchPanel(initialChar) {{
                    await ensureNotesLoaded();

                    searchPanel = d3.select(_kgContainer).append('div')
                        .attr('class', 'kg-search-panel');
                    searchPanel.append('div').attr('class', 'kg-search-label')
                        .text('Link from \u201c' + (sourceNode.short_label || sourceNode.title) + '\u201d to\u2026');

                    const acWrap = searchPanel.append('div').attr('class', 'kg-autocomplete-wrap');
                    const input = acWrap.append('input')
                        .attr('type', 'text')
                        .attr('placeholder', 'Search notes...')
                        .attr('autocomplete', 'off')
                        .property('value', initialChar);
                    const dropdown = acWrap.append('div')
                        .attr('class', 'kg-autocomplete-dropdown').style('display', 'none');

                    let selectedIdx = -1;
                    let currentMatches = [];

                    updateLineToPanel();

                    function renderDropdown(matches) {{
                        currentMatches = matches; selectedIdx = -1;
                        dropdown.html('');
                        if (matches.length === 0) {{ dropdown.style('display', 'none'); return; }}
                        dropdown.style('display', 'block');
                        matches.forEach(m => {{
                            const item = dropdown.append('div').attr('class', 'kg-autocomplete-item')
                                .on('click', () => selectAndCreate(m));
                            item.append('span').text(m.title);
                            item.append('span').attr('class', 'kg-autocomplete-badge').text(m.node_type);
                        }});
                    }}

                    function selectAndCreate(m) {{
                        cleanup();
                        createEdge(sourceNode.id, m.key, m);
                    }}

                    function doSearch() {{
                        const q = input.property('value').toLowerCase().trim();
                        if (q.length === 0) {{ dropdown.style('display', 'none'); return; }}
                        const matches = _kgAllNotes
                            .filter(n => n.key !== sourceNode.id &&
                                (n.title.toLowerCase().includes(q) || n.key.toLowerCase().includes(q)))
                            .slice(0, 10);
                        renderDropdown(matches);
                    }}

                    input.on('input', doSearch);
                    input.on('keydown', function(event) {{
                        if (event.key === 'ArrowDown') {{
                            event.preventDefault();
                            if (currentMatches.length > 0) {{
                                selectedIdx = Math.min(selectedIdx + 1, currentMatches.length - 1);
                                dropdown.selectAll('.kg-autocomplete-item').classed('active', (d, i) => i === selectedIdx);
                            }}
                        }} else if (event.key === 'ArrowUp') {{
                            event.preventDefault();
                            selectedIdx = Math.max(selectedIdx - 1, 0);
                            dropdown.selectAll('.kg-autocomplete-item').classed('active', (d, i) => i === selectedIdx);
                        }} else if (event.key === 'Enter') {{
                            event.preventDefault();
                            if (selectedIdx >= 0 && selectedIdx < currentMatches.length) {{
                                selectAndCreate(currentMatches[selectedIdx]);
                            }}
                        }} else if (event.key === 'Escape') {{
                            cleanup();
                        }}
                    }});

                    // Do initial search with the typed character
                    doSearch();
                    setTimeout(() => {{
                        const el = input.node();
                        el.focus();
                        el.setSelectionRange(el.value.length, el.value.length);
                    }}, 10);
                }}

                window.addEventListener('pointermove', onMove);
                window.addEventListener('pointerup', onUp);
                window.addEventListener('keydown', onKeyDown);
            }}

            // --- Annotation editor ---
            function openAnnotationEditor(d, event) {{
                // Close any existing editor
                d3.selectAll('.kg-annotation-editor').remove();

                const srcId = typeof d.source === 'object' ? d.source.id : d.source;
                const tgtId = typeof d.target === 'object' ? d.target.id : d.target;
                const srcTitle = nodeMap[srcId] ? nodeMap[srcId].title : srcId;
                const tgtTitle = nodeMap[tgtId] ? nodeMap[tgtId].title : tgtId;

                const editor = d3.select(_kgContainer).append('div')
                    .attr('class', 'kg-annotation-editor')
                    .style('left', (event.offsetX + 10) + 'px')
                    .style('top', (event.offsetY - 10) + 'px');

                editor.append('div').attr('class', 'kg-ann-label')
                    .text(srcTitle + ' \u2192 ' + tgtTitle);

                const textarea = editor.append('textarea')
                    .attr('placeholder', 'Describe this link...')
                    .property('value', d.annotation || '');

                const actions = editor.append('div').attr('class', 'kg-ann-actions');
                actions.append('button').attr('class', 'btn secondary').text('Cancel')
                    .on('click', () => editor.remove());
                actions.append('button').attr('class', 'btn danger').text('Delete')
                    .style('background', '#dc322f').style('margin-right', 'auto')
                    .on('click', function() {{
                        if (!confirm('Remove this manual link?')) return;
                        const btn = d3.select(this);
                        btn.attr('disabled', true).text('Deleting...');
                        editor.remove();
                        deleteEdge(srcId, tgtId);
                    }});
                const saveBtn = actions.append('button').attr('class', 'btn').text('Save')
                    .on('click', () => {{
                        const ann = textarea.property('value').trim() || null;
                        saveBtn.attr('disabled', true).text('Saving...');
                        fetch('/api/graph/edge/annotation', {{
                            method: 'POST',
                            headers: {{ 'Content-Type': 'application/json' }},
                            body: JSON.stringify({{ source: srcId, target: tgtId, annotation: ann }})
                        }}).then(r => {{
                            if (r.ok) {{
                                d.annotation = ann;
                                updateAnnotationVisuals();
                                editor.remove();
                            }} else {{
                                r.text().then(t => alert('Error: ' + t));
                                saveBtn.attr('disabled', null).text('Save');
                            }}
                        }}).catch(e => {{
                            alert('Network error: ' + e);
                            saveBtn.attr('disabled', null).text('Save');
                        }});
                    }});

                // Close on Escape
                function escHandler(event) {{
                    if (event.key === 'Escape') {{
                        editor.remove();
                        document.removeEventListener('keydown', escHandler);
                    }}
                }}
                document.addEventListener('keydown', escHandler);
                setTimeout(() => textarea.node().focus(), 10);
            }}
        {fn_close}
        </script>"##,
        d3_tag = d3_tag,
        fn_open = fn_open,
        fn_close = fn_close,
        container_sel = container_sel,
        center_key_js = center_key_js,
        is_mini = is_mini,
        logged_in = logged_in,
        show_arrows = show_arrows,
        show_edge_tooltips = show_edge_tooltips,
        auto_fit = auto_fit,
        max_nodes = max_nodes,
        notes_data_js = notes_data_js,
        data_loader_js = data_loader_js,
    )
}
