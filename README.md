# MF_RUST_SDK

An unofficial **Rust client for the Claude Agent SDK** — drive [Claude Code](https://github.com/anthropics/claude-code) agent sessions from Rust.

It communicates with the `claude` CLI over its streaming-JSON interface — the same approach Anthropic's official [TypeScript](https://github.com/anthropics/claude-agent-sdk-typescript) and Python SDKs take — letting you send prompts, stream responses and tool calls, gate tool use, run subagents, and resume sessions from a Rust program.

> **Status: early scaffold.** The client isn't implemented yet — this repo is currently just the project skeleton and license.

## How it relates to the official SDKs

This is **not** a fork, and it does **not** reimplement Claude Code's engine. Claude Code's agent loop, built-in tools, and session handling live inside the `claude` CLI. Like the official TypeScript and Python SDKs, this crate is a thin **client** that drives that CLI — here, a Rust port of the client protocol the TypeScript SDK uses.

## Requirements

- The [`claude` CLI](https://github.com/anthropics/claude-code) installed and available on your `PATH`.

## Usage

_Coming soon — the client is not implemented yet._

## Staying in sync with upstream

The CLI's client protocol has no formal stability guarantee, so this crate targets specific `claude` CLI and official-SDK versions, tracks upstream releases, and uses a conformance test suite (run against the installed CLI) to catch breaking protocol changes.

## License

MIT — see [LICENSE](LICENSE). Derived from Anthropic's MIT-licensed [`claude-agent-sdk-typescript`](https://github.com/anthropics/claude-agent-sdk-typescript); see [NOTICE](NOTICE) for attribution.

## Disclaimer

Unofficial. Not affiliated with, authorized, or endorsed by Anthropic.
