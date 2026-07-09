# MF_RUST_SDK

An unofficial **Rust client for the Claude Agent SDK** — drive [Claude Code](https://github.com/anthropics/claude-code) agent sessions from Rust.

It communicates with the `claude` CLI over its streaming-JSON interface — the same approach Anthropic's official [TypeScript](https://github.com/anthropics/claude-agent-sdk-typescript) and Python SDKs take — letting you send prompts, stream responses and tool calls, gate tool use, run subagents, and resume sessions from a Rust program.

> **Status:** The core client is implemented — a streaming `query()` over the `claude` CLI (process transport plus NDJSON parsing). APIs may still change.

## How it relates to the official SDKs

This is **not** a fork, and it does **not** reimplement Claude Code's engine. Claude Code's agent loop, built-in tools, and session handling live inside the `claude` CLI. Like the official TypeScript and Python SDKs, this crate is a thin **client** that drives that CLI — here, a Rust port of the client protocol the TypeScript SDK uses.

## Requirements

- The [`claude` CLI](https://github.com/anthropics/claude-code) installed and available on your `PATH`.

## Usage

`query` spawns the `claude` CLI and returns a [`futures`](https://docs.rs/futures)
`Stream` of `Result<Message, Error>`. Build an `Options` with `Options::default()`
and its fluent setters, then consume the stream with
`futures_util::StreamExt`. The crate targets [`tokio`](https://tokio.rs).

```rust
use futures_util::StreamExt;
use mf_rust_sdk::{query, ContentBlock, Message, Options};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // permission_mode is left at its default (no override).
    let options = Options::default()
        .model("claude-sonnet")
        .allowed_tools(["Read"]);

    let mut stream = Box::pin(query("Summarize the README in one sentence", options));

    while let Some(item) = stream.next().await {
        match item? {
            // Print each text block the assistant emits.
            Message::Assistant { message, .. } => {
                for block in message.content {
                    if let ContentBlock::Text { text } = block {
                        println!("{text}");
                    }
                }
            }
            // The Result message is terminal — it ends the stream.
            Message::Result {
                subtype, is_error, ..
            } => {
                println!("done ({subtype}, is_error={is_error})");
            }
            _ => {}
        }
    }

    Ok(())
}
```

> Requires the [`claude` CLI](https://github.com/anthropics/claude-code)
> installed and on your `PATH`.

## Staying in sync with upstream

The CLI's client protocol has no formal stability guarantee, so this crate targets specific `claude` CLI and official-SDK versions, tracks upstream releases, and uses a conformance test suite (run against the installed CLI) to catch breaking protocol changes.

### Pinned versions

- **Vendored reference:** [`anthropics/claude-agent-sdk-typescript`](https://github.com/anthropics/claude-agent-sdk-typescript), pinned as a git submodule at `vendor/claude-agent-sdk-typescript`, commit `95e94bf8ba194fc956262ed77d83ce41a70d9e6d` (the upstream default-branch `main` HEAD at pin time).
- **Note:** This upstream repo currently contains documentation, examples, and CI only — it has **no** root `package.json` and **no** published SDK `src/` at this commit (the compiled SDK ships via the npm package `@anthropic-ai/claude-agent-sdk`). This port is therefore derived from the **official public `claude` CLI `stream-json` protocol** — captured from the installed CLI and from Anthropic's official Claude Code / Agent SDK documentation — rather than from TypeScript source in the vendored repo. The submodule is pinned for attribution and to track upstream docs/examples.
- **`claude` CLI version targeted:** `2.1.205 (Claude Code)` — the version installed and used for conformance testing.
- **Protocol source of truth:** the official docs at [code.claude.com](https://code.claude.com) (CLI reference, headless, agent-sdk) plus the live CLI's `--output-format stream-json` output.

## License

MIT — see [LICENSE](LICENSE). Derived from Anthropic's MIT-licensed [`claude-agent-sdk-typescript`](https://github.com/anthropics/claude-agent-sdk-typescript); see [NOTICE](NOTICE) for attribution.

## Disclaimer

Unofficial. Not affiliated with, authorized, or endorsed by Anthropic.
