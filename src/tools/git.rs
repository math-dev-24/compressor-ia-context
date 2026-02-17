use crate::compress::Compressor;
use crate::compress::git::GitCompressor;
use crate::runner;
use super::{Tool, footer};

/// Git tool: builds git commands with smart defaults, compresses output.
pub struct GitTool {
    args: Vec<String>,
}

impl GitTool {
    pub fn new(args: Vec<String>) -> Self {
        Self { args }
    }

    /// Inject sensible defaults per sub-command.
    fn build_args(&self) -> (String, Vec<String>) {
        if self.args.is_empty() {
            return ("status".into(), vec!["status".into()]);
        }

        let sub = self.args[0].clone();
        let rest = &self.args[1..];
        let mut out = vec![sub.clone()];

        match sub.as_str() {
            "log" => {
                if !rest.iter().any(|a| a.starts_with("--format") || a.starts_with("--pretty")) {
                    out.push("--oneline".into());
                }
                if !rest.iter().any(|a| a.starts_with("-n") || a.starts_with("--max-count")) {
                    out.push("-n30".into());
                }
                out.extend(rest.iter().cloned());
            }
            "diff" => {
                if !rest.iter().any(|a| a == "--stat" || a == "--name-only" || a == "--cached") {
                    out.push("--stat".into());
                }
                out.extend(rest.iter().cloned());
            }
            "branch" => {
                if rest.is_empty() {
                    out.push("-a".into());
                }
                out.extend(rest.iter().cloned());
            }
            "stash" => {
                if rest.is_empty() {
                    // default stash subcommand = list
                    out.push("list".into());
                }
                out.extend(rest.iter().cloned());
            }
            "remote" => {
                if rest.is_empty() {
                    out.push("-v".into());
                }
                out.extend(rest.iter().cloned());
            }
            "tag" => {
                if !rest.iter().any(|a| a == "-l" || a == "--list") && rest.is_empty() {
                    out.push("-l".into());
                }
                out.extend(rest.iter().cloned());
            }
            "clean" => {
                // Default to dry-run if -f not explicitly passed
                if !rest.iter().any(|a| a == "-f" || a == "--force") {
                    out.push("-n".into());
                }
                out.extend(rest.iter().cloned());
            }
            "blame" => {
                out.extend(rest.iter().cloned());
            }
            _ => {
                out.extend(rest.iter().cloned());
            }
        }

        (sub, out)
    }
}

impl Tool for GitTool {
    fn run(&self) -> String {
        let (sub, args) = self.build_args();

        match runner::exec("git", &args) {
            Ok(result) => {
                let raw = result.combined();
                let compressor = GitCompressor;
                let compressed = compressor.compress(&raw, Some(&sub));
                format!("{compressed}{}", footer("git", &result))
            }
            Err(e) => format!("[git] error: {e}"),
        }
    }
}
