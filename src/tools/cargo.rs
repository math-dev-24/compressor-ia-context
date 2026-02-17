use super::{Tool, footer};
use crate::compress::Compressor;
use crate::compress::cargo::CargoCompressor;
use crate::runner;

/// Cargo tool: runs cargo sub-commands with smart defaults, compresses output.
pub struct CargoTool {
    args: Vec<String>,
}

impl CargoTool {
    pub fn new(args: Vec<String>) -> Self {
        Self { args }
    }

    /// Inject sensible defaults per sub-command.
    fn build_args(&self) -> Vec<String> {
        if self.args.is_empty() {
            return vec!["check".into()];
        }

        let sub = &self.args[0];
        let rest = &self.args[1..];
        let mut out = vec![sub.clone()];

        match sub.as_str() {
            "fmt" => {
                // Add --check by default (don't modify files without asking)
                if !rest.iter().any(|a| a == "--check") && rest.is_empty() {
                    out.push("--check".into());
                }
                out.extend(rest.iter().cloned());
            }
            "clippy" => {
                if !rest.iter().any(|a| a.starts_with("--message-format")) {
                    out.push("--message-format=short".into());
                }
                out.extend(rest.iter().cloned());
            }
            "doc" => {
                if !rest.iter().any(|a| a == "--no-deps") {
                    out.push("--no-deps".into());
                }
                out.extend(rest.iter().cloned());
            }
            _ => {
                out.extend(rest.iter().cloned());
            }
        }

        out
    }
}

impl Tool for CargoTool {
    fn run(&self) -> String {
        if self.args.is_empty() {
            return "[cargo] error: needs a subcommand (build, test, clippy, …)".into();
        }

        let sub = &self.args[0];
        let args = self.build_args();

        match runner::exec("cargo", &args) {
            Ok(result) => {
                let raw = result.combined();
                let compressor = CargoCompressor;
                let compressed = compressor.compress(&raw, Some(sub));
                format!("{compressed}{}", footer("cargo", &result))
            }
            Err(e) => format!("[cargo] error: {e}"),
        }
    }
}
