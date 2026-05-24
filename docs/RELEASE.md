# Release Process

Hyperblaze releases are published from the default branch after CI passes.

## Checklist

1. Update `CHANGELOG.md`.
2. Run:

   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace --all-targets
   cargo test --workspace --doc
   cargo build --release
   ```

3. Create a tag:

   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

4. Create a GitHub Release with the changelog entry as release notes.

5. Verify the release page, CI, and `funding.json` link.
