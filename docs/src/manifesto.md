# Manifesto

Ownership is necessary for an open agent ecosystem.

Ownership is not configuration. A configured agent is one where you picked from
someone else's menu. An owned agent is one where you decided what's on the menu.
Ownership is the power to compose your own stack.

Every agent application today rebuilds session management, command dispatch, and
event streaming from scratch — then bundles it alongside search, browser
automation, PDF parsing, TTS, image processing, and dozens of tools you didn't
ask for into one process. If you want a Telegram bot with search, you carry
nineteen other channels and every integration. If you want a coding agent, you
carry TTS and image generation. The process is theirs. The choices are theirs.
You run it.

This happens because the daemon layer is missing. Without it, every application
must become the daemon. And a daemon that is also an application ships its
opinion of what your agent should be.

CrabTalk is that daemon layer. It manages sessions, dispatches commands, and
streams the full execution lifecycle to your client. It does not bundle search.
It does not bundle gateways. It does not bundle tools. You put what you need on
your PATH. They connect as clients. They crash alone. They swap without
restarts. The daemon never loads them.

An agent daemon is not an agent application. An agent daemon empowers you to
build the application you want — and only the application you want. This is the
essence of ownership.

We cannot expect agent platforms to give us ownership out of their beneficence.
It is to their advantage to bundle, to lock in, to ship their choices as yours.
We should expect that they will bundle. The only way to preserve choice is to
never take it away in the first place.

We don't much care if you prefer a batteries-included experience. You could
build an OpenClaw-like assistant or a Hermes-like agent on top of CrabTalk. You
can't build a CrabTalk underneath them. The daemon must come first. The
architecture must be right. Everything else follows.

Let us proceed.
