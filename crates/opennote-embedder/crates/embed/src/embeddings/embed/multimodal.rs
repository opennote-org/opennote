use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use base64::Engine;
use image::{DynamicImage, ImageFormat};

use super::types::{EmbedData, EmbeddingResult};
use crate::Dtype;
use crate::embeddings::local::colpali::get_images_from_pdf;
use crate::embeddings::local::ort_embeddinggemma2::OrtEmbeddingGemma2Embedder;

/// Embedders that map text and other modalities into one shared vector space.
pub enum MultiModalEmbedder {
    EmbeddingGemma2(Box<OrtEmbeddingGemma2Embedder>),
}

impl MultiModalEmbedder {
    pub fn from_pretrained_hf(
        architecture: &str,
        model_id: &str,
        revision: Option<&str>,
        token: Option<&str>,
        dtype: Option<Dtype>,
    ) -> Result<Self> {
        match architecture {
            "EmbeddingGemma2Model" => Ok(Self::EmbeddingGemma2(Box::new(
                OrtEmbeddingGemma2Embedder::new(model_id, revision, token, dtype, None)?,
            ))),
            _ => Err(anyhow!("Model not supported")),
        }
    }

    pub fn from_pretrained_ort(
        model_architecture: &str,
        revision: Option<&str>,
        model_id: Option<&str>,
        dtype: Option<Dtype>,
        path_in_repo: Option<&str>,
    ) -> Result<Self> {
        match model_architecture {
            "EmbeddingGemma2Model" => {
                let model_id = model_id.ok_or_else(|| anyhow!("Model ID is required"))?;
                Ok(Self::EmbeddingGemma2(Box::new(
                    OrtEmbeddingGemma2Embedder::new(model_id, revision, None, dtype, path_in_repo)?,
                )))
            }
            _ => Err(anyhow!("Model not supported")),
        }
    }

    pub async fn embed_text(
        &self,
        text_batch: &[&str],
        batch_size: Option<usize>,
    ) -> Result<Vec<EmbeddingResult>> {
        match self {
            Self::EmbeddingGemma2(embedder) => embedder.embed(text_batch, batch_size),
        }
    }

    /// Embeds a single image into the shared space.
    pub async fn embed_image(
        &self,
        image_path: impl AsRef<Path>,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<EmbedData> {
        let path = image_path.as_ref();
        match self {
            Self::EmbeddingGemma2(embedder) => {
                let vector = embedder.embed_image(path)?;
                Ok(EmbedData::new(
                    EmbeddingResult::DenseVector(vector),
                    Some(path.to_string_lossy().into_owned()),
                    metadata,
                ))
            }
        }
    }

    pub async fn embed_image_batch(
        &self,
        image_paths: &[PathBuf],
        batch_size: Option<usize>,
    ) -> Result<Vec<EmbedData>> {
        match self {
            Self::EmbeddingGemma2(embedder) => {
                let vectors = embedder.embed_image_batch(image_paths, batch_size)?;
                vectors
                    .into_iter()
                    .zip(image_paths)
                    .map(|(vector, path)| {
                        let mut metadata = HashMap::new();
                        metadata
                            .insert("file_name".to_string(), path.to_string_lossy().into_owned());
                        Ok(EmbedData::new(
                            EmbeddingResult::DenseVector(vector),
                            Some(path.to_string_lossy().into_owned()),
                            Some(metadata),
                        ))
                    })
                    .collect()
            }
        }
    }

    /// Decodes and embeds encoded image bytes in input order.
    pub async fn embed_image_bytes(
        &self,
        image_bytes: &[Vec<u8>],
        batch_size: Option<usize>,
    ) -> Result<Vec<EmbedData>> {
        let images = image_bytes
            .iter()
            .map(|bytes| image::load_from_memory(bytes))
            .collect::<image::ImageResult<Vec<_>>>()?;

        match self {
            Self::EmbeddingGemma2(embedder) => {
                let vectors = embedder.embed_images(&images, batch_size)?;
                Ok(vectors
                    .into_iter()
                    .map(|vector| EmbedData::new(EmbeddingResult::DenseVector(vector), None, None))
                    .collect())
            }
        }
    }

    /// Embeds every page of a PDF as a separate image vector.
    pub async fn embed_pdf(
        &self,
        pdf_path: impl AsRef<Path>,
        batch_size: Option<usize>,
    ) -> Result<Vec<EmbedData>> {
        let path = pdf_path.as_ref();
        let pages: Vec<DynamicImage> = get_images_from_pdf(&path)?;
        match self {
            Self::EmbeddingGemma2(embedder) => {
                let vectors = embedder.embed_images(&pages, batch_size)?;
                let mut results = Vec::with_capacity(vectors.len());
                for (index, (vector, page)) in vectors.into_iter().zip(pages.iter()).enumerate() {
                    let mut metadata = HashMap::new();
                    metadata.insert("page_number".to_string(), (index + 1).to_string());
                    metadata.insert("file_path".to_string(), path.to_string_lossy().into_owned());

                    let mut encoded = Vec::new();
                    page.write_to(&mut std::io::Cursor::new(&mut encoded), ImageFormat::Png)?;
                    metadata.insert(
                        "image".to_string(),
                        base64::engine::general_purpose::STANDARD.encode(encoded),
                    );

                    results.push(EmbedData::new(
                        EmbeddingResult::DenseVector(vector),
                        None,
                        Some(metadata),
                    ));
                }
                Ok(results)
            }
        }
    }
}

pub trait MultiModalEmbed {
    fn embed(
        &self,
        text_batch: &[&str],
        batch_size: Option<usize>,
    ) -> impl Future<Output = anyhow::Result<Vec<EmbeddingResult>>>;
}

impl MultiModalEmbed for MultiModalEmbedder {
    fn embed(
        &self,
        text_batch: &[&str],
        batch_size: Option<usize>,
    ) -> impl Future<Output = anyhow::Result<Vec<EmbeddingResult>>> {
        self.embed_text(text_batch, batch_size)
    }
}
