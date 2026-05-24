# Getting Started with Hyperblaze

## Installation

### From Source (Recommended)

Requires Rust 1.75+ and Cargo:

```bash
git clone https://github.com/hyperblaze/hyperblaze.git
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
hyperblaze build

# Build specific targets
hyperblaze build //src:main //lib:core

# Build with verbose output
hyperblaze build -v //...

# Build with specific job count
hyperblaze build -j 8 //...
```

---

## Running Tests

```bash
hyperblaze test //...
```

---

## Running a Binary

```bash
hyperblaze run //src:main -- --flag1 --flag2
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
output_dir = ".hb-out"     # Build output directory

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
| `build.output_dir` | `.hb-out` | Where build outputs are written |
| `remote.cache_url` | None | Remote cache server URL |
| `remote.exec_url` | None | Remote execution server URL |

---

## CLI Reference

```
hyperblaze [OPTIONS] <COMMAND>

Commands:
  build    Build the specified targets
  test     Run tests
  run      Run a binary target
  clean    Clean build outputs
  init     Initialize a new workspace
  query    Query the dependency graph
  fmt      Format BUILD.hb files
  info     Show build system information
  doctor   Diagnose common issues
  graph    Show the dependency graph

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
- Installed compilers (rustc, go, gcc)
- Version control (git)
- Workspace configuration

---

## Next Steps

- Read [ARCHITECTURE.md](ARCHITECTURE.md) for technical details
- Read [DESIGN.md](DESIGN.md) for design philosophy
- Check the [Roadmap](../README.md#roadmap) for upcoming features
