use super::Compressor;
use super::truncate::{truncate, dedup_lines};

/// Pure compressor for Python ecosystem output (pytest, ruff, pip, mypy, uv).
pub struct PythonCompressor;

impl Compressor for PythonCompressor {
    fn compress(&self, raw: &str, sub: Option<&str>) -> String {
        match sub.unwrap_or("") {
            "pytest" | "test" => compress_pytest(raw),
            "ruff" => compress_ruff(raw),
            "mypy" => compress_mypy(raw),
            "pip" | "install" | "uninstall" => compress_pip_install(raw),
            "list" | "freeze" => compress_pip_list(raw),
            "outdated" => compress_pip_outdated(raw),
            "sync" => compress_uv_sync(raw),
            "run" => truncate(raw),
            "lock" => compress_uv_lock(raw),
            "add" | "remove" => compress_uv_dep(sub.unwrap_or(""), raw),
            _ => truncate(raw),
        }
    }
}

// ── pytest ──

/// Compress pytest output: keep only failures + summary line.
fn compress_pytest(raw: &str) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    let mut out = Vec::new();
    let mut in_failure = false;
    let mut failure_lines = 0usize;

    for line in &lines {
        // Summary lines (always keep)
        if line.starts_with("=") && (line.contains("passed") || line.contains("failed") || line.contains("error")) {
            out.push(*line);
            continue;
        }
        // FAILURES header
        if line.contains("FAILURES") && line.starts_with("=") {
            in_failure = true;
            out.push(*line);
            continue;
        }
        // Individual failure header
        if line.starts_with("___") && line.ends_with("___") {
            in_failure = true;
            failure_lines = 0;
            out.push(*line);
            continue;
        }
        // Capture failure details (cap per failure)
        if in_failure {
            failure_lines += 1;
            if failure_lines <= 20 {
                out.push(*line);
            }
            // End of failure block
            if line.starts_with("=") || (line.is_empty() && failure_lines > 2) {
                in_failure = false;
            }
            continue;
        }
        // Short test summary
        if line.starts_with("FAILED ") || line.starts_with("ERROR ") {
            out.push(*line);
        }
    }

    if out.is_empty() {
        return truncate(raw);
    }

    format!("[pytest]\n{}", out.join("\n"))
}

// ── ruff ──

/// Compress ruff check / ruff format output: group by rule.
fn compress_ruff(raw: &str) -> String {
    if raw.trim().is_empty() {
        return "[ruff] clean".into();
    }

    let mut diagnostics: Vec<&str> = Vec::new();
    let mut summary: Option<&str> = None;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Found ") || trimmed.starts_with("All checks") || trimmed.starts_with("Would reformat") || trimmed.starts_with("reformatted") {
            summary = Some(trimmed);
        } else if !trimmed.is_empty() && (trimmed.contains(".py:") || trimmed.contains(".pyi:")) {
            diagnostics.push(trimmed);
        }
    }

    let mut out = String::new();
    if let Some(s) = summary {
        out.push_str(&format!("[ruff] {s}\n"));
    } else {
        out.push_str(&format!("[ruff] {} issues\n", diagnostics.len()));
    }

    for d in diagnostics.iter().take(30) {
        out.push_str(&format!("  {d}\n"));
    }
    if diagnostics.len() > 30 {
        out.push_str(&format!("  … +{} more\n", diagnostics.len() - 30));
    }

    out
}

// ── mypy ──

/// Compress mypy output: group errors, keep summary.
fn compress_mypy(raw: &str) -> String {
    let mut errors: Vec<&str> = Vec::new();
    let mut summary: Option<&str> = None;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Found ") || trimmed.starts_with("Success") {
            summary = Some(trimmed);
        } else if trimmed.contains(": error:") || trimmed.contains(": note:") {
            errors.push(trimmed);
        }
    }

    if errors.is_empty() {
        if let Some(s) = summary {
            return format!("[mypy] {s}");
        }
        return truncate(raw);
    }

    let mut out = format!("[mypy] {} errors\n", errors.len());
    for e in errors.iter().take(30) {
        out.push_str(&format!("  {e}\n"));
    }
    if errors.len() > 30 {
        out.push_str(&format!("  … +{} more\n", errors.len() - 30));
    }
    if let Some(s) = summary {
        out.push_str(&format!("{s}\n"));
    }
    out
}

