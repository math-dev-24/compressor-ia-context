use crate::compress::Compressor;
use crate::compress::docker::DockerCompressor;
use crate::runner;
use super::{Tool, footer};

/// Docker tool: runs docker sub-commands, compresses output.
pub struct DockerTool {
    args: Vec<String>,
}

impl DockerTool {
    pub fn new(args: Vec<String>) -> Self {
        Self { args }
    }
}

impl Tool for DockerTool {
    fn run(&self) -> String {
        if self.args.is_empty() {
            return "[docker] error: needs a subcommand (ps, images, logs, …)".into();
        }

        let sub = &self.args[0];

        match runner::exec("docker", &self.args) {
            Ok(result) => {
                let raw = result.combined();
                let compressor = DockerCompressor;
                let compressed = compressor.compress(&raw, Some(sub));
                format!("{compressed}{}", footer("docker", &result))
            }
            Err(e) => format!("[docker] error: {e}"),
        }
    }
}
