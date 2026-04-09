//! Filesystem [`Storage`](runtime::Storage) backend.
//!
//! The daemon is the fs owner in the crabtalk layering: runtime defines
//! the trait, daemon provides the real implementation. Everything the
//! runtime needs to persist lands under a single `root` directory
//! (`config_dir`), addressed by flat `/`-separated keys.

mod fs;

pub use fs::FsStorage;
