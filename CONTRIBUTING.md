# Contributing to DBeaver Proxy

Thank you for your interest in contributing! This document provides guidelines and instructions.

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment for everyone.

## How to Contribute

### Reporting Bugs

1. Check if the bug has already been reported in Issues
2. Open a new issue with:
   - A clear title and description
   - Steps to reproduce
   - Expected vs actual behavior
   - Your environment (OS, Rust version, DBeaver version)

### Suggesting Features

1. Open an issue describing the feature
2. Explain why it would be useful
3. If possible, outline how it could be implemented

### Pull Requests

1. Fork the repository
2. Create a branch: `git checkout -b feature/your-feature-name`
3. Make your changes
4. Ensure the code compiles: `cargo build --release`
5. Run tests: `cargo test`
6. Run linter: `cargo clippy -- -D warnings`
7. Format code: `cargo fmt`
8. Commit with clear messages
9. Push and open a Pull Request

## Development Setup

### Prerequisites

- Rust 1.81+ (install via [rustup](https://rustup.rs/))
- Git

### Building

```bash
# Clone the repository
git clone https://github.com/yourusername/dbeaver-proxy-rust
cd dbeaver-proxy-rust

# Build
cargo build --release

# Run tests
cargo test

# Check linting
cargo clippy -- -D warnings
```

### Project Structure

```
src/
├── main.rs              # Entry point
├── cli.rs               # CLI interface (init, start)
├── config.rs            # Configuration (TOML + env vars)
├── client.rs            # Backend HTTP client
├── models.rs            # Data types (serde DTOs)
├── router.rs            # HTTP router + middleware
├── sse.rs               # SSE streaming
├── metrics.rs           # Optional metrics collection
├── handlers/
│   ├── models.rs        # GET /v1/models
│   └── responses.rs     # POST /v1/responses + passthrough + health
└── translation/
    ├── request.rs       # DBeaver → Backend translation
    └── response.rs      # Backend → DBeaver translation
```

## Coding Standards

- Follow Rust idioms and patterns
- Use `cargo fmt` for formatting
- Use `cargo clippy` — no warnings allowed
- Write tests for new functionality
- Document public APIs with doc comments
- Keep error messages clear and actionable
- Match the existing code style

## Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

## Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add support for X
fix: correct Y behavior
docs: update README
refactor: simplify Z module
test: add tests for X
ci: update build workflow
```

## Release Process

Releases are triggered manually via GitHub Actions:

1. Go to **Actions → Release → Run workflow**
2. Enter the version (e.g., `0.2.0`)
3. The workflow:
   - Updates `Cargo.toml` version
   - Builds binaries for Linux (x86_64), Windows (x86_64), macOS (Intel + Apple Silicon)
   - Creates macOS Universal Binary via `lipo`
   - Creates a GitHub Release with all binaries + checksums
   - Auto-generates release notes from commits

## Questions?

Open a discussion or issue — we're happy to help!
