/// Instructions advertised to MCP clients via `ServerInfo::with_instructions`.
///
/// These tell an AI agent how the two exposed tools, `read_blocks` and `search`,
/// are meant to be used together. The workflow described here mirrors the
/// actual tool schemas and server implementation:
/// - `read_blocks` with an empty `block_ids` list returns *every* block in the
///   notebook (`BlockQuery::All`), and `has_payload` controls whether the heavy
///   text content is included.
/// - `search` is scoped to a set of `block_ids` that the caller must obtain
///   beforehand, and returns `RawSearchResult { block_id, payload_id, score }`.
pub const INSTRUCTIONS: &str = r#"You are connected to the user's OpenNote, a personal notebook. You have two tools — `read_blocks` and `search` — that are designed to be used together.

## Tool: read_blocks
- `block_ids` (array of strings): The IDs of the blocks to read. Leave this EMPTY (`[]`) to list every block in the notebook.
- `has_payload` (boolean): Set to `true` to return the full text content of the blocks. NEVER set `has_payload` to `true` when `block_ids` is empty — that would attempt to stream the entire notebook's content at once. Keep it `false` for the initial overview.

Returns an array of blocks. Each block has `id`, `parent_id`, `is_deleted`, and `payloads` (the `payloads` are only populated when `has_payload` is `true`).

## Tool: search
- `search_method` (`"keyword"` | `"semantic"`): Use `"keyword"` for exact term matching and `"semantic"` for meaning/concept matching.
- `block_ids` (array of strings): The scope of the search. You MUST obtain these IDs from `read_blocks` first.
- `query` (string): What to search for.
- `top_n` (number): How many results to return. `20` is recommended for a first attempt.

Returns an array of results. Each result has `block_id`, `payload_id`, and `score` (similarity). Search results do NOT contain the block content — only references.

## Recommended workflow
1. Map: Call `read_blocks` with `block_ids: []` and `has_payload: false` to get a lightweight inventory of every block and its ID. This is cheap and is your map of the notebook.
2. Find: Call `search` with a `query`, a `search_method`, `top_n: 20`, and the `block_ids` you want to search across (all of them, or a relevant subset chosen from the map).
3. Read: Pick the most relevant result(s) by `score`, then call `read_blocks` again with those specific `block_id`s and `has_payload: true` to retrieve the actual content.

In short: `read_blocks` to map → `search` to find → `read_blocks` to read.
"#;
