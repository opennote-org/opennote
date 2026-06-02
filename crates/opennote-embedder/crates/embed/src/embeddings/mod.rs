//! Embedding model implementations and utilities.
//!
//! Provides local and cloud-based embedding models for generating vector
//! representations from text, image, and audio data.

use candle_core::{Device, Tensor};

pub mod cloud;
pub mod embed;
pub mod local;
pub mod utils;

pub fn normalize_l2(v: &Tensor) -> candle_core::Result<Tensor> {
    v.broadcast_div(&v.sqr()?.sum_keepdim(1)?.sqrt()?)
}

pub fn select_device() -> Device {
    #[cfg(feature = "metal")]
    {
        Device::new_metal(0).unwrap_or(Device::Cpu)
    }
    #[cfg(all(not(feature = "metal"), feature = "cuda"))]
    {
        Device::cuda_if_available(0).unwrap_or(Device::Cpu)
    }
    #[cfg(not(any(feature = "metal", feature = "cuda")))]
    {
        Device::Cpu
    }
}
