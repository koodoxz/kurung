use kurung_core::{AttestationSink, ExecutionReceipt, KurungEngine, KurungError};
use kurung_policy::SandboxPolicy;
use kurung_vfs::VirtualFileSystem;
use std::sync::{Arc, Mutex};

struct MemorySink {
    receipts: Mutex<Vec<ExecutionReceipt>>,
}

impl AttestationSink for MemorySink {
    fn record_receipt(&self, receipt: &ExecutionReceipt) {
        self.receipts.lock().unwrap().push(receipt.clone());
    }
}

#[test]
fn req_krg_01_infinite_loop_is_terminated_by_fuel_limiter() {
    let mut policy = SandboxPolicy::strict_default();
    policy.max_fuel = 10_000;
    let sink = Arc::new(MemorySink {
        receipts: Mutex::new(Vec::new()),
    });
    let engine = KurungEngine::new(policy)
        .expect("valid policy")
        .with_attestation_sink(sink.clone());

    let infinite_loop_wat = r#"
        (module
            (func (export "run") (result i64)
                (loop $inf
                    br $inf
                )
                i64.const 0
            )
        )
    "#;

    let err = engine
        .execute_wat(infinite_loop_wat, "run", b"")
        .expect_err("infinite loop must fail with FuelExhausted");

    assert!(
        matches!(err, KurungError::FuelExhausted { limit: 10_000, .. }),
        "Expected FuelExhausted, got: {err:?}"
    );

    let recorded = sink.receipts.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].status, "FUEL_EXHAUSTED");
}

#[test]
fn req_krg_02_memory_bomb_is_blocked_by_linear_memory_ceiling() {
    let mut policy = SandboxPolicy::strict_default();
    policy.max_memory_bytes = 2 * 65536; // Max 2 WASM pages (128 KB)
    let engine = KurungEngine::new(policy).expect("valid policy");

    // 1. Dynamic memory.grow beyond 2 pages returns -1 (denied)
    let memory_bomb_wat = r#"
        (module
            (memory (export "memory") 1)
            (func (export "run") (result i64)
                (i32.const 10)
                (memory.grow)
                (i64.extend_i32_s)
            )
        )
    "#;

    let outcome = engine
        .execute_wat(memory_bomb_wat, "run", b"")
        .expect("memory.grow returns -1 on denied growth without crashing host");
    assert_eq!(
        outcome.return_value, -1,
        "memory.grow exceeding SandboxPolicy.max_memory_bytes must return -1 (allocation denied)"
    );
    assert!(outcome.receipt.peak_memory_bytes <= 2 * 65536);

    // 2. Initial memory declaration exceeding max_memory_bytes is rejected at instantiation
    let initial_memory_bomb_wat = r#"
        (module
            (memory 10)
            (func (export "run") (result i64)
                i64.const 1
            )
        )
    "#;

    let init_err = engine
        .execute_wat(initial_memory_bomb_wat, "run", b"")
        .expect_err("initial 10-page allocation must be rejected");
    assert!(matches!(init_err, KurungError::MemoryLimitExceeded { .. }));
}

#[test]
fn req_krg_03_vfs_host_bridge_blocks_traversal_and_allows_authorized_io() {
    let mut policy = SandboxPolicy::strict_default();
    policy.allow_vfs_read = true;
    policy.allow_vfs_write = true;
    policy.max_vfs_bytes = 1024;
    policy.allowed_prefixes = vec!["/workspace".to_string()];

    let engine = KurungEngine::new(policy.clone()).unwrap();
    let vfs = VirtualFileSystem::new(policy);

    let vfs_wat = r#"
        (module
            (import "kurung" "vfs_write" (func $vfs_write (param i32 i32 i32 i32) (result i32)))
            (import "kurung" "vfs_read" (func $vfs_read (param i32 i32 i32 i32) (result i32)))
            (memory (export "memory") 1)
            (data (i32.const 0) "/workspace/agent.txt")
            (data (i32.const 32) "SAFE_DATA")
            (data (i32.const 64) "../../etc/passwd")
            (func (export "run") (result i64)
                ;; 1. Attempt illegal path traversal write -> must return -2
                (if (i32.ne (call $vfs_write (i32.const 64) (i32.const 16) (i32.const 32) (i32.const 9)) (i32.const -2))
                    (then (return (i64.const -100)))
                )
                ;; 2. Perform authorized write to /workspace/agent.txt -> returns 9
                (drop (call $vfs_write (i32.const 0) (i32.const 20) (i32.const 32) (i32.const 9)))
                ;; 3. Read back from /workspace/agent.txt into offset 128 -> returns 9
                (i64.extend_i32_s (call $vfs_read (i32.const 0) (i32.const 20) (i32.const 128) (i32.const 32)))
            )
        )
    "#;

    let wasm_bytes = wat::parse_str(vfs_wat).unwrap();
    let outcome = engine
        .execute_wasm_with_vfs(&wasm_bytes, "run", b"", vfs)
        .expect("vfs execution should succeed");
    assert_eq!(outcome.return_value, 9);
}

