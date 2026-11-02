# kurung

Deterministic WASI/WASM confinement sandbox for autonomous AI agent tool synthesis and MCP servers.

Part of the Nusantara Suite under the MECOB architecture.

## Overview

`kurung` provides bounded, deterministic execution of untrusted WebAssembly (`wasm32-unknown-unknown` and `wasm32-wasip1`) modules with zero container overhead. It is designed for low-latency host systems where spawning Docker containers or microVMs introduces unacceptable startup delays.

### Core Guarantees

1. Deterministic Fuel Metering: CPU instruction counting using exact instruction deductions. Infinite loops and algorithmic complexity attacks terminate at the fuel boundary regardless of host CPU frequency.
2. Linear Memory Ceiling: Strict page-aligned ceiling enforced by host resource limiters. Dynamic memory expansion (`memory.grow`) beyond the configured quota returns an allocation failure without terminating or destabilizing the host process.
3. Isolated In-Memory VFS: Default-deny filesystem abstraction. Path traversal sequences (`..`), backslashes, drive letters, and null bytes are rejected at the policy layer. No host filesystem access is permitted.
4. Bounded Stdout Buffer: Prevents memory exhaustion attacks via excessive stdout/stderr emission by enforcing deterministic truncation.
5. Content-Addressable Module Cache: Thread-safe in-memory caching keyed by module SHA-256 digest, reducing repeated invocation overhead to microsecond levels.
6. Cryptographic Execution Receipt: Every execution outputs a structured receipt containing SHA-256 hashes of the module, input payload, and output buffer, alongside exact fuel consumption and execution duration.
7. Zero Unsafe Code: The entire workspace strictly enforces `#![forbid(unsafe_code)]`.

## Workspace Structure

The workspace is organized into four discrete crates:

- `crates/kurung-policy`: Defines security boundaries, limits, and path sanitization.
- `crates/kurung-vfs`: In-memory virtual filesystem and bounded output ring buffers.
- `crates/kurung-core`: Engine implementation, host function bindings, memory limiters, module caching, and execution receipts.
- `crates/kurung-cli`: Command-line interface for execution, benchmarking, and self-auditing.

## Verified Benchmarks

Environment: `x86_64-pc-windows-msvc`, Release Profile (`cargo build --release`), 1,000 iterations.

```
Cold Start Latency (Compile + Validate + Exec) : 787 us (0.787 ms)
Warm Cached Latency p50                         : 35 us (0.035 ms)
Warm Cached Latency p99                         : 56 us (0.056 ms)
Fuel Consumed per Run                          : 406 instructions
SLA Verification (Cold <= 2,000 us)            : PASSED
```

## Building and Verification

### Prerequisites

- Rust 1.80+ (stable toolchain)
- Cargo

### Test Suite Execution

Run the workspace test suite:

```bash
cargo test --workspace --all-targets
```

Run static analysis with zero tolerance for warnings:

```bash
cargo clippy --all-targets -- -D warnings
```

Execute the built-in 4-stage adversarial verification suite:

```bash
cargo run --release -p kurung-cli -- audit-self
```

Run the performance benchmark:

```bash
cargo run --release -p kurung-cli -- bench --iterations 1000
```

## CLI Usage

### Running a WebAssembly Module

Execute a `.wat` or `.wasm` file with resource constraints:

```bash
cargo run --release -p kurung-cli -- run path/to/module.wat --entry run --input "payload" --max-fuel 500000 --max-memory-mb 16 --json
```

### JSON Execution Receipt Format

```json
{
  "return_value": 399,
  "stdout": [75, 85, 82, 85, 78, 71],
  "receipt": {
    "module_sha256": "bf412c7a742f99d973782f895281825d7f8324ba8334a5b34dceadab74e6924e",
    "input_sha256": "014412f92abd48abbe79f1d7e40ef3780770a818b2a5a4349385184f41f0f279",
    "output_sha256": "e42b58544c970d5b1f78cc28c4eb82210fea8272c02ba0d3d311ffa5d2514eb1",
    "fuel_limit": 500000,
    "fuel_consumed": 57,
    "peak_memory_bytes": 65536,
    "output_truncated": false,
    "cache_hit": false,
    "duration_us": 308,
    "status": "OK"
  }
}
```

## Host ABI Specification

Guest modules can import host functions under the `"kurung"` namespace:

- `input_len() -> i32`: Returns byte length of host input payload.
- `read_input(dest_ptr: i32, max_len: i32) -> i32`: Copies input payload into guest memory.
- `write_output(src_ptr: i32, len: i32) -> i32`: Writes guest bytes to bounded output buffer.
- `vfs_read(path_ptr: i32, path_len: i32, dest_ptr: i32, dest_cap: i32) -> i32`: Reads from virtual path.
- `vfs_write(path_ptr: i32, path_len: i32, src_ptr: i32, src_len: i32) -> i32`: Writes to virtual path within byte quota.

## License

Dual-licensed under MIT OR Apache-2.0.
