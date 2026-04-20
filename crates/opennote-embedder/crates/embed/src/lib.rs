#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/StarlightSearch/EmbedAnything/refs/heads/main/docs/assets/icon.ico"
)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/StarlightSearch/EmbedAnything/refs/heads/main/docs/assets/Square310x310Logo.png"
)]
#![doc(issue_tracker_base_url = "https://github.com/StarlightSearch/EmbedAnything/issues/")]
//! embed_anything is a minimalist, highly performant, lightning-fast, lightweight, multisource,
//! multimodal, and local embedding pipeline.
//!
//! Whether you're working with text, images, audio, PDFs, websites, or other media, embed_anything
//! streamlines the process of generating embeddings from various sources and seamlessly streaming
//! (memory-efficient-indexing) them to a vector database.
//!
//! It supports dense, sparse, [ONNX](https://github.com/onnx/onnx) and late-interaction embeddings,
//! offering flexibility for a wide range of use cases.
//!
//! # Usage
//!
//! ## Creating an [Embedder]
//!
//! To get started, you'll need to create an [Embedder] for the type of content you want to embed.
//! We offer some utility functions to streamline creating embedders from various sources, such as
//! [Embedder::from_pretrained_hf], [Embedder::from_pretrained_onnx], and
//! [Embedder::from_pretrained_cloud]. You can use any of these to quickly create an Embedder like so:
//!
//! ```rust
//! use embed_anything::embeddings::embed::Embedder;
//!
//! // Create a local CLIP embedder from a Hugging Face model
//! let clip_embedder = Embedder::from_pretrained_hf("CLIP", "jina-clip-v2", None);
//!
//! // Create a cloud OpenAI embedder
//! let openai_embedder = Embedder::from_pretrained_cloud("OpenAI", "gpt-3.5-turbo", Some("my-api-key".to_string()));
//! ```
//!
//! If needed, you can also create an instance of [Embedder] manually, allowing you to create your
//! own embedder! Here's an example of manually creating embedders:
//!
//! ```rust
//! use embed_anything::embeddings::embed::{Embedder, TextEmbedder};
//! use embed_anything::embeddings::local::jina::JinaEmbedder;
//!
//! let jina_embedder = Embedder::Text(TextEmbedder::Jina(Box::new(JinaEmbedder::default())));
//! ```
//!
//! ## Generate embeddings
//!
//! # Example: Embed a text file
//!
//! Let's see how embed_anything can help us generate embeddings from a plain text file:
//!
//! ```rust
//! use embed_anything::{embed_file, embeddings::embed::EmbedderBuilder};
//! use embed_anything::config::TextEmbedConfig;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create an embedder using a pre-trained model from Hugging Face
//! let embedder = EmbedderBuilder::new()
//!     .model_architecture("jina")
//!     .model_id(Some("jinaai/jina-embeddings-v2-small-en"))
//!     .from_pretrained_hf()?;
//! let config = TextEmbedConfig::default();
//!
//! // Generate embeddings for any supported file type
//! let embeddings = embed_file("document.pdf", &embedder, Some(&config), None).await?;
//! # Ok(())
//! # }
//! ```
//! //! # Feature flags
//!
//! The crate can be configured with the following [Cargo features](https://doc.rust-lang.org/cargo/reference/features.html):
//!
//! | Feature | Description |
//! |---------|-------------|
//! | **default** | Enables `rustls-tls` for TLS. Keep this unless you need a different TLS backend. |
//! | **rustls-tls** | Use [Rustls](https://github.com/rustls/rustls) for TLS in HTTP clients (reqwest, hf-hub, tokenizers). |
//! | **mkl** | Intel [MKL](https://www.intel.com/content/www/us/en/developer/tools/oneapi/onemkl.html) (Math Kernel Library) for CPU-accelerated linear algebra. |
//! | **accelerate** | Apple [Accelerate](https://developer.apple.com/documentation/accelerate) framework for optimized math on macOS. |
//! | **cuda** | [NVIDIA CUDA](https://developer.nvidia.com/cuda-toolkit) support for GPU-accelerated inference. |
//! | **cudnn** | [cuDNN](https://developer.nvidia.com/cudnn) for CUDA-accelerated deep learning (enables `cuda`-related optimizations). |
//! | **flash-attn** | [Flash Attention](https://github.com/Dao-AILab/flash-attention) for faster attention on CUDA GPUs. Implies **cuda**. |
//! | **metal** | Apple [Metal](https://developer.apple.com/metal/) for GPU-accelerated inference on macOS. |
//! | **audio** | Audio embedding via [Symphonia](https://github.com/pdeljanov/Symphonia): transcribe and embed audio files (e.g. `emb_audio`). |
//! | **ort** | [ONNX Runtime](https://onnxruntime.ai/) for running ONNX models (e.g. reranker and ONNX-based embedders). |
//! | **aws** | [AWS SDK](https://aws.amazon.com/sdk-for-rust/) for loading objects from S3 (e.g. `s3_loader` module). |
//!
//! ## Example
//!
//! Enable only the features you need to reduce build time and binary size:
//!
//! ```toml
//! [dependencies]
//! embed_anything = { path = "rust", default-features = false, features = ["rustls-tls", "audio", "aws"] }
//! ```
//!

pub mod embeddings;
pub mod models;
#[cfg(feature = "ort")]
pub mod reranker;

extern crate anyhow;

use anyhow::Result;
use embeddings::embed::{EmbedData, EmbedImage};
use std::sync::Arc;

/// Numerical precision types for model weights and computations.
pub enum Dtype {
    /// 16-bit floating point.
    F16,
    /// 8-bit signed integer.
    INT8,
    /// 4-bit quantized.
    Q4,
    /// 8-bit unsigned integer.
    UINT8,
    /// 4-bit BitsAndBytes quantization.
    BNB4,
    /// 32-bit floating point.
    F32,
    /// 4-bit quantized with 16-bit float scale.
    Q4F16,
    /// Generic quantized format.
    QUANTIZED,
    /// 16-bit brain floating point.
    BF16,
}

fn is_video_extension(extension: &std::ffi::OsStr) -> bool {
    match extension.to_str().map(|ext| ext.to_ascii_lowercase()) {
        Some(ext) => matches!(
            ext.as_str(),
            "mp4" | "mov" | "avi" | "mkv" | "webm" | "m4v" | "flv" | "wmv"
        ),
        None => false,
    }
}

async fn process_images<E: EmbedImage + Send + Sync + 'static>(
    image_buffer: &[String],
    embedder: Arc<E>,
    batch_size: Option<usize>,
) -> Result<Arc<Vec<EmbedData>>> {
    let embeddings = embedder.embed_image_batch(image_buffer, batch_size).await?;
    Ok(Arc::new(embeddings))
}
