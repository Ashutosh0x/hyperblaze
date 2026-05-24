# Hyperblaze Design Philosophy

## Why Build a New Build System?

### The Bazel Problem

Bazel is the industry standard for large-scale builds, but it has fundamental issues that can't be fixed without a rewrite:

1. **JVM Tax**: Every `bazel build` starts a JVM. Cold start is 3-15 seconds. Warm start still takes 500ms+. This is architecturally unfixable -- it's Java.

2. **Memory Monster**: Bazel's Skyframe keeps the entire dependency graph in JVM heap memory. For large projects, this means 8-14GB of RAM just for the build system. Google's response (Skyfocus) is a band-aid -- it drops incremental state to save memory, defeating the purpose of incrementality.

3. **Complexity Explosion**: WORKSPACE files, BUILD files, .bazelrc, Starlark macros, aspects, configurations, platforms, toolchains, transitions... The learning curve is measured in months. Many companies hire dedicated "Bazel engineers."

4. **Skyframe's Restart Protocol**: When a SkyFunction needs a dependency that isn't ready, it returns `null` and gets restarted from scratch. All computation up to that point is wasted. This is the single most wasteful design decision in a build system.

### The Buck2 Problem

Buck2 is technically excellent (Rust, async, DICE engine) but:

1. **Meta-centric**: Rules, documentation, and tooling assume you're a Meta engineer.
2. **Poor documentation**: Sparse, assumes internal knowledge.
3. **Small community**: Hard to get help, few third-party rules.
4. **RE-first design**: Local builds feel like an afterthought.

### The Pants Problem

Pants has great DX for Python but:

1. **No scale ambitions**: Not designed for Google/Meta-scale monorepos.
2. **Python plugin system**: Performance ceiling for extensions.
3. **No remote execution**: Only remote caching.
4. **Limited language breadth**: Great for Python, mediocre for everything else.

---

## Hyperblaze's Answer

### Core Principle: Speed x Simplicity

The build system market has a false dichotomy:
- **Fast but complex** (Bazel, Buck2)
- **Simple but slow** (Make, Cargo, npm scripts)

Hyperblaze breaks this tradeoff by using Rust's zero-cost abstractions to achieve both.

### Why Rust?

| Feature | Benefit for Build Systems |
|---------|--------------------------|
| No garbage collector | Predictable, low memory usage |
| Zero-cost abstractions | Fast without sacrificing ergonomics |
| `async/await` | Efficient parallel evaluation without threads |
| Ownership system | No data races in concurrent graph access |
| Native binary | 0.4ms startup, no runtime dependencies |
| WASM target | Future: cross-platform sandboxing |

### Design Decisions

#### 1. Single Binary, Zero Config
```bash
# This should "just work" for 80% of projects:
hyperblaze init
hyperblaze build
```

No WORKSPACE files. No BUILD files for common cases. Auto-detect languages, scan imports, infer dependencies.

#### 2. Async Suspension, Not Restarts
When a compute function needs a dependency:
- **Bazel**: Returns null -> entire function restarts from scratch
- **Hyperblaze**: `await` -> function suspends -> resumes exactly where it left off

This is not just cleaner -- it's faster. No wasted computation.

#### 3. Lock-Free Graph
The dependency graph uses `DashMap` (a concurrent hash map) instead of `synchronized` blocks. Multiple threads can read and write simultaneously without contention.

#### 4. Content-Addressed Everything
- File identity = BLAKE3 hash of contents (not timestamp)
- Action cache key = BLAKE3(input hashes + command + env)
- Build outputs stored by content hash

This means: rename a file without changing its contents -> zero rebuild.

#### 5. Beautiful Errors
Using `miette` for rich error diagnostics:
```
  Error: hyperblaze::build::target_not_found

  x Target '//src:main' not found
  |-> No BUILD.hb file found in 'src/'

  help: Run `hyperblaze init` to auto-generate BUILD files
        or create src/BUILD.hb manually
```

Not this (Bazel):
```
ERROR: /home/user/src/BUILD:3:1: no such target '//src:main':
target 'main' not found in package 'src'. Did you mean...
```

---

## Performance Targets

| Metric | Target | Rationale |
|--------|--------|-----------|
| Cold startup | < 1ms | No JVM, native binary |
| Warm startup | < 1ms | Persistent daemon with in-memory graph |
| No-op rebuild | < 10ms | File watcher + incremental graph |
| Memory (10K targets) | < 200MB | Native data structures, no GC |
| Memory (100K targets) | < 1GB | vs Bazel's 14GB |
| Cache lookup | < 1ms | Disk cache with sharded index |
| Hashing throughput | > 5 GB/s | BLAKE3 on modern CPUs |

---

## What We Learned from Competitors

| Lesson | Source | How We Apply It |
|--------|--------|----------------|
| Async incremental computation works | Buck2 DICE | HyperGraph engine |
| Dependency inference is killer DX | Pants | Auto-scan imports |
| Single config file is better | Turborepo | HYPERBLAZE.toml |
| JVM is the wrong choice | Bazel complaints | Rust native binary |
| Remote execution matters | Bazel/Buck2 | RE API compatibility planned |
| Supply chain security is mandatory | Industry 2026 | SLSA provenance planned |
| Beautiful TUI makes demos memorable | Modern CLI tools | ratatui integration planned |
