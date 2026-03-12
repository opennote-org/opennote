//! Embedding library based on Candle framework
//!
//! Supports sentence-transformers models like all-MiniLM-L6-v2
//!
//! # Example
//!
//! ```rust,ignore
//! use embedder::{LocalEmbedder, Embedder};
//!
//! let embedder = LocalEmbedder::new(
//!     "model.safetensors",
//!     "config.json",
//!     "tokenizer.json",
//!     None, // Use CPU by default
//! )?;
//!
//! let embeddings = embedder.embed(&["Hello world"]).await?;
//! ```

// Trait definition
pub mod embedder;

// Local implementation
pub mod local_embedder;

// Re-export public API for convenience
pub use embedder::EmbedderTrait;
pub use local_embedder::LocalEmbedder;
