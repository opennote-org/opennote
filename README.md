# OpenNote

A block-based note-taking application with AI-powered semantic search, built purely in Rust.

**Status: Heavy Development** — APIs, architecture, and workflows are actively changing. Contributions, feedback, and experimental use are all welcome.

## Overview

Everything in OpenNote is a **Block** — a tree-structured unit of content that can hold text (markdown, titles), images, or JSON payloads. Blocks form hierarchical documents and support vector embeddings for semantic search.

OpenNote runs in two modes:

- **Desktop App** — native GUI built with GPUI (Zed's UI framework)
- **Backend Server** — Actix-web HTTP server exposing a REST API
- **MCP Server** — an MCP (Model Context Protocol) server for AI agent integration

## Features

- Block-based hierarchical documents (Notion/Obsidian-like)
- Semantic search via local or remote embedding models (in-development)
- Keyword search (in-development)
- Document import from webpages, text files, and relational databases (in-development)
- MCP server for AI agent integration (in-development)
- Vim-style keybindings (in-development)
- Local-first, MIT licensed

## Project Structure

```
opennote/
├── crates/
│   ├── opennote-ui              # Native desktop GUI (GPUI)
│   ├── opennote-server          # HTTP + MCP server (Actix-web)
│   ├── opennote-mcp-server      # Model Context Protocol implementation
│   ├── opennote-data            # Data access, search, vector database
│   ├── opennote-embedder        # Embedding/vectorization layer
│   ├── opennote-models          # Domain models, configs, types
│   ├── opennote-entities        # SeaORM entity models
│   ├── opennote-core-logics     # Business logic orchestration
│   ├── opennote-connectors      # Data import connectors
│   ├── opennote-texts-processing # Text chunking
│   ├── opennote-tasks-scheduler  # Async task tracking
│   └── opennote-bootstrap       # App bootstrapping & DI
├── assets/                       # Language files, binary assets
├── scripts/                      # Dev/build scripts
└── Cargo.toml                    # Workspace config
```

## Getting Started

> Detailed build instructions coming soon. For now, explore the crates and code.

```bash
cargo build --release -p opennote-server   # Build the server
cargo build --release -p opennote-ui       # Build the desktop app
```

## Contributing

OpenNote is in active development and welcomes all forms of contribution — bug reports, feature ideas, code, documentation, or design.

- Open issues and PRs on GitHub
- Check existing crate documentation in the source code
- Reach out with questions or ideas

## Credits

Kudo to all libraries used by this project. See them in [Cargo.toml](./Cargo.toml) and also Cargo.toml in each sub-crate.

## License

MIT — see [LICENSE](./LICENSE).

Contains portions derived from [Zed](https://zed.dev) under Apache 2.0 — see [NOTICE.md](./NOTICE.md).
