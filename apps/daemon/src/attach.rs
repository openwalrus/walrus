//! First-time LLM endpoint setup.

use anyhow::Result;
use dialoguer::{Password, theme::ColorfulTheme};
use std::io::{self, BufRead, Write};
use std::path::Path;
use toml_edit::{DocumentMut, Item, Table, value};

/// Line-oriented prompt — robust to bracketed paste, unlike dialoguer's
/// char-by-char `Input`.
fn prompt_line(prompt: &str, default: Option<&str>) -> Result<String> {
    match default {
        Some(d) => print!("{prompt} [{d}]: "),
        None => print!("{prompt}: "),
    }
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    let trimmed = line.trim();
    Ok(if trimmed.is_empty() {
        default.unwrap_or("").to_string()
    } else {
        trimmed.to_string()
    })
}

/// Interactive LLM endpoint setup for first-time daemon start. Writes a
/// `[llm]` section with `base_url` and `api_key` to the daemon's config.
pub fn setup_llm(config_path: &Path) -> Result<()> {
    let theme = ColorfulTheme::default();
    println!("\nNo LLM endpoint configured. Let's set one up.\n");

    let base_url = prompt_line(
        "Base URL (OpenAI-compatible)",
        Some("http://localhost:4000/v1"),
    )?;
    if base_url.is_empty() {
        anyhow::bail!("base URL is required");
    }

    let api_key: String = Password::with_theme(&theme)
        .with_prompt("API key (empty = none)")
        .allow_empty_password(true)
        .interact()?;

    let content = std::fs::read_to_string(config_path)?;
    let mut doc: DocumentMut = content.parse()?;

    let mut llm = Table::new();
    llm.insert("base_url", value(base_url.as_str()));
    if !api_key.is_empty() {
        llm.insert("api_key", value(api_key.as_str()));
    }
    doc.insert("llm", Item::Table(llm));

    std::fs::write(config_path, doc.to_string())?;
    println!("\nSaved to {}\n", config_path.display());
    Ok(())
}
