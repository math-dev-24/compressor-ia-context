use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "cx",
    version,
    about = "CLI proxy — compresses shell outputs for AI context (Cursor, Claude, Copilot, …)"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Git proxy: status, diff, log, branch, stash, merge, push, …
    Git {
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Cargo proxy: build, test, clippy, fmt, run, bench, doc, …
    Cargo {
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Compact directory tree listing
    Ls {
        /// Path to list (default: current directory)
        path: Option<String>,
    },

    /// Search with grep or ripgrep, grouped by file
    Grep {
        /// Pattern to search for
        pattern: String,
        /// Directory to search in (default: .)
        path: Option<String>,
        /// Use ripgrep instead of grep
        #[arg(short, long)]
        rg: bool,
    },

    /// Python / UV proxy: pytest, ruff, mypy, pip, sync, …
    #[command(alias = "py", alias = "uv")]
    Python {
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Docker / container commands
    Docker {
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Run any command and truncate output
    Run {
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Show detected project type and current config
    Info,

    /// Generate a default .cx.toml config file
    Init {
        /// Generate in ~/.config/cx/ instead of current directory
        #[arg(long)]
        global: bool,
    },
}
