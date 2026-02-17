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
cargo test -v test_specific     # Verbose output for specific test

# Run Application
cargo run                       # Launch TUI
cargo run -- -p "prompt"        # Single-prompt mode
```

## Project Structure
- Strict layered architecture:
  - `core/`: Business logic (NO UI dependencies)
  - `tui/`: Text User Interface
  - `main.rs`: Application entry point

## Code Style Guidelines

### Import Organization
```rust
// Import priority (blank lines between groups):
use std::collections::HashMap;     // Standard library
use tokio::sync::Mutex;            // External async primitives
use serde::{Deserialize, Serialize}; // Serialization
use crate::core::config::Config;   // Internal modules
```

### Naming Conventions
- `snake_case`: Functions, variables
- `PascalCase`: Types, Structs, Enums, Traits
- `SCREAMING_SNAKE_CASE`: Constants
- Modules: `snake_case`

### Type Handling
- Explicit type annotations
- Concrete types over generics
- `Arc<T>` for shared async ownership
- `Option<T>` and `Result<T, E>` for robust error management

### Error Handling Strategy
- Domain-specific error enums
- Implement `std::fmt::Display` and `std::error::Error`
- Use `thiserror` for implementations
- Provide context in error messages

### Testing Philosophy
- Unit tests in `#[cfg(test)]` modules
- Cover successful and error scenarios
- Use `#[test]` attribute
- Aim for comprehensive edge case coverage
- Use `proptest` for property-based testing

### Async Best Practices
- Use `tokio` runtime
- Leverage `CancellationToken`
- Ensure `Send + Sync`
- Judicious `.await`

### Performance Considerations
- Minimize dynamic allocations
- Prefer borrowing over ownership
- `#[inline]` for small functions
- Profile with `cargo flamegraph`

## Environment & Configuration
Required Environment Variables:
- `OPENROUTER_API_KEY`: API authentication
- `OPENROUTER_MODEL`: Default model
- `MAX_CONVERSATIONS`: History limit

## Continuous Integration
Every contribution must pass:
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`

## Development Workflow
- Branch naming: `feature/*` or `fix/*` for work merged into `master`
- Squash commits when merging PRs
- Commit messages MUST follow [Conventional Commits](https://www.conventionalcommits.org/) (required for release-please)
- Update documentation with code changes

### Conventional Commits
Use these prefixes for commit messages (especially squash-merge messages):
- `feat:` — new feature (minor version bump)
- `fix:` — bug fix (patch version bump)
- `docs:`, `style:`, `refactor:`, `perf:`, `test:`, `build:`, `ci:`, `chore:` — no version bump
- `feat!:`, `fix!:` — breaking change (major version bump)

Example: `feat: add custom slash commands`

## Releasing
- [docs/RELEASING.md](docs/RELEASING.md) — version bumps, GitHub releases, pre-built binaries
- release-please manages versions from Conventional Commits; merge Release PRs to publish

## Build Verification
Always `cargo build` after modifications to verify compilation.

## Community
- [Code of Conduct](CODE_OF_CONDUCT.md)
- [License](LICENSE) — Polyform Noncommercial 1.0.0