# crabtalk-command-codegen

Proc-macro codegen for crabtalk service commands.

Implements the `#[command(kind = "mcp"|"client")]` attribute macro that
generates a `Service` impl, a clap `Subcommand` enum with start/stop/run/logs
variants, and an `exec` dispatcher for the annotated struct.

## License

GPL-3.0
