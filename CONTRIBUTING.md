# Contributing to Hyperblaze

Hyperblaze is a solo project by [@Ashutosh0x](https://github.com/Ashutosh0x), but contributions are welcome and encouraged.

## Getting Started

```bash
git clone https://github.com/Ashutosh0x/hyperblaze.git
cd hyperblaze
cargo build
cargo test --workspace
```

## Good First Issues

If you want to contribute, here are some accessible starting points:

| Task | Difficulty | Area |
|------|-----------|------|
| Add `--verbose` flag to build command showing rustc args | Easy | hb-cli |
| Add `rust_test` rule that runs `rustc --test` | Easy | hb-core/rules |
| Write a test for library-depends-on-library builds | Easy | hb-core/rules |
| Add `--json` output format for build results | Easy | hb-cli |
| Improve error messages with file/line spans via miette | Medium | hb-core/error |
| Add `go_binary` rule that shells out to `go build` | Medium | hb-core/rules |
| Implement `hyperblaze query` to list targets in BUILD.hb | Medium | hb-cli |
| Add `hyperblaze fmt` to auto-format BUILD.hb files | Medium | hb-cli |
| Debounced file watcher daemon over named pipe | Hard | hb-core/watcher |
| Persistent graph cache via redb | Hard | hb-graph |

## Development Guidelines

### Code Style

- Run `cargo fmt --all` before committing
- Run `cargo clippy --workspace --all-targets -- -D warnings` -- zero warnings policy
- All public APIs must have doc comments

### Testing

- Every new feature needs at least one test
- Integration tests go in `crates/hb-cli/tests/`
- Unit tests go in the same file as the code (Rust convention)
- Tests that invoke `rustc` should use `tempfile::tempdir()`

### Commit Messages

Use conventional commits:

```
feat: add rust_test rule
fix: correct cache key when deps change  
test: add integration test for library builds
docs: update architecture diagram
refactor: extract rule execution into trait
```

### PR Checklist

- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] New code has tests
- [ ] Doc comments on public APIs

## Architecture Overview

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for details. Key files:

| File | What it does |
|------|-------------|
| `hb-core/src/rules.rs` | Rust build rules (rustc invocation, caching) |
| `hb-core/src/build_file.rs` | BUILD.hb TOML parser |
| `hb-core/src/digest.rs` | BLAKE3 content hashing |
| `hb-graph/src/evaluator.rs` | Async parallel graph evaluator |
| `hb-graph/src/node.rs` | Node state machine with early cutoff |
| `hb-cli/src/commands/build.rs` | Build command (parses BUILD.hb, invokes rules) |

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

Please also follow the project [Code of Conduct](CODE_OF_CONDUCT.md). If you discover a vulnerability, report it privately using the process in [SECURITY.md](SECURITY.md).
