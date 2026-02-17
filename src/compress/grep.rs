use super::Compressor;

/// Pure compressor for grep / ripgrep output.
pub struct GrepCompressor;

impl Compressor for GrepCompressor {
    fn compress(&self, raw: &str, _sub: Option<&str>) -> String {
        compress_grep(raw)
    }
}

/// Group grep results by file and truncate per-file hits.
fn compress_grep(raw: &str) -> String {
    if raw.trim().is_empty() {
        return "[grep] no matches".into();
    }

    let mut files: Vec<(&str, Vec<&str>)> = Vec::new();
    let mut total_matches = 0usize;

    for line in raw.lines() {
        total_matches += 1;
        if let Some(colon) = line.find(':') {
            let file = &line[..colon];
            let rest = &line[colon + 1..];

            if let Some(last) = files.last_mut()
                && last.0 == file
            {
                last.1.push(rest);
                continue;
            }
            files.push((file, vec![rest]));
        }
    }

    let mut out = format!("[grep] {total_matches} matches in {} files\n", files.len());
    for (file, matches) in &files {
        out.push_str(&format!("\n── {} ({} hits)\n", file, matches.len()));
        for m in matches.iter().take(10) {
            let display = if m.len() > 200 { &m[..200] } else { m };
            out.push_str(&format!("  {display}\n"));
        }
        if matches.len() > 10 {
            out.push_str(&format!("  … +{} more\n", matches.len() - 10));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compress::Compressor;

    #[test]
    fn test_empty_input() {
        assert_eq!(compress_grep(""), "[grep] no matches");
    }

    #[test]
    fn test_whitespace_only() {
        assert_eq!(compress_grep("   \n  \n"), "[grep] no matches");
    }

    #[test]
    fn test_single_match() {
        let raw = "src/main.rs:10:fn main() {";
        let result = compress_grep(raw);
        assert!(result.contains("1 matches in 1 files"));
        assert!(result.contains("── src/main.rs (1 hits)"));
        assert!(result.contains("10:fn main() {"));
    }

    #[test]
    fn test_grouping_by_file() {
        let raw = "\
src/a.rs:1:fn foo()
src/a.rs:5:fn bar()
src/b.rs:2:fn baz()
";
        let result = compress_grep(raw);
        assert!(result.contains("3 matches in 2 files"));
        assert!(result.contains("── src/a.rs (2 hits)"));
        assert!(result.contains("── src/b.rs (1 hits)"));
    }

    #[test]
    fn test_many_hits_per_file_truncated() {
        let mut raw = String::new();
        for i in 0..15 {
            raw.push_str(&format!("big_file.rs:{i}:match line {i}\n"));
        }
        let result = compress_grep(&raw);
        assert!(result.contains("15 matches in 1 files"));
        assert!(result.contains("── big_file.rs (15 hits)"));
        assert!(result.contains("… +5 more"));
    }

    #[test]
    fn test_long_match_line_truncated() {
        let long_content = "x".repeat(300);
        let raw = format!("file.rs:1:{long_content}");
        let result = compress_grep(&raw);
        assert!(result.len() < raw.len());
    }

    #[test]
    fn test_non_colon_line_counted() {
        let raw = "no colon here\nsrc/a.rs:1:match\n";
        let result = compress_grep(raw);
        assert!(result.contains("2 matches in 1 files"));
    }

    #[test]
    fn test_multiple_files_ordering() {
        let raw = "\
z.rs:1:last
a.rs:1:first
m.rs:1:middle
";
        let result = compress_grep(raw);
        assert!(result.contains("3 matches in 3 files"));
        assert!(result.contains("── z.rs"));
        assert!(result.contains("── a.rs"));
        assert!(result.contains("── m.rs"));
    }

    #[test]
    fn test_trait_compress() {
        let c = GrepCompressor;
        let raw = "f.rs:1:hello\n";
        let result = c.compress(raw, None);
        assert!(result.contains("[grep]"));
    }

    #[test]
    fn test_trait_ignores_sub() {
        let c = GrepCompressor;
        let result = c.compress("", Some("anything"));
        assert_eq!(result, "[grep] no matches");
    }
}
