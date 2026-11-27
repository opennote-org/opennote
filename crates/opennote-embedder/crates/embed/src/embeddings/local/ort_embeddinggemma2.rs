//! ONNX EmbeddingGemma 2 embedder covering text and image inputs.
//!
//! The checkpoint ships a text graph and a vision graph that share one 768
//! dimensional space, so an image vector can be scored against a text query
//! with a plain dot product. Weights are exported with external data, so both
//! the `.onnx` file and its sibling `.onnx_data` are fetched up front.

use std::path::{Path, PathBuf};
use std::sync::RwLock;

use anyhow::Error as E;
use anyhow::{Result, anyhow};
use image::DynamicImage;
use image::imageops::FilterType;
use ndarray::prelude::*;
use ort::session::Session;
use ort::value::TensorRef;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};

use crate::Dtype;
use crate::embeddings::embed::EmbeddingResult;
use crate::embeddings::hub::HubModelRepo;
use crate::ort_files::{OnnxGraph, stage_onnx_graphs};
use crate::ort_session::load_session;

/// Native embedding width reported by the model config.
const DEFAULT_EMBEDDING_DIM: usize = 768;
/// Width of the vectors the modality encoders feed into the text graph.
const DEFAULT_TEXT_HIDDEN_SIZE: usize = 512;
/// Context window EmbeddingGemma 2 is trained for.
const CONTEXT_LENGTH: usize = 8_192;

/// The processor replaces each of these in a prompt with a run of soft tokens.
const IMAGE_TOKEN: &str = "<|image|>";
const IMAGE_BEGIN: &str = "<|image>";
const IMAGE_END: &str = "<image|>";

/// Settings from `processor_config.json` needed to reproduce the vision input.
#[derive(Debug, Clone, Copy)]
struct ImageProcessor {
    patch_size: usize,
    pooling_kernel_size: usize,
    max_soft_tokens: usize,
    rescale_factor: f32,
}

impl Default for ImageProcessor {
    fn default() -> Self {
        Self {
            patch_size: 16,
            pooling_kernel_size: 3,
            max_soft_tokens: 280,
            rescale_factor: 1.0 / 255.0,
        }
    }
}

impl ImageProcessor {
    /// Number of padded patch slots the vision graph expects per image.
    fn max_patches(&self) -> usize {
        self.max_soft_tokens * self.pooling_kernel_size.pow(2)
    }

    /// Flattened size of one 16x16 RGB patch.
    fn patch_pixels(&self) -> usize {
        3 * self.patch_size * self.patch_size
    }
}

struct PreparedImage {
    /// One row per real patch, each row laid out as `(pixel row, column, channel)`.
    patches: Vec<f32>,
    /// `(x, y)` patch coordinates, one pair per real patch.
    positions: Vec<i64>,
    num_patches: usize,
    num_soft_tokens: usize,
}

/// Local EmbeddingGemma 2 ONNX embedder.
pub struct OrtEmbeddingGemma2Embedder {
    text: RwLock<Session>,
    vision: RwLock<Session>,
    tokenizer: Tokenizer,
    image_processor: ImageProcessor,
    embedding_dim: usize,
    text_hidden_size: usize,
    image_token_id: i64,
}

