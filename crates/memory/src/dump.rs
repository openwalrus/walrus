//! Dump / load: canonical serialization of the db as a markdown tree.
//!
//! Layout:
//! ```text
//! brain/
//!   SUMMARY.md                ← auto-generated mdbook ToC (ignored on load)
//!   notes/{name}.md           ← EntryKind::Note
//!   archives/{name}.md        ← EntryKind::Archive
//!   prompts/{name}.md         ← EntryKind::Prompt
//! ```
//!
//! Entry file format: raw content, then an optional trailing refs section:
//! ```markdown
//! whatever the content is.
//!
//! ## Refs
//!
//! - ship
//! - release
//! ```
//!
//! On load we scan backwards for the last `## Refs` heading; everything
//! above it is content, bullets below it are aliases (verbatim — users
//! who annotate like `- ship (legacy)` see their annotation preserved).
//!
//! `created_at` is not round-tripped through the tree — the db file is
//! the canonical backup. Loading an entry gives it a fresh `created_at`.

use crate::{
    entry::{Entry, EntryKind},
    error::{Error, Result},
};
use std::{collections::HashMap, fs, path::Path};

pub(crate) const REFS_HEADING: &str = "## Refs";

/// All kinds that the dump tree knows about, paired with their
/// subdirectory name. Single source of truth for both dump and load.
pub(crate) const KIND_SECTIONS: &[(EntryKind, &str, &str)] = &[
    (EntryKind::Note, "notes", "Notes"),
    (EntryKind::Archive, "archives", "Archives"),
    (EntryKind::Prompt, "prompts", "Prompts"),
];

/// Reject names that aren't safe as filesystem basenames. POSIX-friendly;
/// does not check Windows-reserved basenames (CON, NUL, etc.) — the dump
/// tree is not intended to roundtrip across Windows.
pub(crate) fn validate_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.starts_with('.')
        || name.ends_with('.')
        || name.ends_with(' ')
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
        || name.contains("..")
        || name.chars().any(|c| c.is_control())
    {
        return Err(Error::InvalidName(name.to_owned()));
    }
    Ok(())
}

pub(crate) fn serialize_entry(e: &Entry) -> String {
    if e.aliases.is_empty() {
        let mut out = e.content.clone();
        if !out.ends_with('\n') {
            out.push('\n');
        }
        return out;
    }

    let content = e.content.trim_end_matches('\n');
    let mut out = String::with_capacity(
        content.len() + 32 + e.aliases.iter().map(|a| a.len() + 4).sum::<usize>(),
    );
    out.push_str(content);
    out.push_str("\n\n");
    out.push_str(REFS_HEADING);
    out.push_str("\n\n");
    for a in &e.aliases {
        out.push_str("- ");
        out.push_str(a);
        out.push('\n');
    }
    out
}

/// Parse a dumped entry markdown file into `(content, aliases)`.
pub(crate) fn parse_entry(text: &str) -> (String, Vec<String>) {
    // Find the last line that is exactly `## Refs` (ignoring trailing
    // whitespace). Everything above = content; bullets below = aliases.
    let mut idx = None;
    for (i, _) in text.match_indices(REFS_HEADING) {
        if is_heading_line(text, i) {
            idx = Some(i);
        }
    }
    let Some(idx) = idx else {
        return (text.trim_end().to_owned(), Vec::new());
    };

    let content = text[..idx].trim_end().to_owned();
    let rest = &text[idx + REFS_HEADING.len()..];
    let mut aliases = Vec::new();
    for line in rest.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("- ") {
            aliases.push(rest.trim().to_owned());
        } else {
            // Something other than a bullet after Refs — stop parsing so
            // unrelated trailing content doesn't get swallowed.
            break;
        }
    }
    (content, aliases)
}

/// True if `idx` begins at a line boundary and the match ends the line
/// (allowing trailing whitespace). Guards against "## Refs" appearing
/// mid-line in user prose.
fn is_heading_line(text: &str, idx: usize) -> bool {
    let before_ok = idx == 0 || text[..idx].ends_with('\n');
    let after = &text[idx + REFS_HEADING.len()..];
    let after_ok = match after.chars().next() {
        None => true,
        Some('\n') => true,
        Some(c) => {
            c.is_whitespace() && after.lines().next().map(str::trim).unwrap_or("").is_empty()
        }
    };
    before_ok && after_ok
}

pub(crate) fn build_summary(by_kind: &HashMap<EntryKind, Vec<&Entry>>) -> String {
    let mut out = String::from("# Summary\n\n");
    for (kind, dir, title) in KIND_SECTIONS {
        let Some(entries) = by_kind.get(kind) else {
            continue;
        };
        if entries.is_empty() {
            continue;
        }
        let mut sorted: Vec<&&Entry> = entries.iter().collect();
        sorted.sort_by(|a, b| a.name.cmp(&b.name));
        out.push_str("# ");
        out.push_str(title);
        out.push_str("\n\n");
        for e in sorted {
            out.push_str(&format!("- [{name}]({dir}/{name}.md)\n", name = e.name));
        }
        out.push('\n');
    }
    out
}

pub(crate) struct Loaded {
    pub(crate) kind: EntryKind,
    pub(crate) name: String,
    pub(crate) content: String,
    pub(crate) aliases: Vec<String>,
}

/// Walk the tree and collect every markdown file as a [`Loaded`].
pub(crate) fn read_tree(dir: &Path) -> Result<Vec<Loaded>> {
    let mut out = Vec::new();
    for (kind, sub, _) in KIND_SECTIONS {
        let path = dir.join(sub);
        if !path.is_dir() {
            continue;
        }
        let mut names: Vec<_> = fs::read_dir(&path)?.collect::<std::io::Result<Vec<_>>>()?;
        names.sort_by_key(|e| e.file_name());
        for ent in names {
            let p = ent.path();
            if p.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let Some(name) = p.file_stem().and_then(|s| s.to_str()).map(str::to_owned) else {
                continue;
            };
            if name.is_empty() {
                continue;
            }
            let raw = fs::read_to_string(&p)?;
            let (content, aliases) = parse_entry(&raw);
            out.push(Loaded {
                kind: *kind,
                name,
                content,
                aliases,
            });
        }
    }
    Ok(out)
}
