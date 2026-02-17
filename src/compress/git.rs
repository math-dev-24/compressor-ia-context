use super::Compressor;
use super::truncate::truncate;

/// Pure compressor for git command output.
pub struct GitCompressor;

impl Compressor for GitCompressor {
    fn compress(&self, raw: &str, sub: Option<&str>) -> String {
        match sub.unwrap_or("") {
            "status" => compress_status(raw),
            "diff" => compress_diff(raw),
            "log" => truncate(raw),
            "push" | "pull" | "fetch" => compress_transfer(sub.unwrap_or(""), raw),
            "add" | "commit" | "reset" | "restore" | "rm" | "mv" => {
                compress_write_op(sub.unwrap_or(""), raw)
            }
            "branch" => compress_branch(raw),
            "tag" => compress_tag(raw),
            "stash" => compress_stash(sub.unwrap_or(""), raw),
            "merge" | "rebase" | "cherry-pick" => compress_merge_like(sub.unwrap_or(""), raw),
            "checkout" | "switch" => compress_checkout(sub.unwrap_or(""), raw),
            "remote" => compress_remote(raw),
            "blame" => compress_blame(raw),
            "show" => truncate(raw),
            "clean" => compress_clean(raw),
            "clone" => compress_transfer("clone", raw),
            "init" => compress_write_op("init", raw),
            _ => truncate(raw),
        }
    }
}

/// Compress `git status` into a structured summary.
fn compress_status(raw: &str) -> String {
    let branch = raw
        .lines()
        .find(|l| l.starts_with("On branch"))
        .map(|l| l.trim_start_matches("On branch "))
        .unwrap_or("(detached)");

    let ahead_behind = raw
        .lines()
        .find(|l| l.starts_with("Your branch"))
        .unwrap_or("");

    let mut staged = Vec::new();
    let mut unstaged = Vec::new();
    let mut untracked = Vec::new();

    #[derive(PartialEq)]
    enum Section {
        None,
        Staged,
        Unstaged,
        Untracked,
    }
    let mut section = Section::None;

    for line in raw.lines() {
        if line.starts_with("Changes to be committed") {
            section = Section::Staged;
        } else if line.starts_with("Changes not staged") {
            section = Section::Unstaged;
        } else if line.starts_with("Untracked files") {
            section = Section::Untracked;
        } else if line.trim().starts_with('(') || line.is_empty() {
            continue;
        } else if line.starts_with('\t') || line.starts_with("  ") {
            let trimmed = line.trim();
            match section {
                Section::Staged => staged.push(trimmed),
                Section::Unstaged => unstaged.push(trimmed),
                Section::Untracked => untracked.push(trimmed),
                Section::None => {}
            }
        }
    }

    let mut out = format!("[branch] {branch}");
    if !ahead_behind.is_empty() && !ahead_behind.contains("up to date") {
        out.push_str(&format!(" | {}", ahead_behind.trim()));
    }
    out.push('\n');

    if !staged.is_empty() {
        out.push_str(&format!(
            "[staged {}] {}\n",
            staged.len(),
            staged.join(", ")
        ));
    }
    if !unstaged.is_empty() {
        out.push_str(&format!(
            "[modified {}] {}\n",
            unstaged.len(),
            unstaged.join(", ")
        ));
    }
    if !untracked.is_empty() {
        out.push_str(&format!(
            "[untracked {}] {}\n",
            untracked.len(),
            untracked.join(", ")
        ));
    }
    if staged.is_empty() && unstaged.is_empty() && untracked.is_empty() {
        out.push_str("[clean]\n");
    }
    out
}

/// Compress `git diff` — keep stat summary + truncate patch.
fn compress_diff(raw: &str) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    let mut stat_lines = Vec::new();
    let mut diff_files = Vec::new();
    let mut current_file: Option<String> = None;
    let mut adds = 0usize;
    let mut dels = 0usize;

    for line in &lines {
        // Stat section (from --stat)
        if line.contains('|') && (line.contains('+') || line.contains('-')) && line.len() < 120 {
            stat_lines.push(*line);
            continue;
        }
        // Diff hunks
        if line.starts_with("diff --git") {
            if let Some(f) = current_file.take() {
                diff_files.push(format!("  {f}: +{adds} -{dels}"));
            }
            current_file = line
                .split(' ')
                .next_back()
                .map(|s| s.trim_start_matches("b/").to_string());
            adds = 0;
            dels = 0;
        } else if line.starts_with('+') && !line.starts_with("+++") {
            adds += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            dels += 1;
        }
    }
    if let Some(f) = current_file {
        diff_files.push(format!("  {f}: +{adds} -{dels}"));
    }

    let mut out = String::from("[diff]\n");
    if !stat_lines.is_empty() {
        for s in &stat_lines {
            out.push_str(&format!("  {}\n", s.trim()));
        }
    } else if !diff_files.is_empty() {
        for f in &diff_files {
            out.push_str(&format!("{f}\n"));
        }
    } else {
        out.push_str("  (no changes)\n");
    }
    out
}

