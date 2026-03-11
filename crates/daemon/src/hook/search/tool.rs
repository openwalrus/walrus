//! Tool schemas and dispatch for web search and fetch.

use schemars::JsonSchema;
use serde::Deserialize;
use wcore::{
    agent::{AsTool, ToolDescription},
    model::Tool,
};

#[derive(Deserialize, JsonSchema)]
pub(crate) struct WebSearch {
    /// The search query string.
    pub query: String,
    /// Maximum number of results to return (default: 10).
    pub max_results: Option<usize>,
}

impl ToolDescription for WebSearch {
    const DESCRIPTION: &'static str = "Search the web for information. Returns titles, URLs, and descriptions from multiple search engines.";
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct WebFetch {
    /// The URL to fetch content from.
    pub url: String,
}

impl ToolDescription for WebFetch {
    const DESCRIPTION: &'static str = "Fetch a web page and return its clean text content with scripts, styles, and navigation stripped.";
}

pub(crate) fn tools() -> Vec<Tool> {
    vec![WebSearch::as_tool(), WebFetch::as_tool()]
}

impl crate::hook::DaemonHook {
    pub(crate) async fn dispatch_web_search(&self, args: &str) -> String {
        let input: WebSearch = match serde_json::from_str(args) {
            Ok(v) => v,
            Err(e) => return format!("invalid arguments: {e}"),
        };

        let max = input.max_results.unwrap_or(10);
        match self.aggregator.search(&input.query, 0).await {
            Ok(mut results) => {
                results.results.truncate(max);
                let count = results.results.len();
                let mut out = format!(
                    "Found {count} results for \"{}\" ({}ms):\n",
                    results.query, results.elapsed_ms
                );
                for (i, r) in results.results.iter().enumerate() {
                    out.push_str(&format!(
                        "\n{}. {}\n   {}\n   {}\n",
                        i + 1,
                        r.title,
                        r.url,
                        r.description
                    ));
                }
                if !results.engine_errors.is_empty() {
                    out.push_str("\nEngine errors:\n");
                    for e in &results.engine_errors {
                        out.push_str(&format!("  - {}: {}\n", e.engine, e.error));
                    }
                }
                out
            }
            Err(e) => format!("web_search failed: {e}"),
        }
    }

    pub(crate) async fn dispatch_web_fetch(&self, args: &str) -> String {
        let input: WebFetch = match serde_json::from_str(args) {
            Ok(v) => v,
            Err(e) => return format!("invalid arguments: {e}"),
        };

        match wsearch::browser::fetch::fetch_url(&input.url, &self.fetch_client).await {
            Ok(result) => {
                format!(
                    "# {}\nURL: {}\nContent-Length: {}\n\n{}",
                    result.title, result.url, result.content_length, result.content
                )
            }
            Err(e) => format!("web_fetch failed: {e}"),
        }
    }
}
