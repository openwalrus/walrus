//! Bash policy — deny list with string containment.

use wcore::BashConfig;

/// Check a command against the deny list. Returns the reason if blocked.
pub fn check(config: &BashConfig, command: &str) -> Option<String> {
    config
        .deny
        .iter()
        .find(|d| command.contains(d.as_str()))
        .map(|d| format!("blocked: command contains '{d}'"))
}

/// Build a system prompt block describing the policy.
pub fn prompt_block(config: &BashConfig) -> Option<String> {
    if config.deny.is_empty() {
        return None;
    }
    Some(format!(
        "\n\n<bash-policy>\ndenied: {}\n</bash-policy>",
        config.deny.join(", ")
    ))
}