/// Compress push/pull/fetch — extract one-liner.
fn compress_transfer(sub: &str, raw: &str) -> String {
    let meaningful: Vec<&str> = raw
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty()
                && !t.starts_with("Enumerating")
                && !t.starts_with("Counting")
                && !t.starts_with("Compressing")
                && !t.starts_with("Writing")
                && !t.starts_with("Total")
                && !t.starts_with("Delta")
                && !t.starts_with("remote:")
                && !t.contains("100%")
        })
        .collect();

    if meaningful.is_empty() {
        format!("[git {sub}] ok")
    } else {
        format!("[git {sub}] {}", meaningful.join(" | "))
    }
}

/// Compress add/commit/reset/restore — just confirm success.
fn compress_write_op(sub: &str, raw: &str) -> String {
    let hash = raw
        .lines()
        .find(|l| l.contains('[') && l.contains(']'))
        .unwrap_or("");

    if hash.is_empty() {
        if raw.trim().is_empty() {
            format!("[git {sub}] ok")
        } else {
            let first_meaningful = raw.lines().find(|l| !l.trim().is_empty()).unwrap_or("ok");
            format!("[git {sub}] {}", first_meaningful.trim())
        }
    } else {
        format!("[git {sub}] {}", hash.trim())
    }
}

/// Compress `git branch` — list branches compactly.
fn compress_branch(raw: &str) -> String {
    let branches: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();

    if branches.is_empty() {
        return "[branches] none".into();
    }

    let current = branches
        .iter()
        .find(|l| l.starts_with('*'))
        .map(|l| l.trim_start_matches("* ").trim())
        .unwrap_or("(none)");

    let others: Vec<&str> = branches
        .iter()
        .filter(|l| !l.starts_with('*'))
        .map(|l| l.trim())
        .collect();

    let mut out = format!("[branches: {}] current: {current}\n", branches.len());
    for b in others.iter().take(30) {
        out.push_str(&format!("  {b}\n"));
    }
    if others.len() > 30 {
        out.push_str(&format!("  … +{} more\n", others.len() - 30));
    }
    out
}

/// Compress `git tag` — list tags compactly.
fn compress_tag(raw: &str) -> String {
    let tags: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();

    if tags.is_empty() {
        return "[tags] none".into();
    }

    let mut out = format!("[tags: {}]\n", tags.len());
    for t in tags.iter().take(30) {
        out.push_str(&format!("  {t}\n"));
    }
    if tags.len() > 30 {
        out.push_str(&format!("  … +{} more\n", tags.len() - 30));
    }
    out
}

/// Compress `git stash` — list/show/pop/apply.
fn compress_stash(_sub: &str, raw: &str) -> String {
    if raw.trim().is_empty() {
        return "[stash] ok".into();
    }

    let lines: Vec<&str> = raw.lines().collect();

    // stash list
    if lines.iter().any(|l| l.starts_with("stash@{")) {
        let mut out = format!("[stash: {} entries]\n", lines.len());
        for s in lines.iter().take(20) {
            out.push_str(&format!("  {s}\n"));
        }
        if lines.len() > 20 {
            out.push_str(&format!("  … +{} more\n", lines.len() - 20));
        }
        return out;
    }

    // stash push/pop/apply — confirmation
    let first = lines.first().map(|l| l.trim()).unwrap_or("ok");
    format!("[stash] {first}")
}

