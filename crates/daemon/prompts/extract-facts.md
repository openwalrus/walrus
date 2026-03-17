Extract key facts as TOML key-value pairs from the conversation summary below.

Rules:
- Output only valid TOML (no markdown fences, no commentary).
- Use snake_case keys.
- Values must be quoted strings.
- Only extract durable facts: names, preferences, tools, languages, locations.
- Skip transient information like specific file paths or task details.
- If no facts are worth extracting, output nothing.
