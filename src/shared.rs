//! Shared notes - collaborative editing via WebSocket + Automerge CRDT.
//!
//! A shared note is a copy of an existing note accessible via a unique URL.
//! External users can edit simultaneously with real-time sync and line-level attribution.

use crate::auth::is_logged_in;
use crate::models::{Note, NoteType, PaperMeta};
use crate::notes::{process_crosslinks, render_markdown};
use crate::templates::{render_editor, render_viewer};
use crate::AppState;
use automerge::{AutoCommit, ObjType, ReadDoc, transaction::Transactable};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, Query, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Json,
};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::broadcast;

/// Global connection ID counter for unique client identification.
static NEXT_CONN_ID: AtomicU64 = AtomicU64::new(1);

// ============================================================================
// Data Structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedNoteMeta {
    pub share_token: String,
    pub source_note_key: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub contributors: Vec<Contributor>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contributor {
    pub id: String,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineAttribution {
    pub lines: Vec<LineAuthor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineAuthor {
    pub contributor_id: String,
    pub timestamp: DateTime<Utc>,
}

/// In-memory state for an active shared note room.
pub struct SharedRoom {
    pub doc: AutoCommit,
    pub meta: SharedNoteMeta,
    pub attribution: LineAttribution,
    pub tx: broadcast::Sender<BroadcastMsg>,
    pub peer_count: usize,
    pub last_snapshot: std::time::Instant,
}

#[derive(Clone, Debug)]
pub struct BroadcastMsg {
    pub sender_id: u64,
    pub kind: BroadcastKind,
}

#[derive(Clone, Debug)]
pub enum BroadcastKind {
    /// Full updated text after an edit
    TextUpdate(String),
    /// Peer count update (JSON)
    PeersUpdate(String),
    /// Attribution update (JSON)
    Attribution(String),
    /// Snapshot saved confirmation
    Saved,
    /// Contributors list update (JSON)
    ContributorsUpdate(String),
}

/// Predefined contributor colors (solarized palette).
const CONTRIBUTOR_COLORS: &[&str] = &[
    "#268bd2", // blue
    "#d33682", // magenta
    "#859900", // green
    "#cb4b16", // orange
    "#6c71c4", // violet
    "#2aa198", // cyan
    "#b58900", // yellow
    "#dc322f", // red
];

// ============================================================================
// Sled helpers
// ============================================================================

fn meta_tree(db: &sled::Db) -> sled::Tree {
    db.open_tree("shared:meta").expect("open shared:meta tree")
}

fn doc_tree(db: &sled::Db, token: &str) -> sled::Tree {
    db.open_tree(format!("shared:doc:{}", token))
        .expect("open shared:doc tree")
}

fn attrib_tree(db: &sled::Db, token: &str) -> sled::Tree {
    db.open_tree(format!("shared:attrib:{}", token))
        .expect("open shared:attrib tree")
}

fn save_meta(db: &sled::Db, meta: &SharedNoteMeta) {
    let tree = meta_tree(db);
    let json = serde_json::to_vec(meta).unwrap();
    tree.insert(meta.share_token.as_bytes(), json).ok();
}

fn load_meta(db: &sled::Db, token: &str) -> Option<SharedNoteMeta> {
    let tree = meta_tree(db);
    tree.get(token.as_bytes())
        .ok()
        .flatten()
        .and_then(|v| serde_json::from_slice(&v).ok())
}

fn save_doc_bytes(db: &sled::Db, token: &str, bytes: &[u8]) {
    let tree = doc_tree(db, token);
    tree.insert("doc", bytes).ok();
}

fn load_doc_bytes(db: &sled::Db, token: &str) -> Option<Vec<u8>> {
    let tree = doc_tree(db, token);
    tree.get("doc").ok().flatten().map(|v| v.to_vec())
}

fn save_attribution(db: &sled::Db, token: &str, attrib: &LineAttribution) {
    let tree = attrib_tree(db, token);
    let json = serde_json::to_vec(attrib).unwrap();
    tree.insert("lines", json).ok();
}

fn load_attribution(db: &sled::Db, token: &str) -> LineAttribution {
    let tree = attrib_tree(db, token);
    tree.get("lines")
        .ok()
        .flatten()
        .and_then(|v| serde_json::from_slice(&v).ok())
        .unwrap_or(LineAttribution { lines: vec![] })
}

fn generate_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 16] = rng.gen();
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// ============================================================================
// Synthetic Note from shared doc
// ============================================================================

/// Build a Note struct from the automerge doc text for use with render_editor/render_viewer.
fn build_note_from_text(meta: &SharedNoteMeta, text: &str) -> Note {
    let (fm, body) = crate::notes::parse_frontmatter(text);
    let note_type = if fm.note_type.as_deref() == Some("paper") || !fm.bibtex_entries.is_empty() {
        NoteType::Paper(PaperMeta {
            bibtex_entries: fm.bibtex_entries,
            canonical_key: fm.canonical_key,
            sources: fm.sources,
        })
    } else {
        NoteType::Note
    };
    Note {
        key: format!("shared-{}", &meta.share_token[..8]),
        path: PathBuf::from("shared"),
        title: fm.title.unwrap_or_else(|| meta.title.clone()),
        date: fm.date,
        note_type,
        parent_key: None,
        time_entries: vec![],
        raw_content: body,
        full_file_content: text.to_string(),
        modified: meta.updated_at,
        pdf: fm.pdf,
        hidden: false,
    }
}

/// Get the current text of the automerge doc, either from in-memory room or sled.
/// This is async because it needs to check the in-memory rooms first.
async fn get_current_text(state: &AppState, token: &str) -> String {
    // Check in-memory room first (most up-to-date if clients are connected)
    {
        let rooms = state.shared_rooms.read().await;
        if let Some(room) = rooms.get(token) {
            return get_doc_text(&room.doc);
        }
    }
    // Fall back to sled
    if let Some(bytes) = load_doc_bytes(&state.db, token) {
        if let Ok(doc) = AutoCommit::load(&bytes) {
            return get_doc_text(&doc);
        }
    }
    String::new()
}

// ============================================================================
// REST Handlers
// ============================================================================

#[derive(Deserialize)]
pub struct CreateSharedRequest {
    pub note_key: String,
    #[serde(default)]
    pub contributors: Vec<ContributorInput>,
}

#[derive(Deserialize)]
pub struct ContributorInput {
    pub name: String,
}

#[derive(Deserialize)]
pub struct SharedPageQuery {
    #[serde(default)]
    pub edit: Option<bool>,
}

/// POST /api/shared/create - Create a new shared note (owner only).
pub async fn create_shared_note(
    jar: CookieJar,
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSharedRequest>,
) -> Response {
    if !is_logged_in(&jar, &state.db) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let notes_map = state.notes_map();
    let note = match notes_map.get(&req.note_key) {
        Some(n) => n,
        None => return (StatusCode::NOT_FOUND, "Note not found").into_response(),
    };

    let token = generate_token();

    let contributors: Vec<Contributor> = req
        .contributors
        .iter()
        .enumerate()
        .map(|(i, c)| Contributor {
            id: format!("c{}", i),
            name: c.name.clone(),
            color: CONTRIBUTOR_COLORS[i % CONTRIBUTOR_COLORS.len()].to_string(),
        })
        .collect();

    let mut doc = AutoCommit::new();
    let text = doc.put_object(automerge::ROOT, "text", ObjType::Text).unwrap();
    doc.splice_text(&text, 0, 0, &note.full_file_content).unwrap();
    let doc_bytes = doc.save();

    let now = Utc::now();
    let meta = SharedNoteMeta {
        share_token: token.clone(),
        source_note_key: req.note_key.clone(),
        title: note.title.clone(),
        created_at: now,
        updated_at: now,
        contributors,
        active: true,
    };

    let line_count = note.full_file_content.lines().count().max(1);
    let attribution = LineAttribution {
        lines: vec![
            LineAuthor {
                contributor_id: "owner".to_string(),
                timestamp: now,
            };
            line_count
        ],
    };

    save_meta(&state.db, &meta);
    save_doc_bytes(&state.db, &token, &doc_bytes);
    save_attribution(&state.db, &token, &attribution);
    snapshot_to_disk(&token, &doc).ok();

    Json(serde_json::json!({
        "token": token,
        "url": format!("/shared/{}", token),
        "title": meta.title,
    }))
    .into_response()
}

/// GET /api/shared/list/{note_key} - List shared notes for a source note (owner only).
pub async fn list_shared_notes(
    jar: CookieJar,
    State(state): State<Arc<AppState>>,
    Path(note_key): Path<String>,
) -> Response {
    if !is_logged_in(&jar, &state.db) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let tree = meta_tree(&state.db);
    let mut shares: Vec<SharedNoteMeta> = vec![];

    for entry in tree.iter() {
        if let Ok((_, v)) = entry {
            if let Ok(meta) = serde_json::from_slice::<SharedNoteMeta>(&v) {
                if meta.source_note_key == note_key {
                    shares.push(meta);
                }
            }
        }
    }

    shares.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Json(shares).into_response()
}

/// POST /api/shared/{token}/deactivate - Deactivate a shared note (owner only).
pub async fn deactivate_shared_note(
    jar: CookieJar,
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Response {
    if !is_logged_in(&jar, &state.db) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let mut meta = match load_meta(&state.db, &token) {
        Some(m) => m,
        None => return (StatusCode::NOT_FOUND, "Shared note not found").into_response(),
    };

    meta.active = !meta.active;
    meta.updated_at = Utc::now();
    save_meta(&state.db, &meta);

    Json(serde_json::json!({ "active": meta.active })).into_response()
}

/// POST /api/shared/{token}/contributors - Manage contributors (owner only).
pub async fn manage_contributors(
    jar: CookieJar,
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
    Json(contributors): Json<Vec<ContributorInput>>,
) -> Response {
    if !is_logged_in(&jar, &state.db) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let mut meta = match load_meta(&state.db, &token) {
        Some(m) => m,
        None => return (StatusCode::NOT_FOUND, "Shared note not found").into_response(),
    };

    meta.contributors = contributors
        .iter()
        .enumerate()
        .map(|(i, c)| Contributor {
            id: format!("c{}", i),
            name: c.name.clone(),
            color: CONTRIBUTOR_COLORS[i % CONTRIBUTOR_COLORS.len()].to_string(),
        })
        .collect();
    meta.updated_at = Utc::now();
    save_meta(&state.db, &meta);

    Json(&meta.contributors).into_response()
}

/// GET /shared/{token} - Serve the shared page (public). View by default, ?edit=true for editor.
pub async fn shared_editor_page(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
    Query(query): Query<SharedPageQuery>,
) -> Response {
    let meta = match load_meta(&state.db, &token) {
        Some(m) => m,
        None => return (StatusCode::NOT_FOUND, "Shared note not found").into_response(),
    };

    if !meta.active {
        return (StatusCode::GONE, "This shared note has been deactivated").into_response();
    }

    // Get current text from in-memory room or sled
    let text = get_current_text(&state, &token).await;
    let note = build_note_from_text(&meta, &text);

    let contributors_json =
        serde_json::to_string(&meta.contributors).unwrap_or_else(|_| "[]".to_string());

    let edit_mode = query.edit.unwrap_or(false);

    if edit_mode {
        // Edit mode: serve the real editor template + inject shared overlay
        let base_html = render_editor(&note, &HashMap::new(), false);
        let overlay = crate::templates::shared_editor::render_shared_overlay(&token, &contributors_json);

        // Inject overlay script before </body>
        let html = base_html.replace("</body>", &format!("{}\n</body>", overlay));
        Html(html).into_response()
    } else {
        // View mode: serve the viewer template + inject attribution overlay
        let content_with_links = process_crosslinks(&note.raw_content, &HashMap::new());
        let rendered_content = render_markdown(&content_with_links);

        let meta_html = crate::handlers::build_note_meta_html(&note, &HashMap::new());
        let is_paper = matches!(note.note_type, NoteType::Paper(_));

        let base_html = render_viewer(
            &note,
            &rendered_content,
            &meta_html,
            "",  // no time tracking
            "",  // no sub notes
            "",  // no history
            false, // not logged in
            is_paper,
        );

        // Inject shared view overlay (attribution hovers, mode toggle, etc.)
        let overlay = crate::templates::shared_editor::render_shared_view_overlay(&token, &contributors_json);
        let html = base_html.replace("</body>", &format!("{}\n</body>", overlay));
        Html(html).into_response()
    }
}

/// GET /api/shared/{token}/attribution - Get attribution data (public).
pub async fn get_attribution(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Response {
    let meta = match load_meta(&state.db, &token) {
        Some(m) if m.active => m,
        Some(_) => return (StatusCode::GONE, "Deactivated").into_response(),
        None => return (StatusCode::NOT_FOUND, "Not found").into_response(),
    };

    // Check in-memory room first
    {
        let rooms = state.shared_rooms.read().await;
        if let Some(room) = rooms.get(&token) {
            return Json(serde_json::json!({
                "attribution": room.attribution,
                "contributors": meta.contributors,
            })).into_response();
        }
    }

    // Fall back to sled
    let attribution = load_attribution(&state.db, &token);
    Json(serde_json::json!({
        "attribution": attribution,
        "contributors": meta.contributors,
    })).into_response()
}

// ============================================================================
// WebSocket Handler
// ============================================================================

/// GET /shared/{token}/ws - WebSocket upgrade for real-time sync (public).
pub async fn ws_handler(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
    ws: WebSocketUpgrade,
) -> Response {
    let meta = match load_meta(&state.db, &token) {
        Some(m) if m.active => m,
        Some(_) => return (StatusCode::GONE, "Deactivated").into_response(),
        None => return (StatusCode::NOT_FOUND, "Not found").into_response(),
    };

    ws.on_upgrade(move |socket| handle_ws(socket, state, token, meta))
}

async fn handle_ws(socket: WebSocket, state: Arc<AppState>, token: String, meta: SharedNoteMeta) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    // Assign unique ID to this connection
    let my_id = NEXT_CONN_ID.fetch_add(1, Ordering::Relaxed);

    // Get or create room
    let (mut broadcast_rx, initial_text, initial_attrib, peer_count) = {
        let mut rooms = state.shared_rooms.write().await;
        let room = rooms.entry(token.clone()).or_insert_with(|| {
            let doc = if let Some(bytes) = load_doc_bytes(&state.db, &token) {
                AutoCommit::load(&bytes).unwrap_or_else(|_| {
                    let mut d = AutoCommit::new();
                    d.put_object(automerge::ROOT, "text", ObjType::Text).ok();
                    d
                })
            } else {
                let mut d = AutoCommit::new();
                d.put_object(automerge::ROOT, "text", ObjType::Text).ok();
                d
            };

            let attribution = load_attribution(&state.db, &token);
            let (tx, _) = broadcast::channel(256);

            SharedRoom {
                doc,
                meta: meta.clone(),
                attribution,
                tx,
                peer_count: 0,
                last_snapshot: std::time::Instant::now(),
            }
        });

        room.peer_count += 1;
        let count = room.peer_count;
        let text = get_doc_text(&room.doc);
        let attrib = serde_json::to_string(&room.attribution).unwrap_or_default();

        // Broadcast peer update to existing clients (the new client hasn't
        // subscribed yet, so it won't see this — it gets count via init message)
        room.tx.send(BroadcastMsg {
            sender_id: 0,
            kind: BroadcastKind::PeersUpdate(
                serde_json::json!({ "count": count }).to_string()
            ),
        }).ok();

        (room.tx.subscribe(), text, attrib, count)
    };

    // Send initial state (including peer count, since the PeersUpdate broadcast
    // was sent before this client subscribed)
    let init_msg = serde_json::json!({
        "type": "init",
        "text": initial_text,
        "attribution": initial_attrib,
        "contributors": meta.contributors,
        "peers": peer_count,
    });
    if ws_tx.send(Message::Text(init_msg.to_string().into())).await.is_err() {
        return;
    }

    let token_clone = token.clone();
    let state_clone = state.clone();

    // Spawn broadcast receiver -> ws_tx forwarder
    // Skips messages from this client (sender_id == my_id)
    let mut forward_task = tokio::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            // Skip messages we sent (except system messages with sender_id=0)
            if msg.sender_id == my_id {
                continue;
            }

            let json = match &msg.kind {
                BroadcastKind::TextUpdate(text) => {
                    serde_json::json!({ "type": "text_update", "text": text }).to_string()
                }
                BroadcastKind::PeersUpdate(data) => {
                    serde_json::json!({ "type": "peers", "data": data }).to_string()
                }
                BroadcastKind::Attribution(data) => {
                    serde_json::json!({ "type": "attribution", "data": data }).to_string()
                }
                BroadcastKind::Saved => {
                    serde_json::json!({ "type": "saved" }).to_string()
                }
                BroadcastKind::ContributorsUpdate(data) => {
                    serde_json::json!({ "type": "contributors", "data": data }).to_string()
                }
            };
            if ws_tx.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    let mut client_contributor_id = format!("conn-{}", my_id);

    // Read messages from client
    loop {
        tokio::select! {
            msg = ws_rx.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                            match parsed.get("type").and_then(|t| t.as_str()) {
                                Some("edit") => {
                                    if let Some(ops) = parsed.get("ops").and_then(|o| o.as_array()) {
                                        let mut rooms = state_clone.shared_rooms.write().await;
                                        if let Some(room) = rooms.get_mut(&token_clone) {
                                            apply_ops_to_doc(room, ops, &client_contributor_id);

                                            let updated_text = get_doc_text(&room.doc);
                                            let attrib_json = serde_json::to_string(&room.attribution).unwrap_or_default();

                                            // Broadcast text update (other clients will receive it)
                                            room.tx.send(BroadcastMsg {
                                                sender_id: my_id,
                                                kind: BroadcastKind::TextUpdate(updated_text),
                                            }).ok();

                                            // Broadcast attribution (all clients including sender)
                                            room.tx.send(BroadcastMsg {
                                                sender_id: 0, // system - all get it
                                                kind: BroadcastKind::Attribution(attrib_json),
                                            }).ok();

                                            // Periodic sled snapshot
                                            if room.last_snapshot.elapsed() > std::time::Duration::from_secs(30) {
                                                let doc_bytes = room.doc.save();
                                                save_doc_bytes(&state_clone.db, &token_clone, &doc_bytes);
                                                save_attribution(&state_clone.db, &token_clone, &room.attribution);
                                                room.last_snapshot = std::time::Instant::now();
                                            }
                                        }
                                    }
                                }
                                Some("snapshot") => {
                                    // Force save to sled + disk
                                    let mut rooms = state_clone.shared_rooms.write().await;
                                    if let Some(room) = rooms.get_mut(&token_clone) {
                                        let doc_bytes = room.doc.save();
                                        save_doc_bytes(&state_clone.db, &token_clone, &doc_bytes);
                                        save_attribution(&state_clone.db, &token_clone, &room.attribution);
                                        snapshot_to_disk(&token_clone, &room.doc).ok();
                                        room.last_snapshot = std::time::Instant::now();
                                        // Send confirmation to all clients
                                        room.tx.send(BroadcastMsg {
                                            sender_id: 0,
                                            kind: BroadcastKind::Saved,
                                        }).ok();
                                    }
                                }
                                Some("identify") => {
                                    let id = parsed.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let name = parsed.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let color = parsed.get("color").and_then(|v| v.as_str()).unwrap_or(CONTRIBUTOR_COLORS[0]).to_string();

                                    if !id.is_empty() {
                                        client_contributor_id = id.clone();

                                        let mut rooms = state_clone.shared_rooms.write().await;
                                        if let Some(room) = rooms.get_mut(&token_clone) {
                                            // Upsert contributor
                                            if let Some(existing) = room.meta.contributors.iter_mut().find(|c| c.id == id) {
                                                existing.name = name;
                                                existing.color = color;
                                            } else {
                                                room.meta.contributors.push(Contributor {
                                                    id,
                                                    name,
                                                    color,
                                                });
                                            }

                                            // Save updated meta to sled
                                            save_meta(&state_clone.db, &room.meta);

                                            // Broadcast updated contributor list
                                            let contributors_json = serde_json::to_string(&room.meta.contributors).unwrap_or_else(|_| "[]".to_string());
                                            room.tx.send(BroadcastMsg {
                                                sender_id: 0,
                                                kind: BroadcastKind::ContributorsUpdate(contributors_json),
                                            }).ok();
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
            _ = &mut forward_task => break,
        }
    }

    // Cleanup
    {
        let mut rooms = state.shared_rooms.write().await;
        if let Some(room) = rooms.get_mut(&token) {
            room.peer_count = room.peer_count.saturating_sub(1);

            let doc_bytes = room.doc.save();
            save_doc_bytes(&state.db, &token, &doc_bytes);
            save_attribution(&state.db, &token, &room.attribution);

            room.tx.send(BroadcastMsg {
                sender_id: 0,
                kind: BroadcastKind::PeersUpdate(
                    serde_json::json!({ "count": room.peer_count }).to_string()
                ),
            }).ok();

            if room.peer_count == 0 {
                let state_for_cleanup = state.clone();
                let token_for_cleanup = token.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                    let mut rooms = state_for_cleanup.shared_rooms.write().await;
                    if let Some(room) = rooms.get(&token_for_cleanup) {
                        if room.peer_count == 0 {
                            snapshot_to_disk(&token_for_cleanup, &room.doc).ok();
                            rooms.remove(&token_for_cleanup);
                        }
                    }
                });
            }
        }
    }

    forward_task.abort();
}

// ============================================================================
// Automerge Helpers
// ============================================================================

fn get_doc_text(doc: &AutoCommit) -> String {
    if let Some((_, text_id)) = doc.get(automerge::ROOT, "text").ok().flatten() {
        doc.text(&text_id).unwrap_or_default()
    } else {
        String::new()
    }
}

fn apply_ops_to_doc(
    room: &mut SharedRoom,
    ops: &[serde_json::Value],
    contributor_id: &str,
) {
    let text_id = match room.doc.get(automerge::ROOT, "text").ok().flatten() {
        Some((_, id)) => id,
        None => return,
    };

    let now = Utc::now();

    for op in ops {
        let op_type = op.get("type").and_then(|t| t.as_str()).unwrap_or("");
        let pos = op.get("pos").and_then(|p| p.as_u64()).unwrap_or(0) as usize;

        match op_type {
            "insert" => {
                if let Some(text) = op.get("text").and_then(|t| t.as_str()) {
                    room.doc.splice_text(&text_id, pos, 0, text).ok();

                    // Update attribution for newlines
                    let new_lines = text.matches('\n').count();
                    if new_lines > 0 {
                        let current_text = get_doc_text(&room.doc);
                        let line_idx = current_text[..pos.min(current_text.len())]
                            .matches('\n')
                            .count();
                        let author = LineAuthor {
                            contributor_id: contributor_id.to_string(),
                            timestamp: now,
                        };
                        for i in 0..new_lines {
                            let idx = line_idx + i + 1;
                            if idx <= room.attribution.lines.len() {
                                room.attribution.lines.insert(idx, author.clone());
                            }
                        }
                    }

                    // Mark current line as edited
                    let current_text = get_doc_text(&room.doc);
                    let line_idx = current_text[..pos.min(current_text.len())]
                        .matches('\n')
                        .count();
                    while room.attribution.lines.len() <= line_idx {
                        room.attribution.lines.push(LineAuthor {
                            contributor_id: contributor_id.to_string(),
                            timestamp: now,
                        });
                    }
                    room.attribution.lines[line_idx] = LineAuthor {
                        contributor_id: contributor_id.to_string(),
                        timestamp: now,
                    };
                }
            }
            "delete" => {
                let len = op.get("len").and_then(|l| l.as_u64()).unwrap_or(1) as usize;
                let current_text = get_doc_text(&room.doc);
                let end = (pos + len).min(current_text.len());
                let deleted_text = &current_text[pos..end];
                let deleted_newlines = deleted_text.matches('\n').count();

                room.doc.splice_text(&text_id, pos, len as isize, "").ok();

                if deleted_newlines > 0 {
                    let line_idx = current_text[..pos].matches('\n').count();
                    for _ in 0..deleted_newlines {
                        if line_idx + 1 < room.attribution.lines.len() {
                            room.attribution.lines.remove(line_idx + 1);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    room.meta.updated_at = now;
}

// ============================================================================
// Disk Snapshots
// ============================================================================

fn snapshot_to_disk(token: &str, doc: &AutoCommit) -> std::io::Result<()> {
    let dir = std::path::Path::new("shared");
    std::fs::create_dir_all(dir)?;
    let text = get_doc_text(doc);
    std::fs::write(dir.join(format!("{}.md", token)), text)?;
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::NoteType;

    fn make_meta() -> SharedNoteMeta {
        SharedNoteMeta {
            share_token: "abcdef1234567890abcdef1234567890".to_string(),
            source_note_key: "test-note".to_string(),
            title: "Test Note".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            contributors: vec![
                Contributor {
                    id: "c0".to_string(),
                    name: "Alice".to_string(),
                    color: "#268bd2".to_string(),
                },
                Contributor {
                    id: "c1".to_string(),
                    name: "Bob".to_string(),
                    color: "#d33682".to_string(),
                },
            ],
            active: true,
        }
    }

    // ---- build_note_from_text tests ----

    #[test]
    fn test_build_note_regular_note() {
        let meta = make_meta();
        let text = "---\ntitle: My Regular Note\ndate: 2024-06-15\n---\n\nSome body content here.\n";
        let note = build_note_from_text(&meta, text);

        assert_eq!(note.title, "My Regular Note");
        assert!(matches!(note.note_type, NoteType::Note));
        assert_eq!(note.date.unwrap().to_string(), "2024-06-15");
        assert_eq!(note.raw_content.trim(), "Some body content here.");
        assert!(!note.raw_content.contains("---"));
        assert!(note.full_file_content.contains("---"));
        assert_eq!(note.key, "shared-abcdef12");
        assert_eq!(note.pdf, None);
        assert!(!note.hidden);
    }

    #[test]
    fn test_build_note_paper_with_bibtex() {
        let meta = make_meta();
        let text = r#"---
title: Attention Is All You Need
type: paper
date: 2017-06-12
bibtex: |
  @article{vaswani2017attention,
    title={Attention is all you need},
    author={Vaswani, Ashish and others},
    journal={NeurIPS},
    year={2017}
  }
---

This paper introduces the transformer architecture.
"#;
        let note = build_note_from_text(&meta, text);

        assert_eq!(note.title, "Attention Is All You Need");
        assert!(matches!(note.note_type, NoteType::Paper(_)));
        if let NoteType::Paper(ref paper) = note.note_type {
            assert!(!paper.bibtex_entries.is_empty());
            assert!(paper.bibtex_entries[0].contains("vaswani2017attention"));
        }
        assert!(note.raw_content.contains("transformer architecture"));
        assert!(!note.raw_content.contains("bibtex:"));
    }

    #[test]
    fn test_build_note_paper_by_type_field() {
        let meta = make_meta();
        let text = "---\ntitle: Some Paper\ntype: paper\n---\n\nBody text.\n";
        let note = build_note_from_text(&meta, text);

        assert!(matches!(note.note_type, NoteType::Paper(_)));
    }

    #[test]
    fn test_build_note_title_fallback_to_meta() {
        let meta = make_meta();
        let text = "---\ndate: 2024-01-01\n---\n\nNo title in frontmatter.\n";
        let note = build_note_from_text(&meta, text);

        // Should fall back to meta.title since frontmatter has no title
        assert_eq!(note.title, "Test Note");
    }

    #[test]
    fn test_build_note_no_frontmatter() {
        let meta = make_meta();
        let text = "Just plain text, no frontmatter at all.\n\nSecond paragraph.";
        let note = build_note_from_text(&meta, text);

        assert_eq!(note.title, "Test Note"); // fallback
        assert!(matches!(note.note_type, NoteType::Note));
        assert!(note.raw_content.contains("Just plain text"));
    }

    #[test]
    fn test_build_note_with_pdf() {
        let meta = make_meta();
        let text = "---\ntitle: Paper With PDF\ntype: paper\npdf: vaswani2017.pdf\n---\n\nContent.\n";
        let note = build_note_from_text(&meta, text);

        assert_eq!(note.pdf, Some("vaswani2017.pdf".to_string()));
    }

    #[test]
    fn test_build_note_with_sources() {
        let meta = make_meta();
        // Sources are parsed as top-level keys: arxiv, doi, url
        let text = "---\ntitle: Sourced Paper\ntype: paper\narxiv: 1706.03762\ndoi: 10.5555/3295222.3295349\n---\n\nContent.\n";
        let note = build_note_from_text(&meta, text);

        if let NoteType::Paper(ref paper) = note.note_type {
            assert_eq!(paper.sources.len(), 2);
            assert_eq!(paper.sources[0].source_type, "arxiv");
            assert_eq!(paper.sources[0].identifier, "1706.03762");
            assert_eq!(paper.sources[1].source_type, "doi");
            assert_eq!(paper.sources[1].identifier, "10.5555/3295222.3295349");
        } else {
            panic!("Expected Paper note type");
        }
    }

    #[test]
    fn test_build_note_key_uses_token_prefix() {
        let meta = make_meta();
        let text = "---\ntitle: Test\n---\n\nBody.\n";
        let note = build_note_from_text(&meta, text);

        assert!(note.key.starts_with("shared-"));
        assert_eq!(note.key, "shared-abcdef12");
    }

    #[test]
    fn test_build_note_hidden_field_always_false() {
        let meta = make_meta();
        // Even if the frontmatter says hidden: true, shared notes are not hidden
        let text = "---\ntitle: Hidden Note\nhidden: true\n---\n\nContent.\n";
        let note = build_note_from_text(&meta, text);

        // hidden is set to false regardless of frontmatter
        assert!(!note.hidden);
    }

    #[test]
    fn test_build_note_path_is_shared() {
        let meta = make_meta();
        let text = "---\ntitle: Test\n---\n\nBody.\n";
        let note = build_note_from_text(&meta, text);

        assert_eq!(note.path, PathBuf::from("shared"));
    }

    #[test]
    fn test_build_note_no_parent_key() {
        let meta = make_meta();
        let text = "---\ntitle: Test\nparent: some-parent\n---\n\nBody.\n";
        let note = build_note_from_text(&meta, text);

        // parent_key is always None in shared context
        assert!(note.parent_key.is_none());
    }

    #[test]
    fn test_build_note_empty_time_entries() {
        let meta = make_meta();
        let text = "---\ntitle: Test\n---\n\nBody.\n";
        let note = build_note_from_text(&meta, text);

        assert!(note.time_entries.is_empty());
    }

    // ---- Sled helpers tests ----

    #[test]
    fn test_save_and_load_meta() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let meta = make_meta();
        save_meta(&db, &meta);

        let loaded = load_meta(&db, &meta.share_token).unwrap();
        assert_eq!(loaded.share_token, meta.share_token);
        assert_eq!(loaded.source_note_key, meta.source_note_key);
        assert_eq!(loaded.title, meta.title);
        assert_eq!(loaded.contributors.len(), 2);
        assert_eq!(loaded.contributors[0].name, "Alice");
        assert!(loaded.active);
    }

    #[test]
    fn test_load_meta_not_found() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let loaded = load_meta(&db, "nonexistent");
        assert!(loaded.is_none());
    }

    #[test]
    fn test_save_and_load_doc_bytes() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let token = "test_token";
        let data = b"test document bytes";
        save_doc_bytes(&db, token, data);

        let loaded = load_doc_bytes(&db, token).unwrap();
        assert_eq!(loaded, data);
    }

    #[test]
    fn test_load_doc_bytes_not_found() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let loaded = load_doc_bytes(&db, "nonexistent");
        assert!(loaded.is_none());
    }

    #[test]
    fn test_save_and_load_attribution() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let token = "test_token";
        let attrib = LineAttribution {
            lines: vec![
                LineAuthor {
                    contributor_id: "c0".to_string(),
                    timestamp: Utc::now(),
                },
                LineAuthor {
                    contributor_id: "c1".to_string(),
                    timestamp: Utc::now(),
                },
            ],
        };
        save_attribution(&db, token, &attrib);

        let loaded = load_attribution(&db, token);
        assert_eq!(loaded.lines.len(), 2);
        assert_eq!(loaded.lines[0].contributor_id, "c0");
        assert_eq!(loaded.lines[1].contributor_id, "c1");
    }

    #[test]
    fn test_load_attribution_default_empty() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let loaded = load_attribution(&db, "nonexistent");
        assert!(loaded.lines.is_empty());
    }

    // ---- Token generation tests ----

    #[test]
    fn test_generate_token_length() {
        let token = generate_token();
        assert_eq!(token.len(), 32); // 16 bytes * 2 hex chars
    }

    #[test]
    fn test_generate_token_uniqueness() {
        let t1 = generate_token();
        let t2 = generate_token();
        assert_ne!(t1, t2);
    }

    #[test]
    fn test_generate_token_hex_chars() {
        let token = generate_token();
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // ---- Automerge helpers tests ----

    #[test]
    fn test_get_doc_text_empty_doc() {
        let doc = AutoCommit::new();
        let text = get_doc_text(&doc);
        assert!(text.is_empty());
    }

    #[test]
    fn test_get_doc_text_with_content() {
        let mut doc = AutoCommit::new();
        let text_id = doc.put_object(automerge::ROOT, "text", ObjType::Text).unwrap();
        doc.splice_text(&text_id, 0, 0, "Hello, world!").unwrap();

        let text = get_doc_text(&doc);
        assert_eq!(text, "Hello, world!");
    }

    #[test]
    fn test_apply_ops_insert() {
        let mut doc = AutoCommit::new();
        let _text_id = doc.put_object(automerge::ROOT, "text", ObjType::Text).unwrap();

        let (tx, _) = broadcast::channel(16);
        let meta = make_meta();
        let mut room = SharedRoom {
            doc,
            meta,
            attribution: LineAttribution { lines: vec![] },
            tx,
            peer_count: 1,
            last_snapshot: std::time::Instant::now(),
        };

        let ops = vec![
            serde_json::json!({"type": "insert", "pos": 0, "text": "Hello"})
        ];
        apply_ops_to_doc(&mut room, &ops, "c0");

        let text = get_doc_text(&room.doc);
        assert_eq!(text, "Hello");
    }

    #[test]
    fn test_apply_ops_insert_with_newlines() {
        let mut doc = AutoCommit::new();
        let _text_id = doc.put_object(automerge::ROOT, "text", ObjType::Text).unwrap();

        let (tx, _) = broadcast::channel(16);
        let meta = make_meta();
        let mut room = SharedRoom {
            doc,
            meta,
            attribution: LineAttribution { lines: vec![] },
            tx,
            peer_count: 1,
            last_snapshot: std::time::Instant::now(),
        };

        let ops = vec![
            serde_json::json!({"type": "insert", "pos": 0, "text": "Line 1\nLine 2\nLine 3"})
        ];
        apply_ops_to_doc(&mut room, &ops, "c0");

        let text = get_doc_text(&room.doc);
        assert_eq!(text, "Line 1\nLine 2\nLine 3");
        // Attribution should have at least one entry (the current line is always tracked)
        assert!(!room.attribution.lines.is_empty());
        assert_eq!(room.attribution.lines[0].contributor_id, "c0");
    }

    #[test]
    fn test_apply_ops_delete() {
        let mut doc = AutoCommit::new();
        let text_id = doc.put_object(automerge::ROOT, "text", ObjType::Text).unwrap();
        doc.splice_text(&text_id, 0, 0, "Hello, world!").unwrap();

        let (tx, _) = broadcast::channel(16);
        let meta = make_meta();
        let mut room = SharedRoom {
            doc,
            meta,
            attribution: LineAttribution { lines: vec![] },
            tx,
            peer_count: 1,
            last_snapshot: std::time::Instant::now(),
        };

        // Delete ", world"
        let ops = vec![
            serde_json::json!({"type": "delete", "pos": 5, "len": 7})
        ];
        apply_ops_to_doc(&mut room, &ops, "c0");

        let text = get_doc_text(&room.doc);
        assert_eq!(text, "Hello!");
    }

    #[test]
    fn test_apply_ops_insert_then_delete() {
        let mut doc = AutoCommit::new();
        let _text_id = doc.put_object(automerge::ROOT, "text", ObjType::Text).unwrap();

        let (tx, _) = broadcast::channel(16);
        let meta = make_meta();
        let mut room = SharedRoom {
            doc,
            meta,
            attribution: LineAttribution { lines: vec![] },
            tx,
            peer_count: 1,
            last_snapshot: std::time::Instant::now(),
        };

        // Insert text
        let ops = vec![
            serde_json::json!({"type": "insert", "pos": 0, "text": "ABCDE"})
        ];
        apply_ops_to_doc(&mut room, &ops, "c0");
        assert_eq!(get_doc_text(&room.doc), "ABCDE");

        // Delete middle chars "BCD"
        let ops = vec![
            serde_json::json!({"type": "delete", "pos": 1, "len": 3})
        ];
        apply_ops_to_doc(&mut room, &ops, "c1");
        assert_eq!(get_doc_text(&room.doc), "AE");
    }

    #[test]
    fn test_apply_ops_attribution_tracking() {
        let mut doc = AutoCommit::new();
        let _text_id = doc.put_object(automerge::ROOT, "text", ObjType::Text).unwrap();

        let (tx, _) = broadcast::channel(16);
        let meta = make_meta();
        let mut room = SharedRoom {
            doc,
            meta,
            attribution: LineAttribution { lines: vec![] },
            tx,
            peer_count: 1,
            last_snapshot: std::time::Instant::now(),
        };

        // Alice inserts first line
        let ops = vec![
            serde_json::json!({"type": "insert", "pos": 0, "text": "Alice's line"})
        ];
        apply_ops_to_doc(&mut room, &ops, "c0");
        assert!(!room.attribution.lines.is_empty());
        assert_eq!(room.attribution.lines[0].contributor_id, "c0");
    }

    #[test]
    fn test_apply_ops_updates_timestamp() {
        let mut doc = AutoCommit::new();
        let _text_id = doc.put_object(automerge::ROOT, "text", ObjType::Text).unwrap();

        let (tx, _) = broadcast::channel(16);
        let meta = make_meta();
        let before = meta.updated_at;
        let mut room = SharedRoom {
            doc,
            meta,
            attribution: LineAttribution { lines: vec![] },
            tx,
            peer_count: 1,
            last_snapshot: std::time::Instant::now(),
        };

        std::thread::sleep(std::time::Duration::from_millis(10));

        let ops = vec![
            serde_json::json!({"type": "insert", "pos": 0, "text": "x"})
        ];
        apply_ops_to_doc(&mut room, &ops, "c0");

        assert!(room.meta.updated_at >= before);
    }

    // ---- Contributor color tests ----

    #[test]
    fn test_contributor_colors_not_empty() {
        assert!(!CONTRIBUTOR_COLORS.is_empty());
        assert!(CONTRIBUTOR_COLORS.len() >= 8);
    }

    #[test]
    fn test_contributor_colors_are_hex() {
        for color in CONTRIBUTOR_COLORS {
            assert!(color.starts_with('#'));
            assert_eq!(color.len(), 7);
        }
    }

    // ---- BroadcastKind / BroadcastMsg tests ----

    #[test]
    fn test_broadcast_msg_clone() {
        let msg = BroadcastMsg {
            sender_id: 42,
            kind: BroadcastKind::TextUpdate("hello".to_string()),
        };
        let cloned = msg.clone();
        assert_eq!(cloned.sender_id, 42);
        if let BroadcastKind::TextUpdate(ref text) = cloned.kind {
            assert_eq!(text, "hello");
        } else {
            panic!("Expected TextUpdate");
        }
    }

    #[test]
    fn test_broadcast_saved_variant() {
        let msg = BroadcastMsg {
            sender_id: 0,
            kind: BroadcastKind::Saved,
        };
        let cloned = msg.clone();
        assert!(matches!(cloned.kind, BroadcastKind::Saved));
    }

    // ---- Serialization tests ----

    #[test]
    fn test_shared_note_meta_serialization() {
        let meta = make_meta();
        let json = serde_json::to_string(&meta).unwrap();
        let deserialized: SharedNoteMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.share_token, meta.share_token);
        assert_eq!(deserialized.title, meta.title);
        assert_eq!(deserialized.contributors.len(), 2);
    }

    #[test]
    fn test_line_attribution_serialization() {
        let attrib = LineAttribution {
            lines: vec![
                LineAuthor {
                    contributor_id: "c0".to_string(),
                    timestamp: Utc::now(),
                },
            ],
        };
        let json = serde_json::to_string(&attrib).unwrap();
        let deserialized: LineAttribution = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.lines.len(), 1);
        assert_eq!(deserialized.lines[0].contributor_id, "c0");
    }

    #[test]
    fn test_contributor_serialization() {
        let contrib = Contributor {
            id: "c0".to_string(),
            name: "Alice".to_string(),
            color: "#268bd2".to_string(),
        };
        let json = serde_json::to_string(&contrib).unwrap();
        assert!(json.contains("Alice"));
        assert!(json.contains("#268bd2"));
    }

    // ---- Meta tree isolation test ----

    #[test]
    fn test_meta_tree_isolation() {
        let db = sled::Config::new().temporary(true).open().unwrap();

        let mut meta1 = make_meta();
        meta1.share_token = "token_aaa_1234567890123456".to_string();
        meta1.title = "Note 1".to_string();

        let mut meta2 = make_meta();
        meta2.share_token = "token_bbb_1234567890123456".to_string();
        meta2.title = "Note 2".to_string();

        save_meta(&db, &meta1);
        save_meta(&db, &meta2);

        let loaded1 = load_meta(&db, "token_aaa_1234567890123456").unwrap();
        let loaded2 = load_meta(&db, "token_bbb_1234567890123456").unwrap();
        assert_eq!(loaded1.title, "Note 1");
        assert_eq!(loaded2.title, "Note 2");
    }

    // ---- Paper metadata through shared note pipeline ----

    #[test]
    fn test_paper_note_bibtex_entries_preserved() {
        let meta = make_meta();
        let text = r#"---
title: Test Paper
type: paper
bibtex: |
  @inproceedings{smith2024test,
    title={A Test Paper},
    author={Smith, John},
    booktitle={ICML},
    year={2024}
  }
---

Paper content here.
"#;
        let note = build_note_from_text(&meta, text);
        if let NoteType::Paper(ref paper) = note.note_type {
            assert_eq!(paper.bibtex_entries.len(), 1);
            assert!(paper.bibtex_entries[0].contains("smith2024test"));
            assert!(paper.bibtex_entries[0].contains("ICML"));
        } else {
            panic!("Expected Paper note type");
        }
    }

    #[test]
    fn test_paper_detected_by_bibtex_alone() {
        // Even without type: paper, bibtex presence should make it a paper
        let meta = make_meta();
        let text = r#"---
title: Implicitly Paper
bibtex: |
  @article{doe2023implicit,
    title={Implicitly a Paper},
    author={Doe, Jane},
    year={2023}
  }
---

Content.
"#;
        let note = build_note_from_text(&meta, text);
        assert!(matches!(note.note_type, NoteType::Paper(_)));
    }

    #[test]
    fn test_full_file_content_preserved() {
        let meta = make_meta();
        let text = "---\ntitle: Test\n---\n\nBody content.\n";
        let note = build_note_from_text(&meta, text);
        assert_eq!(note.full_file_content, text);
    }

    #[test]
    fn test_modified_timestamp_from_meta() {
        let meta = make_meta();
        let text = "---\ntitle: Test\n---\n\nBody.\n";
        let note = build_note_from_text(&meta, text);
        assert_eq!(note.modified, meta.updated_at);
    }
}
