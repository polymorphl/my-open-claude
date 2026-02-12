---
name: rust-code-quality-analysis
description: Analyzes Rust codebases for quality, architecture, concurrency, async usage, error handling, performance, and maintainability. Use when reviewing Rust projects, assessing code quality, or when the user asks for a Rust code review or analysis.
---

# Rust Code Quality Analysis

When analyzing Rust codebases, systematically evaluate and report on the following dimensions.

## Analysis Dimensions

### 1. Architecture & Modularity

- **Layering**: Check separation of concerns (core vs UI vs entry point). Core should not depend on UI.
- **Module boundaries**: One module per file; `mod.rs` re-exports. Cohesive responsibilities.
- **Dependency direction**: No circular deps; clear ownership flow.
- **File length**: ~250 lines → consider extraction; ~400 lines → extract.

### 2. Concurrency & Async

- **Runtime choice**: Tokio/async-std; multi-thread vs current-thread.
- **Blocking vs async**: Avoid `block_on` on async runtime worker threads; use `spawn_blocking` for CPU/blocking work.
- **Cancellation**: Use `CancellationToken` or similar for long-running async tasks.
- **Channels**: Prefer `mpsc`/`broadcast` for cross-thread communication.
- **Send + Sync**: Ensure futures and types crossing thread boundaries are `Send + Sync`.

### 3. Error Handling

- **Domain errors**: Use `enum` with variants (e.g. `ApiAuth`, `ToolArgs` with `source`).
- **Implement `std::error::Error`** with `source()` for chainable errors.
- **Avoid**: Swallowing errors with `eprintln`; overuse of `unwrap`/`expect`; stringly-typed errors.
- **Tests**: Include error-mapping unit tests (e.g. `map_api_error`).

### 4. Performance & Resource Management

- **Caching**: `OnceLock`/`Lazy` for expensive one-time init.
- **Cloning**: Prefer `Arc` for shared mutable state; `Arc::make_mut` for copy-on-write.
- **Allocations**: Reduce repeated `Vec::new` in hot paths; consider iterators.
- **Bounds**: Max sizes for streams, tool outputs, context; truncate with UTF-8-safe boundaries.

### 5. Code Quality

- **Naming**: `snake_case` (vars/fns), `PascalCase` (types). Clear, consistent.
- **Duplication**: Extract shared logic (e.g. spawn helpers, config builders).
- **Parameter count**: `#[allow(clippy::too_many_arguments)]` suggests builder or context struct.
- **Idioms**: Prefer `Option::and_then`, `Result::map_err`, `?`; avoid nested `if let`.

### 6. Safety & Correctness

- **Ownership**: No unnecessary clones; borrow where possible.
- **Type system**: Use `Option`/`Result`; avoid `unwrap` in library paths.
- **Unsafe**: Document necessity; prefer safe abstractions.

## Output Format

Provide feedback as:

- **Critical**: Must fix (safety, correctness, major architectural issues).
- **High**: Strongly recommended (performance, error handling, duplication).
- **Medium**: Improvement (readability, minor refactors).
- **Low**: Nice to have (style, optional optimizations).

Include concrete file:line references and suggested code changes where helpful.

## Checklist (run before finishing)

- [ ] Architecture dependency rules satisfied
- [ ] No blocking calls on async worker threads
- [ ] Error types implement `Error` and `source()`
- [ ] No swallowed errors (eprintln + return default without logging)
- [ ] Send/Sync on types crossing thread boundaries
- [ ] Clippy suggestions reviewed
- [ ] Tests cover error paths and edge cases
