#![forbid(unsafe_code)]

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use clap::{Parser, Subcommand};
use kurung_core::{KurungEngine, KurungError};
use kurung_policy::SandboxPolicy;

#[derive(Parser)]
#[command(
    name = "kurung",
    version,
    about = "Sub-millisecond deterministic WASI/WASM confinement sandbox for AI agents"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a .wasm or .wat module inside the Kurung deterministic sandbox
    Run {
        /// Path to the .wasm or .wat file
        file: PathBuf,
        /// Exported entrypoint function name (must take () and return i64)
        #[arg(long, default_value = "run")]
        entry: String,
        /// UTF-8 input payload passed to the sandbox
        #[arg(long, default_value = "")]
        input: String,
        /// Maximum allowed CPU instructions (fuel)
        #[arg(long, default_value_t = 500_000)]
        max_fuel: u64,
        /// Maximum allowed linear memory in MiB
        #[arg(long, default_value_t = 16)]
        max_memory_mb: usize,
        /// Emit full JSON execution outcome and cryptographic receipt
        #[arg(long)]
        json: bool,
    },
    /// Run cold-start and warm-cache sub-millisecond performance benchmarks
    Bench {
        /// Number of warm iterations to run
        #[arg(long, default_value_t = 1000)]
        iterations: usize,
    },
    /// Execute built-in 4-stage Red-Team adversarial security verification
    AuditSelf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            file,
            entry,
            input,
            max_fuel,
            max_memory_mb,
            json,
        } => {
            let mut policy = SandboxPolicy::strict_default();
            policy.max_fuel = max_fuel;
            policy.max_memory_bytes = max_memory_mb * 1024 * 1024;

            let engine = KurungEngine::new(policy)?;
            let raw_bytes = fs::read(&file)?;

            let outcome = if file.extension().is_some_and(|ext| ext == "wat") {
                let wat_str = std::str::from_utf8(&raw_bytes)?;
                engine.execute_wat(wat_str, &entry, input.as_bytes())?
            } else {
                engine.execute_wasm(&raw_bytes, &entry, input.as_bytes())?
            };

            if json {
                println!("{}", serde_json::to_string_pretty(&outcome)?);
            } else {
                println!("=== KURUNG EXECUTION RECEIPT ===");
                println!("Return Value     : {}", outcome.return_value);
                println!(
                    "Stdout           : {}",
                    String::from_utf8_lossy(&outcome.stdout)
                );
                println!("Module SHA-256   : {}", outcome.receipt.module_sha256);
                println!(
                    "Fuel Consumed    : {} / {}",
                    outcome.receipt.fuel_consumed, outcome.receipt.fuel_limit
                );
                println!(
                    "Peak Memory      : {} bytes",
                    outcome.receipt.peak_memory_bytes
                );
                println!("Duration         : {} us", outcome.receipt.duration_us);
                println!("Status           : {}", outcome.receipt.status);
            }
        }
        Commands::Bench { iterations } => {
            let policy = SandboxPolicy::strict_default();
            let engine = KurungEngine::new(policy)?;

            let bench_wat = r#"
                (module
                    (import "kurung" "input_len" (func $input_len (result i32)))
                    (import "kurung" "write_output" (func $write_output (param i32 i32) (result i32)))
                    (memory (export "memory") 1)
                    (data (i32.const 0) "OK")
                    (func (export "run") (result i64)
                        (local $i i32)
                        (local $acc i64)
                        (loop $l
                            (local.set $acc (i64.add (local.get $acc) (i64.const 3)))
                            (local.set $i (i32.add (local.get $i) (i32.const 1)))
                            (br_if $l (i32.lt_u (local.get $i) (i32.const 100)))
                        )
                        (drop (call $write_output (i32.const 0) (i32.const 2)))
                        (i64.add (local.get $acc) (i64.extend_i32_s (call $input_len)))
                    )
                )
            "#;

            let wasm_bytes = wat::parse_str(bench_wat)?;

            let t0 = Instant::now();
            let cold_outcome = engine.execute_wasm(&wasm_bytes, "run", b"BENCH_PAYLOAD")?;
            let cold_us = t0.elapsed().as_micros();

            let mut latencies_us = Vec::with_capacity(iterations);
            for _ in 0..iterations {
                let ts = Instant::now();
                let _ = engine.execute_wasm(&wasm_bytes, "run", b"BENCH_PAYLOAD")?;
                latencies_us.push(ts.elapsed().as_micros() as u64);
            }
            latencies_us.sort_unstable();
            let p50 = latencies_us[iterations / 2];
            let p99 = latencies_us[(iterations * 99) / 100];

            println!("================================================================");
            println!(
                "KURUNG DETERMINISTIC SANDBOX BENCHMARK ({} ITERATIONS)",
                iterations
            );
            println!("================================================================");
            println!(
                "Cold Start Latency (Compile + Exec) : {} us ({:.3} ms)",
                cold_us,
                cold_us as f64 / 1000.0
            );
            println!(
                "Warm Cached Latency p50             : {} us ({:.3} ms)",
                p50,
                p50 as f64 / 1000.0
            );
            println!(
                "Warm Cached Latency p99             : {} us ({:.3} ms)",
                p99,
                p99 as f64 / 1000.0
            );
            println!(
                "Fuel Consumed per Run               : {} instructions",
                cold_outcome.receipt.fuel_consumed
            );
            println!(
                "Module SHA-256                      : {}",
                cold_outcome.receipt.module_sha256
            );
            println!(
                "SLA Verification (Cold <= 2000 us)  : {}",
                if cold_us <= 2000 { "PASSED" } else { "FAILED" }
            );
            println!("================================================================");
        }
        Commands::AuditSelf => {
            println!("Running Kurung 4-Stage Red-Team Adversarial Self-Audit...");

            // 1. Fuel Cutoff
            let mut p1 = SandboxPolicy::strict_default();
            p1.max_fuel = 5_000;
            let e1 = KurungEngine::new(p1)?;
            let inf_wat =
                r#"(module (func (export "run") (result i64) (loop $l br $l) i64.const 0))"#;
            assert!(matches!(
                e1.execute_wat(inf_wat, "run", b""),
                Err(KurungError::FuelExhausted { .. })
            ));
            println!("  [PASS] Stage 1: Infinite Loop Deterministic Fuel Cutoff");

            // 2. Memory Bomb
            let mut p2 = SandboxPolicy::strict_default();
            p2.max_memory_bytes = 65_536;
            let e2 = KurungEngine::new(p2)?;
            let mem_wat = r#"(module (memory 4) (func (export "run") (result i64) i64.const 1))"#;
            assert!(matches!(
                e2.execute_wat(mem_wat, "run", b""),
                Err(KurungError::MemoryLimitExceeded { .. })
            ));
            println!("  [PASS] Stage 2: Linear Memory Bomb Rejection");

            // 3. Log Bomb Truncation
            let mut p3 = SandboxPolicy::strict_default();
            p3.max_output_bytes = 4;
            let e3 = KurungEngine::new(p3)?;
            let log_wat = r#"
                (module
                    (import "kurung" "write_output" (func $w (param i32 i32) (result i32)))
                    (memory (export "memory") 1)
                    (data (i32.const 0) "FLOOD_DATA_12345")
                    (func (export "run") (result i64)
                        (i64.extend_i32_s (call $w (i32.const 0) (i32.const 16)))
                    )
                )
            "#;
            let out3 = e3.execute_wat(log_wat, "run", b"")?;
            assert_eq!(out3.stdout, b"FLOO");
            assert!(out3.receipt.output_truncated);
            println!("  [PASS] Stage 3: Stdout Flood Bounded Truncation");

            // 4. Path Traversal Guard
            let p4 = SandboxPolicy::strict_default();
            assert!(p4
                .sanitize_virtual_path("../../Windows/System32", false)
                .is_err());
            println!("  [PASS] Stage 4: VFS Default-Deny & Path Traversal Guard");

            println!("ALL 4 ADVERSARIAL SECURITY GATES PASSED.");
        }
    }

    Ok(())
}
