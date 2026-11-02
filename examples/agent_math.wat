;; Example AI Agent Tool: Computes sum of squares (1..=10) + input payload length
;; and writes an execution marker to bounded stdout.
(module
    (import "kurung" "input_len" (func $input_len (result i32)))
    (import "kurung" "write_output" (func $write_output (param i32 i32) (result i32)))
    (memory (export "memory") 1)
    (data (i32.const 0) "KURUNG_AGENT_TOOL_OK")
    (func (export "run") (result i64)
        (local $i i64)
        (local $acc i64)
        (local.set $i (i64.const 1))
        (loop $loop
            (local.set $acc
                (i64.add
                    (local.get $acc)
                    (i64.mul (local.get $i) (local.get $i))
                )
            )
            (local.set $i (i64.add (local.get $i) (i64.const 1)))
            (br_if $loop (i64.le_u (local.get $i) (i64.const 10)))
        )
        (drop (call $write_output (i32.const 0) (i32.const 20)))
        (i64.add (local.get $acc) (i64.extend_i32_s (call $input_len)))
    )
)

