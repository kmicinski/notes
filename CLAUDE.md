# Notes App — Claude Code Guide

## Git Rules
- **NEVER push `content/` or `pdfs/` to the remote repository** — these contain personal research data. They are tracked locally (git is the app's version history), but must never reach the public `main` branch.
- The `my-notes` branch is the local working branch (has content/pdfs). The `main` branch is the public feature branch (code only).
- When merging to `main`, only include source code, config, and documentation — never content/pdfs.
- When committing on `my-notes`, adding content/pdfs is fine (that's how the app works).
- When committing on `main` or preparing merges to `main`, always add files by name. Never use `git add -A` or `git add .`.

A Rust/Axum personal knowledge management system for academic papers and research notes.
Runs at **127.0.0.1:7000**. Notes are markdown files in `content/`, PDFs in `pdfs/`, metadata in sled DB at `.notes_db/`.

## Quick Reference

### Build & Run
```bash
cargo build --release
NOTES_PASSWORD=yourpassword ./target/release/notes   # with auth
./target/release/notes                                # read-only mode
```

### Project Layout
```
src/
  main.rs            — Tokio entry point, all route definitions
  lib.rs             — AppState struct, config constants, path validation, startup reconciliation
  models.rs          — All data types: Note, PaperMeta, TimeEntry, GraphNode/Edge, GraphQuery, SmartAdd types
  notes.rs           — File I/O: load/parse notes, frontmatter parsing, markdown rendering, BibTeX, git history
  handlers.rs        — All HTTP handlers (~2500 lines): CRUD, auth, PDF, search, smart-add, citations, sharing, graph
  auth.rs            — Argon2 password hashing, sled sessions, CSRF tokens, rate limiting
  smart_add.rs       — Input detection (arXiv/DOI/URL/text), external API queries, BibTeX import
  citations.rs       — PDF text extraction (pdftotext), reference parsing, matching against note pool
  graph.rs           — Knowledge graph construction from sled index, D3.js page rendering
  graph_index.rs     — Sled-backed materialized graph: IndexedNode/Edge, incremental reindex
  shared.rs          — Collaborative editing: Automerge CRDT, WebSocket sync, line attribution
  url_validator.rs   — SSRF protection: domain allowlist (57 domains), private IP blocking
  templates/
    mod.rs           — Template module exports
    styles.rs        — Solarized Light CSS (all inline)
    components.rs    — Nav bar, Smart Add modal/FAB, base_html wrapper
    editor.rs        — Monaco editor with PDF split-pane, emacs bindings, auto-save, crosslink autocomplete
    viewer.rs        — Read-only view with rendered markdown, PDF viewer, meta block, history
    shared_editor.rs — Collaborative editor UI with attribution highlighting
content/             — Markdown notes (LOCAL ONLY — never commit)
pdfs/                — PDF attachments (LOCAL ONLY — never commit)
shared/              — Shared note workspace
.notes_db/           — Sled database (sessions, graph index, citations cache, shared docs)
```

### Key Architecture Decisions
- **No external templates** — all HTML/CSS/JS are inline Rust strings in `src/templates/`
- **No JS build step** — Monaco, D3.js, PDF.js loaded from CDN
- **Markdown files are the source of truth** — sled is only for sessions, graph index, caches
- **Git is the version history** — auto-commits on save via shell `git` commands
- **In-memory note cache** — `RwLock<HashMap>` in AppState, invalidated on save

### Note Format (Frontmatter)
```yaml
---
title: Paper Title
date: 2024-01-15
type: paper                  # or omit for regular note
bibtex: |
  @article{key, ...}
sources:
  - type: arxiv              # arxiv | doi | url
    identifier: 2301.00001
pdf: filename.pdf
parent: parent-note-key
hidden: false
time:
  - date: 2024-01-15
    minutes: 45
    category: reading        # reading | programming | writing | teaching | service | other
    description: First pass
---
```

### Cross-linking
Use `[@key]` in markdown body to link to another note. The key is the filename without `.md`.

### Route Map (main.rs)
**Pages:** `/` (index), `/search`, `/papers`, `/time`, `/graph`, `/new`, `/login`, `/logout`
**Note CRUD:** `GET /note/{key}`, `POST /api/note/{key}`, `DELETE /api/note/{key}`, `POST /api/note/{key}/toggle-hidden`
**History:** `GET /note/{key}/history/{commit}`
**Smart Add:** `POST /api/smart-add/{lookup,create,attach}`, `POST /api/smart-add/quick-note`, `POST /api/bib-import/{analyze,execute}`
**PDFs:** `POST /api/pdf/{upload,download-url,rename,unlink,smart-find}`, `GET /pdfs/{file}` (static)
**Citations:** `POST /api/citations/{scan,write,scan-all}`
**Graph:** `GET /api/graph?q=...`
**Sharing:** `POST /api/shared/{create,list/{key},{token}/deactivate,{token}/contributors}`, `GET /shared/{token}`, `GET /shared/{token}/ws`
**Export:** `GET /bibliography.bib`

### Sled DB Trees
- `sessions` — auth sessions (32-byte hex IDs)
- `csrf_tokens` — one-time CSRF tokens (10-min TTL)
- `kg:nodes` — IndexedNode per note key (JSON)
- `kg:edges` — edges keyed by `src\0tgt\0type` (weight as u32)
- `citations` — cached PDF scan results
- `shared:meta` — SharedNoteMeta per token
- `shared:doc:{token}` — Automerge document bytes
- `shared:attrib:{token}` — line-level attribution

### Key Dependencies
axum 0.8, tokio, sled 0.34, pulldown-cmark 0.10, automerge 0.5, reqwest 0.11, argon2 0.5, ammonia 4, chrono, sha2, tower-http 0.6

### Working with handlers.rs
This is the largest file (~2500 lines). Key sections by approximate line ranges:
- Index/search/papers/time handlers: top portion
- Note CRUD (view, save, delete, create): middle
- Auth (login, logout): after CRUD
- PDF handlers (upload, download, rename, unlink, smart-find): after auth
- Citation handlers (scan, write, scan-all): after PDF
- Graph handlers: after citations
- Shared note handlers (create, list, deactivate, WebSocket): end of file

### Graph Query Language
Used in `/graph?q=...` and the graph UI search bar:
`from:KEY` `depth:N` `type:paper|note` `has:time` `links:>N` `orphans` `hubs` `path:A->B` `cluster:type|parent`
