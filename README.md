# cx

**CLI proxy that compresses shell output for AI context windows.**

`cx` wraps everyday dev commands (`git`, `cargo`, `pytest`, `docker`, ...) and returns a compact, token-efficient summary instead of raw verbose output. Built for AI coding agents (Cursor, Claude, Copilot) but useful for any workflow where you need concise terminal feedback.

## Why?

AI agents that run shell commands waste context tokens on thousands of lines of compiler noise, diff hunks, and test output. `cx` sits between the agent and the shell, compressing output by 5-20x while keeping the essential information (errors, summaries, changed files).

## Install

```bash
cargo install cx-proxy
```

Or build from source:

```bash
git clone https://github.com/user/cx-proxy
cd cx-proxy
cargo install --path .
```

## Quick start

```bash
# Instead of `git status` → use:
cx git status

# Instead of `cargo test` → use:
cx cargo test

# Instead of `pytest` → use:
cx python pytest

# List project files compactly:
cx ls

# Run anything, get truncated output:
cx run npm install
```

## Commands

| Command | Description | Smart defaults |
|---------|-------------|----------------|
| `cx git [args]` | Git proxy | `status` if no args, `log` adds `--oneline -n30`, `diff` adds `--stat`, `clean` adds `-n` |
| `cx cargo [args]` | Cargo proxy | `check` if no args, `fmt` adds `--check`, `clippy` adds `--message-format=short` |
| `cx python [args]` | Python/UV proxy | `pytest` adds `-x -q`, `ruff` adds `check`, routes `pip` through `uv` |
| `cx docker [args]` | Docker proxy | Compresses `ps`, `images`, `logs` output |
| `cx grep <pat> [path]` | Search proxy | Groups results by file, `--rg` for ripgrep |
| `cx ls [path]` | Directory tree | Skips `target/`, `node_modules/`, etc. |
| `cx run [args]` | Any command | Truncates output to 150 lines |
| `cx info` | Show config | Detected project type + active settings |
| `cx init [--global]` | Generate config | Creates `.cx.toml` or global config |

Aliases: `cx py` and `cx uv` both work for `cx python`.

## How compression works

`cx` uses a two-layer architecture:

1. **Tool layer** — builds the command with smart defaults, executes it, captures stdout/stderr/timing
2. **Compressor layer** — pure string transformation, extracts the essential info

Examples of what each compressor does:

### Git

```
# cx git status → instead of 30 lines:
branch: main
staged: src/lib.rs, tests/mod.rs
modified: README.md
untracked: notes.txt

# cx git diff → instead of 200 lines:
 src/main.rs | 12 ++++----
 src/lib.rs  |  3 +++
 2 files changed, 9 insertions(+), 6 deletions(-)
```

### Cargo

```
# cx cargo test → instead of 100 lines:
FAIL test_parse_config (src/config.rs:45)
  assertion failed: expected 10, got 0
---
23 passed, 1 failed
```

### Python

```
# cx python pytest → instead of 80 lines:
FAIL test_login (tests/test_auth.py:23)
  AssertionError: 401 != 200
---
12 passed, 1 failed
```

## Usage with AI agents

`cx` is designed to be used by AI coding agents. Below are ready-to-use rules/instructions for each platform.

### Cursor

Create `.cursor/rules/use-cx.mdc` in your project (or add as a global User Rule in Settings > Cursor Rules):

```markdown
---
description: Use cx proxy for all shell commands to save context tokens
globs:
alwaysApply: true
---

# Use `cx` for shell commands

Always use `cx` instead of raw shell commands. It compresses output to save tokens.

| Instead of           | Use                        |
|----------------------|----------------------------|
| git status           | cx git status              |
| git diff             | cx git diff                |
| git log              | cx git log                 |
| cargo build          | cx cargo build             |
| cargo test           | cx cargo test              |
| cargo clippy         | cx cargo clippy            |
| pytest               | cx python pytest           |
| ruff check .         | cx python ruff             |
| mypy src/            | cx python mypy src/        |
| pip install X        | cx python pip install X    |
| uv sync              | cx python sync             |
| docker ps            | cx docker ps               |
| ls / tree            | cx ls                      |
| grep -rn "pat" .     | cx grep "pat"              |
| rg "pat"             | cx grep "pat" --rg         |
| any other command    | cx run <command>           |

Do NOT use cx for: interactive commands, file reading (cat/head), or file editing.
Use `cx info` to check project detection. Use `cx ls` for directory structure.
```

### Claude (CLAUDE.md / system prompt)

Add to your `CLAUDE.md` at the project root:

