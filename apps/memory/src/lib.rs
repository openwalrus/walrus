//! Graph-based memory service for walrus agents.
//!
//! Provides entity/relation/journal storage backed by LanceDB with
//! candle-based sentence embeddings (all-MiniLM-L6-v2).

#[cfg(feature = "serve")]
pub mod cmd;
pub mod config;
pub mod dispatch;
pub mod embedder;
pub mod lance;
#[cfg(feature = "serve")]
pub mod tool;
