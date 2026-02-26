# Notes

A personal knowledge management system for academic papers and research notes, built in Rust with Axum.

Manage your research library with markdown notes, PDF attachments, BibTeX metadata, a knowledge graph, and automatic citation extraction — all from a single self-hosted web app with no external database.

## Quick Start

```bash
git clone https://github.com/kmicinski/notes.git && cd notes
cargo build --release
NOTES_PASSWORD=yourpassword ./target/release/notes
```

Open **http://localhost:3000**. Without `NOTES_PASSWORD`, the server runs in read-only mode.

### Docker

```bash
NOTES_PASSWORD=yourpassword docker compose up -d
```

The compose file includes security hardening (read-only filesystem, dropped capabilities, seccomp, resource limits).

## Features

### Notes and Papers
- **Markdown notes** with YAML frontmatter for structured metadata
- **Paper management** — BibTeX support (multiple entries per paper), arXiv/DOI/URL identifiers
- **Cross-linking** between notes using `[@key]` syntax
- **Full-text search** across all notes
- **Git-backed version history** — automatic commits on save, browsable diffs
- **Time tracking** per note (reading, programming, writing, teaching, service)
- **Hierarchical organization** — parent-child relationships, hidden drafts

### Smart Add
Intelligent paper creation via the floating action button:
- **Auto-detects** arXiv URLs, DOIs, generic URLs, and plain text
- **Fetches metadata** from arXiv, CrossRef, and Semantic Scholar APIs
- **Duplicate detection** by title, DOI, and arXiv ID
- **BibTeX bulk import** with deduplication

### PDF Management
- **Upload** via drag-and-drop or file picker
- **Download from URL** — paste a link to fetch and attach
- **Smart Find** — searches arXiv, Semantic Scholar, and Unpaywall for open-access PDFs
- **Split-pane viewer** — read PDFs side-by-side with notes (PDF.js)

### Citation Scanner
- **Extracts references** from attached PDFs using `pdftotext` and `pdf-extract`
- **Matches citations** against your note library via DOI, arXiv ID, fuzzy title matching (Levenshtein distance), and author+year
- **Writes cross-links** back into notes as `[@key]` references

### Knowledge Graph
- **Interactive D3.js** force-directed visualization of your note network
- **Query language**: `from:KEY depth:2 type:paper author:NAME year:2024 hubs orphans`
- **Manual linking** — drag between nodes or search to create edges
- **Edge annotations** — add context to any connection
- **Per-note mini-graph** panel on the note viewer

### Editor
- **Monaco editor** with Solarized Light theme
- **Emacs keybindings** (C-f/C-b, M-f/M-b, C-x C-s, etc.)
- **Cross-link autocomplete** — `[@` triggers note suggestions
- **Auto-save** with automatic git commit
- **Quick note buttons** — Definition, Question, Highlight, study timer

### Collaborative Editing
- **Real-time co-editing** via Automerge CRDT and WebSocket sync
- **Shareable links** with contributor tracking and line-level attribution

### Export
- **BibTeX bibliography** at `/bibliography.bib` — combined from all papers

## Note Format

Notes are markdown files with YAML frontmatter:

```yaml
---
title: Attention Is All You Need
date: 2024-01-15
type: paper
bibtex: |
  @inproceedings{vaswani2017attention,
    title = {Attention Is All You Need},
    author = {Vaswani, Ashish and Shazeer, Noam and Parmar, Niki},
    booktitle = {NeurIPS},
    year = {2017}
  }
pdf: vaswani2017attention.pdf
time:
  - date: 2024-01-15
    minutes: 45
    category: reading
    description: First pass
---

## Summary

The transformer architecture...

See also [@other-paper-key] for related work.
```

## Project Structure

```
src/
  main.rs            Entry point, route definitions
  lib.rs             AppState, configuration, startup
  models.rs          Note, PaperMeta, GraphNode/Edge, etc.
  notes.rs           File I/O, frontmatter parsing, markdown, search
  handlers.rs        HTTP handlers (CRUD, auth, PDF, search, citations, graph)
  auth.rs            Argon2 password hashing, sessions, CSRF, rate limiting
  smart_add.rs       arXiv/DOI/URL detection, external API queries, BibTeX import
  citations.rs       PDF text extraction, reference parsing, fuzzy matching
  graph.rs           Knowledge graph construction, D3.js rendering
  graph_index.rs     Sled-backed materialized graph index
  graph_query.rs     Graph query language parser
  shared.rs          Collaborative editing (Automerge CRDT, WebSocket)
  url_validator.rs   SSRF protection (domain allowlist, private IP blocking)
  templates/         Inline HTML/CSS/JS (no build step, no external templates)
content/             Markdown notes (created at runtime)
pdfs/                PDF attachments (created at runtime)
.notes_db/           Sled database (sessions, graph index, citation cache)
```

## Architecture

- **Markdown files are the source of truth** — no database for content, git is the version history
- **Sled DB** — sessions, CSRF tokens, materialized graph index, citation cache
- **No JS build step** — Monaco, D3.js, PDF.js loaded from CDN; all other JS is inline
- **In-memory note cache** — `RwLock<Vec<Note>>` invalidated on save

## Security

Designed for **local, single-user use** behind a reverse proxy.

- **Argon2id** password hashing with random salt
- **Cryptographic sessions** (32-byte random IDs, sled-backed)
- **CSRF protection** (one-time tokens, 10-minute TTL)
- **Rate limiting** with exponential backoff on failed logins
- **SSRF protection** — domain allowlist for external fetches
- **Docker hardening** — read-only filesystem, dropped capabilities, seccomp profile, non-root user

Do not expose directly to the internet without TLS and a reverse proxy (e.g., Caddy, nginx).

## License

MIT

---

*Built with [Claude Code](https://claude.ai/claude-code)*
