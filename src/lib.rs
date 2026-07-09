//! # MF_RUST_SDK
//!
//! An unofficial Rust client for the Claude Agent SDK.
//!
//! Like Anthropic's official TypeScript and Python SDKs, this crate does **not**
//! reimplement Claude Code's engine — it drives the `claude` CLI over its
//! streaming-JSON interface. It is a Rust port of the client protocol used by the
//! official TypeScript SDK
//! (<https://github.com/anthropics/claude-agent-sdk-typescript>).
//!
//! Unofficial; not affiliated with or endorsed by Anthropic.
//!
//! ## Example
//!
//! ```no_run
//! use futures_util::StreamExt;
//! use mf_rust_sdk::{query, ContentBlock, Message, Options, PermissionMode};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let options = Options::default()
//!         .model("claude-sonnet")
//!         .allowed_tools(["Read", "Bash"])
//!         .permission_mode(PermissionMode::AcceptEdits);
//!
//!     let mut stream = Box::pin(query("Summarize the README.", options));
//!
//!     while let Some(item) = stream.next().await {
//!         match item? {
//!             Message::Assistant { message, .. } => {
//!                 for block in message.content {
//!                     if let ContentBlock::Text { text } = block {
//!                         println!("{text}");
//!                     }
//!                 }
//!             }
//!             Message::Result { result, .. } => {
//!                 if let Some(text) = result {
//!                     println!("final: {text}");
//!                 }
//!             }
//!             _ => {}
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```

mod error;
mod message;
mod options;
mod query;
mod transport;

pub use error::Error;
pub use message::{AnthropicMessage, ContentBlock, Message, MessageParam};
pub use options::{Options, PermissionMode};
pub use query::query;
