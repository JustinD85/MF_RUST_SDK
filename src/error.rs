//! Error types for the SDK.

use std::process::ExitStatus;

use thiserror::Error;

/// Errors produced while spawning the `claude` CLI, reading its output, or
/// decoding the stream-json protocol.
#[derive(Debug, Error)]
pub enum Error {
    /// The `claude` executable could not be found on `PATH` (or at the
    /// configured override path).
    #[error("claude CLI not found: {0}")]
    CliNotFound(String),

    /// The `claude` process could not be spawned for a reason other than the
    /// executable being missing.
    #[error("failed to spawn claude CLI: {0}")]
    Spawn(#[source] std::io::Error),

    /// An I/O error occurred while reading the CLI's stdout stream.
    #[error("I/O error reading claude CLI output: {0}")]
    Io(#[source] std::io::Error),

    /// A line of output could not be parsed as a [`crate::Message`]. The raw
    /// offending line is captured so callers can diagnose the failure.
    #[error("failed to parse message from line: {line:?}: {source}")]
    Json {
        /// The raw JSON line that failed to deserialize.
        line: String,
        /// The underlying serde error.
        #[source]
        source: serde_json::Error,
    },

    /// The `claude` process exited with a non-zero status.
    #[error("claude CLI exited with status {status}: {stderr}")]
    CliExited {
        /// The process exit status.
        status: ExitStatus,
        /// Captured contents of the process's stderr.
        stderr: String,
    },

    /// A general, otherwise-uncategorized error.
    #[error("{0}")]
    Other(String),
}
