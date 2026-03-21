# Embedders Module

## Overview

The `embedders` module is a core component of the OpenNote backend system responsible for text vectorization, providing the capability to convert text content into vector representations for semantic search functionality. The module features a flexible architecture that supports multiple embedding providers.

## Technical Architecture

### Directory Structure

```
embedders/
├── traits.rs          # Core trait definitions and provider enum
├── entry.rs           # Module entry point, provides unified access interface
├── shared.rs          # Shared factory function for creating embedder instances
├── native.rs          # Native embedding implementation (using HF model)
├── remote.rs          # Remote API embedding implementation (compatible with OpenAI format)
├── other.rs           # Other third-party embedding implementations (via catsu library)
├── native_embedder/   # Native embedder submodule
│   ├── mod.rs
│   ├── embedder.rs    # Native embedder trait
│   └── native_embedder.rs  # Concrete implementation
└── mod.rs             # Module exports
```

### Core Components

#### 1. **Embedder Trait** (`traits.rs`)

Defines the core interface for embedders:

```rust
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn vectorize(&self, queries: &Vec<DocumentChunk>) -> anyhow::Result<Vec<Vec<f32>>>;
}
```

All concrete embedder implementations must implement this trait.

#### 2. **EmbedderProvider Enum** (`traits.rs`)

Supports three embedder provider types:

- `Native`: Locally running embedding model
- `Remote`: Remote HTTP API (compatible with OpenAI format)
- `Other(String)`: Other third-party services (supported via catsu library)

#### 3. **Concrete Implementations**

##### Native Embedder (`native.rs`)
- Uses Hugging Face local model
- Implemented via `embed_anything` crate
- Only requires configuring the `model` field in actual usage
- Automatically downloads model from Hugging Face and executes (via huggingface-cli)
- **Note**: Although only the `model` field is used at runtime, all fields must still be present in the configuration file (can be left empty or filled with default values)

##### Remote Embedder (`remote.rs`)
- Calls remote HTTP API
- Compatible with OpenAI API format
- Supports custom `base_url` and `api_key`
- Includes error handling and logging

##### Other Embedder (`other.rs`)
- Uses `catsu` library for third-party service integration
- Supports multiple embedding providers
- Flexible switching through configuration

#### 4. **Factory Pattern** (`shared.rs`)

The `create_embedder` function dynamically creates corresponding embedder instances based on configuration:

```rust
pub async fn create_embedder(config: &Config) -> Result<Arc<dyn Embedder>>
```

#### 5. **Entry Point** (`entry.rs`)

The `EmbedderEntry` struct provides a unified access point:

```rust
pub struct EmbedderEntry {
    pub embedder: Arc<dyn Embedder>,
}
```

## Design Features

- **Strategy Pattern**: Flexible switching between embedding providers through traits
- **Dependency Injection**: Uses `Arc<dyn Embedder>` for testability and decoupling
- **Asynchronous**: Full `async/await` support for I/O performance
- **Error Handling**: Unified `anyhow::Result` with friendly error messages
- **Extensible**: Easy to add new providers via `Embedder` trait implementation

## Usage

### 1. Configuration Examples

Set the embedder in the system configuration file:

#### Native Provider Configuration

```json
{
  "embedder": {
    "provider": "native",
    "model": "BAAI/bge-small-zh-v1.5",
    "base_url": "",                    // Not used in Native mode, leave empty
    "api_key": "",                     // Not used in Native mode, leave empty
    "encoding_format": "float",        // Recommended to keep this value
    "vectorization_batch_size": 32,    // Batch processing size
    "dimensions": 512                  // Vector dimensions (according to model specification)
  }
}
```

**Note**: In Native mode, only the `model` field is actually used; other fields are required configuration items but will not take effect.

#### Remote Provider Configuration

```json
{
  "embedder": {
    "provider": "remote",
    "model": "text-embedding-3-small",
    "base_url": "https://api.openai.com/v1/embeddings",
    "api_key": "your-api-key",
    "encoding_format": "float",
    "vectorization_batch_size": 32,
    "dimensions": 1536
  }
}
```

#### Other Provider Configuration

```json
{
  "embedder": {
    "provider": "your-provider-name",  // Custom provider name
    "model": "embedding-model-v1",
    "base_url": "https://your-provider.com/api",
    "api_key": "your-api-key",
    "encoding_format": "float",
    "vectorization_batch_size": 32,
    "dimensions": 768
  }
}
```

### 2. Initializing the Embedder

```rust
use crate::embedders::entry::EmbedderEntry;

// Create embedder entry from configuration
let embedder_entry = EmbedderEntry::new(&config).await?;
```

### 3. Vectorizing Text

```rust
use crate::documents::document_chunk::DocumentChunk;

// Prepare document chunks
let chunks = vec![
    DocumentChunk { content: "First text segment".to_string(), .. },
    DocumentChunk { content: "Second text segment".to_string(), .. },
];

// Execute vectorization
let vectors = embedder_entry.embedder.vectorize(&chunks).await?;
```

### 4. Directly Using Concrete Implementations

```rust
// Local embedder
let native = Native::new(&config).await?;
let vectors = native.vectorize(&chunks).await?;

// Remote embedder
let remote = Remote::new(&config).await?;
let vectors = remote.vectorize(&chunks).await?;
```

## EmbedderConfig Configuration Fields Reference

According to the definition in [`system.rs`](../configurations/system.rs), `EmbedderConfig` contains the following required fields:

| Field | Type | Description | Native Mode |
|-------|------|-------------|-------------|
| `provider` | `EmbedderProvider` | Embedder provider type (native/remote/other) | ✅ Used |
| `model` | `String` | Embedding model name | ✅ Used |
| `base_url` | `String` | URL for remote API | ❌ Not used |
| `api_key` | `String` | API authentication key | ❌ Not used |
| `encoding_format` | `String` | Vector encoding format (typically "float") | ⚠️ Reserved but unused |
| `vectorization_batch_size` | `usize` | Batch processing size for vectorization | ⚠️ Reserved but unused |
| `dimensions` | `usize` | Vector dimensions (must match model) | ⚠️ Reserved but unused |

**Important**: All fields are required in the configuration file (non-`Option` types), even if certain fields are not actually used in Native mode.

## Data Flow

```
DocumentChunk (Text) 
    ↓
Embedder::vectorize()
    ↓
[Native/Remote/Other] Implementation
    ↓
Vector computation (local model / remote API)
    ↓
Vec<Vec<f32>> (Floating-point vectors)
```

## Dependencies

Main dependencies include:

- `async_trait`: Async trait support
- `serde`: Serialization/deserialization
- `reqwest`: HTTP client (Remote embedder)
- `embed_anything`: Embedding model library (Native embedder)
- `catsu`: Third-party embedding service client (Other embedder)
- `anyhow`: Error handling


