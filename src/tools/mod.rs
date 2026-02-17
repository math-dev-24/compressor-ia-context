pub mod git;
pub mod cargo;
pub mod grep;
pub mod python;
pub mod fs;
pub mod docker;
pub mod generic;

use crate::runner::RunResult;

/// A tool knows how to build a command and which compressor to apply.
pub trait Tool {
    /// Execute the tool and return compressed output.
    fn run(&self) -> String;
}

/// Format a one-line footer with timing and exit code.
pub fn footer(label: &str, result: &RunResult) -> String {
    let status = if result.success() { "ok" } else { "FAIL" };
    format!(
        "[{label}] {status} ({}ms, exit {})",
        result.elapsed_ms, result.exit_code
    )
}
