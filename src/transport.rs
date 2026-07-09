//! Spawns the `claude` CLI and exposes its stdout as an async line reader.

use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdout, Command};

use crate::error::Error;
use crate::options::Options;

/// A live handle to a spawned `claude` process.
///
/// Holds the child so it is killed and reaped when the transport is dropped,
/// and exposes an async line reader over the process's stdout.
///
/// The prompt is written to the child's stdin as a single newline-delimited
/// JSON `user` message, after which stdin is closed (dropped) to signal EOF.
/// Verified against `claude` v2.1.205: a single `user` line followed by EOF
/// causes the CLI to run the turn and emit its terminal `result`, then exit —
/// no `initialize` control message or held-open stdin is required.
pub struct Transport {
    child: Child,
    reader: BufReader<ChildStdout>,
    stderr: Option<ChildStderr>,
}

impl Transport {
    /// Spawn `claude` in streaming-input mode and deliver the prompt over stdin.
    ///
    /// Mirrors the official Python SDK's subprocess transport: the CLI is run
    /// with `--output-format stream-json --verbose --input-format stream-json`
    /// (never `--print`), and the prompt is written to the child's stdin as a
    /// single newline-delimited JSON `user` message. stdin is then closed to
    /// signal EOF, which the CLI treats as end-of-input for the turn.
    ///
    /// Sets the SDK entrypoint environment variables the CLI expects, the
    /// working directory when configured, and pipes stdio. A missing executable
    /// maps to [`Error::CliNotFound`]; any other spawn failure maps to
    /// [`Error::Spawn`]; a failure writing the prompt maps to [`Error::Io`].
    pub async fn spawn(prompt: &str, options: &Options) -> Result<Self, Error> {
        let program = options
            .path_to_claude_executable
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "claude".to_string());

        let mut command = Command::new(&program);
        command.args(build_args(options));

        if let Some(cwd) = &options.cwd {
            command.current_dir(cwd);
            // The Python SDK also exports PWD alongside the spawn cwd.
            command.env("PWD", cwd);
        }

        // Entrypoint identification, matching the Python SDK's env setup. Python
        // uses "sdk-py"; this Rust client identifies itself as "sdk-rust".
        command.env("CLAUDE_CODE_ENTRYPOINT", "sdk-rust");
        command.env("CLAUDE_AGENT_SDK_VERSION", env!("CARGO_PKG_VERSION"));
        // The CLI must not think it is running inside a Claude Code session.
        command.env_remove("CLAUDECODE");

        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        command.stdin(Stdio::piped());
        // Ensure the child is killed if the stream is dropped before EOF
        command.kill_on_drop(true);

        let mut child = command.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::CliNotFound(program.clone())
            } else {
                Error::Spawn(e)
            }
        })?;

        let mut stdin = child.stdin.take().ok_or_else(|| {
            Error::Other("claude process did not expose a stdin handle".to_string())
        })?;
        // Write exactly one NDJSON `user` message and flush, then drop stdin so
        // the CLI sees EOF and knows the turn's input is complete.
        let line = build_user_message_line(prompt);
        stdin.write_all(line.as_bytes()).await.map_err(Error::Io)?;
        stdin.write_all(b"\n").await.map_err(Error::Io)?;
        stdin.flush().await.map_err(Error::Io)?;
        drop(stdin);

        let stdout = child.stdout.take().ok_or_else(|| {
            Error::Other("claude process did not expose a stdout handle".to_string())
        })?;
        let stderr = child.stderr.take();

        Ok(Self {
            child,
            reader: BufReader::new(stdout),
            stderr,
        })
    }

    /// Read the next line of stdout, or `None` at end of stream.
    ///
    /// I/O failures map to [`Error::Io`].
    pub async fn next_line(&mut self) -> Result<Option<String>, Error> {
        self.reader.read_line_owned().await
    }

    /// Await the child's exit. On a non-zero status, drain stderr and return
    /// [`Error::CliExited`].
    pub async fn finish(&mut self) -> Result<(), Error> {
        // Drain stderr CONCURRENTLY with awaiting exit so the child never blocks
        // on a full stderr pipe (which would deadlock against wait()). stderr is
        // taken out first so its borrow is a local, splitting it cleanly from the
        // borrow of self.child. Draining happens on BOTH the success and error
        // paths, so the pipe is never left to fill.
        let mut stderr = self.stderr.take();
        let mut stderr_text = String::new();
        let drain = async {
            if let Some(s) = stderr.as_mut() {
                // Best-effort drain; if reading stderr itself fails we still want
                // to surface the exit, so log and continue with what we have.
                if let Err(e) = s.read_to_string(&mut stderr_text).await {
                    eprintln!("warning: failed to read claude stderr: {e}");
                }
            }
        };

        let (status_res, ()) = tokio::join!(self.child.wait(), drain);
        let status = status_res.map_err(Error::Io)?;
        if status.success() {
            return Ok(());
        }

        Err(Error::CliExited {
            status,
            stderr: stderr_text,
        })
    }
}

