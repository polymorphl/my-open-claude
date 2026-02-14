# AGENTS.md: Development Guidelines for AI Coding Agents

## Project Overview
A Rust-based LLM-powered coding assistant with a Text User Interface (TUI), supporting OpenAI-compatible APIs.

## Build, Lint, and Test Commands

### Quick Reference
```bash
# Build
cargo build                     # Debug build
cargo build --release           # Optimized release build

# Format
cargo fmt                       # Auto-format code
cargo fmt --check               # Verify formatting compliance

# Lint
cargo clippy --all-targets -- -D warnings

# Testing
cargo test                      # Run all tests
cargo test test_name            # Run specific test by name
cargo test module::test_name    # Run test in specific module
cargo test --test integration   # Run tests in a specific test file
cargo test -- --nocapture       # Show print/debug statements

# Run Application
cargo run                       # Launch TUI
cargo run -- -p "prompt"        # Single-prompt mode
```

## Code Style Guidelines

### Project Architecture
- Strict layered architecture:
  - `core/`: Business logic (NO UI dependencies)
  - `tui/`: Text User Interface
  - `main.rs`: Application entry point and orchestration

### Import Organization
```rust
// Import groups (blank lines between):
use std::collections::HashMap;     // Standard library
use tokio::sync::Mutex;            // External async primitives
use serde::{Deserialize, Serialize}; # Serialization
use crate::core::config::Config;   # Internal modules
```

### Naming Conventions
- `snake_case`: Functions, variables
- `PascalCase`: Types, Structs, Enums, Traits
- `SCREAMING_SNAKE_CASE`: Constants
- Modules: `snake_case`

### Type Handling
- Use explicit type annotations
- Prefer concrete types over generics
- Leverage `Arc<T>` for shared async ownership
- Use `Option<T>` and `Result<T, E>` for robust error management

### Error Handling Strategy
- Create domain-specific error enums
- Implement `std::fmt::Display` and `std::error::Error`
- Use `thiserror` or manual implementations
- Provide context in error messages

Example Error Enum:
```rust
#[derive(Debug)]
pub enum ApiError {
    Authentication(String),
    NetworkFailure { 
        context: String, 
        source: std::io::Error 
    },
    Parsing(serde_json::Error),
}
```

### Testing Philosophy
- Unit tests in `#[cfg(test)]` modules
- Test successful and error scenarios
- Use `#[test]` attribute
- Aim for comprehensive edge case coverage
- Leverage `proptest` for property-based testing

### Async Best Practices
- Use `tokio` runtime
- Leverage `CancellationToken`
- Ensure thread-safety with `Send + Sync`
- Use `.await` judiciously

### Code Organization Heuristics
- One responsibility per module
- Document public APIs with `///`
- Extract code when file exceeds ~300 lines
- Prefer composition over inheritance

### Dependency Management
1. `core/` MUST NOT import `tui`
2. `tui/` MAY import `core`
3. `main.rs` orchestrates cross-module interactions

### Performance Considerations
- Minimize dynamic allocations
- Prefer borrowing (`&T`) over ownership
- Use `#[inline]` for small, frequently called functions
- Profile with `cargo flamegraph`

## Configuration & Environment
Required Environment Variables:
- `OPENROUTER_API_KEY`: API authentication
- `OPENROUTER_MODEL`: Default language model
- `MAX_CONVERSATIONS`: History limit

## Continuous Integration Requirements
Every contribution must pass:
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`

## Development Workflow
- Use feature branches
- Squash commits before merging
- Write meaningful commit messages
- Update documentation alongside code changes

## Community
- [Code of Conduct](CODE_OF_CONDUCT.md)
- [License](LICENSE) — Polyform Noncommercial 1.0.0 (no commercial use)