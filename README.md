# OpenNote

A block-based note-taking application with AI-powered semantic search, built purely in Rust.

**Status: Heavy Development** — APIs, architecture, and workflows are actively changing. Contributions, feedback, and experimental use are all welcome.

## Overview

OpenNote is designed to be simple, efficient and practical. Instead of doing everything, we aim to do a few things well.

OpenNote runs in two modes:

- **Desktop App** — native GUI built with GPUI (early preview)
- **Backend Server** — Actix-web HTTP server exposing a REST API (in development)
- **MCP Server** — an MCP (Model Context Protocol) server for AI agent integration (in development)

## Features

- Tree structured hierarchical documents like Notion and Obsidian
- Semantic search via local or remote embedding models
- Keyword search
- Document import from webpages, text files, and relational databases (in-development)
- MCP server for AI agent integration (in-development)
- Vim-style keybindings (in-development)
- Local-first, MIT licensed

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

Thanks to [appify](https://github.com/akx/appify) for bundling the executable binary into macOS app.

## License

MIT — see [LICENSE](./LICENSE).

Contains portions derived from [Zed](https://zed.dev) under Apache 2.0 — see [NOTICE.md](./NOTICE.md).
