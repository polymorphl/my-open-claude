# My Open Claude

[![CI](https://github.com/polymorphl/my-open-claude/actions/workflows/ci.yml/badge.svg)](https://github.com/polymorphl/my-open-claude/actions/workflows/ci.yml)

![my-open-claude TUI](screenshot.png)

LLM-powered coding assistant written in Rust. It understands code and performs actions through tool calls, with a CLI and an interactive TUI chat mode.

## Features

- **TUI interface**: interactive chat in the terminal (ratatui + crossterm)
- **Single-prompt mode**: send one request and exit without opening the TUI
- **Tool calling**: OpenAI-compatible API (read, write, bash, etc.)
- **OpenRouter**: use models via the OpenRouter API (or other OpenAI-compatible backends)
- **Model selector**: choose from tool-capable models (Alt+M / F2), with persisted selection
- **Persistent conversation history**: conversations saved to disk; load with Alt+H, new conversation with Alt+N
- **Credit balance**: OpenRouter balance displayed in the header; click to open your account settings

## Prerequisites

- [Rust](https://www.rust-lang.org/) (rustc 1.93+)
- An OpenRouter API key (or other OpenAI-compatible provider)

## Installation

```sh
git clone https://github.com/polymorphl/my-open-claude.git
cd my-open-claude
cargo build --release
```

## Configuration

The app relies on environment variables. Use a `.env` file in the project root:

1. Copy the example file:
   ```sh
   cp env.example .env
   ```
2. Edit `.env` and set the values (e.g. `OPENROUTER_API_KEY`). See comments in `env.example` for details.

### Environment variables

| Variable | Required | Description |
|----------|----------|-------------|
| `OPENROUTER_API_KEY` | Yes | Your OpenRouter API key |
| `OPENROUTER_MODEL` | No | Default model ID (used when no last model saved). Default: `anthropic/claude-haiku-4.5` |
| `OPENROUTER_BASE_URL` | No | API base URL. Default: `https://openrouter.ai/api/v1` |
| `MY_OPEN_CLAUDE_MAX_CONVERSATIONS` | No | Max number of conversations to keep; older ones are pruned. Default: 50. Set to 0 for no limit. |

### Configuration paths

| Usage | Linux | macOS | Windows |
|-------|-------|-------|---------|
| Config (last selected model) | `~/.config/io/polymorphl/my-open-claude/` | `~/Library/Application Support/io.polymorphl.my-open-claude/` | `%APPDATA%\io\polymorphl\my-open-claude\` |
| Conversations | `~/.local/share/io/polymorphl/my-open-claude/conversations/` | `~/Library/Application Support/io.polymorphl.my-open-claude/conversations/` | `%APPDATA%\io\polymorphl\my-open-claude\conversations\` |
| Cache (models list, 24h TTL) | `~/.cache/io/polymorphl/my-open-claude/models.json` | `~/Library/Caches/io.polymorphl.my-open-claude/models.json` | `%LOCALAPPDATA%\io\polymorphl\my-open-claude\Cache\models.json` |

## Usage

**TUI mode (default)** — open the interactive chat:

```sh
cargo run
# or, after building:
./target/release/my-open-claude
```

**Single-prompt mode** — one request then exit:

```sh
cargo run -- -p "Explain what this project does"
```

### Credit balance

The header shows your OpenRouter credit balance (total minus usage). Click it to open your credits page. Balance is fetched on startup and refreshed every 30 minutes. Requires a Management API key; regular keys may see "—" instead.

### Conversation history

- **Alt+H** : open conversation history
- **Alt+N** : new conversation (current one is saved first)
- Conversations are saved automatically after each turn and on exit. A `*` in the title indicates unsaved changes.

### Model selection

- Press **Alt+M** to open the model selector.
- Only models that support tool calling are listed (sorted alphabetically).
- Type to filter by model name or ID.
- The last selected model is saved and reused on next launch.

## Project structure

- `src/main.rs` — entry point, CLI parsing, TUI or prompt mode launch
- `src/core/` — business logic: config, credits, confirm, persistence, history (conversations), LLM (agent loop, tools), models (fetch, cache)
- `src/core/llm/` — chat, agent loop, tool execution, streaming
- `src/core/models/` — model discovery, 24h cache, filtering
- `src/core/tools/` — read, write, bash
- `src/tui/` — terminal UI: app state, handlers (key/mouse), draw (header, history, input, popups)
