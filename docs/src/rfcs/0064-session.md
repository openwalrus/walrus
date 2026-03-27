# 0064 - Session

- Feature Name: Session System
- Start Date: 2026-02-25
- Discussion: [#64](https://github.com/crabtalk/crabtalk/issues/64)
- Crates: core, daemon

## Summary

Append-only JSONL session persistence with compact markers, identity-based file
naming, and an auto-injected message lifecycle that separates ephemeral context
from durable history.

## Motivation

An agent daemon needs conversation persistence that is simple, inspectable, and
crash-safe. Database-backed persistence adds operational weight for what is
fundamentally a sequential log. The session format must support:

- Resuming conversations across daemon restarts.
- Compaction — summarizing long histories without losing them.
- Multiple identities — the same agent can talk to different users/platforms.
- Ephemeral context injection — memory recall, environment blocks, and agent
  descriptions must be fresh each run, never accumulating in history.

## Design

### File format

Each session is a JSONL file. Line 1 is metadata, subsequent lines are messages
or compact markers.

```
{"agent":"crab","created_by":"user","created_at":"...","title":"","uptime_secs":0}
{"role":"user","content":"hello"}
{"role":"assistant","content":"hi there"}
{"compact":"Summary of conversation so far..."}
{"role":"user","content":"what were we talking about?"}
```

### Naming

Files live in a flat `sessions/` directory:
`{agent}_{sender_slug}_{seq}.jsonl`

- `sender_slug` — sanitized identity (e.g. `user`, `tg-12345`).
- `seq` — monotonically increasing per (agent, sender) pair.
- After `set_title`, the file is renamed to append a title slug.

### Compact markers

When history exceeds a threshold, the agent compacts: the LLM summarizes the
conversation, and a `{"compact":"..."}` line is appended. On load,
`load_context` reads from the **last compact marker forward**. The compact
summary is injected as a `{"role":"user"}` message — the agent sees it as
context, not as a special marker.

History before the last compact marker is archived in place — still in the file,
but not loaded. Nothing is deleted.

### Auto-injected messages

Messages marked `auto_injected: true` are:

- **Not persisted** to JSONL (skipped in `append_messages`).
- **Stripped** before each run (prevents accumulation).
- **Re-injected fresh** via `Hook::on_before_run()` every execution.

This covers memory recall results, environment blocks, agent description lists,
and working directory announcements. They must be current, not stale from a
previous run.

### Session identity

Sessions are bound to an (agent, sender) pair. `find_latest_session` scans the
directory for the matching prefix and returns the highest seq number. New chats
increment the seq.

### Uptime tracking

Each session tracks `uptime_secs` — accumulated active time, persisted to the
meta line. The meta line is rewritten by reading the full file and writing it
back with the updated first line. This is the one non-append operation — it
trades the append-only guarantee for keeping metadata current. Crash during
rewrite can lose the meta line but not the conversation history (messages are
append-only and survive).

## Alternatives

**SQLite.** Adds a dependency and operational surface for what is a sequential
append log. JSONL files are inspectable with standard tools and trivially
backupable. Appends are crash-safe (partial last line is just a truncated write).

**One file per message.** Too many files. The append-only JSONL approach gives
one file per conversation with clear boundaries.

**No compaction.** Works for short conversations but becomes expensive as
history grows. The compact marker approach keeps the file intact while bounding
the working context.

## Unresolved Questions

- Should session files be organized in date-based subdirectories for easier
  cleanup?
- Should compact threshold be per-agent configurable or global?