impl OrtEmbeddingGemma2Embedder {
    pub fn new(
        model_id: &str,
        revision: Option<&str>,
        token: Option<&str>,
        dtype: Option<Dtype>,
        path_in_repo: Option<&str>,
    ) -> Result<Self> {
        let repo = HubModelRepo::new(model_id, revision, token)?;
        let suffix = onnx_suffix(dtype);
        let base = match path_in_repo {
            Some(path) => path
                .rsplit_once('/')
                .map(|(directory, _)| format!("{directory}/"))
                .unwrap_or_default(),
            // This repository keeps every graph under `onnx/`. Other exports
            // place them at the root, which `resolve_onnx` also tries.
            None => "onnx/".to_string(),
        };

        let text_graph = resolve_onnx(&repo, &base, "model", suffix)?;
        let vision_graph = resolve_onnx(&repo, &base, "vision_encoder", suffix)?;

        let config = read_json(&repo.get("config.json")?)?;
        let text_config = &config["text_config"];
        let embedding_dim = text_config["embedding_dim"]
            .as_u64()
            .unwrap_or(DEFAULT_EMBEDDING_DIM as u64) as usize;
        let text_hidden_size = text_config["hidden_size"]
            .as_u64()
            .unwrap_or(DEFAULT_TEXT_HIDDEN_SIZE as u64) as usize;
        let image_token_id = config["image_token_id"].as_u64().unwrap_or(258_880) as i64;

        let image_processor = image_processor_config(&repo)?;
        let mut tokenizer = Tokenizer::from_file(repo.get("tokenizer.json")?).map_err(E::msg)?;
        tokenizer
            .with_padding(Some(PaddingParams {
                strategy: PaddingStrategy::BatchLongest,
                ..Default::default()
            }))
            .with_truncation(Some(TruncationParams {
                max_length: CONTEXT_LENGTH,
                ..Default::default()
            }))
            .map_err(E::msg)?;

        let [text_weights, vision_weights] = stage_onnx_graphs([text_graph, vision_graph])?;
        Ok(Self {
            text: RwLock::new(load_session(&text_weights)?),
            vision: RwLock::new(load_session(&vision_weights)?),
            tokenizer,
            image_processor,
            embedding_dim,
            text_hidden_size,
            image_token_id,
        })
    }

    pub fn embedding_dim(&self) -> usize {
        self.embedding_dim
    }

    /// Embeds a batch of text with the text graph only.
    pub fn embed(
        &self,
        text_batch: &[&str],
        batch_size: Option<usize>,
    ) -> Result<Vec<EmbeddingResult>> {
        let batch_size = batch_size.unwrap_or(32).max(1);
        let mut results = Vec::with_capacity(text_batch.len());
        for chunk in text_batch.chunks(batch_size) {
            for vector in self.run_text(chunk, &[])? {
                results.push(EmbeddingResult::DenseVector(vector));
            }
        }
        Ok(results)
    }

    /// Embeds paths to images, one vector per image.
    pub fn embed_image_batch(
        &self,
        image_paths: &[PathBuf],
        batch_size: Option<usize>,
    ) -> Result<Vec<Vec<f32>>> {
        let images = image_paths
            .iter()
            .map(|path| load_image(path))
            .collect::<Result<Vec<_>>>()?;
        self.embed_images(&images, batch_size)
    }

    pub fn embed_image(&self, image_path: impl AsRef<Path>) -> Result<Vec<f32>> {
        let image = load_image(image_path.as_ref())?;
        Ok(self.embed_images(&[image], None)?.remove(0))
    }

    pub fn embed_image_bytes(&self, bytes: &[u8]) -> Result<Vec<f32>> {
        let image = image::load_from_memory(bytes)?;
        Ok(self.embed_images(&[image], None)?.remove(0))
    }

    /// Encodes images through the vision graph and then the text graph.
    pub fn embed_images(
        &self,
        images: &[DynamicImage],
        batch_size: Option<usize>,
    ) -> Result<Vec<Vec<f32>>> {
        let batch_size = batch_size.unwrap_or(8).max(1);
        let mut vectors = Vec::with_capacity(images.len());

        for chunk in images.chunks(batch_size) {
            let prepared = chunk
                .iter()
                .map(|image| self.prepare_image(image))
                .collect::<Result<Vec<_>>>()?;
            let features = self.run_vision(&prepared)?;

            for feature in features {
                let prompt = image_prompt(feature.len() / self.text_hidden_size);
                vectors.push(self.run_text(&[prompt.as_str()], &feature)?.remove(0));
            }
        }

        Ok(vectors)
    }

