use std::fmt::Display;

/// The quantization of the GGUF model.
///
/// see <https://huggingface.co/docs/hub/gguf> for more details
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[allow(non_camel_case_types)]
pub enum Quantization {
    /// 64-bit standard IEEE 754 double-precision floating-point number.
    F64,
    /// 64-bit fixed-width integer number.
    I64,
    /// 32-bit standard IEEE 754 single-precision floating-point number.
    F32,
    /// 32-bit fixed-width integer number.
    I32,
    /// 16-bit standard IEEE 754 half-precision floating-point number.
    F16,
    /// 16-bit shortened version of the 32-bit IEEE 754 single-precision floating-point number.
    BF16,
    /// 16-bit fixed-width integer number.
    I16,
    /// 8-bit round-to-nearest quantization (q). Each block has 32 weights.
    /// Weight formula: w = q * block_scale. Legacy quantization method not used widely as of today.
    Q8_0,
    /// 8-bit round-to-nearest quantization (q). Each block has 32 weights.
    /// Weight formula: w = q * block_scale + block_minimum. Legacy quantization method not used widely as of today.
    Q8_1,
    /// 8-bit quantization (q). Each block has 256 weights.
    /// Used only for quantizing intermediate results. Weight formula: w = q * block_scale.
    Q8_K,
    /// 8-bit fixed-width integer number.
    I8,
    /// 6-bit quantization (q). Super-blocks with 16 blocks, each block has 16 weights.
    /// Weight formula: w = q * block_scale(8-bit), resulting in 6.5625 bits-per-weight.
    Q6_K,
    /// 5-bit round-to-nearest quantization (q). Each block has 32 weights.
    /// Weight formula: w = q * block_scale. Legacy quantization method not used widely as of today.
    Q5_0,
    /// 5-bit round-to-nearest quantization (q). Each block has 32 weights.
    /// Weight formula: w = q * block_scale + block_minimum. Legacy quantization method not used widely as of today.
    Q5_1,
    /// 5-bit quantization (q). Super-blocks with 8 blocks, each block has 32 weights.
    /// Weight formula: w = q * block_scale + block_min(6-bit), resulting in 5.5 bits-per-weight.
    Q5_K,
    /// 4-bit round-to-nearest quantization (q). Each block has 32 weights.
    /// Weight formula: w = q * block_scale. Legacy quantization method not used widely as of today.
    #[default]
    Q4_0,
    /// 4-bit round-to-nearest quantization (q). Each block has 32 weights.
    /// Weight formula: w = q * block_scale + block_minimum. Legacy quantization method not used widely as of today.
    Q4_1,
    /// 4-bit quantization (q). Super-blocks with 8 blocks, each block has 32 weights.
    /// Weight formula: w = q * block_scale(6-bit) + block_min(6-bit), resulting in 4.5 bits-per-weight.
    Q4_K,
    /// 4-bit quantization (q). Super-blocks with 8 blocks, each block has 32 weights.
    /// Weight formula: w = q * block_scale(6-bit) + block_min(6-bit), resulting in 4.5 bits-per-weight.
    ///
    /// in small size
    Q4_K_S,
    /// 4-bit quantization (q). Super-blocks with 8 blocks, each block has 32 weights.
    /// Weight formula: w = q * block_scale(6-bit) + block_min(6-bit), resulting in 4.5 bits-per-weight.
    ///
    /// in medium size
    Q4_K_M,
    /// 3-bit quantization (q). Super-blocks with 16 blocks, each block has 16 weights.
    /// Weight formula: w = q * block_scale(6-bit), resulting in 3.4375 bits-per-weight.
    Q3_K,
    /// 2-bit quantization (q). Super-blocks with 16 blocks, each block has 16 weights.
    /// Weight formula: w = q * block_scale(4-bit) + block_min(4-bit), resulting in 2.5625 bits-per-weight.
    Q2_K,
    /// 4-bit quantization (q). Super-blocks with 256 weights. Weight w is obtained using super_block_scale & importance matrix.
    IQ4_NL,
    /// 4-bit quantization (q). Super-blocks with 256 weights. Weight w is obtained using super_block_scale & importance matrix, resulting in 4.25 bits-per-weight.
    IQ4_XS,
    /// 3-bit quantization (q). Super-blocks with 256 weights. Weight w is obtained using super_block_scale & importance matrix, resulting in 3.44 bits-per-weight.
    IQ3_S,
    /// 3-bit quantization (q). Super-blocks with 256 weights. Weight w is obtained using super_block_scale & importance matrix, resulting in 3.06 bits-per-weight.
    IQ3_XXS,
    /// 2-bit quantization (q). Super-blocks with 256 weights. Weight w is obtained using super_block_scale & importance matrix, resulting in 2.06 bits-per-weight.
    IQ2_XXS,
    /// 2-bit quantization (q). Super-blocks with 256 weights. Weight w is obtained using super_block_scale & importance matrix, resulting in 2.5 bits-per-weight.
    IQ2_S,
    /// 2-bit quantization (q). Super-blocks with 256 weights. Weight w is obtained using super_block_scale & importance matrix, resulting in 2.31 bits-per-weight.
    IQ2_XS,
    /// 1-bit quantization (q). Super-blocks with 256 weights. Weight w is obtained using super_block_scale & importance matrix, resulting in 1.56 bits-per-weight.
    IQ1_S,
    /// 1-bit quantization (q). Super-blocks with 256 weights. Weight w is obtained using super_block_scale & importance matrix, resulting in 1.75 bits-per-weight.
    IQ1_M,
}

impl Display for Quantization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
