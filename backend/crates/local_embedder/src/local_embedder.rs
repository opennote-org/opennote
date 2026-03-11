use crate::embedder::Embedder;
use anyhow::{Context, Result};
use async_trait::async_trait;
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
use std::path::Path;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer};

/// 本地实现的 Embedder，基于 Candle 框架
/// 适配模型：sentence-transformers/all-MiniLM-L6-v2
pub struct LocalEmbedder {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
    #[allow(dead_code)]
    hidden_size: usize,
}

impl LocalEmbedder {
    /// 创建一个新的 LocalEmbedder 实例
    ///
    /// # Arguments
    /// * `model_path` - 模型权重文件路径 (model.safetensors)
    /// * `config_path` - 模型配置文件路径 (config.json)
    /// * `tokenizer_path` - 分词器文件路径 (tokenizer.json)
    /// * `device` - 运行设备 (Device::Cpu 或 Device::new_cuda(0)?)
    pub fn new<P: AsRef<Path>>(
        model_path: P,
        config_path: P,
        tokenizer_path: P,
        device: Option<Device>,
    ) -> Result<Self> {
        // 默认使用 CPU
        let device: Device = device.unwrap_or(Device::Cpu);

        // 1. 加载配置 (config.json)
        let config_content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {:?}", config_path.as_ref()))?;
        let config: BertConfig = serde_json::from_str(&config_content).with_context(
            || "Failed to parse config.json; ensure it matches the BertConfig schema",
        )?;

        // 2. 加载权重 (model.safetensors) - 需要 unsafe block
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[model_path.as_ref()], DType::F32, &device)
        }
        .with_context(|| "Failed to load model weights")?;

        // 3. 初始化模型
        // all-MiniLM-L6-v2 是标准的 BERT 架构，可以直接使用 BertModel
        let model =
            BertModel::load(vb, &config).with_context(|| "Failed to initialize BertModel")?;

        // 4. 加载分词器 (tokenizer.json)
        let mut tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow::Error::msg(format!("Failed to load tokenizer: {}", e)))?;

        // 配置分词器启用 Padding，以便批量处理
        // all-MiniLM-L6-v2 通常使用 [PAD] token，id 通常为 0
        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::BatchLongest,
            pad_token: "[PAD]".to_string(),
            pad_id: config.pad_token_id as u32,
            ..Default::default()
        }));

        Ok(Self {
            model,
            tokenizer,
            device,
            hidden_size: config.hidden_size,
        })
    }

    /// 内部同步推理逻辑
    fn embed_sync(&self, sentences: &[&str]) -> Result<Vec<Vec<f32>>> {
        if sentences.is_empty() {
            return Ok(vec![]);
        }

        // 1. 分词 (Tokenization)
        let encodings = self
            .tokenizer
            .encode_batch(sentences.to_vec(), true)
            .map_err(|e| anyhow::Error::msg(format!("Tokenization failed: {}", e)))?;

        // 2. 提取输入 IDs 和 Attention Mask
        let mut input_ids = Vec::new();
        let mut attention_masks = Vec::new();

        for encoding in &encodings {
            input_ids.push(encoding.get_ids().to_vec());
            attention_masks.push(encoding.get_attention_mask().to_vec());
        }

        // 3. 转换为 Candle Tensor
        let batch_size = input_ids.len();
        let seq_len = input_ids[0].len();

        // input_ids: (batch, seq)
        let input_ids_tensor = Tensor::new(
            input_ids.iter().flatten().copied().collect::<Vec<u32>>(),
            &self.device,
        )?
        .reshape((batch_size, seq_len))?;

        // attention_mask: (batch, seq)
        let attention_mask_tensor = Tensor::new(
            attention_masks
                .iter()
                .flatten()
                .copied()
                .collect::<Vec<u32>>(),
            &self.device,
        )?
        .reshape((batch_size, seq_len))?;

        // token_type_ids: 全为 0 (batch, seq)
        // all-MiniLM-L6-v2 不需要 segment 信息
        let token_type_ids = Tensor::zeros_like(&input_ids_tensor)?;

        // 4. 模型前向传播
        // 输出形状：(batch, seq_len, hidden_size)
        let output = self.model.forward(
            &input_ids_tensor,
            &token_type_ids,
            Some(&attention_mask_tensor),
        )?;

        // 5. 均值池化 (Mean Pooling)
        // 将 (batch, seq, hidden) 池化为 (batch, hidden)
        let embeddings = mean_pooling(&output, &attention_mask_tensor)?;

        // 6. L2 归一化 (Normalize)
        // sentence-transformers 模型通常需要归一化以便使用余弦相似度
        let embeddings = normalize(&embeddings)?;

        // 7. 转换为 Vec<Vec<f32>>
        let data = embeddings.to_vec2::<f32>()?;
        Ok(data)
    }
}

#[async_trait]
impl Embedder for LocalEmbedder {
    /// 实现 trait 定义的 embed 函数
    /// 注意：Candle 计算是同步阻塞的。
    /// 如果在高并发异步场景下使用，建议在调用处使用 spawn_blocking 包裹，
    /// 但在此库实现中，为了保持依赖最小化（无 tokio 依赖），直接执行同步代码。
    async fn embed(&self, sentences: &[&str]) -> Result<Vec<Vec<f32>>> {
        self.embed_sync(sentences)
    }
}

/// 均值池化辅助函数
///
/// 逻辑：
/// 1. 将 attention_mask 扩展维度以匹配 input 形状
/// 2. 将 input 中 padding 部分置为 0
/// 3. 在 sequence 维度求和
/// 4. 除以有效 token 数量 (mask 的和)
///
/// # Arguments
/// * `input` - 模型输出隐藏状态，形状 (batch, seq_len, hidden_size)
/// * `mask` - 注意力掩码，形状 (batch, seq_len)
///
/// # Returns
/// 池化后的向量，形状 (batch, hidden_size)
fn mean_pooling(input: &Tensor, mask: &Tensor) -> Result<Tensor> {
    // 将 mask 从 (batch, seq) 扩展为 (batch, seq, 1)
    let mask_expanded = mask.unsqueeze(2)?.expand(input.dims())?;

    // 确保数据类型为 F32 以便进行数学运算
    let mask_f32 = mask_expanded.to_dtype(DType::F32)?;
    let input_f32 = input.to_dtype(DType::F32)?;

    // sum_embeddings = sum(hidden_states * mask)
    // 这一步会将 padding 位置的向量变为 0
    let sum_embeddings = (input_f32 * &mask_f32)?.sum(1)?;

    // sum_mask = sum(mask)
    // 计算每个句子中有效 token 的数量
    // clamp 防止除以 0 (虽然正常情况下 mask 和至少为 1)
    let sum_mask = mask_f32.sum(1)?.clamp(1e-9, f32::MAX)?;

    // 平均值 = 总和 / 数量
    let embeddings = sum_embeddings.broadcast_div(&sum_mask)?;
    Ok(embeddings)
}

/// L2 归一化辅助函数
/// 用于将向量长度缩放到 1，便于计算余弦相似度
fn normalize(input: &Tensor) -> Result<Tensor> {
    // 计算 L2 范数：sqrt(sum(x^2))
    // keepdim(1) 保持维度以便广播除法
    let norm = input.sqr()?.sum_keepdim(1)?.sqrt()?;

    // 除以范数
    let normalized = input.broadcast_div(&norm)?;
    Ok(normalized)
}
