//! Tools example — interactive REPL with tools LLMs can't do natively.
//!
//! Registers `current_time`, `random_number`, and `word_count` — things
//! that require real computation. The REPL lets you ask questions and watch
//! the LLM decide when to use tools.
//!
//! Requires DEEPSEEK_API_KEY. Run with:
//! ```sh
//! cargo run -p walrus-runtime --example tools
//! ```

mod common;

use walrus_runtime::prelude::*;

#[tokio::main]
async fn main() {
    common::init_tracing();
    let mut runtime = common::build_runtime();

    // Tool 1: current time (LLMs don't know the current time).
    let time_tool = Tool {
        name: "current_time".into(),
        description: "Returns the current UTC date and time.".into(),
        parameters: serde_json::from_value(serde_json::json!({
            "type": "object",
            "properties": {}
        }))
        .unwrap(),
        strict: false,
    };
    runtime.register(time_tool, |_| async move {
        let now = chrono_free_now();
        format!("Current UTC time: {now}")
    });

    // Tool 2: random number (LLMs can't generate true randomness).
    let rand_tool = Tool {
        name: "random_number".into(),
        description: "Generate a random integer between min and max (inclusive).".into(),
        parameters: serde_json::from_value(serde_json::json!({
            "type": "object",
            "properties": {
                "min": { "type": "integer", "description": "Lower bound (inclusive)" },
                "max": { "type": "integer", "description": "Upper bound (inclusive)" }
            },
            "required": ["min", "max"]
        }))
        .unwrap(),
        strict: false,
    };
    runtime.register(rand_tool, |args| async move {
        use rand::Rng;
        let parsed: serde_json::Value = serde_json::from_str(&args).unwrap_or_default();
        let min = parsed["min"].as_i64().unwrap_or(1);
        let max = parsed["max"].as_i64().unwrap_or(100);
        let result = rand::thread_rng().gen_range(min..=max);
        format!("{result}")
    });

    // Tool 3: word count (LLMs approximate poorly on exact counts).
    let wc_tool = Tool {
        name: "word_count".into(),
        description: "Count words, characters, and lines in a text string.".into(),
        parameters: serde_json::from_value(serde_json::json!({
            "type": "object",
            "properties": {
                "text": { "type": "string", "description": "The text to analyze" }
            },
            "required": ["text"]
        }))
        .unwrap(),
        strict: false,
    };
    runtime.register(wc_tool, |args| async move {
        let parsed: serde_json::Value = serde_json::from_str(&args).unwrap_or_default();
        let text = parsed["text"].as_str().unwrap_or("");
        let words = text.split_whitespace().count();
        let chars = text.chars().count();
        let lines = text.lines().count();
        format!("Words: {words}, Characters: {chars}, Lines: {lines}")
    });

    runtime.add_agent(
        Agent::new("assistant")
            .system_prompt(
                "You are a helpful assistant with access to tools. \
                 Use them when the user needs the current time, random numbers, \
                 or exact text analysis.",
            )
            .tool("current_time")
            .tool("random_number")
            .tool("word_count")
            .tool("remember"),
    );

    println!("Tools REPL — try asking:");
    println!("  'What time is it?'");
    println!("  'Pick a random number between 1 and 100'");
    println!("  'Count the words in: The quick brown fox jumps over the lazy dog'");
    println!("(type 'exit' to quit)");
    println!("---");
    common::repl(&mut runtime, "assistant").await;
}

/// Format the current UTC time without chrono (using std only).
fn chrono_free_now() -> String {
    use std::time::SystemTime;
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Simple UTC timestamp formatting.
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    // Days since 1970-01-01 to Y-M-D (simplified Gregorian).
    let (year, month, day) = days_to_date(days);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

fn days_to_date(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let month_days: &[u64] = if is_leap(year) {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 0;
    for &md in month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year, month + 1, days + 1)
}

fn is_leap(y: u64) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}
