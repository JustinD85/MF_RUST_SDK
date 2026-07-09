# MF_RUST_SDK

A Rust port of Anthropic's official [Claude Agent SDK for TypeScript](https://github.com/anthropics/claude-agent-sdk-typescript).

## What this is

A Rust client for the Claude Agent SDK. Like the official TypeScript and Python SDKs, it does **not** reimplement the agent engine — it **drives the `claude` CLI** over its stream-json subprocess protocol. This is a **port / derivative** of the official MIT-licensed TypeScript SDK — **not a fork** (a fork would be a same-language copy; this is a reimplementation in Rust that mirrors the same client protocol).

## Status

**Early scaffold.** The port has not been implemented yet — this repo currently contains only the project skeleton, license, and governance. See the roadmap below.

## Tracking upstream (drift detection)

Because the CLI subprocess protocol has no formal stability contract, this crate tracks the official SDK deliberately:

1. **Pinned vendored reference** — the official TS SDK is vendored as a git submodule pinned to a known commit; `git diff` on a bump shows exactly what changed.
2. **Release watch** — notifications on new `@anthropic-ai/claude-agent-sdk` releases and `claude` CLI versions, compared against the pins.
3. **Conformance tests** — run this crate against the actually-installed `claude` CLI; behavioral drift fails loudly.

Pinned versions (recorded at first port): `@anthropic-ai/claude-agent-sdk` `<TBD>`, `claude` CLI `<TBD>`.

## Governance

This repository is **public** and contains **only** content derived from the official, MIT-licensed Anthropic Agent SDK. It contains **no** private or organization-internal data, endpoints, credentials, or proprietary logic. Anything private is kept elsewhere and is never committed here. Tests and fixtures are generic by rule.

## License

MIT — see [LICENSE](LICENSE). This project is derived from Anthropic's `claude-agent-sdk-typescript` (MIT); see [NOTICE](NOTICE) for attribution.
