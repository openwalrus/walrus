//! Streaming markdown renderer with syntax-highlighted code blocks.

use console::{Style, Term, style};
use heck::ToUpperCamelCase;
use std::{
    io::{BufWriter, Stdout, Write},
    sync::LazyLock,
};
use syntect::{
    easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet,
    util::as_24_bit_terminal_escaped,
};

// Reusable styles.
static S_BOLD: LazyLock<Style> = LazyLock::new(|| Style::new().bold());
static S_DIM: LazyLock<Style> = LazyLock::new(|| Style::new().dim());
static S_ITALIC: LazyLock<Style> = LazyLock::new(|| Style::new().italic());
static S_H1: LazyLock<Style> = LazyLock::new(|| Style::new().bold().cyan().bright());
static S_H2: LazyLock<Style> = LazyLock::new(|| Style::new().bold().blue().bright());
static S_H3: LazyLock<Style> = LazyLock::new(|| Style::new().bold().white().bright());
static S_CODE: LazyLock<Style> = LazyLock::new(|| Style::new().cyan());
static S_PROMPT: LazyLock<Style> = LazyLock::new(|| Style::new().bold().green().bright());
static S_BANNER: LazyLock<Style> = LazyLock::new(|| Style::new().bold().yellow().bright());
static S_GREEN: LazyLock<Style> = LazyLock::new(|| Style::new().green().bright());
static S_RED: LazyLock<Style> = LazyLock::new(|| Style::new().red().bright());

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

/// Leading marker for the first line of agent response.
const MARKER: &str = "⏺ ";
/// Padding for continuation lines (same visual width as MARKER).
const PAD: &str = "  ";

/// ANSI blink on.
const BLINK_ON: &str = "\x1b[5m";
/// ANSI reset.
const RESET: &str = "\x1b[0m";
/// ANSI erase entire line.
const ERASE_LINE: &str = "\x1b[2K";

/// Terminal width with fallback.
fn term_width() -> usize {
    Term::stdout().size().1 as usize
}

/// Renderer state tracking whether we're in normal text, a code block, or thinking.
#[derive(Default)]
enum RenderState {
    #[default]
    Normal,
    CodeBlock {
        lang: String,
        code: String,
    },
    Thinking,
}

