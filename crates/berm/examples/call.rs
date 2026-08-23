//! Exercise `berm.call`: a harness reaching a harness.
//!
//! ```sh
//! cargo build --release -p berm-fixture --target riscv64imac-unknown-none-elf
//! cargo run --example call -p crabtalk-berm
//! ```
//!
//! Driven through [`BermHarness`] rather than `Berm::load`, because the whole
//! of what this checks lives there: the name a guest asks for is resolved
//! against the declaring agent's own resolution, and nothing else is.
//!
//! One ELF is deployed under three names. What a name resolves to is a
//! registry entry, not a distinct image, so the same bytes standing in for
//! caller and target is the arrangement rather than a shortcut around one.

use anyhow::{Context, Result};
use crabtalk_berm::{BermHarness, Dispatch};
use runtime::{Harness as _, ToolDispatch};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, OnceLock},
};
use store::{AgentConfig, AgentId, HarnessConfig};

const GUEST: &str = "target/riscv64imac-unknown-none-elf/release/fixture";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("warn,harness=info"))
        .init();

    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .context("no workspace root")?
        .to_path_buf();
    let elf = fs::read(workspace.join(GUEST)).with_context(|| {
        format!("build the guest first: cargo build --release -p berm-fixture --target riscv64imac-unknown-none-elf ({GUEST})")
    })?;

    // The daemon reads `{name}.elf` from its image directory, so deploying one
    // ELF under three names is three files.
    let images = std::env::temp_dir().join("berm-call");
    let _ = fs::remove_dir_all(&images);
    fs::create_dir_all(&images)?;
    for name in ["caller", "target", "failing"] {
        fs::write(images.join(format!("{name}.elf")), &elf)?;
    }

    let protocol: Arc<OnceLock<Dispatch>> = Arc::new(OnceLock::new());
    let harnesses = BermHarness::new(
        protocol,
        images,
        std::env::temp_dir().join("berm-call-cache"),
    )?;

    // `absent` is never declared, which is what makes asking for it a refusal
    // rather than a failure.
    let agent = AgentConfig {
        id: AgentId::default(),
        harnesses: ["caller", "target", "failing"]
            .into_iter()
            .map(|name| HarnessConfig {
                name: name.to_owned(),
                root: None,
                hosts: Vec::new(),
            })
            .collect(),
        ..Default::default()
    };
    harnesses.load(&agent.id, &agent);

    println!("== the payload survives the hop ==");
    let reached = show(
        &harnesses,
        &agent,
        "target",
        "echo",
        Some(r#"{"query":"hi"}"#),
    )
    .await;
    assert!(
        reached.contains(r#"{"query":"hi"}"#),
        "the nested call lost the payload: {reached}"
    );

    println!("== a name nobody declared ==");
    let refused = show(&harnesses, &agent, "absent", "echo", None).await;
    assert!(
        refused.starts_with("refused: "),
        "an undeclared name was not a refusal: {refused}"
    );

    println!("== a name that is declared, and a tool that is not ==");
    let missing = show(&harnesses, &agent, "target", "nonesuch", None).await;
    assert!(
        missing.starts_with("refused: "),
        "an unknown tool was reported as having run: {missing}"
    );

    println!("== a target that ran and said no ==");
    let failed = show(&harnesses, &agent, "failing", "boom", None).await;
    assert!(
        failed.starts_with("failed: "),
        "a target that ran was reported as never having run: {failed}"
    );

    println!("\nok");
    Ok(())
}

/// Call `nest` on the `caller` image, pointing it at `harness`.`tool`.
async fn show(
    harnesses: &BermHarness,
    agent: &AgentConfig,
    harness: &str,
    tool: &str,
    args: Option<&str>,
) -> String {
    let forwarded = match args {
        Some(args) => format!(r#","args":{}"#, serde_json::json!(args)),
        None => String::new(),
    };
    let call = ToolDispatch {
        args: format!(r#"{{"harness":"{harness}","tool":"{tool}"{forwarded}}}"#),
        agent: agent.id,
        sender: String::new(),
        session_id: None,
        call_id: String::new(),
        root: None,
    };

    println!("$ nest -> {harness}.{tool}");
    let outcome = harnesses
        .dispatch("nest", call)
        .expect("the caller image serves nest")
        .await;

    // A harness reporting its own failure is a tool result, and `nest` writes
    // which kind it got into exactly that. Both land here as one string.
    let text = match outcome {
        Ok(result) | Err(result) => result,
    };
    println!("{text}\n");
    text
}
