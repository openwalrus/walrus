//! Cydonia candle utils re-exports

mod device;
mod inference;
mod loader;
mod model;
mod processor;
mod token;

pub use {
    device::detect as device,
    inference::Inference,
    loader::Loader,
    model::Model,
    processor::{Processor, ProcessorConfig, SampleBuilder},
    token::{TokenStream, Tokenizer},
};

/// The Llama model
pub type Llama = Model<candle_transformers::models::quantized_llama::ModelWeights>;
