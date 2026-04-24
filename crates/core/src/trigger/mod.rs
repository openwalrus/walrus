//! Trigger types — sources of external events that drive agent invocations.
//!
//! Cron is the first and only implementation today. Webhook, file-watch,
//! and other trigger kinds will live alongside `cron` when they appear.
//!
//! This module holds only pure data types and validators. Deployment-specific
//! concerns (file I/O, storage format, transport) live in the consuming
//! binary or service.

pub mod cron;
