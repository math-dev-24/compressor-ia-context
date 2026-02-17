use super::Tool;
use crate::config::Config;
use std::fs;
use std::path::Path;

/// FS tool: compact tree listing (no external command needed).
pub struct FsTool {
    path: String,
    max_depth: usize,
    max_entries: usize,
    skip: Vec<String>,
}

impl FsTool {
    pub fn new(path: String, cfg: &Config) -> Self {
        Self {
            path,
            max_depth: cfg.ls_max_depth,
            max_entries: cfg.ls_max_entries,
            skip: cfg.ls_skip.clone(),
        }
    }
}

impl Tool for FsTool {
    fn run(&self) -> String {
        let root = Path::new(&self.path);
        if !root.exists() {
            return format!("[ls] error: `{}` does not exist", self.path);
        }
        if root.is_file() {
            return format!("[ls] {} (file)", self.path);
        }

        let mut lines = Vec::new();
        let mut count = 0;
        let ctx = WalkCtx {
            max_depth: self.max_depth,
            max_entries: self.max_entries,
            skip: &self.skip,
        };
        walk(root, "", 0, &ctx, &mut lines, &mut count);

        if count > self.max_entries {
            lines.push(format!(
                "… {count} entries total, showing first {}",
                self.max_entries
            ));
        }

        format!("[ls] {} ({count} entries)\n{}", self.path, lines.join("\n"))
    }
}

struct WalkCtx<'a> {
    max_depth: usize,
    max_entries: usize,
    skip: &'a [String],
}

fn walk(
    dir: &Path,
    prefix: &str,
    depth: usize,
    ctx: &WalkCtx<'_>,
    out: &mut Vec<String>,
    count: &mut usize,
) {
    if depth > ctx.max_depth || *count > ctx.max_entries {
        return;
    }

    let mut entries: Vec<_> = match fs::read_dir(dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
        Err(_) => return,
    };
    entries.sort_by_key(|e| e.file_name());

    let visible: Vec<_> = entries
        .iter()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            !ctx.skip.iter().any(|s| s == &name)
        })
        .collect();
    let total = visible.len();

    for (i, entry) in visible.iter().enumerate() {
        if *count > ctx.max_entries {
            return;
        }

        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        let is_last = i == total - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let is_dir = entry.path().is_dir();
        let suffix = if is_dir { "/" } else { "" };

        out.push(format!("{prefix}{connector}{name_str}{suffix}"));
        *count += 1;

        if is_dir {
            let child_prefix = if is_last {
                format!("{prefix}    ")
            } else {
                format!("{prefix}│   ")
            };
            walk(&entry.path(), &child_prefix, depth + 1, ctx, out, count);
        }
    }
}
