use crate::compress::Compressor;
use crate::compress::grep::GrepCompressor;
use crate::runner;
use super::{Tool, footer};

/// Grep tool: runs grep or ripgrep, compresses grouped output.
pub struct GrepTool {
    pattern: String,
    path: String,
    use_rg: bool,
}

impl GrepTool {
    pub fn new(pattern: String, path: String, use_rg: bool) -> Self {
        Self { pattern, path, use_rg }
    }

    fn build_args(&self) -> (&str, Vec<String>) {
        if self.use_rg {
            (
                "rg",
                vec![
                    "--no-heading".into(),
                    "-n".into(),
                    self.pattern.clone(),
                    self.path.clone(),
                ],
            )
        } else {
            (
                "grep",
                vec!["-rn".into(), self.pattern.clone(), self.path.clone()],
            )
        }
    }
}

impl Tool for GrepTool {
    fn run(&self) -> String {
        let (program, args) = self.build_args();

        match runner::exec(program, &args) {
            Ok(result) => {
                let raw = result.combined();
                let compressor = GrepCompressor;
                let compressed = compressor.compress(&raw, None);
                format!("{compressed}{}", footer("grep", &result))
            }
            Err(e) => format!("[grep] error: {e}"),
        }
    }
}