/// Streaming markdown renderer that buffers lines and emits styled output.
pub struct MarkdownRenderer {
    line_buf: String,
    state: RenderState,
    out: BufWriter<Stdout>,
    started: bool,
    first_line: bool,
    tool_labels: Vec<String>,
    tool_result_lines: usize,
    after_tool: bool,
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        Self {
            line_buf: String::new(),
            state: RenderState::Normal,
            out: BufWriter::new(std::io::stdout()),
            started: false,
            first_line: false,
            tool_labels: Vec::new(),
            tool_result_lines: 0,
            after_tool: false,
        }
    }

    fn ensure_started(&mut self) {
        if !self.started {
            let _ = write!(self.out, "{MARKER}");
            self.started = true;
            self.first_line = true;
        }
    }

    pub fn push_text(&mut self, chunk: &str) {
        if matches!(self.state, RenderState::Thinking) {
            self.state = RenderState::Normal;
        }

        self.ensure_started();

        if self.after_tool {
            let _ = write!(self.out, "{PAD}");
            self.after_tool = false;
        }

        for ch in chunk.chars() {
            if ch == '\n' {
                let line = std::mem::take(&mut self.line_buf);
                self.render_line(&line);
            } else {
                self.line_buf.push(ch);
            }
        }
        let _ = self.out.flush();
    }

    pub fn push_thinking(&mut self, chunk: &str) {
        if !matches!(self.state, RenderState::Thinking) {
            self.state = RenderState::Thinking;
        }
        let styled = style(chunk).dim().italic();
        let _ = write!(self.out, "{styled}");
        let _ = self.out.flush();
    }

    pub fn push_tool_start(&mut self, calls: &[(String, String)]) {
        self.flush_thinking();
        if !self.line_buf.is_empty() {
            let line = std::mem::take(&mut self.line_buf);
            self.render_inline(&line);
            let _ = writeln!(self.out);
        }
        let _ = writeln!(self.out);
        let _ = self.out.flush();

        self.tool_labels.clear();
        for (name, args) in calls {
            let label = format_tool_label(name, args);
            let _ = writeln!(
                self.out,
                "{PAD}{BLINK_ON}⏺{RESET} {}",
                style(&label).bold().dim()
            );
            self.tool_labels.push(label);
        }
        let _ = writeln!(self.out);
        let _ = self.out.flush();
    }

    pub fn push_tool_result(&mut self, output: &str) {
        let tree = format_tool_result(output);
        let width = term_width().saturating_sub(PAD.len() + 4);
        let mut count = 0;
        for (i, node) in tree.iter().enumerate() {
            let is_last = i == tree.len() - 1;
            let branch = if is_last { "└ " } else { "├ " };
            let cont = if is_last { "  " } else { "│ " };

            let _ = writeln!(
                self.out,
                "{PAD}  {}{}",
                S_DIM.apply_to(branch),
                S_DIM.apply_to(&node.label)
            );
            count += 1;

            for line in &node.lines {
                let wrapped = textwrap::fill(line, width);
                for wl in wrapped.lines() {
                    let _ = writeln!(
                        self.out,
                        "{PAD}  {}{}",
                        S_DIM.apply_to(cont),
                        S_DIM.apply_to(wl)
                    );
                    count += 1;
                }
            }
        }
        self.tool_result_lines += count;
        let _ = self.out.flush();
    }

    pub fn push_tool_done(&mut self, success: bool) {
        let count = self.tool_labels.len();
        if count == 0 {
            return;
        }

        let _ = self.out.flush();
        let marker = if success {
            S_GREEN.apply_to("⏺")
        } else {
            S_RED.apply_to("⏺")
        };

        let up = count + 1 + self.tool_result_lines;
        let _ = write!(self.out, "\x1b[{up}A");
        for label in &self.tool_labels {
            let _ = write!(
                self.out,
                "\r{ERASE_LINE}{PAD}{marker} {}\n",
                style(label).bold().dim()
            );
        }
        let skip = self.tool_result_lines + 1;
        let _ = write!(self.out, "\x1b[{skip}B");
        let _ = self.out.flush();
        self.tool_labels.clear();
        self.tool_result_lines = 0;
        self.after_tool = true;
    }

    pub fn finish(&mut self) {
        if !self.tool_labels.is_empty() {
            self.push_tool_done(false);
        }
        self.flush_thinking();

        if !self.line_buf.is_empty() {
            let line = std::mem::take(&mut self.line_buf);
            match &self.state {
                RenderState::CodeBlock { .. } => {
                    self.flush_code_block_raw(&line);
                }
                _ => {
                    self.ensure_started();
                    let prefix = self.line_prefix();
                    self.render_wrapped(&line, prefix);
                }
            }
        }

        if let RenderState::CodeBlock { lang, code } = &self.state {
            let lang = lang.clone();
            let code = code.clone();
            self.emit_code_block(&lang, &code);
            self.state = RenderState::Normal;
        }

        let _ = self.out.flush();
    }

    fn render_line(&mut self, line: &str) {
        match &mut self.state {
            RenderState::CodeBlock { lang: _, code } => {
                if line.starts_with("```") {
                    let lang = if let RenderState::CodeBlock { lang, .. } = &self.state {
                        lang.clone()
                    } else {
                        String::new()
                    };
                    let code = if let RenderState::CodeBlock { code, .. } = &self.state {
                        code.clone()
                    } else {
                        String::new()
                    };
                    self.emit_code_block(&lang, &code);
                    self.state = RenderState::Normal;
                } else {
                    code.push_str(line);
                    code.push('\n');
                }
            }
            RenderState::Normal | RenderState::Thinking => {
                if let Some(rest) = line.strip_prefix("```") {
                    let lang = rest.trim().to_string();
                    self.print_code_border_top(&lang);
                    self.state = RenderState::CodeBlock {
                        lang,
                        code: String::new(),
                    };
                } else {
                    let prefix = self.line_prefix();
                    if let Some(rest) = line.strip_prefix("### ") {
                        let _ = writeln!(self.out, "{prefix}{}", S_H3.apply_to(rest));
                    } else if let Some(rest) = line.strip_prefix("## ") {
                        let _ = writeln!(self.out, "{prefix}{}", S_H2.apply_to(rest));
                    } else if let Some(rest) = line.strip_prefix("# ") {
                        let _ = writeln!(self.out, "{prefix}{}", S_H1.apply_to(rest));
                    } else if line == "---" || line == "***" || line == "___" {
                        let w = term_width().min(60);
                        let rule: String = "─".repeat(w);
                        let _ = writeln!(self.out, "{prefix}{}", S_DIM.apply_to(rule));
                    } else if let Some(rest) =
                        line.strip_prefix("- ").or_else(|| line.strip_prefix("* "))
                    {
                        let first = format!("{prefix}  • ");
                        self.render_wrapped(rest, &first);
                    } else if is_ordered_list(line) {
                        let (num_prefix, rest) = split_ordered_list(line);
                        let first = format!("{prefix}  {num_prefix}");
                        self.render_wrapped(rest, &first);
                    } else if line.starts_with('.') {
                        let first = format!("{prefix} ");
                        self.render_wrapped(line, &first);
                    } else {
                        self.render_wrapped(line, prefix);
                    }
                }
            }
        }
    }

    fn render_inline(&mut self, text: &str) {
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            if i + 1 < len
                && chars[i] == '*'
                && chars[i + 1] == '*'
                && let Some(end) = find_closing(&chars, i + 2, "**")
            {
                let inner: String = chars[i + 2..end].iter().collect();
                let _ = write!(self.out, "{}", S_BOLD.apply_to(&inner));
                i = end + 2;
                continue;
            }
            if chars[i] == '*'
                && (i + 1 >= len || chars[i + 1] != '*')
                && let Some(end) = find_closing_char(&chars, i + 1, '*')
            {
                let inner: String = chars[i + 1..end].iter().collect();
                let _ = write!(self.out, "{}", S_ITALIC.apply_to(&inner));
                i = end + 1;
                continue;
            }
            if chars[i] == '`'
                && let Some(end) = find_closing_char(&chars, i + 1, '`')
            {
                let inner: String = chars[i + 1..end].iter().collect();
                let _ = write!(self.out, "{}", S_CODE.apply_to(&inner));
                i = end + 1;
                continue;
            }
            let _ = write!(self.out, "{}", chars[i]);
            i += 1;
        }
    }

    fn render_wrapped(&mut self, text: &str, first_prefix: &str) {
        let indent = first_prefix.len().max(PAD.len());
        let width = term_width().saturating_sub(indent);
        let wrapped = textwrap::fill(text, width);
        for (i, line) in wrapped.lines().enumerate() {
            let p = if i == 0 { first_prefix } else { PAD };
            let _ = write!(self.out, "{p}");
            self.render_inline(line);
            let _ = writeln!(self.out);
        }
    }

    fn line_prefix(&mut self) -> &'static str {
        if self.first_line {
            self.first_line = false;
            ""
        } else {
            PAD
        }
    }

    fn print_code_border_top(&mut self, lang: &str) {
        let prefix = self.line_prefix();
        let label = if lang.is_empty() {
            "┌─".to_string()
        } else {
            format!("┌ {lang} ─")
        };
        let _ = writeln!(self.out, "{prefix}{}", S_DIM.apply_to(label));
    }

    fn emit_code_block(&mut self, lang: &str, code: &str) {
        let syntax = if lang.is_empty() {
            SYNTAX_SET.find_syntax_plain_text()
        } else {
            SYNTAX_SET
                .find_syntax_by_token(lang)
                .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text())
        };

        let theme = &THEME_SET.themes["base16-ocean.dark"];
        let mut h = HighlightLines::new(syntax, theme);
        let pipe = S_DIM.apply_to("│");

        for line in code.lines() {
            match h.highlight_line(line, &SYNTAX_SET) {
                Ok(ranges) => {
                    let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
                    let _ = writeln!(self.out, "{PAD}{pipe} {escaped}\x1b[0m");
                }
                Err(_) => {
                    let _ = writeln!(self.out, "{PAD}{pipe} {line}");
                }
            }
        }

        let _ = writeln!(self.out, "{PAD}{}", S_DIM.apply_to("└─"));
    }

    fn flush_code_block_raw(&mut self, extra: &str) {
        let pipe = S_DIM.apply_to("│");
        if let RenderState::CodeBlock { code, .. } = &self.state {
            let full = format!("{code}{extra}");
            for line in full.lines() {
                let _ = writeln!(self.out, "{PAD}{pipe} {line}");
            }
        }
        let _ = writeln!(self.out, "{PAD}{}", S_DIM.apply_to("└─"));
        self.state = RenderState::Normal;
    }

    fn flush_thinking(&mut self) {
        if matches!(self.state, RenderState::Thinking) {
            self.state = RenderState::Normal;
        }
    }
}

