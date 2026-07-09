//! The top-level [`query`] entry point: spawn `claude`, stream decoded messages.

use async_stream::stream;
use futures_core::stream::Stream;
use tokio::io::AsyncBufReadExt;

use crate::error::Error;
use crate::message::Message;
use crate::options::Options;
use crate::transport::Transport;

/// Parse one non-empty NDJSON line into a message result item.
///
/// A successfully decoded line yields `Ok(Message)`; a malformed line yields
/// `Err(Error::Json { .. })` carrying the raw line. This is the single source of
/// truth for per-line parse behavior, shared by [`query`] and
/// [`parse_message_lines`].
fn parse_line(line: String) -> Result<Message, Error> {
    match serde_json::from_str::<Message>(&line) {
        Ok(msg) => Ok(msg),
        Err(source) => Err(Error::Json { line, source }),
    }
}

/// Parse an NDJSON line source into a stream of messages. Each non-empty line
/// is parsed as a [`Message`]; a malformed line yields one `Err(Error::Json)`
/// item and parsing CONTINUES with the next line (the stream is not aborted).
///
/// This is the testable core of [`query`], independent of process spawning: any
/// `AsyncBufRead` (including an in-memory `&[u8]`) can be fed in.
// Exercised by the unit tests below; `query` drives the process path inline so
// it can interleave `transport.finish()` after EOF.
#[cfg_attr(not(test), allow(dead_code))]
fn parse_message_lines(
    reader: impl tokio::io::AsyncBufRead + Unpin,
) -> impl Stream<Item = Result<Message, Error>> {
    stream! {
        let mut lines = reader.lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    if line.trim().is_empty() {
                        continue;
                    }
                    // Malformed lines are surfaced as an item and parsing keeps
                    // going — one bad line never aborts the stream.
                    yield parse_line(line);
                }
                Ok(None) => break, // EOF
                Err(e) => {
                    // Read error: surface it and stop.
                    yield Err(Error::Io(e));
                    return;
                }
            }
        }
    }
}

/// Run a single-shot query against the `claude` CLI and stream back decoded
/// [`Message`]s.
///
/// Spawns the CLI in streaming-JSON mode (delivering the prompt over stdin),
/// reads newline-delimited JSON from its stdout, and yields one item per
/// non-empty line:
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
    stream! {
        let mut transport = match Transport::spawn(&prompt, &options).await {
            Ok(t) => t,
            Err(e) => {
                yield Err(e);
                return;
            }
        };

        loop {
            match transport.next_line().await {
                Ok(Some(line)) => {
                    if line.trim().is_empty() {
                        continue;
                    }
                    // Yield every non-empty line's parse result (including a
                    // malformed line's Err) and keep reading — one bad line
                    // never aborts the stream, so the terminal `result` message
                    // and exit-code check below still run.
                    yield parse_line(line);
                }
                Ok(None) => break, // EOF
                Err(e) => {
                    // Read error: surface it and stop.
                    yield Err(e);
                    return;
                }
            }
        }

        // Reap the child and surface a non-zero exit.
        if let Err(e) = transport.finish().await {
            yield Err(e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;

    #[tokio::test]
    async fn bad_line_does_not_terminate_stream() {
        // Three NDJSON lines: a valid system/init, a malformed line, then a
        // valid terminal result. The malformed line in the middle must NOT
        // abort the stream — the terminal result must still be delivered.
        let system = r#"{"type":"system","subtype":"init","session_id":"sess_test","model":"claude-sonnet"}"#;
        let malformed = "{not valid json";
        let result = r#"{"type":"result","subtype":"success","is_error":false,"num_turns":1,"session_id":"sess_test","result":"done"}"#;
        let buf = format!("{system}\n{malformed}\n{result}\n");

        let reader = tokio::io::BufReader::new(buf.as_bytes());
        let items: Vec<Result<Message, Error>> = parse_message_lines(reader).collect().await;

        assert_eq!(items.len(), 3, "expected exactly 3 items, got {items:?}");

        assert!(
            matches!(&items[0], Ok(Message::System { subtype, .. }) if subtype == "init"),
            "item[0] should be a System init message, got {:?}",
            items[0]
        );
        assert!(
            matches!(&items[1], Err(Error::Json { .. })),
            "item[1] should be Err(Error::Json), got {:?}",
            items[1]
        );
        assert!(
            matches!(&items[2], Ok(Message::Result { subtype, .. }) if subtype == "success"),
            "item[2] should be a Result success message, got {:?}",
            items[2]
        );
    }
}
