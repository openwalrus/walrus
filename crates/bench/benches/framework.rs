//! Cross-framework benchmark — same tasks, same mock MCP, different agent runtimes.
//!
//! Prerequisites:
//! 1. Local LLM via ollama (fixed model version)
//! 2. Frameworks running and connected to mock MCP + same LLM.
//!    Mock MCP is started in-process automatically.
//!    Unreachable frameworks are skipped with a warning.
//!
//! Ports are configurable via env vars:
//!   MOCK_MCP_PORT (default: 0 = random), CRABTALK_PORT (6688),
//!   OPENCLAW_PORT (18789), OPENCODE_PORT (4096), HERMES_PORT (8080)

use crabtalk_bench::{
    gateway::{
        Gateway, check_reachable, crabtalk::CrabtalkGateway, hermes::HermesGateway,
        openclaw::OpenClawGateway, opencode::OpenCodeGateway,
    },
    mock_mcp,
    task::tasks,
};
use criterion::{Criterion, criterion_group, criterion_main};

struct ValidationRecord {
    framework: &'static str,
    task: &'static str,
    expected_calls: usize,
    actual_calls: usize,
    success: bool,
    wall_clock_ms: u64,
    tool_names: Vec<String>,
}

fn env_port(var: &str, default: u16) -> u16 {
    std::env::var(var)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn bench_framework(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let task_defs = tasks();
    let mcp_port = env_port("MOCK_MCP_PORT", 0);
    let mcp_handle = rt.block_on(mock_mcp::start(mcp_port, &task_defs));
    eprintln!("mock MCP listening at http://{}/mcp", mcp_handle.addr());

    let all_gateways: Vec<(&str, u16, Box<dyn Gateway>)> = vec![
        {
            let port = env_port("CRABTALK_PORT", 6688);
            ("crabtalk", port, Box::new(CrabtalkGateway::new(port)))
        },
        {
            let port = env_port("OPENCLAW_PORT", 18789);
            let token = std::env::var("OPENCLAW_TOKEN").unwrap_or_default();
            (
                "openclaw",
                port,
                Box::new(OpenClawGateway::new(port, token)),
            )
        },
        {
            let port = env_port("OPENCODE_PORT", 4096);
            ("opencode", port, Box::new(OpenCodeGateway::new(port)))
        },
        {
            let port = env_port("HERMES_PORT", 8080);
            ("hermes", port, Box::new(HermesGateway::new(port)))
        },
    ];

    // Skip frameworks that aren't running.
    let gateways: Vec<_> = all_gateways
        .into_iter()
        .filter(|(name, port, _)| {
            if check_reachable(*port) {
                true
            } else {
                eprintln!("SKIP {name}: not reachable on port {port}");
                false
            }
        })
        .collect();

    if gateways.is_empty() {
        eprintln!("no frameworks available — nothing to benchmark");
        return;
    }

    for task in &task_defs {
        let mut group = c.benchmark_group(task.name);
        // These are real LLM calls — use fewer samples and longer measurement.
        group.sample_size(10);
        group.measurement_time(std::time::Duration::from_secs(30));

        for (name, _, gw) in &gateways {
            // Load this task's responses before benchmarking. Subsequent
            // iterations reuse the last scripted response per tool, which is
            // fine for latency measurement — correctness is checked in the
            // validation pass below.
            mcp_handle.load_task(task);
            group.bench_function(*name, |b| {
                b.iter(|| gw.run_task(&rt, task));
            });
        }
        group.finish();
    }

    // ── Validation pass: run each task once per framework, collect metrics ──
    let mut records = Vec::new();
    for task in &task_defs {
        for (name, _, gw) in &gateways {
            mcp_handle.load_task(task);
            let result = gw.run_task(&rt, task);
            let metrics = mcp_handle.metrics();
            records.push(ValidationRecord {
                framework: name,
                task: task.name,
                expected_calls: task.expected_tool_calls,
                actual_calls: metrics.len(),
                success: result.success,
                wall_clock_ms: result.wall_clock_ms,
                tool_names: metrics.iter().map(|r| r.tool.clone()).collect(),
            });
        }
    }
    print_summary(&records);

    rt.block_on(mcp_handle.shutdown());
}

fn print_summary(records: &[ValidationRecord]) {
    eprintln!();
    eprintln!(
        "{:<22} {:<12} {:<8} {:<14} {:>10}",
        "TASK", "FRAMEWORK", "STATUS", "TOOL CALLS", "TIME(ms)"
    );
    eprintln!("{}", "-".repeat(68));
    for r in records {
        let status = if r.success { "OK" } else { "FAIL" };
        let calls = if r.actual_calls == r.expected_calls {
            format!("{}/{}", r.actual_calls, r.expected_calls)
        } else {
            format!("{}/{} !", r.actual_calls, r.expected_calls)
        };
        eprintln!(
            "{:<22} {:<12} {:<8} {:<14} {:>10}",
            r.task, r.framework, status, calls, r.wall_clock_ms
        );
    }

    let mismatches: Vec<_> = records
        .iter()
        .filter(|r| r.actual_calls != r.expected_calls)
        .collect();
    if !mismatches.is_empty() {
        eprintln!();
        eprintln!("Tool call mismatches:");
        for r in &mismatches {
            eprintln!(
                "  {}/{}: expected {}, got {} -- {:?}",
                r.task, r.framework, r.expected_calls, r.actual_calls, r.tool_names
            );
        }
    }
}

criterion_group!(benches, bench_framework);
criterion_main!(benches);