/// Build the styled prompt string for rustyline.
pub fn styled_prompt(agent: &str) -> String {
    format!("{} > ", S_PROMPT.apply_to(agent))
}

/// Build a welcome banner with optional model name.
pub fn welcome_banner(model: Option<&str>) -> String {
    let model_part = match model {
        Some(m) => format!(" ({m})"),
        None => String::new(),
    };
    let title = format!("  Crabtalk{model_part} — Ctrl+D to exit");
    let width = title.len().min(60);
    let rule: String = "─".repeat(width);
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "?".to_string());
    let cwd_line = style(format!("  ~ {cwd}")).bold().dim();
    format!(
        "{}\n{}\n{cwd_line}",
        S_BANNER.apply_to(&title),
        S_DIM.apply_to(format!("  {rule}")),
    )
}

fn is_ordered_list(line: &str) -> bool {
    let mut chars = line.chars();
    if !chars.next().is_some_and(|c| c.is_ascii_digit()) {
        return false;
    }
    for c in chars {
        if c == '.' {
            return true;
        }
        if !c.is_ascii_digit() {
            return false;
        }
    }
    false
}

fn split_ordered_list(line: &str) -> (&str, &str) {
    if let Some(dot) = line.find(". ") {
        (&line[..dot + 2], &line[dot + 2..])
    } else {
        (line, "")
    }
}

