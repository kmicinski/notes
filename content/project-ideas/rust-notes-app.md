---
title: Rust Notes App Implementation
date: 2025-01-12
parent: 8264bc
time:
  - date: 2025-01-12
    minutes: 120
    category: programming
    description: Core implementation
  - date: 2025-01-13
    minutes: 90
    category: programming
    description: Template and styling
---

# Rust Notes App Implementation

This is a sub-note of [@8264bc] detailing the implementation of the personal notes system.

## Technology Stack

- **Rust** - Systems language for performance and safety
- **Axum** - Ergonomic web framework
- **Sled** - Embedded database for metadata
- **pulldown-cmark** - Markdown parsing

## Architecture

```
content/           <- Markdown files (git-tracked)
.notes_db/         <- Sled database (metadata cache)
src/main.rs        <- Single-file application
```

## Features Implemented

### Core
- [x] Frontmatter parsing (YAML-style)
- [x] Markdown rendering with pulldown-cmark
- [x] Short hash keys for notes (6 hex chars)
- [x] Cross-linking with `[@key]` syntax

### Paper Support
- [x] Special paper type with bib_key
- [x] Raw bibtex storage
- [x] Bibliography.bib export

### Time Tracking
- [x] Time entries in frontmatter
- [x] Category-based classification
- [x] Aggregated time view with visualization

### Git Integration
- [x] Edit history from git log
- [x] View file at specific commits
- [x] Follows renames

## Next Steps

- [ ] Full-text search
- [ ] Tag support
- [ ] Mobile-responsive improvements
