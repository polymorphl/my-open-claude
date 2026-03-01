# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build                              # Debug build (run after any code change)
cargo build --release                    # Release build
cargo run                                # Launch TUI
cargo run -- -p "prompt"                 # Single-prompt mode
cargo test                               # Run all tests
cargo test test_name                     # Run a specific test by name
cargo test module::test_name             # Run test in specific module
cargo test --test integration            # Run tests in a specific test file
cargo test -- --nocapture                # Show stdout in tests
cargo fmt                                # Auto-format
cargo fmt --check                        # Check formatting (CI)
cargo clippy --all-targets -- -D warnings # Lint (CI)
```

**Always run `cargo build` after code changes** to verify compilation before considering a task complete.

## Architecture

Strict three-layer architecture — dependency flows one way only:

```
main.rs / cli.rs / run.rs   ← orchestration, entry point
        ↓
    core/                   ← business logic, NO UI imports allowed
        ↓
    tui/                    ← ratatui TUI, may import core
```

### core/ modules

| Module | Responsibility |
|--------|---------------|
| `llm/` | Agent loop, streaming, tool dispatch, context truncation |
| `tools/` | Tool trait + implementations (read, write, edit, bash, grep, glob, list_dir) |
| `models/` | Model discovery, 24h disk cache, filtering to tool-capable models |
| `history/` | Conversation persistence (index + per-file JSON storage) |
| `templates/` | Custom slash command templates (load, save, validate) |
| `workspace/` | Git root detection, project type, AGENTS.md loading |
| `commands.rs` | Built-in slash command definitions |
| `config.rs`, `api_key.rs` | Config and stored API key (config dir) |
| `paths.rs` | Platform-specific config/cache/data dirs |

### tui/ modules

| Module | Responsibility |
|--------|---------------|
| `app/` | App state, popup state, `ChatMessage` types, `CopyTarget` |
| `handlers/` | Keyboard and mouse event handling |
| `draw/` | Widget rendering and layout |
| `text/` | Markdown parsing, line wrapping, segment types |
| `syntax.rs` | Syntax highlighting via `syntect` |
| `chat_result.rs` | Post-chat state updates, save logic |

### Agent loop (`core/llm/agent_loop.rs`)

Stream API call → parse chunks → if tool calls: execute each tool → append result → loop. Returns `Complete` when no tool calls remain, or `NeedsConfirmation` when a destructive tool (e.g. `rm`) requires user approval.

### Ask vs Build mode

- **Ask**: Read, Grep, ListDir, Glob only (no writes or shell)
- **Build**: all tools enabled

### New code placement

| What | Where |
|------|-------|
| Business logic, data models | `core/` |
| New agent tool | `core/tools/<name>.rs` + register in `core/tools/mod.rs` |
| LLM/agent logic | `core/llm/` |
| New slash command | `core/commands.rs` (built-in) or `core/templates/` (custom) |
| Rendering/widgets | `tui/draw/` |
| Key/mouse event handling | `tui/handlers/` |
| Text formatting/markdown | `tui/text/` |

## Key Conventions

- **Conventional Commits** are required (release-please reads them for version bumps): `feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `perf:`, `test:`, `build:`, `ci:`. Breaking changes: `feat!:` / `fix!:`.
- **Branch naming**: `feature/*` or `fix/*`, merged into `master` with squash commits.
- **Error types**: use `thiserror`; define domain-specific error enums (e.g. `core/llm/error.rs`).
- **Async**: `tokio` runtime; use `CancellationToken` from `tokio-util` for request cancellation.
- **File length**: extract into a submodule around 250–400 lines when responsibilities diverge.
- **SOLID**: extend via `impl Tool` (new tool = new file, no changes to `tool_execution.rs`); prefer trait methods over `match` on tool names.
- **Module docs**: `//!` doc comments at the top of each file.

## Configuration

API key resolution order: env var `OPENROUTER_API_KEY` → stored key in config dir → `.env` in CWD.

Copy `env.example` to `.env` for local development. The built-in slash command prompts can be customized in `config/builtin-commands.json` before `cargo build`.

## CI Requirements

All three must pass:
```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
