# Changelog

All notable changes to Hyperblaze will be documented in this file.

## Unreleased

### Added

- Added `funding.json` using the fundingjson.org v1.1.0 manifest schema.
- Added GitHub Sponsors metadata in `.github/FUNDING.yml`.
- Added project community health files: `CODE_OF_CONDUCT.md`, `SECURITY.md`, and this changelog.

### Fixed

- Fixed GitHub Actions failures from rustfmt, clippy, and integration-test warnings.
- Made `hyperblaze build //:target` include transitive `BUILD.hb` dependencies.
- Strengthened CLI integration tests so they exercise real `BUILD.hb` Rust builds.
- Updated GitHub Actions checkout to the Node 24 compatible release line.

## 0.1.0 - 2026-05-24

### Added

- Initial Rust workspace with `hb-core`, `hb-graph`, `hb-exec`, and `hb-cli`.
- HyperGraph async incremental computation prototype.
- BLAKE3 content hashing and file metadata cache.
- `hyperblaze init`, `info`, `doctor`, `build`, and `clean` commands.
- Real `rust_binary` and `rust_library` rules that invoke `rustc`.
- TOML-based `BUILD.hb` parser with `[[target]]` definitions.
- GitHub Actions CI for check, test, rustfmt, clippy, and release smoke tests.