// ── pip / uv pip ──

/// Compress pip install / uv pip install output.
fn compress_pip_install(raw: &str) -> String {
    let mut installed = Vec::new();
    let mut already = 0usize;
    let mut summary_line: Option<&str> = None;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Successfully installed") {
            summary_line = Some(trimmed);
        } else if trimmed.contains("already satisfied") || trimmed.contains("Already installed") {
            already += 1;
        } else if trimmed.starts_with("Installing") || trimmed.starts_with("Installed") {
            installed.push(trimmed);
        }
    }

    let mut out = String::new();
    if let Some(s) = summary_line {
        out.push_str(&format!("[pip] {s}\n"));
    } else if !installed.is_empty() {
        out.push_str(&format!("[pip] installed {}\n", installed.len()));
        for pkg in installed.iter().take(20) {
            out.push_str(&format!("  {pkg}\n"));
        }
    }
    if already > 0 {
        out.push_str(&format!("[pip] {already} already satisfied\n"));
    }

    if out.is_empty() {
        return truncate(raw);
    }
    out
}

/// Compress pip list / uv pip list / pip freeze.
fn compress_pip_list(raw: &str) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.is_empty() {
        return "[pip list] empty".into();
    }

    // Count packages (skip header lines with dashes)
    let packages: Vec<&str> = lines
        .iter()
        .filter(|l| !l.starts_with("Package") && !l.starts_with("---") && !l.trim().is_empty())
        .copied()
        .collect();

    let mut out = format!("[packages: {}]\n", packages.len());
    for p in packages.iter().take(50) {
        out.push_str(&format!("  {p}\n"));
    }
    if packages.len() > 50 {
        out.push_str(&format!("  … +{} more\n", packages.len() - 50));
    }
    out
}

/// Compress pip list --outdated output.
fn compress_pip_outdated(raw: &str) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    let packages: Vec<&str> = lines
        .iter()
        .filter(|l| !l.starts_with("Package") && !l.starts_with("---") && !l.trim().is_empty())
        .copied()
        .collect();

    if packages.is_empty() {
        return "[pip] all up to date".into();
    }

    let mut out = format!("[outdated: {}]\n", packages.len());
    for p in packages.iter().take(30) {
        out.push_str(&format!("  {p}\n"));
    }
    if packages.len() > 30 {
        out.push_str(&format!("  … +{} more\n", packages.len() - 30));
    }
    out
}

// ── uv specific ──

/// Compress `uv sync` output.
fn compress_uv_sync(raw: &str) -> String {
    let mut installed = 0usize;
    let mut uninstalled = 0usize;
    let mut resolved: Option<&str> = None;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Resolved") || trimmed.starts_with("Audited") {
            resolved = Some(trimmed);
        } else if trimmed.starts_with('+') || trimmed.starts_with("Installed") {
            installed += 1;
        } else if trimmed.starts_with('-') || trimmed.starts_with("Uninstalled") {
            uninstalled += 1;
        }
    }

    let mut out = String::from("[uv sync] ");
    if let Some(r) = resolved {
        out.push_str(&format!("{r} | "));
    }
    out.push_str(&format!("+{installed} -{uninstalled}\n"));

    if raw.lines().count() > 10 {
        out.push_str(&dedup_lines(raw));
    }
    out
}

/// Compress `uv lock` output.
fn compress_uv_lock(raw: &str) -> String {
    let resolved = raw.lines().find(|l| l.trim().starts_with("Resolved"));

    if let Some(r) = resolved {
        format!("[uv lock] {}", r.trim())
    } else {
        truncate(raw)
    }
}

