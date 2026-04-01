# Rust Style

## Imports

- Always use group imports: `use foo::{Bar, Baz};` — never individual `use` lines for the same crate.
- No empty lines between `use` items. `mod` / `pub mod` declarations go after all `use` items.
- Never use `super::` in imports — always use `crate::`.

## File Organization

- Each `.rs` file should have a single, focused responsibility.
- A trait impl for a struct (for traits we define) goes in its own file — e.g., `impl Skill for MyAgent` lives in `skill.rs`, not alongside the struct definition.
- Small utility/helper functions that don't belong to a specific struct go in `utils.rs`.

## Tests

- Do not write tests unless the user explicitly asks for them.
- When writing tests, they live in a `tests/` directory next to `src/`, never in `#[cfg(test)] mod tests` inline blocks.

## Dependencies

- Always inherit dependencies from the workspace — declare them in the root `[workspace.dependencies]` and use `xxx.workspace = true` in member crates. Use `{ workspace = true, features = [...] }` only when features are needed. Never declare a version directly in a member's `Cargo.toml`.

## Binary Crates

- Binary entry points always go in `src/bin/main.rs`, never `src/main.rs`. Use `[[bin]] path = "src/bin/main.rs"` in `Cargo.toml`.

## Constants

- Extract magic strings (API prefixes, header values, URL paths) to `const` when they carry semantic meaning or could cause silent bugs if mistyped. Format strings and error messages are fine inline.

## No Indirection

- Prefer plain functions over traits with one implementor.
- Prefer inline logic over helpers used once.
- Prefer flat module structure over deep nesting.
- No generics, type parameters, or abstractions "for future use."
- Never wrap a field access or inner method call — make the field `pub` or use `Deref`/`DerefMut`. Exception: when the wrapper changes semantics (e.g., different return type).
- When a conversion maps to a std trait (`From`, `TryFrom`, `Display`, `Deref`, etc.), implement the trait — don't write a custom method.
