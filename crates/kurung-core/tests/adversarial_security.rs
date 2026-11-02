use kurung_core::{AttestationSink, ExecutionReceipt, KurungEngine, KurungError};
use kurung_policy::SandboxPolicy;
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
    let engine = KurungEngine::new(policy).expect("valid policy");

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
}

#[test]
fn req_krg_02_memory_bomb_is_blocked_by_linear_memory_ceiling() {
    let mut policy = SandboxPolicy::strict_default();
    policy.max_memory_bytes = 2 * 65536; // Max 2 WASM pages (128 KB)
    let engine = KurungEngine::new(policy).expect("valid policy");

    // Module starts with 1 page and tries to grow by 10 pages (640 KB > 128 KB limit)
    let memory_bomb_wat = r#"
        (module
            (memory 1)
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
        "memory.grow exceedingSandboxPolicy.max_memory_bytes must return -1 (allocation denied)"
    );
    assert!(outcome.receipt.peak_memory_bytes <= 2 * 65536);
}

#[test]
fn req_krg_04_and_06_host_io_bridge_and_sha256_receipt_and_sink() {
    let mut policy = SandboxPolicy::strict_default();
    policy.max_output_bytes = 8; // Small output cap to also verify truncation
    let sink = Arc::new(MemorySink {
        receipts: Mutex::new(Vec::new()),
    });
    let engine = KurungEngine::new(policy)
        .unwrap()
        .with_attestation_sink(sink.clone());

    let io_wat = r#"
        (module
            (import "kurung" "input_len" (func $input_len (result i32)))
            (import "kurung" "write_output" (func $write_output (param i32 i32) (result i32)))
            (memory (export "memory") 1)
            (data (i32.const 0) "HELLO_KURUNG_WORLD")
            (func (export "run") (result i64)
                (drop (call $write_output (i32.const 0) (i32.const 18)))
                (i64.extend_i32_s (call $input_len))
            )
        )
    "#;

    let outcome = engine
        .execute_wat(io_wat, "run", b"AGENT_PAYLOAD")
        .expect("valid execution");

    assert_eq!(outcome.return_value, 13);
    assert_eq!(outcome.stdout, b"HELLO_KU"); // Truncated at 8 bytes!
    assert!(outcome.receipt.output_truncated);
    assert!(outcome.receipt.fuel_consumed > 0);
    assert_eq!(outcome.receipt.module_sha256.len(), 64);

    let recorded = sink.receipts.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0], outcome.receipt);
}

