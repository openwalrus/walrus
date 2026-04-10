//! Crabtalk skill integration — SKILL.md parsing.
//!
//! The [`Skill`] domain type lives in core. This module provides the
//! SKILL.md parser (used by the node's `FsStorage`).

pub use wcore::repos::Skill;

pub mod loader;
