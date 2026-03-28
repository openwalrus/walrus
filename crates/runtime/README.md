# crabtalk-runtime

Agent runtime — tool dispatch, MCP bridge, skills, memory, and session management.

Provides `Session` for stateful agent execution, `McpBridge` for connecting to
MCP servers (stdio and HTTP transports), `SkillRegistry` for loading skill
directories, and `MemoryStore` for agent memory. Includes an inline MCP client
that implements `initialize`, `tools/list`, and `tools/call` over JSON-RPC 2.0.

## License

MIT OR Apache-2.0
