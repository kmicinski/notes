# Notes

A personal knowledge management system for academic papers and research notes, built in Rust with Axum.

## Setup

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- Git (for version history features)
- Optional: [Claude CLI](https://docs.anthropic.com/en/docs/claude-cli) (used as a fallback for metadata extraction from URLs)

### Security Warning

This application is designed for **local, single-user use**. It binds to `127.0.0.1` (localhost only) by default. Before running, understand the following:

- **Without `NOTES_PASSWORD`**: The server runs in read-only mode. Anyone who can reach the port can view your notes.
- **With `NOTES_PASSWORD`**: Editing is gated behind a password, but authentication uses simple cookie-based sessions stored in a local sled database. This is **not** designed for public-facing deployment.
- **Do not expose this server to the internet** without additional hardening (reverse proxy, TLS, proper auth, etc.).

### Build & Run

```bash
# Clone and build
git clone <repo-url> && cd notes
cargo build --release

# Run in read-only mode
./target/release/notes

# Run with editing enabled
NOTES_PASSWORD=yourpassword ./target/release/notes
```

The server starts at **http://127.0.0.1:7000**. Notes are stored as markdown files in `content/` and PDFs in `pdfs/`.

## Features

### Notes & Papers
- **Markdown notes** with YAML frontmatter
- **Paper management** with full BibTeX support (multiple entries per paper)
- **Paper sources** — attach arXiv, DOI, and URL identifiers to papers
- **Cross-linking** between notes using `[@key]` syntax
- **Full-text search** across all notes
- **Git integration** — automatic commits on save, browsable version history
- **Time tracking** per note with categories (programming, reading, writing, etc.)
- **Hidden notes** — toggle visibility to keep drafts out of listings
- **Hierarchical notes** — notes can have parent-child relationships via `parent:` frontmatter

### PDF Management
- **Upload** PDFs via drag-and-drop or file picker
- **Download from URL** — paste a direct link to fetch and attach a PDF
- **Smart Find** — automatically searches arXiv, Semantic Scholar, and Unpaywall for open-access PDFs
- **Split-pane viewer** — read PDFs side-by-side with notes (PDF.js-based, with zoom and page controls)
- **Rename** attached PDFs
- **Unlink** a PDF from a note without deleting the file

### Smart Add
Floating action button (+) for intelligent paper/note creation:
- **Auto-detection** of input type (arXiv URL, DOI, generic URL, plain text)
- **External API integration**: arXiv, CrossRef, Semantic Scholar
- **Metadata extraction** from web pages (citation meta tags, Open Graph)
- **Duplicate detection** — checks existing notes by title, DOI, arXiv ID
- **Claude CLI fallback** for URLs that APIs can't parse

### BibTeX Import
- **Bulk import** from `.bib` files
- **Deduplication** — detects papers already in your library
- **Secondary entries** — can add additional BibTeX entries to existing papers

### Editor
- **Monaco editor** with solarized-light theme
- **Emacs keybindings** (C-f/C-b, M-f/M-b, C-x C-s, etc.)
- **Note autocomplete** — `[@` triggers suggestions for cross-linking
- **Auto-save** after 90 seconds of inactivity with automatic git commit
- **Quick note types** — Definition, Question, Highlight, Begin/End study buttons when reading a PDF

### Visualization & Export
- **Knowledge graph** with D3.js force-directed layout
- **Graph query language**: `from:KEY depth:2 type:paper has:time orphans hubs`
- **Bibliography export** at `/bibliography.bib` (combined BibTeX from all papers)

## Note Format

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
arxiv: 1706.03762
doi: 10.5555/3295222.3295349
pdf: vaswani2017attention.pdf
time:
  - date: 2024-01-15
    minutes: 45
    category: reading
    description: First pass
---

## Summary

The transformer architecture...

## Notes

See also [@other-paper-key] for related work.
```

## Project Structure

```
notes/
├── src/
│   ├── main.rs          # Server entrypoint and route definitions
│   ├── lib.rs           # AppState, configuration, path validation
│   ├── models.rs        # Data structures (Note, PaperMeta, TimeEntry, etc.)
│   ├── notes.rs         # Note loading, parsing, search, markdown rendering
│   ├── handlers.rs      # HTTP route handlers (CRUD, PDF, search, etc.)
│   ├── smart_add.rs     # Smart paper addition and BibTeX import
│   ├── auth.rs          # Session management and authentication
│   ├── graph.rs         # Knowledge graph building and visualization
│   ├── url_validator.rs # URL/path validation utilities
│   └── templates/       # HTML/CSS/JS templates (inline, no build step)
│       ├── viewer.rs    # Note viewer with split-pane PDF support
│       ├── editor.rs    # Monaco editor with PDF support
│       └── ...
├── content/             # Markdown notes (git-tracked)
├── pdfs/                # PDF files
├── Cargo.toml
└── README.md
```

### Architecture Decisions

- **No database for notes** — notes are markdown files, git is the version history
- **Sled DB** — only used for session management
- **Inline CSS/JS** — no build step, everything served from Rust template strings
- **In-memory note index** — notes are re-loaded from disk on each request (simple, no cache invalidation)

---

*Built with Claude Code*
