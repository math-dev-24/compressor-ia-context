//! Shared truncation utilities — the building blocks for all compressors.

const MAX_LINES: usize = 150;
const MAX_LINE_LEN: usize = 300;

/// Truncate output: cap total lines and per-line length.
pub fn truncate(raw: &str) -> String {
    truncate_with(raw, MAX_LINES, MAX_LINE_LEN)
}

/// Truncate with custom limits.
pub fn truncate_with(raw: &str, max_lines: usize, max_line_len: usize) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    let total = lines.len();
    let mut out = Vec::with_capacity(max_lines.min(total) + 1);

    for line in lines.iter().take(max_lines) {
        if line.len() > max_line_len {
            let mut s = line[..max_line_len].to_string();
            s.push_str(" …");
            out.push(s);
        } else {
            out.push(line.to_string());
        }
    }

    if total > max_lines {
        out.push(format!(
            "\n[cx] … {total} lines total, showing first {max_lines}"
        ));
    }
    out.join("\n")
}

/// Keep only lines matching a predicate, then truncate.
#[allow(dead_code)]
pub fn filter_and_truncate<F>(raw: &str, keep: F) -> String
where
    F: Fn(&str) -> bool,
{
    let filtered: String = raw
        .lines()
        .filter(|l| keep(l))
        .collect::<Vec<_>>()
        .join("\n");
    truncate(&filtered)
}

/// Deduplicate consecutive identical lines, showing counts.
pub fn dedup_lines(raw: &str) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.is_empty() {
        return String::new();
    }

    let mut out = Vec::new();
    let mut current = lines[0];
    let mut count = 1usize;

    for line in lines.iter().skip(1) {
        if *line == current {
            count += 1;
        } else {
            if count > 1 {
                out.push(format!("{current}  (×{count})"));
            } else {
                out.push(current.to_string());
            }
            current = line;
            count = 1;
        }
    }
    if count > 1 {
        out.push(format!("{current}  (×{count})"));
    } else {
        out.push(current.to_string());
    }

    truncate(&out.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short_input() {
        let raw = "line1\nline2\nline3";
        let result = truncate(raw);
        assert_eq!(result, "line1\nline2\nline3");
    }

    #[test]
    fn test_truncate_respects_max_lines() {
        let raw = (0..200).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");
        let result = truncate(&raw);
        let lines: Vec<&str> = result.lines().collect();
        // 150 lines + 1 blank + 1 footer
        assert!(lines.len() <= 153);
        assert!(result.contains("200 lines total, showing first 150"));
    }

    #[test]
    fn test_truncate_caps_long_lines() {
        let long = "x".repeat(500);
        let result = truncate(&long);
        assert!(result.contains("…"));
        assert!(result.len() < 500);
    }

    #[test]
    fn test_truncate_with_custom_limits() {
        let raw = "aaa\nbbb\nccc\nddd\neee";
        let result = truncate_with(raw, 3, 100);
        assert!(result.contains("aaa"));
        assert!(result.contains("bbb"));
        assert!(result.contains("ccc"));
        assert!(result.contains("5 lines total, showing first 3"));
        assert!(!result.contains("ddd"));
    }

    #[test]
    fn test_truncate_with_line_len_cap() {
        let raw = "short\nthis_line_is_way_too_long_for_the_cap";
        let result = truncate_with(raw, 100, 10);
        assert!(result.contains("short"));
        assert!(result.contains("this_line_ …"));
    }

    #[test]
    fn test_truncate_exact_limit_no_footer() {
        let raw = (0..150).map(|i| format!("L{i}")).collect::<Vec<_>>().join("\n");
        let result = truncate(&raw);
        assert!(!result.contains("lines total"));
    }

    #[test]
    fn test_filter_and_truncate() {
        let raw = "error: bad\ninfo: ok\nerror: worse\ninfo: fine";
        let result = filter_and_truncate(raw, |l| l.starts_with("error"));
        assert!(result.contains("error: bad"));
        assert!(result.contains("error: worse"));
        assert!(!result.contains("info:"));
    }

    #[test]
    fn test_filter_and_truncate_nothing_kept() {
        let raw = "info: ok\ninfo: fine";
        let result = filter_and_truncate(raw, |l| l.starts_with("error"));
        assert_eq!(result, "");
    }

    #[test]
    fn test_dedup_lines_no_dupes() {
        let raw = "a\nb\nc";
        let result = dedup_lines(raw);
        assert!(result.contains("a"));
        assert!(result.contains("b"));
        assert!(result.contains("c"));
        assert!(!result.contains("×"));
    }

    #[test]
    fn test_dedup_lines_consecutive_dupes() {
        let raw = "log entry\nlog entry\nlog entry\nother\nother";
        let result = dedup_lines(raw);
        assert!(result.contains("log entry  (×3)"));
        assert!(result.contains("other  (×2)"));
    }

    #[test]
    fn test_dedup_lines_single_line() {
        let raw = "only line";
        let result = dedup_lines(raw);
        assert_eq!(result, "only line");
    }

    #[test]
    fn test_dedup_lines_empty() {
        let result = dedup_lines("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_dedup_lines_non_consecutive_not_merged() {
        let raw = "a\nb\na";
        let result = dedup_lines(raw);
        assert!(!result.contains("×"));
    }
}
