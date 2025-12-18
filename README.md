# Cydonia

The Agent framework.

## Command line interface

```
$ cargo b -p ullm-cli
$ ./target/debug/ullm chat --help
Unified LLM Interface CLI

Usage: ullm [OPTIONS] <COMMAND>

Commands:
  chat      Chat with an LLM
  generate  Generate the configuration file
  help      Print this message or the help of the given subcommand(s)

Options:
  -s, --stream      Enable streaming mode
  -v, --verbose...  Verbosity level (use -v, -vv, -vvv, etc.)
  -h, --help        Print help
  -V, --version     Print version
```

For the ullm CLI, the config is located at `~/.config/ullm.toml`.

## LICENSE

GLP-3.0
