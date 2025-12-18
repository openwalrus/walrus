//! Device detection

use candle_core::{Device, utils};

/// Detect the device
pub fn detect(cpu: bool) -> anyhow::Result<Device> {
    if cpu {
        Ok(Device::Cpu)
    } else if utils::cuda_is_available() {
        Ok(Device::new_cuda(0)?)
    } else if utils::metal_is_available() {
        Ok(Device::new_metal(0)?)
    } else {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            tracing::warn!(
                "Running on CPU, to run on GPU(metal), build this library with `--features metal`"
            );
        }
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        {
            tracing::warn!(
                "Running on CPU, to run on GPU, build this library with `--features cuda`"
            );
        }
        Ok(Device::Cpu)
    }
}
