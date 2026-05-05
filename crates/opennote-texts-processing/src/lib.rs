use std::io::Read;

use chunk::chunk;

use opennote_models::payload::Payload;

/// Create chunks out of the input text content
pub fn slice_texts(
    content: &str,
    chunk_max_words: usize,
    document_metadata_id: &str,
    collection_metadata_id: &str,
) -> Vec<Payload> {
    let mut chunks: Vec<Payload> = Vec::new();

    let raw_chunks: Vec<_> = chunk(content.as_bytes())
        .consecutive()
        .delimiters("\n.?!。，！".as_bytes())
        .size(chunk_max_words)
        .collect();

    for mut chunk in raw_chunks {
        let mut bytes = Vec::new();
        match chunk.read_to_end(&mut bytes) {
            Ok(_) => {
                chunks.push(DocumentChunk::new(
                    String::from_utf8_lossy(&bytes).to_string(),
                    document_metadata_id,
                    collection_metadata_id,
                ));
            }
            Err(error) => {
                log::warn!("Error reading chunk: {} Chunk content: {:?}", error, chunk);
            }
        }
    }

    chunks
}
