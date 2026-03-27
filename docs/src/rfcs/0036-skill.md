# 0036 - Skill System

- Feature Name: Skill System
- Start Date: 2026-03-27
- Discussion: [#36](https://github.com/crabtalk/crabtalk/issues/36)
- Crates: runtime

## Summary

A file-based skill system where agent behaviors are defined as `SKILL.md`
markdown files with YAML frontmatter, loaded into a registry at startup, and
invoked as slash commands or tool calls during conversation.

## Motivation

Agents need extensible behavior without recompilation. The original approaches
(extensions, hooks, commands) were either too heavyweight or too rigid. Skills
are the simplest unit that works: a markdown file with a name, description, and
a prompt body. No code generation, no plugin API, no runtime linking.

The skill is the agent's equivalent of a function — a named, scoped, reusable
piece of behavior that the agent can invoke on demand. The format follows the
agentskills.io convention for interoperability.

## Design

### SKILL.md format

Each skill lives in its own directory containing a `SKILL.md` file:

```
skills/
  check-feeds/
    SKILL.md
  summarize/
    SKILL.md
```

The file uses YAML frontmatter followed by a markdown body:

```markdown
---
name: check-feeds
description: Check RSS feeds for new entries
allowed-tools:
  - web_fetch
---

You are a feed checker. Given the user's feed list...
```

Required fields:

- `name` — unique identifier, used as the slash command name.
- `description` — one-line summary for listing and fuzzy search.

Optional fields:

- `allowed-tools` — tool whitelist for this skill's execution context.

The markdown body after the frontmatter is the skill's prompt — injected into
the agent's context when the skill is invoked.

### SkillRegistry

A `Vec<Skill>` with lookup methods:

- `add(skill)` — append.
- `upsert(skill)` — replace by name if exists, else append.
- `contains(name)` — name check.
- `skills()` — list all.

No indexing by name. The registry is small enough that linear scan is fine.
Wrapped in `Mutex` inside `SkillHandler` for concurrent access from tool
dispatch.

### SkillHandler

Owns the registry and the list of skill directories to search. Two
responsibilities:

1. **Startup load** — `SkillHandler::load(dirs)` scans each directory
   recursively for `SKILL.md` files. Skips hidden directories (`.`-prefixed).
   Duplicate names across directories are detected and skipped with a warning
   (first-loaded wins, in config-defined directory order).

2. **Runtime state** — holds `Mutex<SkillRegistry>` for the tool dispatch path
   to read and update.

### Loader

`loader::load_skills_dir(path)` walks a directory tree. For each subdirectory:

- If it contains `SKILL.md`, parse it and add to the registry.
- If it does not, recurse into it.

This allows nested organization (`skills/category/my-skill/SKILL.md`) without
requiring flat layout. Parsing uses `serde_yml` for frontmatter and
`split_yaml_frontmatter` (from `crabtalk-core::utils`) for the split.

### Tool dispatch (`dispatch_skill`)

Exposed as a tool the agent can call. Input: `{ name: string }`.

Resolution order:

1. **Scope check** — if the agent has a skill scope defined and the requested
   skill is not in it, reject immediately.
2. **Path traversal guard** — reject names containing `..`, `/`, or `\`.
3. **Exact load** — for each skill directory, check
   `{dir}/{name}/SKILL.md`. If found, parse it, upsert into the registry,
   return the body plus the skill directory path.
4. **Fuzzy fallback** — if no exact match, search the registry by
   case-insensitive substring match on name and description. Return matching
   skill names and descriptions. If the input name is empty, list all available
   skills (respecting scope).

The upsert on exact load means skills can be updated on disk and picked up on
next invocation without daemon restart.

### Skill scoping

Agents can be restricted to a subset of skills via the `scopes` map on
`RuntimeHook`. If an agent's scope has a non-empty `skills` list, only those
skills are available. Empty list means unrestricted. Scoping applies to both
exact load and fuzzy listing.

## Alternatives

**Code-based plugins (dylib / WASM).** Far more powerful but far more complex.
Skills are prompt injection, not code execution. The simplicity of markdown
files is the point.

**Database-backed registry.** Adds persistence complexity for a registry that
rebuilds in milliseconds from disk. Not needed.

**Strict schema validation.** The frontmatter is loose by design — unknown
fields are ignored. This allows the format to evolve without breaking existing
skills.

## Unresolved Questions

- Should skills support arguments beyond the skill name (parameterized
  prompts)?
- Should there be a package manager or registry service for sharing skills
  across installations?
- Should `allowed-tools` be enforced at the runtime level? Currently it is not
  enforced — it exists in the format but has no runtime effect.