    /// Embeds one interleaved text/image input into a single vector.
    ///
    /// Every `<|image|>` in `text` is replaced with the expanded placeholder
    /// for the image at the same position in `images`.
    pub fn embed_multimodal(&self, text: &str, image_paths: &[PathBuf]) -> Result<Vec<f32>> {
        let images = image_paths
            .iter()
            .map(|path| load_image(path))
            .collect::<Result<Vec<_>>>()?;
        self.embed_multimodal_images(text, &images)
    }

    pub fn embed_multimodal_images(&self, text: &str, images: &[DynamicImage]) -> Result<Vec<f32>> {
        let prepared = images
            .iter()
            .map(|image| self.prepare_image(image))
            .collect::<Result<Vec<_>>>()?;
        let features = self.run_vision(&prepared)?;
        let counts: Vec<usize> = features
            .iter()
            .map(|feature| feature.len() / self.text_hidden_size)
            .collect();
        let prompt = build_prompt(text, &counts)?;
        let flat_features = features.concat();

        Ok(self.run_text(&[prompt.as_str()], &flat_features)?.remove(0))
    }

    /// Runs the text graph. `image_features` is the concatenation of every
    /// image's soft-token vectors, in prompt order.
    fn run_text(&self, prompts: &[&str], image_features: &[f32]) -> Result<Vec<Vec<f32>>> {
        if prompts.is_empty() {
            return Ok(Vec::new());
        }

        let encodings = self
            .tokenizer
            .encode_batch(prompts.to_vec(), true)
            .map_err(E::msg)?;
        let ids: Vec<Vec<i64>> = encodings
            .iter()
            .map(|encoding| encoding.get_ids().iter().map(|&id| id as i64).collect())
            .collect();
        let masks: Vec<Vec<i64>> = encodings
            .iter()
            .map(|encoding| {
                encoding
                    .get_attention_mask()
                    .iter()
                    .map(|&value| value as i64)
                    .collect()
            })
            .collect();

        let placeholders = ids
            .iter()
            .flatten()
            .filter(|&&id| id == self.image_token_id)
            .count();
        let feature_rows = image_features.len() / self.text_hidden_size;
        if placeholders != feature_rows {
            return Err(anyhow!(
                "Image placeholder count ({placeholders}) does not match the feature count ({feature_rows})"
            ));
        }

        let input_ids = stack_i64(&ids)?;
        let attention_mask = stack_i64(&masks)?;
        let features = Array2::from_shape_vec(
            (feature_rows, self.text_hidden_size),
            image_features.to_vec(),
        )?;
        let empty = Array2::<f32>::zeros((0, self.text_hidden_size));

        let input_ids_tensor = TensorRef::from_array_view(&input_ids)?;
        let attention_mask_tensor = TensorRef::from_array_view(&attention_mask)?;
        let image_features_tensor = TensorRef::from_array_view(&features)?;
        let video_features_tensor = TensorRef::from_array_view(&empty)?;
        let audio_features_tensor = TensorRef::from_array_view(&empty)?;

        let mut model = self.text.write().unwrap();
        let outputs = model.run(ort::inputs![
            "input_ids" => input_ids_tensor,
            "attention_mask" => attention_mask_tensor,
            "image_features" => image_features_tensor,
            "video_features" => video_features_tensor,
            "audio_features" => audio_features_tensor,
        ])?;

        let embeddings = outputs["sentence_embedding"]
            .try_extract_array::<f32>()?
            .into_dimensionality::<Ix2>()?
            .to_owned();
        if embeddings.ncols() != self.embedding_dim {
            return Err(anyhow!(
                "Text graph returned {} dimensions, expected {}",
                embeddings.ncols(),
                self.embedding_dim
            ));
        }

        Ok((0..embeddings.nrows())
            .map(|row| normalize(embeddings.row(row)))
            .collect())
    }

