# Introduction

This is the crabtalk development book — the knowledge base you check before
building. It captures what crabtalk stands for, how the system is shaped, and
the design decisions that govern its evolution.

For user-facing documentation (installation, configuration, commands), see
[crabtalk.ai](https://crabtalk.ai).

## How this book is organized

- **[Manifesto](manifesto.md)** — What crabtalk is and what it stands for.
- **[Architecture](architecture.md)** — The system shape: crate layering,
  where features go, boundary contracts, and what the system can do today.
- **[RFCs](rfcs/README.md)** — Design decisions. Each RFC captures a specific
  problem, the decision we made, and why.

## Contributing

Build with `cargo check --workspace`, test with `cargo nextest run --workspace`.
Conventions and code style are documented in the repository.
