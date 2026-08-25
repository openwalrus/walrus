//! What a line beginning with `/` asks for.

/// The commands, and what each is for.
pub const HELP: &[(&str, &str)] = &[
    (
        "/reconnect",
        "rebuild the engine from the config as it now reads",
    ),
    ("/help", "this list"),
    ("/exit", "leave"),
];

pub enum Command {
    Reconnect,
    Help,
    Exit,
    Unknown(String),
}

impl Command {
    /// Parse a submitted line, or `None` when it is a message rather than
    /// a command.
    pub fn parse(line: &str) -> Option<Self> {
        let line = line.trim();
        let name = line.strip_prefix('/')?;
        Some(match name {
            "reconnect" => Self::Reconnect,
            "help" => Self::Help,
            "exit" | "quit" => Self::Exit,
            other => Self::Unknown(other.to_owned()),
        })
    }
}
