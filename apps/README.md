# Components

Independent binaries that extend the daemon. Each component runs as its
own process and connects via auto-discovery (`crabtalk <name>` finds
`crabtalk-<name>` on PATH).

| Component | Crate | What it does |
| --------- | ----- | ------------ |
| [Hub](hub) | `crabhub` | Package management |
| [Outlook](outlook) | `crabtalk-outlook` | Outlook MCP server (email + calendar) |
| [Search](search) | `crabtalk-search` | Meta-search aggregator |
| [Telegram](telegram) | `crabtalk-telegram` | Telegram gateway |
| [WeChat](wechat) | `crabtalk-wechat` | WeChat gateway |
