# Roadmap

Hyperblaze is early alpha. This roadmap is intentionally concrete so contributors and funders can see what is real, what is next, and what is still research.

## Phase 0 - Complete

- Rust workspace with `hb-core`, `hb-graph`, `hb-exec`, and `hb-cli`.
- Async HyperGraph prototype.
- BLAKE3 content hashing.
- `BUILD.hb` TOML parser.
- Real `rust_binary` and `rust_library` rules invoking `rustc`.
- Local disk cache for no-op rebuilds.
- CI across Linux, macOS, and Windows.

## Phase 1 - In Progress

- File watcher daemon using `notify`.
- Early cutoff based on stable value digests.
- Persistent graph cache with `redb`.
- Reproducible benchmark harness for small-to-medium Rust monorepos.
- Better diagnostics with source spans.

## Phase 2 - Planned

- `rust_test` rule.
- Real `hyperblaze run` execution path.
- Dependency inference for Rust imports.
- Go rules.
- BUILD.hb formatter and query command.

## Phase 3 - Planned

- Remote cache client.
- Remote execution API exploration.
- WASM sandbox prototype for actions.

## Phase 4 - Planned

- TUI progress display.
- Provenance/SBOM output.
- Release automation with signed artifacts.

## Phase 5 - Planned

- Public benchmark report against Bazel and Buck2.
- Stabilized v0.2 command surface.
- Wider contributor onboarding.