fn find_closing(chars: &[char], start: usize, pattern: &str) -> Option<usize> {
    let pat: Vec<char> = pattern.chars().collect();
    let plen = pat.len();
    for i in start..chars.len().saturating_sub(plen - 1) {
        if chars[i..i + plen] == pat[..] {
            return Some(i);
        }
    }
    None
}

fn find_closing_char(chars: &[char], start: usize, closing: char) -> Option<usize> {
    (start..chars.len()).find(|&i| chars[i] == closing)
}

struct ResultNode {
    label: String,
    lines: Vec<String>,
}

fn format_tool_result(output: &str) -> Vec<ResultNode> {
    let max_lines = 10;

    if let Ok(v) = serde_json::from_str::<serde_json::Value>(output)
        && let (Some(stdout), Some(stderr), Some(exit_code)) = (
            v.get("stdout").and_then(|s| s.as_str()),
            v.get("stderr").and_then(|s| s.as_str()),
            v.get("exit_code").and_then(|c| c.as_i64()),
        )
    {
        let mut nodes = Vec::new();
        if !stdout.is_empty() {
            let mut lines: Vec<String> = stdout
                .lines()
                .take(max_lines)
                .map(|l| l.to_string())
                .collect();
            let total = stdout.lines().count();
            if total > max_lines {
                lines.push(format!("... ({} more lines)", total - max_lines));
            }
            nodes.push(ResultNode {
                label: "stdout".to_string(),
                lines,
            });
        }
        if !stderr.is_empty() {
            let lines: Vec<String> = stderr.lines().take(4).map(|l| l.to_string()).collect();
            nodes.push(ResultNode {
                label: "stderr".to_string(),
                lines,
            });
        }
        if exit_code != 0 {
            nodes.push(ResultNode {
                label: format!("exit_code: {exit_code}"),
                lines: vec![],
            });
        }
        if nodes.is_empty() {
            nodes.push(ResultNode {
                label: "(no output)".to_string(),
                lines: vec![],
            });
        }
        return nodes;
    }

    let mut lines: Vec<String> = output
        .lines()
        .take(max_lines)
        .map(|l| l.to_string())
        .collect();
    let total = output.lines().count();
    if total > max_lines {
        lines.push(format!("... ({} more lines)", total - max_lines));
    }
    if lines.is_empty() {
        return vec![ResultNode {
            label: "(no output)".to_string(),
            lines: vec![],
        }];
    }
    vec![ResultNode {
        label: "output".to_string(),
        lines,
    }]
}

fn format_tool_label(name: &str, args: &str) -> String {
    let pascal = name.to_upper_camel_case();

    if name != "bash" {
        return pascal;
    }

    let Ok(v) = serde_json::from_str::<serde_json::Value>(args) else {
        return pascal;
    };

    let Some(cmd) = v.get("command").and_then(|c| c.as_str()) else {
        return pascal;
    };

    let max = term_width().saturating_sub(PAD.len() + 8);
    let cmd = if cmd.len() > max {
        format!("{}...", &cmd[..max])
    } else {
        cmd.to_string()
    };
    format!("Bash({cmd})")
}
