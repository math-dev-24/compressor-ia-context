use super::{Tool, footer};
use crate::compress::Compressor;
use crate::compress::python::PythonCompressor;
use crate::runner;

/// Python/UV tool: dispatches to the right program and compresses output.
pub struct PythonTool {
    args: Vec<String>,
}

impl PythonTool {
    pub fn new(args: Vec<String>) -> Self {
        Self { args }
    }

    /// Determine which program to run and what compress sub-key to use.
    fn build_command(&self) -> (&str, Vec<String>, &str) {
        if self.args.is_empty() {
            return ("python", vec!["--version".into()], "run");
        }

        let sub = self.args[0].as_str();
        let rest: Vec<String> = self.args[1..].to_vec();

        match sub {
            // Testing
            "pytest" | "test" => {
                let mut args = vec!["-x".into(), "-q".into()];
                // Don't add -q if user already passed verbosity flags
                if rest
                    .iter()
                    .any(|a| a == "-v" || a == "--verbose" || a == "-q")
                {
                    args = Vec::new();
                }
                args.extend(rest);
                ("pytest", args, "pytest")
            }
            // Linting
            "ruff" => {
                let mut args = vec!["check".into()];
                if !rest.is_empty() {
                    args = rest;
                }
                ("ruff", args, "ruff")
            }
            // Type checking
            "mypy" => ("mypy", rest, "mypy"),
            // Package management (uv pip)
            "pip" => {
                if let Some(pip_sub) = rest.first() {
                    let compress_key = match pip_sub.as_str() {
                        "list" | "freeze" => "list",
                        "install" | "uninstall" => "pip",
                        "outdated" => "outdated",
                        _ => "pip",
                    };
                    ("uv", [vec!["pip".into()], rest].concat(), compress_key)
                } else {
                    ("uv", vec!["pip".into(), "list".into()], "list")
                }
            }
            // uv direct commands
            "sync" => ("uv", [vec!["sync".into()], rest].concat(), "sync"),
            "lock" => ("uv", [vec!["lock".into()], rest].concat(), "lock"),
            "add" => ("uv", [vec!["add".into()], rest].concat(), "add"),
            "remove" => ("uv", [vec!["remove".into()], rest].concat(), "remove"),
            "run" => ("uv", [vec!["run".into()], rest].concat(), "run"),
            "init" => ("uv", [vec!["init".into()], rest].concat(), "run"),
            "venv" => ("uv", [vec!["venv".into()], rest].concat(), "run"),
            // Fallback: run as uv subcommand
            _ => ("uv", self.args.clone(), "run"),
        }
    }
}

impl Tool for PythonTool {
    fn run(&self) -> String {
        let (program, args, compress_key) = self.build_command();

        match runner::exec(program, &args) {
            Ok(result) => {
                let raw = result.combined();
                let compressor = PythonCompressor;
                let compressed = compressor.compress(&raw, Some(compress_key));
                format!("{compressed}{}", footer("python", &result))
            }
            Err(e) => format!("[python] error: {e}"),
        }
    }
}
