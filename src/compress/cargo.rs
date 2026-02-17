use super::Compressor;
use super::truncate::truncate;

/// Pure compressor for cargo command output.
pub struct CargoCompressor;

impl Compressor for CargoCompressor {
    fn compress(&self, raw: &str, sub: Option<&str>) -> String {
        match sub.unwrap_or("") {
            "test" | "nextest" => compress_test(raw),
            "build" | "check" => compress_build(raw),
            "clippy" => compress_clippy(raw),
            "fmt" => compress_fmt(raw),
            "run" => compress_run(raw),
            "bench" => compress_bench(raw),
            "doc" => compress_doc(raw),
            "add" | "remove" => compress_dep_change(sub.unwrap_or(""), raw),
            "update" => compress_update(raw),
            "install" => compress_install(raw),
            "publish" => compress_publish(raw),
            _ => truncate(raw),
        }
    }
}

/// Compress `cargo test`: keep summary + failures only.
fn compress_test(raw: &str) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    let mut out = Vec::new();
    let mut in_failure = false;

    for line in &lines {
        if line.starts_with("---- ") && line.ends_with(" ----") {
            in_failure = true;
        }
        if in_failure {
            out.push(*line);
            if line.is_empty() {
                in_failure = false;
            }
        }
        if (line.starts_with("test result:")
            || line.starts_with("failures:")
            || line.contains("FAILED")
            || line.starts_with("running "))
            && !out.contains(line)
        {
            out.push(*line);
        }
    }

    if out.is_empty() {
        return truncate(raw);
    }

    let mut result = String::from("[cargo test]\n");
    result.push_str(&out.join("\n"));
    result
}

/// Compress `cargo build`/`check`: keep errors + warnings summary.
fn compress_build(raw: &str) -> String {
    let mut errors: Vec<&str> = Vec::new();
    let mut warnings: Vec<&str> = Vec::new();
    let mut summary: Vec<&str> = Vec::new();

    for line in raw.lines() {
        if line.starts_with("error") {
            errors.push(line);
        } else if line.starts_with("warning") && !line.starts_with("warning: unused") {
            warnings.push(line);
        } else if line.contains("Finished")
            || line.contains("could not compile")
            || (line.contains("Compiling") && line.contains("v"))
        {
            summary.push(line);
        }
    }

    let mut out = String::new();

    if !summary.is_empty() {
        for s in &summary {
            out.push_str(&format!("{}\n", s.trim()));
        }
    }
    if !errors.is_empty() {
        out.push_str(&format!("[errors: {}]\n", errors.len()));
        for e in &errors {
            out.push_str(&format!("  {e}\n"));
        }
    }
    if !warnings.is_empty() {
        out.push_str(&format!("[warnings: {}]\n", warnings.len()));
        for w in warnings.iter().take(10) {
            out.push_str(&format!("  {w}\n"));
        }
        if warnings.len() > 10 {
            out.push_str(&format!("  … +{} more\n", warnings.len() - 10));
        }
    }

    if out.is_empty() {
        return truncate(raw);
    }
    out
}

/// Compress `cargo clippy`: group diagnostics.
fn compress_clippy(raw: &str) -> String {
    let mut lints: Vec<&str> = Vec::new();

    for line in raw.lines() {
        if line.starts_with("warning:") || line.starts_with("error:") {
            lints.push(line);
        }
    }

    if lints.is_empty() {
        return truncate(raw);
    }

    let mut out = format!("[clippy: {} diagnostics]\n", lints.len());
    for lint in lints.iter().take(30) {
        out.push_str(&format!("  {lint}\n"));
    }
    if lints.len() > 30 {
        out.push_str(&format!("  … +{} more\n", lints.len() - 30));
    }
    out
}

/// Compress `cargo fmt` — show reformatted files or confirm clean.
fn compress_fmt(raw: &str) -> String {
    if raw.trim().is_empty() {
        return "[cargo fmt] clean".into();
    }

    let diffs: Vec<&str> = raw
        .lines()
        .filter(|l| l.starts_with("Diff in") || l.starts_with("Would reformat"))
        .collect();

    if !diffs.is_empty() {
        let mut out = format!("[cargo fmt] {} files need formatting\n", diffs.len());
        for d in diffs.iter().take(20) {
            out.push_str(&format!("  {d}\n"));
        }
        return out;
    }

    truncate(raw)
}