#[test]
fn req_krg_04_05_06_07_io_bridge_caching_receipt_and_sink() {
    let mut policy = SandboxPolicy::strict_default();
    policy.max_output_bytes = 8; // Small output cap to verify truncation
    let sink = Arc::new(MemorySink {
        receipts: Mutex::new(Vec::new()),
    });
    let engine = KurungEngine::new(policy)
        .unwrap()
        .with_attestation_sink(sink.clone());

    let io_wat = r#"
        (module
            (import "kurung" "input_len" (func $input_len (result i32)))
            (import "kurung" "read_input" (func $read_input (param i32 i32) (result i32)))
            (import "kurung" "write_output" (func $write_output (param i32 i32) (result i32)))
            (memory (export "memory") 1)
            (data (i32.const 0) "HELLO_KURUNG_WORLD")
            (func (export "run") (result i64)
                (drop (call $read_input (i32.const 64) (i32.const 32)))
                (drop (call $write_output (i32.const 0) (i32.const 18)))
                (i64.extend_i32_s (call $input_len))
            )
        )
    "#;

    // First run: cold compilation (cache_hit = false)
    let outcome1 = engine
        .execute_wat(io_wat, "run", b"AGENT_PAYLOAD")
        .expect("valid execution");

    assert_eq!(outcome1.return_value, 13);
    assert_eq!(outcome1.stdout, b"HELLO_KU"); // Truncated at 8 bytes
    assert!(outcome1.receipt.output_truncated);
    assert!(!outcome1.receipt.cache_hit);
    assert!(outcome1.receipt.fuel_consumed > 0);
    assert_eq!(outcome1.receipt.module_sha256.len(), 64);

    // Second run: warm module cache hit (cache_hit = true)
    let outcome2 = engine
        .execute_wat(io_wat, "run", b"AGENT_PAYLOAD")
        .expect("warm cached execution");
    assert!(outcome2.receipt.cache_hit);
    assert_eq!(engine.cached_module_count(), 1);

    let recorded = sink.receipts.lock().unwrap();
    assert_eq!(recorded.len(), 2);
}

#[test]
fn req_audit_01_host_memory_amplification_is_strictly_clamped() {
    let policy = SandboxPolicy::strict_default();
    let engine = KurungEngine::new(policy).expect("valid policy");

    // Guest has only 1 page (64 KB). It attempts to request 100 MB buffer allocation.
    let amplification_wat = r#"
        (module
            (import "kurung" "write_output" (func $write_output (param i32 i32) (result i32)))
            (import "kurung" "vfs_write" (func $vfs_write (param i32 i32 i32 i32) (result i32)))
            (memory (export "memory") 1)
            (data (i32.const 0) "/workspace/test.txt")
            (func (export "run") (result i64)
                ;; 1. write_output with 100 MB len -> must be rejected with -1 before host allocation
                (if (i32.ne (call $write_output (i32.const 0) (i32.const 100000000)) (i32.const -1))
                    (then (return (i64.const -1)))
                )
                ;; 2. vfs_write with 100 MB path_len -> must be rejected with -1
                (if (i32.ne (call $vfs_write (i32.const 0) (i32.const 100000000) (i32.const 0) (i32.const 10)) (i32.const -1))
                    (then (return (i64.const -2)))
                )
                ;; 3. vfs_write with 100 MB src_len -> must be rejected with -1
                (if (i32.ne (call $vfs_write (i32.const 0) (i32.const 19) (i32.const 0) (i32.const 100000000)) (i32.const -1))
                    (then (return (i64.const -3)))
                )
                i64.const 42
            )
        )
    "#;

    let outcome = engine
        .execute_wat(amplification_wat, "run", b"")
        .expect("execution must succeed with rejected oversized calls");
    assert_eq!(outcome.return_value, 42);
}

#[test]
fn req_audit_02_trap_code_exactness_not_spoofed_by_error_string() {
    let mut policy = SandboxPolicy::strict_default();
    policy.max_fuel = 1_000_000;
    let engine = KurungEngine::new(policy).expect("valid policy");

    // A function whose debug name contains "fuel" that traps with integer divide by zero
    let div_zero_wat = r#"
        (module
            (func $trap_while_calculating_fuel (export "run") (result i64)
                i32.const 1
                i32.const 0
                i32.div_s
                i64.extend_i32_s
            )
        )
    "#;

    let err = engine
        .execute_wat(div_zero_wat, "run", b"")
        .expect_err("division by zero must trap");

    assert!(
        matches!(err, KurungError::RuntimeTrap(_)),
        "Integer divide by zero must be RuntimeTrap, not FuelExhausted. Got: {err:?}"
    );
}
