# CLAUDE.md — Instructions for AI agents

This file provides context for Claude, Cursor, Copilot, and other AI coding agents working on this project.

## What is cx?

`cx` is a CLI proxy that compresses shell command output for AI context windows. It wraps common dev tools (git, cargo, pytest, docker, grep) and returns compact summaries instead of verbose raw output.

## Build & test

```bash
cargo build          # compile
cargo test           # run all 159+ unit tests
cargo clippy         # lint check
cargo run -- info    # check project detection + config
cargo run -- git status  # test a proxied command
```

## Architecture — two layers

The codebase separates **compression** (pure string transforms) from **tool execution** (I/O, process spawning):

### Compression layer (`src/compress/`)

- **Trait**: `Compressor` with `fn compress(&self, raw: &str, sub: Option<&str>) -> String`
- **No I/O** — pure functions, easy to test
- Each compressor handles one domain: `GitCompressor`, `CargoCompressor`, `PythonCompressor`, `DockerCompressor`, `GrepCompressor`, `GenericCompressor`
- Shared utilities in `truncate.rs`: `truncate()`, `truncate_with()`, `dedup_lines()`

### Tool layer (`src/tools/`)

- **Trait**: `Tool` with `fn run(&self) -> String`
- Each tool: builds command args (with smart defaults) → calls `runner::exec()` → pipes output through its compressor → appends footer
- `runner::exec()` returns `RunResult { stdout, stderr, exit_code, elapsed_ms }`

### Other modules

- `cli.rs` — clap definitions for all subcommands
- `config.rs` — loads `.cx.toml` (project) > `~/.config/cx/config.toml` (global) > defaults. Also has `detect_project()`.
- `main.rs` — entry point, dispatches CLI commands to tools

## Key conventions

- **Every compressor must have unit tests.** Tests are `#[cfg(test)] mod tests` inside each file.
- **Compressors are pure.** Never do I/O in `src/compress/`. All I/O goes through `runner::exec()` in the tool layer.
- **Smart defaults** are applied in the tool layer, not the compressor. Example: `GitTool` adds `--oneline -n30` to `git log`, then `GitCompressor` compresses the resulting output.
- **Truncation limits**: 150 lines max, 300 chars per line (configurable via `Config`).
- Footer format: `[label] ok/FAIL (Xms, exit Y)`

## Adding a new command

1. Create `src/compress/foo.rs` — implement `Compressor` trait + tests
2. Create `src/tools/foo.rs` — implement `Tool` trait
3. Add the module to `src/compress/mod.rs` and `src/tools/mod.rs`
4. Add the CLI subcommand in `src/cli.rs`
5. Wire it in `src/main.rs` match

## File map

| File | Purpose |
|------|---------|
| `src/main.rs` | Entry point, CLI dispatch |
| `src/cli.rs` | clap command definitions |
| `src/config.rs` | Config loading, project detection |
| `src/runner.rs` | Process execution wrapper |
| `src/compress/mod.rs` | `Compressor` trait |
| `src/compress/truncate.rs` | Shared truncation utilities |
| `src/compress/git.rs` | Git output compression (status, diff, log, branch, etc.) |
| `src/compress/cargo.rs` | Cargo output compression (test, build, clippy, fmt, etc.) |
| `src/compress/python.rs` | Python tool compression (pytest, ruff, mypy, pip, uv) |
| `src/compress/docker.rs` | Docker output compression (ps, images, logs) |
| `src/compress/grep.rs` | Grep/rg output compression |
| `src/compress/generic.rs` | Fallback truncation |
| `src/tools/*.rs` | Tool implementations (smart defaults + exec) |

## Common patterns

### Compression pattern
```rust
fn compress_foo(raw: &str) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    // filter/transform lines
    // return compact summary
}
```

### Test pattern
```rust
#[test]
fn test_foo_with_errors() {
    let input = "simulated raw output...";
    let result = compress_foo(input);
    assert!(result.contains("expected content"));
}
```

## Do not

- Do not add I/O or process execution in `src/compress/`
- Do not skip writing tests for new compressors
- Do not hardcode paths — use `Config` for configurable values
- Do not break the `Tool` / `Compressor` trait separation
