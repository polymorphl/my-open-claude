# My Open Claude

LLM-powered coding assistant written in Rust. It understands code and performs actions through tool calls, with a CLI and an interactive TUI chat mode.

## Features

- **TUI interface**: interactive chat in the terminal (ratatui + crossterm)
- **Single-prompt mode**: send one request and exit without opening the TUI
- **Tool calling**: OpenAI-compatible API (read, write, bash, etc.)
- **OpenRouter**: use models via the OpenRouter API (or other OpenAI-compatible backends)

## Prerequisites

- [Rust](https://www.rust-lang.org/) (rustc 1.92+)
- An OpenRouter API key (or other OpenAI-compatible provider)

## Installation

```sh
git clone https://github.com/<your-username>/my-open-claude.git
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

## Project structure

- `src/main.rs` — entry point, CLI parsing, TUI or prompt mode launch
- `src/core/` — LLM logic, config, agent loop, and tools (read, write, bash)
- `src/tui/` — terminal UI (app, drawing, text handling)
- `src/confirm.rs` — confirmation before executing actions
