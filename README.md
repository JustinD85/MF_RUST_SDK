# MF_RUST_SDK

An unofficial **Rust client for the Claude Agent SDK** — drive [Claude Code](https://github.com/anthropics/claude-code) agent sessions from Rust.

It communicates with the `claude` CLI over its streaming-JSON interface — the same approach Anthropic's official [TypeScript](https://github.com/anthropics/claude-agent-sdk-typescript) and Python SDKs take — letting you send prompts, stream responses and tool calls, gate tool use, run subagents, and resume sessions from a Rust program.

> **Status:** The core client is implemented — a streaming `query()` over the `claude` CLI (process transport plus NDJSON parsing). APIs may still change.

## How it relates to the official SDKs

This is **not** a fork, and it does **not** reimplement Claude Code's engine. Claude Code's agent loop, built-in tools, and session handling live inside the `claude` CLI. Like the official TypeScript and Python SDKs, this crate is a thin **client** that drives that CLI — here, a Rust port of the client protocol the official **Python SDK** (`claude-agent-sdk`) implements.

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

- **Reference SDK (source of truth):** [`anthropics/claude-agent-sdk-python`](https://github.com/anthropics/claude-agent-sdk-python), vendored as a pinned git submodule at `vendor/claude-agent-sdk-python` — package `claude-agent-sdk` version **0.2.114**, pinned at commit **`fdee0adc99f46e65ae9d6d029a6f4fb31bb8cffa`** (the upstream default-branch HEAD at pin time; tag `v0.2.114`). This crate's transport, message/block types, options, and query loop are ported from this SDK's actual implementation (`src/claude_agent_sdk/`).
- **Secondary reference (docs/examples only):** [`anthropics/claude-agent-sdk-typescript`](https://github.com/anthropics/claude-agent-sdk-typescript), pinned as a submodule at `vendor/claude-agent-sdk-typescript`, commit `95e94bf8ba194fc956262ed77d83ce41a70d9e6d`. This TS repo contains only docs, examples, and CI at that commit (no published SDK `src/`), so it is retained for reference and attribution, **not** as the porting source.
- **`claude` CLI version targeted:** `2.1.205 (Claude Code)` — installed and used for the live conformance test.
- **Protocol note:** the port mirrors the Python SDK's transport faithfully — it drives the CLI with `--output-format stream-json --verbose --input-format stream-json` and writes the prompt to the child's stdin as a newline-delimited JSON `user` message (it does **not** use `--print`), and sets the SDK entrypoint env vars. The interactive control protocol (initialize handshake, `canUseTool`, hooks, interrupts, session resume) is deferred to follow-up work.

## License

MIT — see [LICENSE](LICENSE). Derived from Anthropic's MIT-licensed [`claude-agent-sdk-typescript`](https://github.com/anthropics/claude-agent-sdk-typescript); see [NOTICE](NOTICE) for attribution.

## Disclaimer

Unofficial. Not affiliated with, authorized, or endorsed by Anthropic.