    /// Runs the vision graph on padded patches and splits the soft-token rows
    /// back into one vector per image.
    fn run_vision(&self, images: &[PreparedImage]) -> Result<Vec<Vec<f32>>> {
        if images.is_empty() {
            return Ok(Vec::new());
        }

        let max_patches = self.image_processor.max_patches();
        let patch_pixels = self.image_processor.patch_pixels();
        let mut pixels = Array3::<f32>::zeros((images.len(), max_patches, patch_pixels));
        let mut positions = Array3::<i64>::from_elem((images.len(), max_patches, 2), -1);

        for (index, image) in images.iter().enumerate() {
            let patches = ArrayView2::from_shape(
                (image.num_patches, patch_pixels),
                image.patches.as_slice(),
            )?;
            pixels
                .slice_mut(s![index, 0..image.num_patches, ..])
                .assign(&patches);
            for patch in 0..image.num_patches {
                positions[[index, patch, 0]] = image.positions[patch * 2];
                positions[[index, patch, 1]] = image.positions[patch * 2 + 1];
            }
        }

        let pixel_values_tensor = TensorRef::from_array_view(&pixels)?;
        let positions_tensor = TensorRef::from_array_view(&positions)?;
        let mut model = self.vision.write().unwrap();
        let outputs = model.run(ort::inputs![
            "pixel_values" => pixel_values_tensor,
            "pixel_position_ids" => positions_tensor,
        ])?;

        let features = outputs["image_features"]
            .try_extract_array::<f32>()?
            .into_dimensionality::<Ix2>()?
            .to_owned();
        let expected: usize = images.iter().map(|image| image.num_soft_tokens).sum();
        if features.nrows() != expected {
            return Err(anyhow!(
                "Vision graph returned {} feature rows, expected {expected}",
                features.nrows()
            ));
        }

        let mut vectors = Vec::with_capacity(images.len());
        let mut offset = 0;
        for image in images {
            let end = offset + image.num_soft_tokens;
            vectors.push(
                features
                    .slice(s![offset..end, ..])
                    .iter()
                    .copied()
                    .collect(),
            );
            offset = end;
        }
        Ok(vectors)
    }

    /// Aspect-ratio preserving resize, rescaling and patchification.
    fn prepare_image(&self, image: &DynamicImage) -> Result<PreparedImage> {
        let patch_size = self.image_processor.patch_size;
        let pooling = self.image_processor.pooling_kernel_size;
        let patch_pixels = self.image_processor.patch_pixels();

        let source = image.to_rgb8();
        let (target_height, target_width) = target_size(
            source.height() as usize,
            source.width() as usize,
            patch_size,
            self.image_processor.max_patches(),
            pooling,
        )?;

        let resized = DynamicImage::ImageRgb8(source).resize_exact(
            target_width as u32,
            target_height as u32,
            FilterType::CatmullRom,
        );
        let rgb = resized.to_rgb8();

        let patch_height = target_height / patch_size;
        let patch_width = target_width / patch_size;
        let num_patches = patch_height * patch_width;
        let stride = target_width * 3;
        let pixels = rgb.as_raw();
        let scale = self.image_processor.rescale_factor;

        let mut patches = vec![0f32; num_patches * patch_pixels];
        for row in 0..patch_height {
            for column in 0..patch_width {
                let destination = (row * patch_width + column) * patch_pixels;
                for y in 0..patch_size {
                    for x in 0..patch_size {
                        let source =
                            (row * patch_size + y) * stride + (column * patch_size + x) * 3;
                        let offset = destination + (y * patch_size + x) * 3;
                        patches[offset] = pixels[source] as f32 * scale;
                        patches[offset + 1] = pixels[source + 1] as f32 * scale;
                        patches[offset + 2] = pixels[source + 2] as f32 * scale;
                    }
                }
            }
        }

        let mut positions = Vec::with_capacity(num_patches * 2);
        for row in 0..patch_height {
            for column in 0..patch_width {
                positions.push(column as i64);
                positions.push(row as i64);
            }
        }

        Ok(PreparedImage {
            patches,
            positions,
            num_patches,
            num_soft_tokens: (patch_height / pooling) * (patch_width / pooling),
        })
    }
}

