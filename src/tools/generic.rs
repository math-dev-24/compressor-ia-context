use crate::compress::Compressor;
use crate::compress::generic::GenericCompressor;
use crate::runner;
use super::{Tool, footer};

/// Generic fallback tool: execute any command, truncate output.
pub struct GenericTool {
    args: Vec<String>,
}

impl GenericTool {
    pub fn new(args: Vec<String>) -> Self {
        Self { args }
    }
}

impl Tool for GenericTool {
    fn run(&self) -> String {
        if self.args.is_empty() {
            return "[run] error: no command provided".into();
        }

        let program = &self.args[0];
        let cmd_args = &self.args[1..];
        let cmd_args_owned: Vec<String> = cmd_args.to_vec();

        match runner::exec(program, &cmd_args_owned) {
            Ok(result) => {
                let raw = result.combined();
                let compressor = GenericCompressor;
                let compressed = compressor.compress(&raw, None);
                format!("{compressed}{}", footer("run", &result))
            }
            Err(e) => format!("[run] error: {e}"),
        }
    }
}