```markdown
## Shell commands

This project uses `cx` as a shell proxy. Always run commands through `cx`:

- `cx git status` instead of `git status`
- `cx cargo test` instead of `cargo test`
- `cx python pytest` instead of `pytest`
- `cx docker ps` instead of `docker ps`
- `cx ls` instead of `ls` or `tree`
- `cx grep "pattern"` instead of `grep -rn "pattern" .`
- `cx run <cmd>` for any other command

`cx` compresses output (errors + summaries only) and adds smart defaults.
Do not use `cx` for interactive commands or file reading.
```

### Claude Code (claude-code / project instructions)

Add to your `.claude/project.md` or pass as `--system-prompt`:

```markdown
## Tool usage

Use `cx` CLI proxy for all shell commands. It compresses output to save context.

Examples:
- Run tests: `cx cargo test` or `cx python pytest`
- Check status: `cx git status`
- View diff: `cx git diff`
- Lint: `cx cargo clippy` or `cx python ruff`
- List files: `cx ls`
- Search: `cx grep "pattern" --rg`
- Anything else: `cx run <command>`

Never use raw git/cargo/pytest/docker commands directly.
```

### GitHub Copilot (workspace instructions)

Add to `.github/copilot-instructions.md`:

```markdown
## Shell proxy

Use `cx` for all shell commands. It compresses output for AI context efficiency.

Command mapping:
- git → cx git
- cargo → cx cargo
- pytest → cx python pytest
- ruff → cx python ruff
- docker → cx docker
- ls/tree → cx ls
- grep/rg → cx grep "pattern"
- other → cx run <command>

Smart defaults are applied automatically (e.g. cx git log adds --oneline -n30).
```

### Windsurf / Codeium

Add to `.windsurfrules` at the project root:

```
Use `cx` CLI proxy for all shell commands to compress output and save context tokens.
Mapping: git→cx git, cargo→cx cargo, pytest→cx python pytest, docker→cx docker, ls→cx ls, grep→cx grep.
For any unsupported command: cx run <command>.
Do not use cx for interactive commands or file reading.
```

### Aider

Add to `.aider.conf.yml` or pass as `--system-prompt-extra`:

```yaml
system-prompt-extra: |
  Use `cx` for all shell commands. It compresses output for AI context.
  git status → cx git status
  cargo test → cx cargo test
  pytest → cx python pytest
  docker ps → cx docker ps
  ls → cx ls
  grep → cx grep "pattern"
  other → cx run <command>
```

---

## Configuration

`cx` loads config with this priority:

1. **`.cx.toml`** (project root) — per-project overrides
2. **`~/.config/cx/config.toml`** — global user defaults
3. **Built-in defaults**

Generate a config file:

```bash
# Per-project
cx init

# Global
cx init --global
```

### Config options

```toml
# Truncation limits
max_lines = 150
max_line_len = 300

# Show timing footer after each command
show_footer = true

# Directories to skip in `cx ls`
ls_skip = [
    "target", "node_modules", ".git", "__pycache__",
    ".DS_Store", "dist", "build", ".next", ".cache",
    "coverage", ".venv", "venv",
]

# Tree listing limits
ls_max_depth = 4
ls_max_entries = 200
```

## Project detection

`cx info` auto-detects the project type:

| File present | Detected type |
|---|---|
| `Cargo.toml` | Rust |
| `package.json` | Node |
| `pyproject.toml` / `setup.py` / `requirements.txt` | Python |
| `go.mod` | Go |
| `Dockerfile` / `docker-compose.yml` | Docker |
| `Makefile` | Make |

## Architecture

```
src/
├── main.rs          # Entry: CLI parse → dispatch
├── cli.rs           # clap command definitions
├── config.rs        # Config loading + project detection
├── runner.rs        # Process execution (spawn, capture, time)
├── compress/
│   ├── mod.rs       # Compressor trait
│   ├── truncate.rs  # Shared truncation utilities
│   ├── git.rs       # Git output compression
│   ├── cargo.rs     # Cargo output compression
│   ├── python.rs    # Python/UV output compression
│   ├── docker.rs    # Docker output compression
│   ├── grep.rs      # Grep/rg output compression
│   └── generic.rs   # Fallback (truncate)
└── tools/
    ├── mod.rs       # Tool trait + footer helper
    ├── git.rs       # Git tool (smart defaults + exec)
    ├── cargo.rs     # Cargo tool
    ├── python.rs    # Python/UV tool
    ├── docker.rs    # Docker tool
    ├── grep.rs      # Grep/rg tool
    ├── fs.rs        # Directory listing (pure Rust)
    └── generic.rs   # Run-anything tool
```

**Adding a new command** means implementing two things:

1. A `Compressor` — pure function: `&str → String`
2. A `Tool` — builds args, calls `runner::exec`, pipes through compressor

## Development

```bash
cargo test          # 159 tests
cargo clippy        # lint
cargo build --release  # optimized binary (with LTO)
```

## License

MIT
