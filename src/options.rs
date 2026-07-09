//! Configuration options for a [`crate::query`] call and rendering of those
//! options into the `claude` CLI argument vector.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The permission mode for a query.
///
/// [`PermissionMode::Default`] is the SDK-faithful default: it means "no
/// override," i.e. the CLI's normal permission behavior. The installed
/// `claude` CLI (v2.1.205) does **not** accept `--permission-mode default`, so
/// `Default` must emit no flag at all — see [`PermissionMode::as_cli_arg`].
///
/// The remaining variants map to the exact camelCase CLI string literals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionMode {
    /// No override — the CLI's normal behavior. Emits no `--permission-mode`
    /// flag.
    #[serde(rename = "default")]
    Default,
    /// `acceptEdits`
    #[serde(rename = "acceptEdits")]
    AcceptEdits,
    /// `bypassPermissions`
    #[serde(rename = "bypassPermissions")]
    BypassPermissions,
    /// `plan`
    #[serde(rename = "plan")]
    Plan,
}

impl PermissionMode {
    /// The value to pass to `--permission-mode`, or `None` when no flag should
    /// be emitted.
    ///
    /// Returns `None` for [`PermissionMode::Default`] (no override) and the
    /// exact camelCase CLI string otherwise. The arg builder pushes the flag
    /// only when this is `Some`, so `Default` never reaches the CLI as an
    /// (unsupported) `--permission-mode default`.
    pub fn as_cli_arg(self) -> Option<&'static str> {
        match self {
            PermissionMode::Default => None,
            PermissionMode::AcceptEdits => Some("acceptEdits"),
            PermissionMode::BypassPermissions => Some("bypassPermissions"),
            PermissionMode::Plan => Some("plan"),
        }
    }
}

/// Options controlling a single-shot `claude` CLI invocation.
///
/// Construct via [`Options::default`] and the fluent setters, e.g.:
///
/// ```
/// use mf_rust_sdk::{Options, PermissionMode};
/// let opts = Options::default()
///     .model("claude-sonnet")
///     .allowed_tools(["Read", "Bash"])
///     .permission_mode(PermissionMode::AcceptEdits);
/// ```
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// The model to use (`--model`).
    pub model: Option<String>,
    /// Tools to allow (`--allowedTools`, comma-joined).
    pub allowed_tools: Vec<String>,
    /// Tools to disallow (`--disallowedTools`, comma-joined).
    pub disallowed_tools: Vec<String>,
    /// MCP configuration path or inline JSON (`--mcp-config`).
    pub mcp_config: Option<PathBuf>,
    /// Working directory for the spawned child process (not a CLI flag).
    pub cwd: Option<PathBuf>,
    /// Permission mode (`--permission-mode`).
    pub permission_mode: Option<PermissionMode>,
    /// Full replacement system prompt (`--system-prompt`).
    pub system_prompt: Option<String>,
    /// Appended system prompt (`--append-system-prompt`).
    pub append_system_prompt: Option<String>,
    /// Path to the `claude` executable; defaults to resolving `claude` on
    /// `PATH` when unset.
    pub path_to_claude_executable: Option<PathBuf>,
}

