use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use ort::session::{Session, builder::GraphOptimizationLevel};
use ort::value::{DynTensorValueType, Tensor};
use tokenizers::Tokenizer;
use ndarray::Array2;

pub struct EmbeddingModel {
    session: Session,
    tokenizer: Tokenizer,
}

impl EmbeddingModel {
    pub fn load(model_path: &Path, tokenizer_path: &Path) -> Result<Self> {
        let mut builder = Session::builder()
            .context("ONNX session builder")?
            .with_optimization_level(GraphOptimizationLevel::All)
            .map_err(|e| anyhow::anyhow!("Set optimization level: {e}"))?;
        let session = builder
            .commit_from_file(model_path)
            .context("Load ONNX model")?;

        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Load tokenizer: {}", e))?;

        Ok(Self { session, tokenizer })
    }

    /// Embed a text string → 768-dimensional f32 vector.
    pub fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
        let encoding = self.tokenizer
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!("Tokenize: {}", e))?;

        let ids: Vec<i64> = encoding.get_ids().iter().map(|&x| x as i64).collect();
        let mask: Vec<i64> = encoding.get_attention_mask().iter().map(|&x| x as i64).collect();
        let type_ids: Vec<i64> = vec![0i64; ids.len()];
        let seq_len = ids.len();

        let input_ids = Tensor::<i64>::from_array(
            Array2::from_shape_vec((1, seq_len), ids).context("Shape input_ids")?
        ).context("Tensor input_ids")?;
        let attention_mask = Tensor::<i64>::from_array(
            Array2::from_shape_vec((1, seq_len), mask).context("Shape attention_mask")?
        ).context("Tensor attention_mask")?;

        let has_type_ids = self.session.inputs().iter().any(|i| i.name() == "token_type_ids");

        let outputs = if has_type_ids {
            let token_type_ids = Tensor::<i64>::from_array(
                Array2::from_shape_vec((1, seq_len), type_ids).context("Shape token_type_ids")?
            ).context("Tensor token_type_ids")?;
            self.session.run(ort::inputs![
                "input_ids" => input_ids,
                "attention_mask" => attention_mask,
                "token_type_ids" => token_type_ids,
            ])?
        } else {
            self.session.run(ort::inputs![
                "input_ids" => input_ids,
                "attention_mask" => attention_mask,
            ])?
        };

        // Try pre-pooled output first, then mean-pool last_hidden_state
        let embedding: Vec<f32> = if let Some(pooled) = outputs.get("sentence_embedding") {
            let t = pooled.downcast_ref::<DynTensorValueType>()
                .context("Downcast sentence_embedding")?;
            t.try_extract_array::<f32>().context("Extract sentence_embedding")?
                .iter().copied().collect()
        } else {
            let hidden = &outputs["last_hidden_state"];
            let t = hidden.downcast_ref::<DynTensorValueType>()
                .context("Downcast last_hidden_state")?;
            let view = t.try_extract_array::<f32>().context("Extract last_hidden_state")?;
            let shape = view.shape();
            let (seq, hidden_size) = (shape[1], shape[2]);
            (0..hidden_size)
                .map(|d| (0..seq).map(|s| view[[0, s, d]]).sum::<f32>() / seq as f32)
                .collect()
        };

        Ok(embedding)
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { 0.0 } else { dot / (norm_a * norm_b) }
}

pub fn vec_to_blob(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

pub fn blob_to_vec(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

pub fn model_paths(app_data_dir: &Path) -> (PathBuf, PathBuf) {
    let models_dir = app_data_dir.join("models");
    (
        models_dir.join("nomic-embed-v1.5-q.onnx"),
        models_dir.join("nomic-tokenizer.json"),
    )
}

pub fn models_exist(app_data_dir: &Path) -> bool {
    let (model_path, tokenizer_path) = model_paths(app_data_dir);
    model_path.exists() && tokenizer_path.exists()
}

/// Download model and tokenizer from HuggingFace; calls `on_progress(downloaded, total)` during model download.
pub async fn download_model_files(
    app_data_dir: &Path,
    on_progress: impl Fn(u64, u64) + Send + 'static,
) -> Result<()> {
    use futures_util::StreamExt;
    use std::io::Write;

    let models_dir = app_data_dir.join("models");
    std::fs::create_dir_all(&models_dir).context("Create models dir")?;

    let client = reqwest::Client::new();

    // Tokenizer first (small, no progress needed)
    let tokenizer_path = models_dir.join("nomic-tokenizer.json");
    if !tokenizer_path.exists() {
        let url = "https://huggingface.co/nomic-ai/nomic-embed-text-v1.5/resolve/main/tokenizer.json";
        let bytes = client.get(url).send().await.context("Fetch tokenizer")?
            .bytes().await.context("Read tokenizer bytes")?;
        std::fs::write(&tokenizer_path, &bytes).context("Write tokenizer")?;
    }

    // Model (large, stream with progress)
    let model_path = models_dir.join("nomic-embed-v1.5-q.onnx");
    if !model_path.exists() {
        let url = "https://huggingface.co/nomic-ai/nomic-embed-text-v1.5/resolve/main/onnx/model_quantized.onnx";
        let resp = client.get(url).send().await.context("Fetch model")?;
        let total = resp.content_length().unwrap_or(0);
        let mut downloaded = 0u64;
        let mut stream = resp.bytes_stream();

        let tmp_path = models_dir.join("nomic-embed-v1.5-q.onnx.tmp");
        let mut file = std::fs::File::create(&tmp_path).context("Create model tmp file")?;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Read model chunk")?;
            file.write_all(&chunk).context("Write model chunk")?;
            downloaded += chunk.len() as u64;
            on_progress(downloaded, total);
        }
        drop(file);
        std::fs::rename(&tmp_path, &model_path).context("Rename model file")?;
    }

    Ok(())
}
