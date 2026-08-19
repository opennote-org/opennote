//! Reading JSON payloads out of `text/event-stream` responses.
//!
//! Request-wise SSE streams begin with a priming event (SEP-1699) whose `data:`
//! line is empty, so the first event is not the answer. Every helper here keeps
//! reading events until one satisfies the caller's extractor.

#![allow(dead_code)]
use std::time::Duration;

use futures::StreamExt;
use serde_json::Value;

/// How long a caller waits for the stream to carry a matching event.
///
/// Long enough for an in-process server handshake on a loaded CI machine, short
/// enough that a transport that never answers fails the test rather than hanging.
const READ_TIMEOUT: Duration = Duration::from_secs(5);

/// How many response bytes are buffered before the search gives up.
///
/// Bounds a stream that stays open forever without ever carrying a match, such as
/// the standalone GET stream.
const MAX_BUFFERED_BYTES: usize = 4096;

/// Returns the first `data:` payload of `body` that `extract` accepts.
pub fn find_in_sse_body<T>(body: &str, extract: impl Fn(&Value) -> Option<T>) -> Option<T> {
    body.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter_map(|data| serde_json::from_str::<Value>(data).ok())
        .find_map(|payload| extract(&payload))
}

/// Reads `response` until a `data:` payload is accepted by `extract`.
///
/// Returns `None` when the stream ends, the byte cap is reached, or the read
/// times out, all of which mean the expected payload never arrived.
pub async fn find_in_sse_stream<T>(
    response: reqwest::Response,
    extract: impl Fn(&Value) -> Option<T>,
) -> Option<T> {
    let mut buffer = Vec::new();
    let mut stream = response.bytes_stream();

    tokio::time::timeout(READ_TIMEOUT, async {
        loop {
            if let Some(found) = find_in_sse_body(&String::from_utf8_lossy(&buffer), &extract) {
                return Some(found);
            }
            if buffer.len() > MAX_BUFFERED_BYTES {
                return None;
            }
            match stream.next().await {
                Some(Ok(bytes)) => buffer.extend_from_slice(&bytes),
                _ => return None,
            }
        }
    })
    .await
    .ok()
    .flatten()
}

/// Reads `response` until a `data:` payload carries a tool result whose text
/// content parses as JSON, and returns that JSON.
///
/// Tool results arrive as a JSON document inside the `text` field of the first
/// content block, so callers wanting the tool's own payload need both hops.
pub async fn find_tool_json_in_sse_stream(response: reqwest::Response) -> Option<Value> {
    find_in_sse_stream(response, |payload| {
        let text = payload.pointer("/result/content/0/text")?.as_str()?;
        serde_json::from_str::<Value>(text).ok()
    })
    .await
}
