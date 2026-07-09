//! Spawns the `claude` CLI and exposes its stdout as an async line reader.

use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdout, Command};

use crate::error::Error;
use crate::options::Options;

/// A live handle to a spawned `claude` process.
///
/// Holds the child so it is killed and reaped when the transport is dropped,
/// and exposes an async line reader over the process's stdout.
pub struct Transport {
    child: Child,
    reader: BufReader<ChildStdout>,
    stderr: Option<ChildStderr>,
}

impl Transport {
    /// Spawn `claude` in single-shot (`--print`) mode.
    ///
    /// Renders the CLI args from `options`, appends `--print <prompt>`, sets the
    /// working directory when configured, pipes stdout/stderr, and starts the
    /// process. A missing executable maps to [`Error::CliNotFound`]; any other
    /// spawn failure maps to [`Error::Spawn`].
    pub fn spawn(prompt: &str, options: &Options) -> Result<Self, Error> {
        let program = options
            .path_to_claude_executable
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "claude".to_string());

        let mut command = Command::new(&program);
        command.args(build_args(options, prompt));

        if let Some(cwd) = &options.cwd {
            command.current_dir(cwd);
        }

        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        command.stdin(Stdio::null());
        // Ensure the child is killed if the stream is dropped before EOF
        command.kill_on_drop(true);

        let mut child = command.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::CliNotFound(program.clone())
            } else {
                Error::Spawn(e)
            }
        })?;

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
        let status = self.child.wait().await.map_err(Error::Io)?;
        if status.success() {
            return Ok(());
        }

        let mut stderr_text = String::new();
        if let Some(mut stderr) = self.stderr.take() {
            // Best-effort drain; if reading stderr itself fails we still want to
            // surface the non-zero exit, so log and continue with what we have.
            if let Err(e) = stderr.read_to_string(&mut stderr_text).await {
                eprintln!("warning: failed to read claude stderr: {e}");
            }
        }

        Err(Error::CliExited {
            status,
            stderr: stderr_text,
        })
    }
}

/// Build the complete CLI argument vector for a single-shot invocation.
///
/// This is the rendered option args (`--output-format stream-json`, `--verbose`,
/// and any configured flags) followed by `--print <prompt>`. Kept as a pure
/// function so the exact argument assembly can be unit-tested without spawning a
/// process.
fn build_args(options: &Options, prompt: &str) -> Vec<String> {
    let mut args = options.render_args();
    args.push("--print".to_string());
    args.push(prompt.to_string());
    args
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
    fn build_args_appends_print_and_streams_verbose() {
        let prompt = "explain the transport layer";
        let args = build_args(&Options::default(), prompt);

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

        // --print present and immediately followed by the exact prompt.
        let print_idx = args
            .iter()
            .position(|a| a == "--print")
            .expect("--print present");
        assert_eq!(
            args.get(print_idx + 1).map(String::as_str),
            Some(prompt),
            "--print must be immediately followed by the exact prompt string"
        );
    }
}