/// Compress merge/rebase/cherry-pick output.
fn compress_merge_like(sub: &str, raw: &str) -> String {
    if raw.trim().is_empty() {
        return format!("[git {sub}] ok");
    }

    // Check for conflicts
    let conflicts: Vec<&str> = raw
        .lines()
        .filter(|l| l.contains("CONFLICT") || l.contains("conflict"))
        .collect();

    if !conflicts.is_empty() {
        let mut out = format!("[git {sub}] CONFLICTS ({})\n", conflicts.len());
        for c in &conflicts {
            out.push_str(&format!("  {}\n", c.trim()));
        }
        return out;
    }

    // Success — extract summary
    let meaningful: Vec<&str> = raw
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with("Auto-merging") && !t.starts_with("Applying:")
        })
        .collect();

    if meaningful.is_empty() {
        format!("[git {sub}] ok")
    } else {
        let summary = meaningful
            .iter()
            .take(5)
            .copied()
            .collect::<Vec<_>>()
            .join(" | ");
        format!("[git {sub}] {summary}")
    }
}

/// Compress checkout/switch output.
fn compress_checkout(sub: &str, raw: &str) -> String {
    if raw.trim().is_empty() {
        return format!("[git {sub}] ok");
    }

    let first = raw.lines().find(|l| !l.trim().is_empty()).unwrap_or("ok");
    format!("[git {sub}] {}", first.trim())
}

/// Compress `git remote -v` output.
fn compress_remote(raw: &str) -> String {
    let remotes: Vec<&str> = raw.lines().filter(|l| l.contains("(fetch)")).collect();

    if remotes.is_empty() {
        if raw.trim().is_empty() {
            return "[remotes] none".into();
        }
        return truncate(raw);
    }

    let mut out = format!("[remotes: {}]\n", remotes.len());
    for r in &remotes {
        out.push_str(&format!("  {r}\n"));
    }
    out
}

/// Compress `git blame` output — compact, keep line refs.
fn compress_blame(raw: &str) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    let total = lines.len();

    if total == 0 {
        return "[blame] empty".into();
    }

    let mut out = format!("[blame: {total} lines]\n");
    for line in lines.iter().take(80) {
        // Shorten long blame lines
        let display = if line.len() > 120 {
            format!("{} …", &line[..120])
        } else {
            line.to_string()
        };
        out.push_str(&format!("{display}\n"));
    }
    if total > 80 {
        out.push_str(&format!("  … +{} more lines\n", total - 80));
    }
    out
}