/// Mirrors `Gemma4ImageProcessor.get_aspect_ratio_preserving_size`.
fn target_size(
    height: usize,
    width: usize,
    patch_size: usize,
    max_patches: usize,
    pooling_kernel_size: usize,
) -> Result<(usize, usize)> {
    let total_pixels = (height * width) as f64;
    let target_pixels = (max_patches * patch_size * patch_size) as f64;
    let factor = (target_pixels / total_pixels).sqrt();
    let side = pooling_kernel_size * patch_size;

    let mut target_height = ((factor * height as f64) / side as f64).floor() as usize * side;
    let mut target_width = ((factor * width as f64) / side as f64).floor() as usize * side;

    if target_height == 0 && target_width == 0 {
        return Err(anyhow!(
            "Resizing a {height}x{width} image would produce an empty image"
        ));
    }

    let max_side = (max_patches / pooling_kernel_size.pow(2)) * side;
    if target_height == 0 {
        target_height = side;
        target_width = ((width / height) * side).min(max_side);
    } else if target_width == 0 {
        target_width = side;
        target_height = ((height / width) * side).min(max_side);
    }

    if target_height * target_width > max_patches * patch_size * patch_size {
        return Err(anyhow!(
            "Resizing a {height}x{width} image to {target_height}x{target_width} exceeds the {max_patches} patch budget"
        ));
    }

    Ok((target_height, target_width))
}

/// Wraps a run of image placeholders in the processor's delimiter tokens.
fn image_prompt(num_soft_tokens: usize) -> String {
    let mut prompt = String::with_capacity(
        IMAGE_BEGIN.len() + IMAGE_END.len() + num_soft_tokens * IMAGE_TOKEN.len(),
    );
    prompt.push_str(IMAGE_BEGIN);
    for _ in 0..num_soft_tokens {
        prompt.push_str(IMAGE_TOKEN);
    }
    prompt.push_str(IMAGE_END);
    prompt
}

fn build_prompt(text: &str, counts: &[usize]) -> Result<String> {
    if text.is_empty() {
        if counts.is_empty() {
            return Err(anyhow!("At least one text or image input is required"));
        }
        return Ok(counts
            .iter()
            .map(|&count| image_prompt(count))
            .collect::<Vec<_>>()
            .join(" "));
    }

    let placeholders = text.matches(IMAGE_TOKEN).count();
    if placeholders != counts.len() {
        return Err(anyhow!(
            "Text contains {placeholders} image placeholders but {} images were provided",
            counts.len()
        ));
    }

    let mut prompt = String::with_capacity(text.len());
    let mut counts = counts.iter().copied();
    let mut parts = text.split(IMAGE_TOKEN);
    prompt.push_str(parts.next().unwrap_or_default());
    for part in parts {
        let count = counts.next().unwrap_or_default();
        prompt.push_str(&image_prompt(count));
        prompt.push_str(part);
    }
    Ok(prompt)
}

fn load_image(path: &Path) -> Result<DynamicImage> {
    Ok(image::ImageReader::open(path)?.decode()?)
}

fn stack_i64(rows: &[Vec<i64>]) -> Result<Array2<i64>> {
    let Some(width) = rows.first().map(Vec::len) else {
        return Err(anyhow!("Cannot build a tensor from an empty batch"));
    };
    let mut values = Vec::with_capacity(rows.len() * width);
    for row in rows {
        if row.len() != width {
            return Err(anyhow!("Tokenizer produced sequences of different lengths"));
        }
        values.extend_from_slice(row);
    }
    Ok(Array2::from_shape_vec((rows.len(), width), values)?)
}