/// Compress `cargo run` — keep program output, strip compilation noise.
fn compress_run(raw: &str) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    let mut out = Vec::new();

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with("Compiling")
            || trimmed.starts_with("Downloading")
            || trimmed.starts_with("Fresh")
        {
            continue;
        }
        if trimmed.starts_with("Finished") || trimmed.starts_with("Running") {
            continue;
        }
        out.push(*line);
    }

    if out.is_empty() {
        return "[cargo run] ok".into();
    }

    truncate(&out.join("\n"))
}

/// Compress `cargo bench` — keep results summary.
fn compress_bench(raw: &str) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    let mut results = Vec::new();
    let mut summary: Option<&str> = None;

    for line in &lines {
        if line.starts_with("test ") && line.contains("bench:") {
            results.push(*line);
        } else if line.starts_with("test result:") {
            summary = Some(line);
        }
    }

    if results.is_empty() {
        return truncate(raw);
    }

    let mut out = format!("[cargo bench] {} benchmarks\n", results.len());
    for r in results.iter().take(30) {
        out.push_str(&format!("  {r}\n"));
    }
    if results.len() > 30 {
        out.push_str(&format!("  … +{} more\n", results.len() - 30));
    }
    if let Some(s) = summary {
        out.push_str(&format!("{s}\n"));
    }
    out
}

/// Compress `cargo doc` — just keep summary.
fn compress_doc(raw: &str) -> String {
    let mut documenting = Vec::new();
    let mut finished: Option<&str> = None;
    let mut warnings = 0usize;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Documenting") {
            documenting.push(trimmed);
        } else if trimmed.starts_with("Finished") {
            finished = Some(trimmed);
        } else if trimmed.starts_with("warning:") {
            warnings += 1;
        }
    }

    let mut out = String::new();
    if !documenting.is_empty() {
        out.push_str(&format!("[cargo doc] {} crates\n", documenting.len()));
    }
    if warnings > 0 {
        out.push_str(&format!("[warnings: {warnings}]\n"));
    }
    if let Some(f) = finished {
        out.push_str(&format!("{f}\n"));
    }

    if out.is_empty() {
        return truncate(raw);
    }
    out
}

/// Compress `cargo add` / `cargo remove` — show dependency changes.
fn compress_dep_change(sub: &str, raw: &str) -> String {
    let meaningful: Vec<&str> = raw
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty()
                && !t.starts_with("Updating")
                && !t.starts_with("Downloading")
                && !t.starts_with("Downloaded")
        })
        .collect();

    if meaningful.is_empty() {
        return format!("[cargo {sub}] ok");
    }

    let mut out = format!("[cargo {sub}]\n");
    for line in meaningful.iter().take(10) {
        out.push_str(&format!("  {}\n", line.trim()));
    }
    out
}

/// Compress `cargo update` — show updated packages.
fn compress_update(raw: &str) -> String {
    let updates: Vec<&str> = raw
        .lines()
        .filter(|l| {
            l.trim().starts_with("Updating")
                || l.trim().starts_with("Adding")
                || l.trim().starts_with("Removing")
                || l.trim().starts_with("Locking")
        })
        .collect();

    if updates.is_empty() {
        if raw.trim().is_empty() {
            return "[cargo update] already up to date".into();
        }
        return truncate(raw);
    }

    let mut out = format!("[cargo update] {} changes\n", updates.len());
    for u in updates.iter().take(30) {
        out.push_str(&format!("  {}\n", u.trim()));
    }
    if updates.len() > 30 {
        out.push_str(&format!("  … +{} more\n", updates.len() - 30));
    }
    out
}

/// Compress `cargo install` — keep summary.
fn compress_install(raw: &str) -> String {
    let mut installed: Option<&str> = None;
    let mut summary: Option<&str> = None;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Installing")
            || trimmed.starts_with("Installed")
            || trimmed.starts_with("Replacing")
        {
            installed = Some(trimmed);
        } else if trimmed.starts_with("Finished") {
            summary = Some(trimmed);
        }
    }

    let mut out = String::from("[cargo install] ");
    if let Some(i) = installed {
        out.push_str(&format!("{i}\n"));
    } else {
        out.push_str("ok\n");
    }
    if let Some(s) = summary {
        out.push_str(&format!("{s}\n"));
    }
    out
}

