(module
  (func (export "run") (result i64)
    (loop $infinite
      (br $infinite)
    )
    (i64.const 0)
  )
)