fn normalize(vector: ArrayView1<f32>) -> Vec<f32> {
    let norm = vector.dot(&vector).sqrt().max(1e-12);
    vector.iter().map(|value| value / norm).collect()
}

fn onnx_suffix(dtype: Option<Dtype>) -> &'static str {
    match dtype {
        Some(Dtype::F32) => "",
        Some(Dtype::F16) | Some(Dtype::BF16) => "_fp16",
        Some(Dtype::Q4F16) => "_q4f16",
        Some(Dtype::INT8) | Some(Dtype::UINT8) | Some(Dtype::QUANTIZED) | Some(Dtype::BNB4) => {
            "_quantized"
        }
        // q4 is the default: roughly half the download of q8 with acceptable
        // quality loss for text and images.
        Some(Dtype::Q4) | None => "_q4",
    }
}

fn read_json(path: &Path) -> Result<serde_json::Value> {
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

fn image_processor_config(repo: &HubModelRepo) -> Result<ImageProcessor> {
    let mut config = ImageProcessor::default();
    let Some(path) = repo.optional("processor_config.json")? else {
        return Ok(config);
    };
    let json = read_json(&path)?;
    let Some(section) = json.get("image_processor") else {
        return Ok(config);
    };

    if let Some(value) = section
        .get("patch_size")
        .and_then(serde_json::Value::as_u64)
    {
        config.patch_size = value as usize;
    }
    if let Some(value) = section
        .get("pooling_kernel_size")
        .and_then(serde_json::Value::as_u64)
    {
        config.pooling_kernel_size = value as usize;
    }
    if let Some(value) = section
        .get("max_soft_tokens")
        .and_then(serde_json::Value::as_u64)
    {
        config.max_soft_tokens = value as usize;
    }
    if let Some(value) = section
        .get("rescale_factor")
        .and_then(serde_json::Value::as_f64)
    {
        config.rescale_factor = value as f32;
    }

    Ok(config)
}

fn resolve_onnx(repo: &HubModelRepo, base: &str, stem: &str, suffix: &str) -> Result<OnnxGraph> {
    let mut candidates = Vec::with_capacity(2);
    if !base.is_empty() {
        candidates.push(format!("{base}{stem}{suffix}.onnx"));
    }
    candidates.push(format!("{stem}{suffix}.onnx"));

    for candidate in candidates {
        match repo.optional(&candidate)? {
            Some(model) => {
                let external_data = repo.optional(&format!("{candidate}_data"))?;
                return Ok(OnnxGraph { model, external_data });
            }
            None => continue,
        }
    }

    Err(anyhow!(
        "Could not find {stem}{suffix}.onnx in the model repository"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aspect_ratio_size_keeps_patch_budget() {
        let (height, width) = target_size(1024, 768, 16, 2520, 3).unwrap();
        assert_eq!(height % 48, 0);
        assert_eq!(width % 48, 0);
        assert!(height * width <= 2520 * 16 * 16);
    }

    #[test]
    fn expanded_prompt_repeats_one_placeholder_per_image() {
        let prompt = build_prompt("before <|image|> between <|image|> after", &[2, 3]).unwrap();
        assert_eq!(
            prompt,
            "before <|image><|image|><|image|><image|> between <|image|><|image|><|image|><|image|><image|> after"
        );
        assert_eq!(prompt.matches(IMAGE_BEGIN).count(), 2);
    }

    #[test]
    fn image_only_prompt_has_no_separator_for_one_image() {
        let prompt = build_prompt("", &[1]).unwrap();
        assert_eq!(prompt, image_prompt(1));
    }

    #[test]
    fn prompt_placeholder_count_must_match_images() {
        assert!(build_prompt("text without image", &[1]).is_err());
        assert!(build_prompt("<|image|>", &[]).is_err());
        assert!(build_prompt("", &[]).is_err());
    }
}
