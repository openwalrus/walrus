pub type EntryId = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntryKind {
    Note,
    Archive,
    /// Curated content auto-injected into the agent's system prompt.
    /// One `global` plus optionally one per agent id.
    Prompt,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub id: EntryId,
    pub name: String,
    pub content: String,
    pub aliases: Vec<String>,
    pub created_at: u64,
    pub kind: EntryKind,
}
