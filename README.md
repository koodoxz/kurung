# kurung

Deterministic WASI/WASM confinement sandbox for AI agent tool execution and MCP servers.

Part of the Nusantara Suite under the MECOB architecture.

## What is kurung?

When AI agents synthesize and execute code dynamically, running that code on the host system risks infinite loops, memory crashes, or unauthorized file access. Spawning virtual machines or Docker containers prevents those risks but adds 100ms to 1,500ms of startup latency.

`kurung` solves this by running untrusted WebAssembly code inside a lightweight, embedded sandbox in Rust with sub-millisecond cold starts (0.78ms) and microsecond cached runs (0.035ms).

### Core Features

- Deterministic CPU Fuel Metering: Counts instructions and stops infinite loops at exact instruction limits, regardless of CPU speed.
- Linear Memory Cap: Enforces strict RAM limits (e.g., 16 MB). Memory allocation attacks are rejected before consuming host RAM.
- Isolated In-Memory Filesystem: Uses a default-deny virtual filesystem in RAM. Prevents path traversal (`..`, `C:\`, `/etc/`) with zero access to host disks.
- Bounded Output Buffer: Limits standard output to prevent memory exhaustion from log floods.
- Fast Module Caching: Caches compiled modules using SHA-256 digests for repeated calls in 35 microseconds.
- Cryptographic Receipts: Emits a SHA-256 execution receipt with exact fuel consumed and runtime metrics for audit trails.
- Zero Unsafe Code: Built with `#![forbid(unsafe_code)]`.

## Workspace Crates

- `crates/kurung-policy`: Enforces limits, permissions, and path sanitization.
- `crates/kurung-vfs`: In-memory filesystem and output buffers.
- `crates/kurung-core`: Execution engine, host function imports, fuel tracking, and receipts.
- `crates/kurung-cli`: CLI binary for execution, self-testing, and benchmarking.

## Benchmark Results

Environment: `x86_64-pc-windows-msvc`, Release Profile, 1,000 iterations.

```
Cold Start Latency (Compile + Exec) : 787 us (0.787 ms)
Warm Cached Latency p50             : 35 us (0.035 ms)
Warm Cached Latency p99             : 56 us (0.056 ms)
Fuel Consumed per Run               : 406 instructions
SLA Verification (Cold <= 2,000 us) : PASSED
```

## Quick Start

### Build and Test

```bash
cargo test --workspace --all-targets
cargo clippy --all-targets -- -D warnings
```

### Run Self-Audit

Verifies loop termination, memory caps, log bounds, and path isolation:

```bash
cargo run --release -p kurung-cli -- audit-self
```

### Execute a WebAssembly Module

```bash
cargo run --release -p kurung-cli -- run path/to/module.wat --entry run --input "hello" --json
```

## License

Dual-licensed under MIT OR Apache-2.0.