/// Compress `uv add` / `uv remove` output.
fn compress_uv_dep(sub: &str, raw: &str) -> String {
    let mut changes = Vec::new();
    let mut resolved: Option<&str> = None;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Resolved") || trimmed.starts_with("Audited") {
            resolved = Some(trimmed);
        } else if trimmed.starts_with('+') || trimmed.starts_with('-') || trimmed.starts_with("Updated") {
            changes.push(trimmed);
        }
    }

    let mut out = format!("[uv {sub}] ");
    if let Some(r) = resolved {
        out.push_str(&format!("{r}\n"));
    } else {
        out.push('\n');
    }
    for c in changes.iter().take(20) {
        out.push_str(&format!("  {c}\n"));
    }
    if changes.len() > 20 {
        out.push_str(&format!("  … +{} more\n", changes.len() - 20));
    }

    if out.trim().is_empty() {
        return truncate(raw);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compress::Compressor;

    // ── pytest ──

    #[test]
    fn test_pytest_all_pass() {
        let raw = "\
============================= test session starts ==============================
collected 10 items

test_math.py ..........

============================== 10 passed in 0.03s ==============================
";
        let result = compress_pytest(raw);
        assert!(result.contains("[pytest]"));
        assert!(result.contains("10 passed"));
    }

    #[test]
    fn test_pytest_with_failures() {
        let raw = "\
============================= test session starts ==============================
collected 3 items

test_math.py .F.

=================================== FAILURES ===================================
___________________________________ test_div ___________________________________

    def test_div():
>       assert 1/0 == 0
E       ZeroDivisionError: division by zero

test_math.py:10: ZeroDivisionError
=========================== short test summary info ============================
FAILED test_math.py::test_div - ZeroDivisionError: division by zero
========================= 1 failed, 2 passed in 0.05s =========================
";
        let result = compress_pytest(raw);
        assert!(result.contains("[pytest]"));
        assert!(result.contains("FAILURES"));
        assert!(result.contains("test_div"));
        assert!(result.contains("ZeroDivisionError"));
        assert!(result.contains("1 failed, 2 passed"));
    }

    #[test]
    fn test_pytest_no_tests() {
        let raw = "no tests ran in 0.01s\n";
        let result = compress_pytest(raw);
        assert!(result.contains("no tests ran"));
    }

    // ── ruff ──

    #[test]
    fn test_ruff_clean() {
        let result = compress_ruff("");
        assert_eq!(result, "[ruff] clean");
    }

    #[test]
    fn test_ruff_with_issues() {
        let raw = "\
src/main.py:10:5: E501 Line too long (120 > 88)
src/main.py:15:1: F401 `os` imported but unused
src/utils.py:3:1: E302 Expected 2 blank lines
Found 3 fixable errors.
";
        let result = compress_ruff(raw);
        assert!(result.contains("[ruff] Found 3 fixable errors."));
        assert!(result.contains("E501"));
        assert!(result.contains("F401"));
        assert!(result.contains("E302"));
    }

    #[test]
    fn test_ruff_many_issues_truncated() {
        let mut raw = String::new();
        for i in 0..35 {
            raw.push_str(&format!("src/file{i}.py:1:1: E501 Line too long\n"));
        }
        raw.push_str("Found 35 fixable errors.\n");
        let result = compress_ruff(&raw);
        assert!(result.contains("[ruff] Found 35 fixable errors."));
        assert!(result.contains("… +5 more"));
    }

    #[test]
    fn test_ruff_format() {
        let raw = "Would reformat: 3 files\n";
        let result = compress_ruff(raw);
        assert!(result.contains("[ruff] Would reformat: 3 files"));
    }

    // ── mypy ──

    #[test]
    fn test_mypy_clean() {
        let raw = "Success: no issues found in 5 source files\n";
        let result = compress_mypy(raw);
        assert_eq!(result, "[mypy] Success: no issues found in 5 source files");
    }

    #[test]
    fn test_mypy_with_errors() {
        let raw = "\
src/main.py:10: error: Incompatible types in assignment
src/main.py:15: error: Missing return statement
src/utils.py:3: note: See class definition
Found 2 errors in 2 files (checked 5 source files)
";
        let result = compress_mypy(raw);
        assert!(result.contains("[mypy] 3 errors"));
        assert!(result.contains("Incompatible types"));
        assert!(result.contains("Missing return"));
        assert!(result.contains("Found 2 errors"));
    }

    // ── pip ──

    #[test]
    fn test_pip_install_success() {
        let raw = "\
Collecting requests
  Downloading requests-2.31.0.tar.gz
Successfully installed requests-2.31.0 urllib3-2.0.4
";
        let result = compress_pip_install(raw);
        assert!(result.contains("[pip] Successfully installed"));
    }

    #[test]
    fn test_pip_already_satisfied() {
        let raw = "\
Requirement already satisfied: requests in ./venv/lib/python3.11/site-packages (2.31.0)
Requirement already satisfied: urllib3 in ./venv/lib/python3.11/site-packages (2.0.4)
";
        let result = compress_pip_install(raw);
        assert!(result.contains("2 already satisfied"));
    }

    #[test]
    fn test_pip_list() {
        let raw = "\
Package    Version
---------- -------
requests   2.31.0
flask      3.0.0
numpy      1.25.0
";
        let result = compress_pip_list(raw);
        assert!(result.contains("[packages: 3]"));
        assert!(result.contains("requests"));
        assert!(result.contains("flask"));
    }

    #[test]
    fn test_pip_list_empty() {
        let result = compress_pip_list("");
        assert_eq!(result, "[pip list] empty");
    }

    #[test]
    fn test_pip_outdated() {
        let raw = "\
Package    Version Latest
---------- ------- ------
requests   2.28.0  2.31.0
flask      2.3.0   3.0.0
";
        let result = compress_pip_outdated(raw);
        assert!(result.contains("[outdated: 2]"));
        assert!(result.contains("requests"));
    }

    #[test]
    fn test_pip_outdated_none() {
        let raw = "\
Package    Version Latest
---------- ------- ------
";
        let result = compress_pip_outdated(raw);
        assert_eq!(result, "[pip] all up to date");
    }

    // ── uv ──

    #[test]
    fn test_uv_sync() {
        let raw = "\
Resolved 42 packages in 1.2s
+ flask==3.0.0
+ requests==2.31.0
- old-package==1.0.0
";
        let result = compress_uv_sync(raw);
        assert!(result.contains("[uv sync]"));
        assert!(result.contains("Resolved 42 packages"));
        assert!(result.contains("+2 -1"));
    }

    #[test]
    fn test_uv_lock() {
        let raw = "Resolved 42 packages in 0.5s\n";
        let result = compress_uv_lock(raw);
        assert!(result.contains("[uv lock] Resolved 42 packages"));
    }

    #[test]
    fn test_uv_add() {
        let raw = "\
Resolved 15 packages in 0.3s
+ requests==2.31.0
+ urllib3==2.0.4
";
        let result = compress_uv_dep("add", raw);
        assert!(result.contains("[uv add]"));
        assert!(result.contains("Resolved 15 packages"));
        assert!(result.contains("+ requests"));
    }

    #[test]
    fn test_uv_remove() {
        let raw = "\
Resolved 13 packages in 0.2s
- requests==2.31.0
- urllib3==2.0.4
";
        let result = compress_uv_dep("remove", raw);
        assert!(result.contains("[uv remove]"));
        assert!(result.contains("- requests"));
    }

    // ── Trait dispatch ──

    #[test]
    fn test_trait_dispatches_pytest() {
        let c = PythonCompressor;
        let raw = "============================== 1 passed in 0.01s ==============================\n";
        let result = c.compress(raw, Some("pytest"));
        assert!(result.contains("[pytest]"));
    }

    #[test]
    fn test_trait_dispatches_ruff() {
        let c = PythonCompressor;
        let result = c.compress("", Some("ruff"));
        assert!(result.contains("[ruff] clean"));
    }

    #[test]
    fn test_trait_dispatches_mypy() {
        let c = PythonCompressor;
        let raw = "Success: no issues found in 1 source file\n";
        let result = c.compress(raw, Some("mypy"));
        assert!(result.contains("[mypy] Success"));
    }

    #[test]
    fn test_trait_dispatches_pip() {
        let c = PythonCompressor;
        let raw = "Successfully installed pkg-1.0\n";
        let result = c.compress(raw, Some("pip"));
        assert!(result.contains("[pip]"));
    }

    #[test]
    fn test_trait_dispatches_list() {
        let c = PythonCompressor;
        let result = c.compress("", Some("list"));
        assert!(result.contains("[pip list]"));
    }

    #[test]
    fn test_trait_dispatches_sync() {
        let c = PythonCompressor;
        let raw = "Resolved 5 packages in 0.1s\n";
        let result = c.compress(raw, Some("sync"));
        assert!(result.contains("[uv sync]"));
    }

    #[test]
    fn test_trait_fallback() {
        let c = PythonCompressor;
        let result = c.compress("some output", None);
        assert!(result.contains("some output"));
    }
}
