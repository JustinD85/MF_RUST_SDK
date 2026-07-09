//! The top-level [`query`] entry point: spawn `claude`, stream decoded messages.

use async_stream::try_stream;
use futures_core::stream::Stream;

use crate::error::Error;
use crate::message::Message;
use crate::options::Options;
use crate::transport::Transport;

/// Run a single-shot query against the `claude` CLI and stream back decoded
/// [`Message`]s.
///
/// Spawns the CLI in `--print` mode, reads newline-delimited JSON from its
/// stdout, and yields one item per non-empty line:
///
/// * a successfully decoded line yields `Ok(Message)`;
/// * a line that fails to parse yields `Err(Error::Json { .. })` carrying the
///   raw line — the stream is **not** aborted, so one malformed line does not
///   drop the rest.
///
/// The stream ends at EOF (which follows the terminal `result` message). If the
/// process exits non-zero, a final `Err(Error::CliExited { .. })` is yielded.
///
/// Errors from spawning or reading are surfaced as stream items rather than
/// panicking.
pub fn query(
    prompt: impl Into<String>,
    options: Options,
) -> impl Stream<Item = Result<Message, Error>> {
    let prompt = prompt.into();
    try_stream! {
        let mut transport = Transport::spawn(&prompt, &options)?;

        while let Some(line) = transport.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<Message>(&line) {
                Ok(msg) => yield msg,
                Err(source) => {
                    // Surface the parse failure as an item; do not drop it and
                    // do not abort the remaining stream.
                    yield Err(Error::Json { line, source })?;
                }
            }
        }

        // Reap the child and surface a non-zero exit.
        transport.finish().await?;
    }
}
