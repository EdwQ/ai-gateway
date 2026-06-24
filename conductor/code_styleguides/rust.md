# Rust Code Style Guide

## Naming Conventions

- **Types** (structs, enums, traits): `PascalCase` — e.g., `UsageRecord`, `AuthError`
- **Functions, methods, variables**: `snake_case` — e.g., `get_user_by_id`, `total_cost`
- **Constants**: `SCREAMING_SNAKE_CASE` — e.g., `MAX_RETRY_COUNT`, `DEFAULT_QUOTA`
- **Module names**: `snake_case`, short and descriptive — e.g., `proxy`, `rate_limit`
- **Lifetime parameters**: single lowercase letter — e.g., `'a`, `'_`

## Code Organization

- One struct/enum per logical concern; prefer composition over inheritance
- `mod.rs` for module re-exports only; keep implementations in named files
- Public API surface minimal: mark items `pub(crate)` by default, `pub` only when necessary
- Group `use` statements:
  1. Standard library (`std::*`)
  2. External crates
  3. Crate internals (`crate::*`)
- Each `use` group separated by a blank line

## Error Handling

- Use `thiserror` or custom error enums, never `Box<dyn Error>` in public APIs
- Define `type Result<T> = std::result::Result<T, AppError>` at crate level
- Map all external errors into domain errors with `map_err`
- Use `.context()` / `.with_context()` (from `anyhow` or custom) for error enrichment
- Avoid bare `.unwrap()` and `.expect()` outside of tests and `main()`
- Use `if let Some(x)` / `if let Ok(x)` instead of `match` for single-arm cases

## async / Await

- Use `tokio` as the async runtime
- Prefer `tokio::spawn` with structured concurrency over raw `JoinHandle`
- Use `tokio::select!` for timeouts and race conditions
- Keep async functions short; extract synchronous logic into separate sync functions
- Use `Stream` / `StreamExt` for streaming responses (SSE, large payloads)

## Testing

- Unit tests in the same file, inside `#[cfg(test)] mod tests { ... }`
- Integration tests in `tests/` directory
- Use `sqlx::test` for database integration tests
- Mock HTTP services with `wiremock` or `httpmock`
- Test both success and error paths; include edge cases (empty input, malformed data)

## Documentation

- All public items must have doc comments (`///`)
- Include `# Example` or `# Panics` / `# Errors` sections where applicable
- Use `//` for internal comments only, prefer explaining "why" not "what"
- Keep module-level docs in `//!` at the top of each file

## Formatting & Linting

- Use `rustfmt` with default settings
- Use `clippy` with `#![warn(clippy::all, clippy::pedantic)]`
- Address all warnings before committing
