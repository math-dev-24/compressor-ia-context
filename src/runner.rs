use std::process::Command;
use std::time::Instant;

/// Result of executing a command: raw output + metadata.
pub struct RunResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub elapsed_ms: u128,
}

impl RunResult {
    /// Merge stdout + stderr into a single string.
    pub fn combined(&self) -> String {
        let mut buf = self.stdout.clone();
        if !self.stderr.is_empty() {
            if !buf.is_empty() {
                buf.push('\n');
            }
            buf.push_str(&self.stderr);
        }
        buf
    }

    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Spawn a process, capture stdout/stderr separately, measure time.
pub fn exec(program: &str, args: &[String]) -> Result<RunResult, String> {
    let start = Instant::now();

    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("failed to run `{program}`: {e}"))?;

    let elapsed_ms = start.elapsed().as_millis();
    let exit_code = output.status.code().unwrap_or(-1);

    Ok(RunResult {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        exit_code,
        elapsed_ms,
    })
}
