//! Short-name → crates.io crate resolution.

const TABLE: &[(&str, &str)] = &[
    ("daemon", "crabtalkd"),
    ("tui", "crabtalk-tui"),
    ("telegram", "crabtalk-telegram"),
    ("wechat", "crabtalk-wechat"),
    ("search", "crabtalk-search"),
    ("outlook", "crabtalk-outlook"),
];

/// Resolve a short name to a crates.io crate name. Unknown names pass through.
pub fn resolve(name: &str) -> &str {
    TABLE
        .iter()
        .find(|(short, _)| *short == name)
        .map(|(_, krate)| *krate)
        .unwrap_or(name)
}

/// Return true if this crate is crabtalk-owned.
pub fn is_crabtalk(krate: &str) -> bool {
    krate == "crabtalkd" || krate.starts_with("crabtalk-") || krate == "crabup"
}
