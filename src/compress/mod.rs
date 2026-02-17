pub mod truncate;
pub mod git;
pub mod cargo;
pub mod grep;
pub mod python;
pub mod docker;
pub mod generic;

/// Pure compression trait.
/// Implementations transform raw command output into a compact form.
/// No I/O — only string-in, string-out.
pub trait Compressor {
    /// Compress raw output, optionally using the sub-command name for context.
    fn compress(&self, raw: &str, sub: Option<&str>) -> String;
}
