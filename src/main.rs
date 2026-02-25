//! Notes application - A personal knowledge management system.
//!
//! This is the main entry point for the notes web server.
//! The application is organized into the following modules:
//!
//! - `models`: Data structures for notes, papers, time tracking, and graphs
//! - `auth`: Session management and authentication
//! - `notes`: Note loading, parsing, search, and content processing
//! - `templates`: HTML/CSS/JS templates and rendering
//! - `graph`: Knowledge graph building and visualization
//! - `smart_add`: Smart paper/reference addition feature
//! - `handlers`: HTTP route handlers

use axum::{extract::DefaultBodyLimit, routing::get, Router};
use std::sync::Arc;
use tower_http::services::ServeDir;

use notes::{auth, citations, graph, handlers, shared, smart_add, AppState, NOTES_DIR};

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() {
    rayon::ThreadPoolBuilder::new()
        .num_threads(16)
        .build_global()
        .unwrap();

    let state = Arc::new(AppState::new());

    let app = Router::new()
        // Core routes
        .route("/", get(handlers::index))
        .route("/search", get(handlers::search))
        .route("/new", get(handlers::new_note_page).post(handlers::create_note))
        .route("/login", get(handlers::login_page).post(handlers::login_submit))
        .route("/logout", get(handlers::logout))
        // Note routes
        .route("/note/{key}", get(handlers::view_note))
        .route(
            "/api/note/{key}",
            axum::routing::post(handlers::save_note).delete(handlers::delete_note),
        )
        .route("/api/note/{key}/toggle-hidden", axum::routing::post(handlers::toggle_hidden))
        .route("/note/{key}/history/{commit}", get(handlers::view_note_history))
        // List routes
        .route("/papers", get(handlers::papers))
        .route("/papers/find-pdfs", get(handlers::find_pdfs_page))
        .route("/time", get(handlers::time_tracking))
        // Graph routes
        .route("/graph", get(graph::graph_page))
        .route("/api/graph", get(graph::graph_api))
        .route("/api/graph/edge", axum::routing::post(handlers::add_graph_edge).delete(handlers::delete_graph_edge))
        .route("/api/graph/edge/annotation", axum::routing::post(handlers::update_edge_annotation))
        .route("/api/notes/list", get(handlers::notes_list_api))
        // Smart Add routes
        .route("/api/smart-add/lookup", axum::routing::post(smart_add::smart_add_lookup))
        .route("/api/smart-add/create", axum::routing::post(smart_add::smart_add_create))
        .route("/api/smart-add/attach", axum::routing::post(smart_add::smart_add_attach))
        .route("/api/smart-add/quick-note", axum::routing::post(smart_add::quick_note_create))
        // BibTeX Import routes
        .route("/api/bib-import/analyze", axum::routing::post(smart_add::bib_import_analyze)
            .layer(DefaultBodyLimit::max(10 * 1024 * 1024)))
        .route("/api/bib-import/execute", axum::routing::post(smart_add::bib_import_execute))
        // Citation routes
        .route("/api/citations/scan", axum::routing::post(citations::citation_scan))
        .route("/api/citations/write", axum::routing::post(citations::citation_write))
        .route("/api/citations/scan-all", axum::routing::post(citations::citation_scan_all))
        // Export routes
        .route("/bibliography.bib", get(handlers::bibliography))
        // Shared notes routes
        .route("/api/shared/create", axum::routing::post(shared::create_shared_note))
        .route("/api/shared/list/{note_key}", get(shared::list_shared_notes))
        .route("/api/shared/{token}/deactivate", axum::routing::post(shared::deactivate_shared_note))
        .route("/api/shared/{token}/contributors", axum::routing::post(shared::manage_contributors))
        .route("/shared/{token}", get(shared::shared_editor_page))
        .route("/shared/{token}/ws", get(shared::ws_handler))
        .route("/api/shared/{token}/attribution", get(shared::get_attribution))
        // PDF routes
        .nest_service("/pdfs", ServeDir::new("pdfs"))
        .route("/api/pdf/upload", axum::routing::post(handlers::upload_pdf)
            .layer(DefaultBodyLimit::max(50 * 1024 * 1024)))
        .route("/api/pdf/download-url", axum::routing::post(handlers::download_pdf_from_url))
        .route("/api/pdf/rename", axum::routing::post(handlers::rename_pdf))
        .route("/api/pdf/unlink", axum::routing::post(handlers::unlink_pdf))
        .route("/api/pdf/smart-find", axum::routing::post(handlers::smart_pdf_find))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to port 3000");

    println!("Notes server running at http://0.0.0.0:3000");
    println!("Notes directory: {}", NOTES_DIR);

    if auth::is_auth_enabled() {
        println!("Authentication: ENABLED (NOTES_PASSWORD set)");
    } else {
        println!("Authentication: DISABLED (set NOTES_PASSWORD env var to enable editing)");
    }

    axum::serve(listener, app).await.expect("Server error");
}
