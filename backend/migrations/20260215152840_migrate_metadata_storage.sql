-- Create table for global metadata settings
CREATE TABLE IF NOT EXISTS metadata_settings (
    id INTEGER PRIMARY KEY,
    embedder_model_in_use TEXT NOT NULL DEFAULT '',
    embedder_model_vector_size_in_use INTEGER NOT NULL DEFAULT 0
);

-- Insert default settings if not exists
INSERT OR IGNORE INTO metadata_settings (id, embedder_model_in_use, embedder_model_vector_size_in_use)
VALUES (1, '', 0);

-- Create table for collections
CREATE TABLE IF NOT EXISTS collections (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    last_modified INTEGER NOT NULL
);

-- Create table for documents
CREATE TABLE IF NOT EXISTS documents (
    id TEXT PRIMARY KEY,
    collection_metadata_id TEXT NOT NULL,
    title TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    last_modified INTEGER NOT NULL,
    FOREIGN KEY (collection_metadata_id) REFERENCES collections(id) ON DELETE CASCADE
);

-- Create table for document chunks
-- The `chunk_order` is a new entry to preserve the original order in the Vecs of `chunks`
CREATE TABLE IF NOT EXISTS document_chunks (
    id TEXT PRIMARY KEY,
    document_metadata_id TEXT NOT NULL,
    collection_metadata_id TEXT NOT NULL,
    content TEXT NOT NULL,
    dense_text_vector BLOB NOT NULL,
    chunk_order INTEGER NOT NULL, 
    FOREIGN KEY (document_metadata_id) REFERENCES documents(id) ON DELETE CASCADE,
    UNIQUE(document_metadata_id, chunk_order)
);
