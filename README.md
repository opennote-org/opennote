# OpenNote

A block-based, AI-powered note-taking app with semantic search — built entirely in Rust.

**Status: Heavy Development** — APIs, architecture, and workflows are evolving quickly. Contributions, feedback, and experimental use are all welcome.

## Features

- **Keyword and semantic search** — find notes instantly, or let your documents answer your questions.
- **Fully local** — everything runs on your machine. Your data stays private.
- **Blazing fast** — built for performance from the ground up.

## Roadmap

- [ ] Self-hosted server for syncing documents across devices
- [ ] Multi-modal support
- [ ] MCP server support
- [ ] Import webpages, databases and files
- [ ] Advanced NLP features to simplify document management (e.g., automatic categorization by semantic similarity)
- [ ] LLM integrations — local-first, always

## Getting Started

1. Visit the [Releases page](https://github.com/opennote-org/opennote/releases).
2. Download the archive for your operating system.
3. Unzip the archive.
4. Double-click to launch the app.
5. Enjoy!

## Contributing

OpenNote is in active development and welcomes contributions of all kinds — bug reports, feature ideas, code, documentation, and design.

- Open issues and pull requests on GitHub
- Explore the crate documentation in the source
- Reach out with questions or ideas

## Credits

Kudos to all the libraries used in this project. See the full list in [Cargo.toml](./Cargo.toml) and in the `Cargo.toml` of each sub-crate.

Thanks to [appify](https://github.com/akx/appify) for bundling the executable as a macOS app.

## License

MIT — see [LICENSE](./LICENSE).

This project includes code derived from [Zed](https://zed.dev) under the Apache 2.0 license — see [NOTICE.md](./NOTICE.md).