/// Build the complete CLI argument vector.
///
/// This is the rendered option args (`--output-format stream-json`, `--verbose`,
/// and any configured flags) followed by the terminal `--input-format
/// stream-json` (matching the Python SDK, which always appends it). The prompt
/// is delivered over stdin, not as an argument. Kept as a pure function so the
/// exact argument assembly can be unit-tested without spawning a process.
fn build_args(options: &Options) -> Vec<String> {
    let mut args = options.render_args();
    args.push("--input-format".to_string());
    args.push("stream-json".to_string());
    args
}

/// Serialize a prompt into the single newline-delimited JSON `user` message the
/// CLI expects on stdin in `--input-format stream-json` mode.
///
/// The prompt is embedded via `serde_json` so it is correctly JSON-escaped —
/// never hand-formatted. Mirrors the Python SDK's stdin message shape.
///
/// Serialization is infallible: the value is a fixed-shape object whose only
/// dynamic member is a `&str` (always valid JSON), so `Value::to_string` cannot
/// error and the helper returns a plain `String`.
fn build_user_message_line(prompt: &str) -> String {
    let msg = serde_json::json!({
        "type": "user",
        "message": { "role": "user", "content": prompt },
        "parent_tool_use_id": serde_json::Value::Null,
        "session_id": "default",
    });
    msg.to_string()
}

impl Drop for Transport {
    fn drop(&mut self) {
        // Ensure the child does not outlive the transport. start_kill sends the
        // kill signal without awaiting; the OS reaps the process.
        if let Err(e) = self.child.start_kill() {
            // If the child already exited, start_kill errors; that is benign.
            eprintln!("warning: failed to signal claude child on drop: {e}");
        }
    }
}

/// Extension: read a single line as an owned `String`, returning `None` at EOF.
trait ReadLineOwned {
    async fn read_line_owned(&mut self) -> Result<Option<String>, Error>;
}

impl ReadLineOwned for BufReader<ChildStdout> {
    async fn read_line_owned(&mut self) -> Result<Option<String>, Error> {
        let mut buf = String::new();
        let n = self.read_line(&mut buf).await.map_err(Error::Io)?;
        if n == 0 {
            Ok(None)
        } else {
            // Trim the trailing newline; NDJSON lines are one object each.
            while buf.ends_with('\n') || buf.ends_with('\r') {
                buf.pop();
            }
            Ok(Some(buf))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_args_uses_stream_json_input_not_print() {
        let args = build_args(&Options::default());

        // --output-format immediately followed by stream-json.
        let fmt_idx = args
            .iter()
            .position(|a| a == "--output-format")
            .expect("--output-format present");
        assert_eq!(
            args.get(fmt_idx + 1).map(String::as_str),
            Some("stream-json"),
            "--output-format must be immediately followed by stream-json"
        );

        // --verbose present.
        assert!(
            args.iter().any(|a| a == "--verbose"),
            "--verbose must be present"
        );

        // --input-format present and immediately followed by stream-json.
        let in_idx = args
            .iter()
            .position(|a| a == "--input-format")
            .expect("--input-format present");
        assert_eq!(
            args.get(in_idx + 1).map(String::as_str),
            Some("stream-json"),
            "--input-format must be immediately followed by stream-json"
        );

        // The prompt is delivered over stdin, so --print must NOT appear.
        assert!(
            !args.iter().any(|a| a == "--print"),
            "--print must not be present; the prompt goes over stdin"
        );

        // --input-format stream-json is the TERMINAL flag pair: the last element
        // is `stream-json` and the element before it is `--input-format`.
        assert_eq!(
            args.last().map(String::as_str),
            Some("stream-json"),
            "the last argv element must be stream-json"
        );
        assert_eq!(
            args.get(args.len() - 2).map(String::as_str),
            Some("--input-format"),
            "the element before the last must be --input-format"
        );
    }

    #[test]
    fn user_message_line_is_valid_escaped_json() {
        // A prompt containing a double-quote AND a newline must round-trip
        // exactly, proving serde_json escaping (not hand-formatting) is used.
        let prompt = "he said \"hi\"\nsecond line";
        let line = build_user_message_line(prompt);

        let v: serde_json::Value =
            serde_json::from_str(&line).expect("line is valid JSON");
        assert_eq!(v["type"], "user");
        assert_eq!(
            v["message"]["content"], prompt,
            "the prompt must round-trip through JSON escaping unchanged"
        );
        assert!(
            v["parent_tool_use_id"].is_null(),
            "parent_tool_use_id must be null"
        );
        assert_eq!(v["session_id"], "default");
    }
}
