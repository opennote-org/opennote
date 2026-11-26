use std::{path::Path, sync::LazyLock, thread::available_parallelism};

use anyhow::{Context, Result, anyhow};
use ort::{
    environment::Environment,
    ep::ExecutionProviderDispatch,
    session::{
        Session,
        builder::{AutoDevicePolicy, GraphOptimizationLevel},
    },
};

// rc.15 only allows one environment per process. Keep it alive across model reloads.
static ENVIRONMENT: LazyLock<ort::Result<Environment>> = LazyLock::new(|| {
    let providers: [ExecutionProviderDispatch; _] = [
        #[cfg(all(feature = "cuda", any(target_os = "linux", target_os = "windows")))]
        ort::ep::CUDA::default().build(),
    ];

    ort::init().with_execution_providers(providers).build()
});

pub(crate) fn load_session(path: impl AsRef<Path>) -> Result<Session> {
    let threads = available_parallelism().map_or(1, |count| count.get());
    load_session_with_threads(path, (threads / 2).max(1), Some(1))
}

pub(crate) fn load_session_with_threads(
    path: impl AsRef<Path>,
    intra_threads: usize,
    inter_threads: Option<usize>,
) -> Result<Session> {
    let environment = ENVIRONMENT
        .as_ref()
        .map_err(|err| anyhow!("Failed to initialize ONNX Runtime: {err}"))?;
    let mut builder = Session::builder(environment)?
        // Don't auto-select CoreML: it cannot handle Gemma's empty modality tensors.
        // Explicitly configured providers (CUDA) still take precedence.
        .with_auto_device(AutoDevicePolicy::PreferCPU)
        .map_err(|err| anyhow!("Failed to set ONNX device policy: {err}"))?
        // Level3 meant ORT_ENABLE_ALL in rc.10.
        .with_optimization_level(GraphOptimizationLevel::All)
        // Do not propagate the non-Send/Sync recovery builder into anyhow.
        .map_err(|err| anyhow!("Failed to set ONNX graph optimization level: {err}"))?
        .with_intra_threads(intra_threads)
        .map_err(|err| anyhow!("Failed to set ONNX intra-op thread count: {err}"))?;

    if let Some(inter_threads) = inter_threads {
        builder = builder
            .with_inter_threads(inter_threads)
            .map_err(|err| anyhow!("Failed to set ONNX inter-op thread count: {err}"))?;
    }

    let path = path.as_ref();
    builder
        .commit_from_file(path)
        .with_context(|| format!("Failed to load ONNX model {}", path.display()))
}
