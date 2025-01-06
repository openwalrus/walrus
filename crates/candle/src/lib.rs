//! Cydonia candle utils re-exports

mod device;
mod inference;
mod loader;
mod model;
mod processor;
mod stream;

pub use {
    device::detect as device,
    inference::Inference,
    loader::Loader,
    model::Model,
    processor::{Processor, ProcessorConfig, SampleBuilder},
    stream::TokenStream,
};

/// The Llama model
pub type Llama = Model<candle_transformers::models::quantized_llama::ModelWeights>;
