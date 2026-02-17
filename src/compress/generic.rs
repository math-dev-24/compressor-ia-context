use super::Compressor;
use super::truncate::truncate;

/// Fallback compressor: just truncate.
pub struct GenericCompressor;

impl Compressor for GenericCompressor {
    fn compress(&self, raw: &str, _sub: Option<&str>) -> String {
        truncate(raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compress::Compressor;

    #[test]
    fn test_short_passthrough() {
        let c = GenericCompressor;
        let raw = "hello world\nsecond line";
        let result = c.compress(raw, None);
        assert_eq!(result, "hello world\nsecond line");
    }

    #[test]
    fn test_long_output_truncated() {
        let c = GenericCompressor;
        let raw = (0..200).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");
        let result = c.compress(&raw, None);
        assert!(result.contains("200 lines total, showing first 150"));
    }

    #[test]
    fn test_ignores_sub() {
        let c = GenericCompressor;
        let raw = "test";
        let a = c.compress(raw, None);
        let b = c.compress(raw, Some("anything"));
        assert_eq!(a, b);
    }

    #[test]
    fn test_empty_input() {
        let c = GenericCompressor;
        let result = c.compress("", None);
        assert_eq!(result, "");
    }
}