/// Compress `git clean` output.
fn compress_clean(raw: &str) -> String {
    let removed: Vec<&str> = raw
        .lines()
        .filter(|l| l.starts_with("Removing") || l.starts_with("Would remove"))
        .collect();

    if removed.is_empty() {
        if raw.trim().is_empty() {
            return "[git clean] nothing to clean".into();
        }
        return truncate(raw);
    }

    let mut out = format!("[git clean] {} items\n", removed.len());
    for r in removed.iter().take(30) {
        out.push_str(&format!("  {r}\n"));
    }
    if removed.len() > 30 {
        out.push_str(&format!("  … +{} more\n", removed.len() - 30));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compress::Compressor;

    // ── compress_status ──

    #[test]
    fn test_status_clean() {
        let raw = "On branch main\nnothing to commit, working tree clean\n";
        let result = compress_status(raw);
        assert!(result.contains("[branch] main"));
        assert!(result.contains("[clean]"));
    }

    #[test]
    fn test_status_with_staged_files() {
        let raw = "\
On branch feature/login
Changes to be committed:
  (use \"git restore --staged <file>...\" to unstage)
\tnew file:   src/auth.rs
\tmodified:   src/main.rs

";
        let result = compress_status(raw);
        assert!(result.contains("[branch] feature/login"));
        assert!(result.contains("[staged 2]"));
        assert!(result.contains("new file:   src/auth.rs"));
        assert!(result.contains("modified:   src/main.rs"));
        assert!(!result.contains("[clean]"));
    }

    #[test]
    fn test_status_with_unstaged_files() {
        let raw = "\
On branch main
Changes not staged for commit:
  (use \"git add <file>...\" to update what will be committed)
\tmodified:   README.md
\tmodified:   Cargo.toml

";
        let result = compress_status(raw);
        assert!(result.contains("[modified 2]"));
        assert!(result.contains("README.md"));
        assert!(result.contains("Cargo.toml"));
    }

    #[test]
    fn test_status_with_untracked() {
        let raw = "\
On branch main
Untracked files:
  (use \"git add <file>...\" to include in what will be committed)
\tnew_file.rs
\ttodo.txt

";
        let result = compress_status(raw);
        assert!(result.contains("[untracked 2]"));
        assert!(result.contains("new_file.rs"));
        assert!(result.contains("todo.txt"));
    }

    #[test]
    fn test_status_mixed_sections() {
        let raw = "\
On branch dev
Your branch is ahead of 'origin/dev' by 3 commits.
Changes to be committed:
\tnew file:   src/lib.rs
Changes not staged for commit:
\tmodified:   src/main.rs
Untracked files:
\ttmp.log

";
        let result = compress_status(raw);
        assert!(result.contains("[branch] dev"));
        assert!(result.contains("ahead"));
        assert!(result.contains("[staged 1]"));
        assert!(result.contains("[modified 1]"));
        assert!(result.contains("[untracked 1]"));
    }

    #[test]
    fn test_status_up_to_date_hides_ahead_behind() {
        let raw = "\
On branch main
Your branch is up to date with 'origin/main'.
nothing to commit, working tree clean
";
        let result = compress_status(raw);
        assert!(result.contains("[branch] main"));
        assert!(!result.contains("up to date"));
        assert!(result.contains("[clean]"));
    }

    #[test]
    fn test_status_detached_head() {
        let raw = "HEAD detached at abc1234\nnothing to commit\n";
        let result = compress_status(raw);
        assert!(result.contains("[branch] (detached)"));
    }

    // ── compress_diff ──

    #[test]
    fn test_diff_with_stat_lines() {
        let raw = "\
 src/main.rs | 10 ++++------
 src/lib.rs  |  3 +++
 2 files changed, 7 insertions(+), 6 deletions(-)
";
        let result = compress_diff(raw);
        assert!(result.contains("[diff]"));
        assert!(result.contains("src/main.rs"));
        assert!(result.contains("src/lib.rs"));
    }

    #[test]
    fn test_diff_with_hunks_no_stat() {
        let raw = "\
diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
+use std::io;
 fn main() {
-    println!(\"old\");
+    println!(\"new\");
 }
";
        let result = compress_diff(raw);
        assert!(result.contains("[diff]"));
        assert!(result.contains("src/main.rs: +2 -1"));
    }

    #[test]
    fn test_diff_multiple_files() {
        let raw = "\
diff --git a/a.rs b/a.rs
+line1
+line2
diff --git a/b.rs b/b.rs
-old
+new
";
        let result = compress_diff(raw);
        assert!(result.contains("a.rs: +2 -0"));
        assert!(result.contains("b.rs: +1 -1"));
    }

    #[test]
    fn test_diff_empty() {
        let result = compress_diff("");
        assert!(result.contains("[diff]"));
        assert!(result.contains("(no changes)"));
    }

    // ── compress_transfer ──

    #[test]
    fn test_push_filters_noise() {
        let raw = "\
Enumerating objects: 5, done.
Counting objects: 100% (5/5), done.
Delta compression using up to 8 threads
Compressing objects: 100% (3/3), done.
Writing objects: 100% (3/3), 330 bytes | 330.00 KiB/s, done.
Total 3 (delta 2), reused 0 (delta 0)
remote: Resolving deltas: 100% (2/2), completed with 2 local objects.
To github.com:user/repo.git
   abc1234..def5678  main -> main
";
        let result = compress_transfer("push", raw);
        assert!(result.starts_with("[git push]"));
        assert!(result.contains("main -> main"));
        assert!(!result.contains("Enumerating"));
        assert!(!result.contains("Counting"));
        assert!(!result.contains("Compressing"));
    }

    #[test]
    fn test_pull_compress() {
        let raw = "\
remote: Enumerating objects: 3, done.
remote: Counting objects: 100% (3/3), done.
Already up to date.
";
        let result = compress_transfer("pull", raw);
        assert!(result.starts_with("[git pull]"));
        assert!(result.contains("Already up to date."));
    }

    #[test]
    fn test_fetch_all_noise() {
        let raw = "\
remote: Enumerating objects: 5, done.
remote: Counting objects: 100% (5/5), done.
remote: Total 3 (delta 2), reused 3 (delta 2)
";
        let result = compress_transfer("fetch", raw);
        assert_eq!(result, "[git fetch] ok");
    }

    // ── compress_write_op ──

    #[test]
    fn test_commit_with_hash() {
        let raw =
            "[main abc1234] Fix bug in parser\n 1 file changed, 2 insertions(+), 1 deletion(-)\n";
        let result = compress_write_op("commit", raw);
        assert!(result.starts_with("[git commit]"));
        assert!(result.contains("[main abc1234] Fix bug in parser"));
    }

    #[test]
    fn test_commit_no_hash() {
        let raw = "nothing to commit\n";
        let result = compress_write_op("commit", raw);
        assert_eq!(result, "[git commit] nothing to commit");
    }

    #[test]
    fn test_add_ok() {
        let raw = "";
        let result = compress_write_op("add", raw);
        assert_eq!(result, "[git add] ok");
    }

    // ── compress_branch ──

    #[test]
    fn test_branch_list() {
        let raw = "  dev\n* main\n  feature/login\n";
        let result = compress_branch(raw);
        assert!(result.contains("[branches: 3]"));
        assert!(result.contains("current: main"));
        assert!(result.contains("dev"));
        assert!(result.contains("feature/login"));
    }

    #[test]
    fn test_branch_empty() {
        let result = compress_branch("");
        assert_eq!(result, "[branches] none");
    }

    // ── compress_tag ──

    #[test]
    fn test_tag_list() {
        let raw = "v0.1.0\nv0.2.0\nv1.0.0\n";
        let result = compress_tag(raw);
        assert!(result.contains("[tags: 3]"));
        assert!(result.contains("v0.1.0"));
        assert!(result.contains("v1.0.0"));
    }

    #[test]
    fn test_tag_empty() {
        let result = compress_tag("");
        assert_eq!(result, "[tags] none");
    }

    // ── compress_stash ──

    #[test]
    fn test_stash_list() {
        let raw =
            "stash@{0}: WIP on main: abc1234 Fix thing\nstash@{1}: WIP on dev: def5678 Other\n";
        let result = compress_stash("stash", raw);
        assert!(result.contains("[stash: 2 entries]"));
        assert!(result.contains("stash@{0}"));
    }

    #[test]
    fn test_stash_push_ok() {
        let raw = "Saved working directory and index state WIP on main: abc1234 msg\n";
        let result = compress_stash("stash", raw);
        assert!(result.contains("[stash] Saved working directory"));
    }

    #[test]
    fn test_stash_empty() {
        let result = compress_stash("stash", "");
        assert_eq!(result, "[stash] ok");
    }

    // ── compress_merge_like ──

    #[test]
    fn test_merge_success() {
        let raw = "Updating abc1234..def5678\nFast-forward\n src/main.rs | 5 +++++\n";
        let result = compress_merge_like("merge", raw);
        assert!(result.contains("[git merge]"));
        assert!(result.contains("Updating"));
    }

    #[test]
    fn test_merge_conflict() {
        let raw = "\
Auto-merging src/main.rs
CONFLICT (content): Merge conflict in src/main.rs
Automatic merge failed; fix conflicts and then commit the result.
";
        let result = compress_merge_like("merge", raw);
        assert!(result.contains("[git merge] CONFLICTS"));
        assert!(result.contains("CONFLICT (content)"));
    }

    #[test]
    fn test_rebase_success() {
        let raw = "Successfully rebased and updated refs/heads/feature.\n";
        let result = compress_merge_like("rebase", raw);
        assert!(result.contains("[git rebase]"));
        assert!(result.contains("Successfully rebased"));
    }

    #[test]
    fn test_cherry_pick_empty() {
        let result = compress_merge_like("cherry-pick", "");
        assert_eq!(result, "[git cherry-pick] ok");
    }

    // ── compress_checkout ──

    #[test]
    fn test_checkout_branch() {
        let raw = "Switched to branch 'feature'\n";
        let result = compress_checkout("checkout", raw);
        assert!(result.contains("[git checkout] Switched to branch 'feature'"));
    }

    #[test]
    fn test_switch_branch() {
        let raw = "Switched to branch 'dev'\n";
        let result = compress_checkout("switch", raw);
        assert!(result.contains("[git switch] Switched to branch 'dev'"));
    }

    #[test]
    fn test_checkout_empty() {
        let result = compress_checkout("checkout", "");
        assert_eq!(result, "[git checkout] ok");
    }

    // ── compress_remote ──

    #[test]
    fn test_remote_list() {
        let raw = "\
origin\thttps://github.com/user/repo.git (fetch)
origin\thttps://github.com/user/repo.git (push)
upstream\thttps://github.com/other/repo.git (fetch)
upstream\thttps://github.com/other/repo.git (push)
";
        let result = compress_remote(raw);
        assert!(result.contains("[remotes: 2]"));
        assert!(result.contains("origin"));
        assert!(result.contains("upstream"));
    }

    #[test]
    fn test_remote_none() {
        let result = compress_remote("");
        assert_eq!(result, "[remotes] none");
    }

    // ── compress_blame ──

    #[test]
    fn test_blame_normal() {
        let raw = "\
abc1234 (John 2024-01-01 10:00:00 +0100  1) fn main() {
def5678 (Jane 2024-01-02 11:00:00 +0100  2)     println!(\"hello\");
abc1234 (John 2024-01-01 10:00:00 +0100  3) }
";
        let result = compress_blame(raw);
        assert!(result.contains("[blame: 3 lines]"));
        assert!(result.contains("John"));
        assert!(result.contains("Jane"));
    }

    #[test]
    fn test_blame_empty() {
        let result = compress_blame("");
        assert_eq!(result, "[blame] empty");
    }

    // ── compress_clean ──

    #[test]
    fn test_clean_dry_run() {
        let raw = "Would remove untracked.txt\nWould remove tmp/\n";
        let result = compress_clean(raw);
        assert!(result.contains("[git clean] 2 items"));
        assert!(result.contains("Would remove untracked.txt"));
    }

    #[test]
    fn test_clean_actual() {
        let raw = "Removing untracked.txt\nRemoving tmp/\n";
        let result = compress_clean(raw);
        assert!(result.contains("[git clean] 2 items"));
    }

    #[test]
    fn test_clean_nothing() {
        let result = compress_clean("");
        assert_eq!(result, "[git clean] nothing to clean");
    }

    // ── Compressor trait dispatch ──

    #[test]
    fn test_trait_dispatches_status() {
        let c = GitCompressor;
        let raw = "On branch test\nnothing to commit, working tree clean\n";
        let result = c.compress(raw, Some("status"));
        assert!(result.contains("[branch] test"));
    }

    #[test]
    fn test_trait_dispatches_diff() {
        let c = GitCompressor;
        let result = c.compress("", Some("diff"));
        assert!(result.contains("[diff]"));
    }

    #[test]
    fn test_trait_dispatches_log() {
        let c = GitCompressor;
        let raw = "abc1234 First commit\ndef5678 Second commit\n";
        let result = c.compress(raw, Some("log"));
        assert!(result.contains("abc1234"));
    }

    #[test]
    fn test_trait_dispatches_branch() {
        let c = GitCompressor;
        let result = c.compress("* main\n  dev\n", Some("branch"));
        assert!(result.contains("[branches: 2]"));
    }

    #[test]
    fn test_trait_dispatches_stash() {
        let c = GitCompressor;
        let result = c.compress("", Some("stash"));
        assert!(result.contains("[stash]"));
    }

    #[test]
    fn test_trait_dispatches_merge() {
        let c = GitCompressor;
        let result = c.compress("", Some("merge"));
        assert!(result.contains("[git merge]"));
    }

    #[test]
    fn test_trait_dispatches_rebase() {
        let c = GitCompressor;
        let result = c.compress("", Some("rebase"));
        assert!(result.contains("[git rebase]"));
    }

    #[test]
    fn test_trait_dispatches_checkout() {
        let c = GitCompressor;
        let result = c.compress("Switched to branch 'x'\n", Some("checkout"));
        assert!(result.contains("[git checkout]"));
    }

    #[test]
    fn test_trait_dispatches_tag() {
        let c = GitCompressor;
        let result = c.compress("v1.0\n", Some("tag"));
        assert!(result.contains("[tags: 1]"));
    }

    #[test]
    fn test_trait_dispatches_remote() {
        let c = GitCompressor;
        let result = c.compress("", Some("remote"));
        assert!(result.contains("[remotes]"));
    }

    #[test]
    fn test_trait_dispatches_clean() {
        let c = GitCompressor;
        let result = c.compress("", Some("clean"));
        assert!(result.contains("[git clean]"));
    }

    #[test]
    fn test_trait_dispatches_blame() {
        let c = GitCompressor;
        let result = c.compress("", Some("blame"));
        assert!(result.contains("[blame]"));
    }

    #[test]
    fn test_trait_dispatches_reset() {
        let c = GitCompressor;
        let result = c.compress(
            "Unstaged changes after reset:\nM\tsrc/main.rs\n",
            Some("reset"),
        );
        assert!(result.contains("[git reset]"));
    }

    #[test]
    fn test_trait_none_sub() {
        let c = GitCompressor;
        let result = c.compress("hello", None);
        assert!(result.contains("hello"));
    }
}
