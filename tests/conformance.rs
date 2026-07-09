//! End-to-end conformance test that drives the real `claude` CLI through the
//! crate's public `query` API.
//!
//! This test is gated on the `claude` CLI being resolvable on `PATH`. When the
//! CLI is absent it early-returns after printing a loud SKIP reason to stderr,
//! rather than using `#[ignore]` (which would hide the skip and is banned by
//! project rules). When the CLI is present it runs a real one-shot query and
//! asserts a terminal `result` message is observed without error.

use std::process::Command;
use std::time::Duration;

use futures_util::StreamExt;
use mf_rust_sdk::{query, Message, Options};

/// Detect whether the `claude` CLI is resolvable and runnable on `PATH`.
fn claude_available() -> bool {
    match Command::new("claude").arg("--version").output() {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

#[tokio::test]
async fn conformance_query_reaches_success_result() {
    if !claude_available() {
        eprintln!("SKIP: conformance test skipped — `claude` CLI not found on PATH");
        return;
    }

    let options = Options::default();
    let mut stream = Box::pin(query("Say the single word: ping", options));

    // Wrap consumption in a timeout so a hang fails loudly rather than blocking
    // the test runner forever.
    let consumed = tokio::time::timeout(Duration::from_secs(120), async {
        let mut messages: Vec<Message> = Vec::new();
        while let Some(item) = stream.next().await {
            match item {
                Ok(msg) => messages.push(msg),
                Err(err) => panic!("stream yielded an error item: {err}"),
            }
        }
        messages
    })
    .await
    .expect("conformance query timed out after 120s waiting for the stream to complete");

    // Find the terminal `result` message and assert it succeeded.
    let result = consumed
        .iter()
        .find_map(|msg| match msg {
            Message::Result {
                subtype, is_error, ..
            } => Some((subtype.clone(), is_error)),
            _ => None,
        })
        .expect("no terminal Message::Result observed in the stream");

    let (subtype, is_error) = result;
    assert!(
        !is_error && subtype == "success",
        "expected a successful terminal result, got subtype={subtype:?} is_error={is_error}"
    );

    eprintln!(
        "PASS: conformance test ran live against `claude` — terminal result subtype={subtype:?} is_error={is_error}, {} messages observed",
        consumed.len()
    );
}