impl Options {
    /// Set the model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the allowed tools.
    pub fn allowed_tools<I, S>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allowed_tools = tools.into_iter().map(Into::into).collect();
        self
    }

    /// Set the disallowed tools.
    pub fn disallowed_tools<I, S>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.disallowed_tools = tools.into_iter().map(Into::into).collect();
        self
    }

    /// Set the MCP configuration path or inline JSON.
    pub fn mcp_config(mut self, path: impl Into<PathBuf>) -> Self {
        self.mcp_config = Some(path.into());
        self
    }

    /// Set the working directory for the child process.
    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Set the permission mode.
    pub fn permission_mode(mut self, mode: PermissionMode) -> Self {
        self.permission_mode = Some(mode);
        self
    }

    /// Set the full replacement system prompt.
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set the appended system prompt.
    pub fn append_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.append_system_prompt = Some(prompt.into());
        self
    }

    /// Set the path to the `claude` executable.
    pub fn path_to_claude_executable(mut self, path: impl Into<PathBuf>) -> Self {
        self.path_to_claude_executable = Some(path.into());
        self
    }

    /// Render the CLI argument vector implied by these options.
    ///
    /// Always includes `--output-format stream-json` and `--verbose`. The
    /// prompt itself and its `--print` flag are appended by the transport, not
    /// here. `cwd` and `path_to_claude_executable` are process-spawn concerns
    /// and are intentionally not rendered as flags.
    pub fn render_args(&self) -> Vec<String> {
        let mut args: Vec<String> = vec![
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--verbose".to_string(),
        ];

        if let Some(model) = &self.model {
            args.push("--model".to_string());
            args.push(model.clone());
        }

        if !self.allowed_tools.is_empty() {
            args.push("--allowedTools".to_string());
            args.push(self.allowed_tools.join(","));
        }

        if !self.disallowed_tools.is_empty() {
            args.push("--disallowedTools".to_string());
            args.push(self.disallowed_tools.join(","));
        }

        if let Some(mcp) = &self.mcp_config {
            args.push("--mcp-config".to_string());
            args.push(mcp.to_string_lossy().into_owned());
        }

        if let Some(mode) = self.permission_mode {
            // `Default` yields `None` — no override, so no flag is emitted.
            if let Some(mode_arg) = mode.as_cli_arg() {
                args.push("--permission-mode".to_string());
                args.push(mode_arg.to_string());
            }
        }

        if let Some(prompt) = &self.append_system_prompt {
            args.push("--append-system-prompt".to_string());
            args.push(prompt.clone());
        }

        if let Some(prompt) = &self.system_prompt {
            args.push("--system-prompt".to_string());
            args.push(prompt.clone());
        }

        args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_mode_maps_to_exact_cli_args() {
        // `Default` is "no override" and emits no flag.
        assert_eq!(PermissionMode::Default.as_cli_arg(), None);
        assert_eq!(
            PermissionMode::AcceptEdits.as_cli_arg(),
            Some("acceptEdits")
        );
        assert_eq!(
            PermissionMode::BypassPermissions.as_cli_arg(),
            Some("bypassPermissions")
        );
        assert_eq!(PermissionMode::Plan.as_cli_arg(), Some("plan"));
    }

    #[test]
    fn renders_expected_cli_arg_vector() {
        let args = Options::default()
            .model("claude-sonnet")
            .allowed_tools(["foo", "bar"])
            .permission_mode(PermissionMode::AcceptEdits)
            .render_args();

        // Always-present flags.
        let joined = args.join(" ");
        assert!(joined.contains("--output-format stream-json"));
        assert!(args.iter().any(|a| a == "--verbose"));

        // --model <value>
        let model_idx = args.iter().position(|a| a == "--model").expect("--model");
        assert_eq!(args[model_idx + 1], "claude-sonnet");

        // --allowedTools comma-joined, no spaces.
        let allowed_idx = args
            .iter()
            .position(|a| a == "--allowedTools")
            .expect("--allowedTools");
        assert_eq!(args[allowed_idx + 1], "foo,bar");

        // --permission-mode immediately followed by acceptEdits.
        let mode_idx = args
            .iter()
            .position(|a| a == "--permission-mode")
            .expect("--permission-mode");
        assert_eq!(args[mode_idx + 1], "acceptEdits");

        // --max-turns must never be emitted (absent from CLI v2.1.205).
        assert!(
            !args.iter().any(|a| a == "--max-turns"),
            "--max-turns must not be emitted"
        );
    }

    #[test]
    fn default_permission_mode_emits_no_flag() {
        // Default Options: no permission mode set at all.
        assert!(
            !Options::default()
                .render_args()
                .iter()
                .any(|a| a == "--permission-mode"),
            "unset permission mode must emit no --permission-mode flag"
        );

        // Explicitly requesting `Default` must also emit no flag.
        let args = Options::default()
            .permission_mode(PermissionMode::Default)
            .render_args();
        assert!(
            !args.iter().any(|a| a == "--permission-mode"),
            "PermissionMode::Default must emit no --permission-mode flag"
        );
    }
}
