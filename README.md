# Notes

A personal knowledge management system for academic papers and research notes, built in Rust with Axum.

## Features

### Core
- **Markdown notes** with YAML frontmatter
- **Paper management** with BibTeX support, authors, venue, year
- **Cross-linking** between notes using `[@key]` syntax
- **Full-text search** across all notes
- **Git integration** for version history
- **Time tracking** per note with categories (programming, reading, writing, etc.)

### Smart Add
Floating action button (+) for intelligent paper/note creation:
- **Auto-detection** of input type (arXiv URL, DOI, generic URL, plain text)
- **External API integration**: arXiv API, CrossRef API
- **Metadata extraction** from web pages (citation meta tags, Open Graph)
- **Duplicate detection** checks existing notes by title, DOI, arXiv ID
- **Multiple sources per paper** - attach arXiv preprint + official publication to same note
- **Claude CLI fallback** for URLs that APIs can't parse

### Editor
- **Full-screen Monaco editor** with solarized-light theme
- **Emacs keybindings** (C-f/C-b, M-f/M-b, C-x C-s, etc.)
- **Auto-save** after 90 seconds of inactivity
- **Automatic git commits** on auto-save with timestamp

### Visualization
- **Knowledge graph** with D3.js force-directed layout
- **Graph query language**: `from:KEY depth:2 type:paper has:time orphans hubs`
- **Bibliography export** at `/bibliography.bib`

## Installation

```bash
cargo build --release
```

## Usage

```bash
# Without authentication (read-only)
./target/release/notes

# With authentication (enables editing)
NOTES_PASSWORD=yourpassword ./target/release/notes
```

Server runs at `http://127.0.0.1:3000`

## Note Format

```yaml
---
title: Attention Is All You Need
date: 2024-01-15
type: paper
bib_key: vaswani2017attention
authors: Ashish Vaswani and Noam Shazeer and Niki Parmar
year: 2017
venue: NeurIPS
arxiv: 1706.03762
doi: 10.5555/3295222.3295349
bibtex: |
  @inproceedings{vaswani2017attention,
    title = {Attention Is All You Need},
    ...
  }
time:
  - date: 2024-01-15
    minutes: 45
    category: reading
    description: First pass
---

## Summary

The transformer architecture...

## Key Contributions

- Self-attention mechanism
- ...

## Notes

See also [@other-paper-key] for related work.
```

## Project Structure

```
notes/
├── src/main.rs      # All application code (~4800 lines)
├── content/         # Markdown notes directory
├── Cargo.toml
└── README.md
```

## Development Notes

### Recent Changes (Jan 2025)

1. **Smart Add feature** - FAB + modal for intelligent paper creation
2. **Multiple sources** - Papers can have both arXiv and DOI sources
3. **Solarized-light theme** - Consistent color scheme across app
4. **Emacs keybindings** - Full emacs mode in editor via monaco-emacs
5. **Auto-save with git** - Commits on save with timestamp message
6. **Enhanced DOI extraction** - Recognizes DOIs from ACM, Springer, Wiley, etc.

### Architecture Decisions

- **Single-file Rust** - All code in main.rs for simplicity
- **No database for notes** - Notes are just markdown files, git is the database
- **Sled DB** - Only used for session management
- **Inline CSS/JS** - No build step, everything served from Rust strings

### Known Limitations

- No mobile-optimized UI yet
- Graph visualization can be slow with many nodes
- Claude CLI fallback requires claude to be installed

## TODO

- [ ] Mobile-responsive design
- [ ] Batch import from Zotero/Mendeley
- [ ] PDF annotation integration
- [ ] Collaborative editing
- [ ] Export to various formats (Org-mode, Notion, etc.)

---

*Built with Claude Code*