/// Compress `cargo publish` — keep result.
fn compress_publish(raw: &str) -> String {
    let meaningful: Vec<&str> = raw
        .lines()
        .filter(|l| {
            let t = l.trim();
            t.starts_with("Uploading")
                || t.starts_with("Uploaded")
                || t.starts_with("Publishing")
                || t.starts_with("Published")
                || t.contains("error")
                || t.contains("warning")
        })
        .collect();

    if meaningful.is_empty() {
        return truncate(raw);
    }

    let mut out = String::from("[cargo publish]\n");
    for line in &meaningful {
        out.push_str(&format!("  {}\n", line.trim()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compress::Compressor;

    // ── compress_test ──

    #[test]
    fn test_all_pass() {
        let raw = "\
running 5 tests
test a ... ok
test b ... ok
test c ... ok
test d ... ok
test e ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
";
        let result = compress_test(raw);
        assert!(result.contains("[cargo test]"));
        assert!(result.contains("running 5 tests"));
        assert!(result.contains("test result: ok."));
        assert!(!result.contains("test a ... ok"));
    }

    #[test]
    fn test_with_failure_block() {
        let raw = "\
running 2 tests
test ok_test ... ok
test bad_test ... FAILED

failures:

---- bad_test stdout ----
thread 'bad_test' panicked at 'assertion failed: false'

failures:
    bad_test

test result: FAILED. 1 passed; 1 failed; 0 ignored
";
        let result = compress_test(raw);
        assert!(result.contains("[cargo test]"));
        assert!(result.contains("FAILED"));
        assert!(result.contains("---- bad_test stdout ----"));
        assert!(result.contains("assertion failed"));
    }

    #[test]
    fn test_no_recognizable_output_fallback() {
        let raw = "some unrelated output\nno test keywords here\n";
        let result = compress_test(raw);
        assert!(result.contains("some unrelated output"));
    }

    // ── compress_build ──

    #[test]
    fn test_build_success() {
        let raw = "   Compiling my-crate v0.1.0\n    Finished `dev` profile in 1.2s\n";
        let result = compress_build(raw);
        assert!(result.contains("Compiling my-crate v0.1.0"));
        assert!(result.contains("Finished"));
    }

    #[test]
    fn test_build_with_errors() {
        let raw = "\
   Compiling my-crate v0.1.0
error[E0308]: mismatched types
  --> src/main.rs:5:10
error: could not compile `my-crate`
";
        let result = compress_build(raw);
        assert!(result.contains("[errors: 2]"));
        assert!(result.contains("error[E0308]"));
        assert!(result.contains("could not compile"));
    }

    #[test]
    fn test_build_with_warnings() {
        let raw = "\
   Compiling my-crate v0.1.0
warning: variable `x` is never used
warning: function `foo` is never used
    Finished `dev` profile in 0.5s
";
        let result = compress_build(raw);
        assert!(result.contains("[warnings: 2]"));
        assert!(result.contains("Finished"));
    }

    #[test]
    fn test_build_filters_unused_warnings() {
        let raw = "\
warning: unused import: `std::io`
warning: real problem here
    Finished `dev` profile in 0.5s
";
        let result = compress_build(raw);
        assert!(result.contains("[warnings: 1]"));
        assert!(result.contains("real problem"));
        assert!(!result.contains("unused import"));
    }

    #[test]
    fn test_build_many_warnings_truncated() {
        let mut raw = String::from("   Compiling my-crate v0.1.0\n");
        for i in 0..15 {
            raw.push_str(&format!("warning: lint {i}\n"));
        }
        raw.push_str("    Finished `dev` profile in 1.0s\n");
        let result = compress_build(&raw);
        assert!(result.contains("[warnings: 15]"));
        assert!(result.contains("… +5 more"));
    }

    #[test]
    fn test_build_empty_fallback() {
        let raw = "nothing recognizable here";
        let result = compress_build(raw);
        assert!(result.contains("nothing recognizable here"));
    }

    // ── compress_clippy ──

    #[test]
    fn test_clippy_with_lints() {
        let raw = "\
warning: this could be simplified
  --> src/main.rs:10:5
warning: redundant clone
  --> src/lib.rs:20:10
error: unused must_use
  --> src/utils.rs:3:1
";
        let result = compress_clippy(raw);
        assert!(result.contains("[clippy: 3 diagnostics]"));
        assert!(result.contains("warning: this could be simplified"));
        assert!(result.contains("warning: redundant clone"));
        assert!(result.contains("error: unused must_use"));
    }

    #[test]
    fn test_clippy_clean() {
        let raw = "    Checking my-crate v0.1.0\n    Finished `dev` profile in 0.3s\n";
        let result = compress_clippy(raw);
        assert!(!result.contains("[clippy:"));
        assert!(result.contains("Checking"));
    }

    #[test]
    fn test_clippy_many_lints_truncated() {
        let mut raw = String::new();
        for i in 0..35 {
            raw.push_str(&format!("warning: lint number {i}\n"));
        }
        let result = compress_clippy(&raw);
        assert!(result.contains("[clippy: 35 diagnostics]"));
        assert!(result.contains("… +5 more"));
    }

    // ── compress_fmt ──

    #[test]
    fn test_fmt_clean() {
        let result = compress_fmt("");
        assert_eq!(result, "[cargo fmt] clean");
    }

    #[test]
    fn test_fmt_with_diffs() {
        let raw = "Diff in /src/main.rs\nDiff in /src/lib.rs\n";
        let result = compress_fmt(raw);
        assert!(result.contains("[cargo fmt] 2 files need formatting"));
    }

    // ── compress_run ──

    #[test]
    fn test_run_strips_compile_noise() {
        let raw = "\
   Compiling my-crate v0.1.0
    Finished `dev` profile in 1.0s
     Running `target/debug/my-crate`
Hello, world!
result: 42
";
        let result = compress_run(raw);
        assert!(!result.contains("Compiling"));
        assert!(!result.contains("Finished"));
        assert!(!result.contains("Running"));
        assert!(result.contains("Hello, world!"));
        assert!(result.contains("result: 42"));
    }

    #[test]
    fn test_run_empty_output() {
        let raw = "   Compiling x v0.1.0\n    Finished `dev` profile in 1s\n     Running `target/debug/x`\n";
        let result = compress_run(raw);
        assert_eq!(result, "[cargo run] ok");
    }

    // ── compress_bench ──

    #[test]
    fn test_bench_results() {
        let raw = "\
running 2 tests
test bench_add ... bench:      100 ns/iter (+/- 5)
test bench_mul ... bench:      200 ns/iter (+/- 10)

test result: ok. 0 passed; 0 failed; 0 ignored; 2 measured
";
        let result = compress_bench(raw);
        assert!(result.contains("[cargo bench] 2 benchmarks"));
        assert!(result.contains("bench_add"));
        assert!(result.contains("bench_mul"));
        assert!(result.contains("test result:"));
    }

    // ── compress_doc ──

    #[test]
    fn test_doc_success() {
        let raw = " Documenting my-crate v0.1.0\n    Finished `doc` profile in 2.0s\n";
        let result = compress_doc(raw);
        assert!(result.contains("[cargo doc] 1 crates"));
        assert!(result.contains("Finished"));
    }

    #[test]
    fn test_doc_with_warnings() {
        let raw = " Documenting my-crate v0.1.0\nwarning: missing docs\nwarning: broken link\n    Finished `doc` profile in 2.0s\n";
        let result = compress_doc(raw);
        assert!(result.contains("[warnings: 2]"));
    }

    // ── compress_dep_change ──

    #[test]
    fn test_cargo_add() {
        let raw = "    Adding serde v1.0.193 to dependencies\n      Features: +derive\n";
        let result = compress_dep_change("add", raw);
        assert!(result.contains("[cargo add]"));
        assert!(result.contains("Adding serde"));
    }

    #[test]
    fn test_cargo_remove() {
        let raw = "    Removing serde from dependencies\n";
        let result = compress_dep_change("remove", raw);
        assert!(result.contains("[cargo remove]"));
        assert!(result.contains("Removing serde"));
    }

    // ── compress_update ──

    #[test]
    fn test_update_with_changes() {
        let raw = "\
    Locking 3 packages to latest versions
    Updating serde v1.0.190 -> v1.0.193
    Updating tokio v1.33.0 -> v1.35.0
    Adding new-dep v0.1.0
";
        let result = compress_update(raw);
        assert!(result.contains("[cargo update] 4 changes"));
        assert!(result.contains("serde"));
        assert!(result.contains("tokio"));
    }

    #[test]
    fn test_update_already_up_to_date() {
        let result = compress_update("");
        assert_eq!(result, "[cargo update] already up to date");
    }

    // ── compress_install ──

    #[test]
    fn test_install_success() {
        let raw = "\
   Compiling ripgrep v14.0.0
    Finished `release` profile in 30.0s
  Installing /home/user/.cargo/bin/rg
   Installed package `ripgrep v14.0.0`
";
        let result = compress_install(raw);
        assert!(result.contains("[cargo install]"));
        assert!(result.contains("Installed package"));
    }

    // ── compress_publish ──

    #[test]
    fn test_publish_success() {
        let raw = "\
   Packaging my-crate v0.1.0
   Uploading my-crate v0.1.0
   Uploaded my-crate v0.1.0
   Published my-crate v0.1.0 at registry crates-io
";
        let result = compress_publish(raw);
        assert!(result.contains("[cargo publish]"));
        assert!(result.contains("Uploading"));
        assert!(result.contains("Published"));
    }

    // ── Compressor trait dispatch ──

    #[test]
    fn test_trait_dispatches_test() {
        let c = CargoCompressor;
        let raw = "running 1 tests\ntest a ... ok\n\ntest result: ok. 1 passed; 0 failed\n";
        let result = c.compress(raw, Some("test"));
        assert!(result.contains("[cargo test]"));
    }

    #[test]
    fn test_trait_dispatches_nextest() {
        let c = CargoCompressor;
        let raw = "running 1 tests\ntest a ... ok\n\ntest result: ok. 1 passed; 0 failed\n";
        let result = c.compress(raw, Some("nextest"));
        assert!(result.contains("[cargo test]"));
    }

    #[test]
    fn test_trait_dispatches_build() {
        let c = CargoCompressor;
        let raw = "   Compiling x v0.1.0\n    Finished `dev` profile in 1s\n";
        let result = c.compress(raw, Some("build"));
        assert!(result.contains("Compiling"));
    }

    #[test]
    fn test_trait_dispatches_clippy() {
        let c = CargoCompressor;
        let raw = "warning: something\n";
        let result = c.compress(raw, Some("clippy"));
        assert!(result.contains("[clippy: 1 diagnostics]"));
    }

    #[test]
    fn test_trait_dispatches_fmt() {
        let c = CargoCompressor;
        let result = c.compress("", Some("fmt"));
        assert!(result.contains("[cargo fmt] clean"));
    }

    #[test]
    fn test_trait_dispatches_run() {
        let c = CargoCompressor;
        let raw = "Hello, world!\n";
        let result = c.compress(raw, Some("run"));
        assert!(result.contains("Hello, world!"));
    }

    #[test]
    fn test_trait_dispatches_bench() {
        let c = CargoCompressor;
        let raw = "test b ... bench:  100 ns/iter (+/- 5)\ntest result: ok. 0 passed; 0 failed; 0 ignored; 1 measured\n";
        let result = c.compress(raw, Some("bench"));
        assert!(result.contains("[cargo bench]"));
    }

    #[test]
    fn test_trait_dispatches_doc() {
        let c = CargoCompressor;
        let raw = " Documenting x v0.1.0\n    Finished `doc` profile in 1s\n";
        let result = c.compress(raw, Some("doc"));
        assert!(result.contains("[cargo doc]"));
    }

    #[test]
    fn test_trait_dispatches_add() {
        let c = CargoCompressor;
        let raw = "    Adding serde v1.0 to dependencies\n";
        let result = c.compress(raw, Some("add"));
        assert!(result.contains("[cargo add]"));
    }

    #[test]
    fn test_trait_dispatches_update() {
        let c = CargoCompressor;
        let result = c.compress("", Some("update"));
        assert!(result.contains("[cargo update]"));
    }

    #[test]
    fn test_trait_dispatches_install() {
        let c = CargoCompressor;
        let raw = "  Installing /home/user/.cargo/bin/x\n";
        let result = c.compress(raw, Some("install"));
        assert!(result.contains("[cargo install]"));
    }

    #[test]
    fn test_trait_dispatches_publish() {
        let c = CargoCompressor;
        let raw = "   Uploading x v0.1.0\n";
        let result = c.compress(raw, Some("publish"));
        assert!(result.contains("[cargo publish]"));
    }

    #[test]
    fn test_trait_fallback() {
        let c = CargoCompressor;
        let raw = "some random output";
        let result = c.compress(raw, Some("tree"));
        assert!(result.contains("some random output"));
    }

    #[test]
    fn test_trait_none_sub() {
        let c = CargoCompressor;
        let raw = "fallback output";
        let result = c.compress(raw, None);
        assert!(result.contains("fallback output"));
    }
}
