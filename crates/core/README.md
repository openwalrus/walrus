# crabtalk-core

Stateful agent execution library.

Provides `Agent<M: Model>` with step/run/run_stream execution, `AgentConfig`,
`AgentBuilder`, `ToolSender` channel for tool dispatch, and event types
(`AgentEvent`, `AgentStep`, `AgentResponse`). Also includes the unified `model`
module with `Model` trait, `Message`, `Tool`, `Request`, and `Response`.

## License

MIT OR Apache-2.0
