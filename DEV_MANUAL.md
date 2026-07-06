# Dev Manual

This manual helps new contributors to better understand the codebase and get over some of the common pitfalls of this project. Feel free to leverage both keyword and semantic search to find the topics you are looking for.

## Where can I get started for code contributions?

OpenNote consists of three major entry points at the moment. They are, `opennote-desktop`, `opennote-server`, and `opennote-mcp-server`. The desktop app provides a GUI for users to use OpenNote. The server provides a self-hostable remote server for users to sync data across devices. And the mcp server is for connecting OpenNote with your LLM chat client, like Claude Desktop and Cherry Studio etc.

If you are familiar with frontend development, you probably will want to dive into the desktop and the gpui-component crates. If you are familiar with the backend development, server and mcp server are the ways to go.

However, if you are good at database, then you probably will want to take a look at `opennote-data`, `opennote-entities`, and `opennote-models`. They are highly relevant to database and open for any database provider to integrate your database.

## Why embedding is so slow?

If you run `opennote-desktop` with `cargo run` but without a `--release` flag, chances are, you will experience a super long embedding process if you are trying to save a large document or typing a huge search query.

This is normal because the debug compilation won't apply optimizations. I often try not to pass large documents when debugging the app, unless the thing that I am trying to test needs to work with super long documents.

## Why performance sucks?

Same as [Why embedding is so slow?](#why-embedding-is-so-slow), if you really want to try it out with performance, compile the app with a `--release` flag.

## I want to contribute to XXX topic, but where is it?

`opennote-bootstrap` - Everything related to the resources to load on both desktop and server startup.
`opennote-core-logics` - All business logics related to the databases, both vector and sql databases.
`opennote-data` - All database operations, both vector and sql databases.
`opennote-embedder` - Everything related to embedding model inferences.
`opennote-entities` - Database models.
`opennote-mcp-server` - OpenNote MCP server implementations.
`opennote-server` - OpenNote Server implementations.
`opennote-desktop` - OpenNote Desktop app implementations.
`gpui-component` - A modified `gpui-component`. It contains almost all UI components used in the desktop app, but be aware that some of the UI components are placed under `opennpte-desktop/src/libs`

There are other crates in this projects that I haven't yet included in the above list, because they are either not yet finished or they are very unlikely to be touched. But in case if you need anyone of them, reach me out. Email: baoxinyuworks@163.com
