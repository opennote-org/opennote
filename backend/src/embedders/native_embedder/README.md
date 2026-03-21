# Native Embedder

Based on [embed_anything](https://docs.rs/embed_anything), loads HuggingFace models and converts text to vectors locally.

## Structure

```
native_embedder/
├── mod.rs              # Module exports
├── embedder.rs         # EmbedderTrait definition
└── native_embedder.rs  # Core implementation
```

## Core Implementation

```rust
pub struct NativeEmbedder {
    anything_embedder: AnythingEmbedder,
}
```

- `new(model_id)`: Load HuggingFace pretrained model
- `embed(sentences)`: Vectorize and return `Vec<Vec<f32>>`

## EmbedderTrait

```rust
#[async_trait]
pub trait EmbedderTrait {
    async fn embed(&self, sentences: &[&str]) -> Result<Vec<Vec<f32>>>;
}
```

## Usage

```rust
use crate::embedders::native_embedder::native_embedder::NativeEmbedder;
use crate::embedders::native_embedder::embedder::EmbedderTrait;

let embedder = NativeEmbedder::new("BAAI/bge-small-zh-v1.5")?;
let vectors = embedder.embed(&["text1", "text2"]).await?;
```

## Recommended Models

| Model | Dimensions |
|-------|------------|
| `BAAI/bge-small-zh-v1.5` | 512 |
| `BAAI/bge-base-zh-v1.5` | 768 |
| `sentence-transformers/all-MiniLM-L6-v2` | 384 |

## Configuration

```json
{
  "embedder": {
    "provider": "native",
    "model": "BAAI/bge-small-zh-v1.5",
    "dimensions": 512
  }
}
```

## Features

- Offline operation, no network required
- Privacy protection, data never leaves local
- Batch processing support

## References

- [embed_anything Documentation](https://docs.rs/embed_anything)
- [HuggingFace Models](https://huggingface.co/models?pipeline_tag=sentence-similarity)
- [Candle Framework](https://github.com/huggingface/candle)
- [BGE Models](https://github.com/FlagOpen/FlagEmbedding)
