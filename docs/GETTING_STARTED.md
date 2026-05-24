# Getting Started with Hyperblaze

## Installation

### From Source (Recommended)

Requires Rust 1.85+ and Cargo:

```bash
git clone https://github.com/Ashutosh0x/hyperblaze.git
cd hyperblaze
cargo install --path crates/hb-cli
```

The binary will be installed as `hyperblaze` in your Cargo bin directory (usually `~/.cargo/bin/`).

### Verify Installation

```bash
hyperblaze --version
# hyperblaze 0.1.0

hyperblaze info
# Hyperblaze v0.1.0
# Platform: Linux-X86_64 (12 CPUs, 32.0 GB RAM)
# Startup time: 0.4ms
```

---

## Initializing a Workspace

Navigate to your project root and run:

```bash
hyperblaze init
```

This will:
1. Create a `HYPERBLAZE.toml` configuration file
2. Create a `.hb-out/` output directory
3. Update `.gitignore` with Hyperblaze entries
4. Auto-detect languages in your project

Create a first Rust target:

```bash
mkdir src
printf 'fn main() { println!("Hello from Hyperblaze!"); }\n' > src/main.rs
cat > BUILD.hb <<'EOF'
[[target]]
name = "hello"
rule = "rust_binary"
srcs = ["src/main.rs"]
edition = "2021"
EOF
```

### Auto-Detection

Hyperblaze automatically detects:
- **Rust** -- `Cargo.toml` found
- **Go** -- `go.mod` found
- **TypeScript/JavaScript** -- `package.json` found
- **Python** -- `pyproject.toml` or `setup.py` found

---

## Building Targets

```bash
# Build all targets
hyperblaze build //...

# Build specific targets
hyperblaze build //:hello

# Build with verbose output
hyperblaze build -v //...

# Build with specific job count
hyperblaze build -j 8 //...
```

---

## Running Tests

`hyperblaze test` is currently a placeholder command. Use Cargo for project tests until the `rust_test` rule lands:

```bash
cargo test
```

---

## Running a Binary

`hyperblaze run` is currently a placeholder command. Build the binary and run it from `.hb-out/` for now:

```bash
hyperblaze build //:hello
./.hb-out/hello
```

---

## Cleaning Build Outputs

```bash
# Remove build outputs
hyperblaze clean

# Remove build outputs AND the action cache
hyperblaze clean --expunge
```

---

## Configuration Reference

The `HYPERBLAZE.toml` file controls all workspace settings:

```toml
[project]
name = "my-project"        # Project name
version = "0.1.0"          # Project version

[build]
jobs = 0                   # Parallel jobs (0 = auto-detect CPU count)
disk_cache = true          # Enable disk-based action cache
output_base = ".hb-out"    # Build output directory

[remote]
# Uncomment to enable remote caching:
# cache_url = "grpc://cache.example.com:8080"

# Uncomment to enable remote execution:
# exec_url = "grpc://exec.example.com:8080"

# TLS for remote connections:
# tls = true

# Remote instance name:
# instance_name = "default"
```

### Key Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `build.jobs` | `0` (auto) | Number of parallel build jobs |
| `build.disk_cache` | `true` | Cache action results on disk |
| `build.output_base` | `.hb-out` | Where build outputs are written |
| `remote.cache_url` | None | Remote cache server URL |
| `remote.exec_url` | None | Remote execution server URL |

---

## CLI Reference

```
hyperblaze [OPTIONS] <COMMAND>

Commands:
  build    Build the specified targets
  test     Placeholder for future test execution
  run      Placeholder for future binary execution
  clean    Clean build outputs
  init     Initialize a new workspace
  query    Planned dependency graph query
  fmt      Planned BUILD.hb formatter
  info     Show build system information
  doctor   Diagnose common issues
  graph    Planned dependency graph visualization

Options:
  -v, --verbose...  Verbose output (-v, -vv, -vvv)
  -q, --quiet       Quiet mode -- only show errors
  -h, --help        Print help
  -V, --version     Print version
```

---

## Diagnosing Issues

Run `hyperblaze doctor` to check your environment:

```bash
hyperblaze doctor
```

This checks:
- Platform compatibility
- CPU core count
- Available memory
- Installed tools (rustc, go, git)
- Version control (git)
- Workspace configuration

---

## Next Steps

- Read [ARCHITECTURE.md](ARCHITECTURE.md) for technical details
- Read [DESIGN.md](DESIGN.md) for design philosophy
- Check the [Roadmap](../ROADMAP.md) for upcoming features
