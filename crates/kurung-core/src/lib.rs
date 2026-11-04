#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use kurung_policy::{PolicyError, SandboxPolicy};
use kurung_vfs::{BoundedOutputBuffer, VfsError, VirtualFileSystem};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use wasmi::core::TrapCode;
use wasmi::{Caller, Config, Engine, Linker, Module, ResourceLimiter, Store};

/// Maximum buffer size (in bytes) allocated on the host per I/O call from guest (64 KiB).
pub const MAX_HOST_IO_CHUNK: usize = 64 * 1024;
/// Maximum length (in bytes) for a virtual filesystem path (4 KiB).
pub const MAX_VFS_PATH_LEN: usize = 4096;

#[derive(Debug, Error)]
pub enum KurungError {
    #[error("policy validation failed: {0}")]
    Policy(#[from] PolicyError),
    #[error("vfs error: {0}")]
    Vfs(#[from] VfsError),
    #[error("wasm compilation error: {0}")]
    Compilation(String),
    #[error("deterministic fuel exhausted: consumed {consumed} of {limit} allowed instructions")]
    FuelExhausted {
        limit: u64,
        consumed: u64,
        receipt: Box<ExecutionReceipt>,
    },
    #[error("linear memory ceiling exceeded: max allowed {max_bytes} bytes")]
    MemoryLimitExceeded {
        max_bytes: usize,
        receipt: Box<ExecutionReceipt>,
    },
    #[error("runtime trap during execution: {0}")]
    RuntimeTrap(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionReceipt {
    pub module_sha256: String,
    pub input_sha256: String,
    pub output_sha256: String,
    pub fuel_limit: u64,
    pub fuel_consumed: u64,
    pub peak_memory_bytes: usize,
    pub output_truncated: bool,
    pub cache_hit: bool,
    pub duration_us: u64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOutcome {
    pub return_value: i64,
    pub stdout: Vec<u8>,
    pub receipt: ExecutionReceipt,
}

/// Open-Core Enterprise Extension Hook:
/// Allows enterprise attestation vaults or `prasasti` time-series engines to ingest receipts in real time.
pub trait AttestationSink: Send + Sync {
    fn record_receipt(&self, receipt: &ExecutionReceipt);
}

struct HostContext {
    input: Vec<u8>,
    output: BoundedOutputBuffer,
    vfs: VirtualFileSystem,
    max_memory_bytes: usize,
    peak_memory_bytes: usize,
    memory_limit_exceeded: bool,
}

impl ResourceLimiter for HostContext {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> Result<bool, wasmi::errors::MemoryError> {
        if let Some(max) = maximum {
            if desired > max {
                return Ok(false);
            }
        }
        if desired > self.max_memory_bytes {
            self.memory_limit_exceeded = true;
            self.peak_memory_bytes = self.peak_memory_bytes.max(current);
            return Ok(false);
        }
        self.peak_memory_bytes = self.peak_memory_bytes.max(desired);
        Ok(true)
    }

    fn table_growing(
        &mut self,
        _current: u32,
        desired: u32,
        maximum: Option<u32>,
    ) -> Result<bool, wasmi::errors::TableError> {
        if let Some(max) = maximum {
            if desired > max {
                return Ok(false);
            }
        }
        Ok(desired <= 10_000)
    }
}

pub struct KurungEngine {
    policy: SandboxPolicy,
    engine: Engine,
    module_cache: Mutex<BTreeMap<String, Module>>,
    sink: Option<Arc<dyn AttestationSink>>,
}

impl KurungEngine {
    pub fn new(policy: SandboxPolicy) -> Result<Self, KurungError> {
        policy.validate()?;
        let mut config = Config::default();
        config.consume_fuel(true);
        let engine = Engine::new(&config);
        Ok(Self {
            policy,
            engine,
            module_cache: Mutex::new(BTreeMap::new()),
            sink: None,
        })
    }

    pub fn with_attestation_sink(mut self, sink: Arc<dyn AttestationSink>) -> Self {
        self.sink = Some(sink);
        self
    }

    pub fn policy(&self) -> &SandboxPolicy {
        &self.policy
    }

    pub fn cached_module_count(&self) -> usize {
        self.module_cache.lock().map(|c| c.len()).unwrap_or(0)
    }

    pub fn execute_wat(
        &self,
        wat_source: &str,
        entrypoint: &str,
        input: &[u8],
    ) -> Result<ExecutionOutcome, KurungError> {
        let wasm_bytes =
            wat::parse_str(wat_source).map_err(|e| KurungError::Compilation(e.to_string()))?;
        self.execute_wasm(&wasm_bytes, entrypoint, input)
    }

    pub fn execute_wasm(
        &self,
        wasm_bytes: &[u8],
        entrypoint: &str,
        input: &[u8],
    ) -> Result<ExecutionOutcome, KurungError> {
        self.execute_wasm_with_vfs(
            wasm_bytes,
            entrypoint,
            input,
            VirtualFileSystem::new(self.policy.clone()),
        )
    }

    pub fn execute_wasm_with_vfs(
        &self,
        wasm_bytes: &[u8],
        entrypoint: &str,
        input: &[u8],
        vfs: VirtualFileSystem,
    ) -> Result<ExecutionOutcome, KurungError> {
        let start_time = Instant::now();
        let module_sha256 = sha256_hex(wasm_bytes);
        let input_sha256 = sha256_hex(input);

        let (module, cache_hit) = {
            let mut cache = self
                .module_cache
                .lock()
                .map_err(|_| KurungError::RuntimeTrap("module cache lock poisoned".to_string()))?;
            if let Some(existing) = cache.get(&module_sha256) {
                (existing.clone(), true)
            } else {
                let compiled = Module::new(&self.engine, wasm_bytes)
                    .map_err(|e| KurungError::Compilation(e.to_string()))?;
                cache.insert(module_sha256.clone(), compiled.clone());
                (compiled, false)
            }
        };

        let host_ctx = HostContext {
            input: input.to_vec(),
            output: BoundedOutputBuffer::new(self.policy.max_output_bytes),
            vfs,
            max_memory_bytes: self.policy.max_memory_bytes,
            peak_memory_bytes: 0,
            memory_limit_exceeded: false,
        };

        let mut store = Store::new(&self.engine, host_ctx);
        store.limiter(|ctx| ctx);
        store
            .set_fuel(self.policy.max_fuel)
            .map_err(|e| KurungError::RuntimeTrap(e.to_string()))?;

        let mut linker = <Linker<HostContext>>::new(&self.engine);
        Self::register_host_functions(&mut linker)?;

        let instance = match linker.instantiate(&mut store, &module) {
            Ok(pre) => match pre.start(&mut store) {
                Ok(inst) => inst,
                Err(err) => {
                    let ctx = store.data();
                    let is_mem = ctx.memory_limit_exceeded;
                    let is_fuel = err.as_trap_code() == Some(TrapCode::OutOfFuel);
                    let status = if is_fuel {
                        "FUEL_EXHAUSTED"
                    } else if is_mem {
                        "MEMORY_LIMIT_EXCEEDED"
                    } else {
                        "TRAP"
                    };
                    let duration_us = start_time.elapsed().as_micros() as u64;
                    let remaining_fuel = store.get_fuel().unwrap_or(0);
                    let fuel_consumed = self.policy.max_fuel.saturating_sub(remaining_fuel);
                    let receipt = ExecutionReceipt {
                        module_sha256,
                        input_sha256,
                        output_sha256: sha256_hex(ctx.output.as_bytes()),
                        fuel_limit: self.policy.max_fuel,
                        fuel_consumed,
                        peak_memory_bytes: ctx.peak_memory_bytes,
                        output_truncated: ctx.output.is_truncated(),
                        cache_hit,
                        duration_us,
                        status: status.to_string(),
                    };
                    if let Some(sink) = &self.sink {
                        sink.record_receipt(&receipt);
                    }
                    if is_fuel {
                        return Err(KurungError::FuelExhausted {
                            limit: self.policy.max_fuel,
                            consumed: fuel_consumed,
                            receipt: Box::new(receipt),
                        });
                    } else if is_mem {
                        return Err(KurungError::MemoryLimitExceeded {
                            max_bytes: self.policy.max_memory_bytes,
                            receipt: Box::new(receipt),
                        });
                    } else {
                        return Err(KurungError::RuntimeTrap(err.to_string()));
                    }
                }
            },
            Err(e) => {
                let ctx = store.data();
                if ctx.memory_limit_exceeded {
                    let duration_us = start_time.elapsed().as_micros() as u64;
                    let receipt = ExecutionReceipt {
                        module_sha256,
                        input_sha256,
                        output_sha256: sha256_hex(ctx.output.as_bytes()),
                        fuel_limit: self.policy.max_fuel,
                        fuel_consumed: 0,
                        peak_memory_bytes: ctx.peak_memory_bytes,
                        output_truncated: ctx.output.is_truncated(),
                        cache_hit,
                        duration_us,
                        status: "MEMORY_LIMIT_EXCEEDED".to_string(),
                    };
                    if let Some(sink) = &self.sink {
                        sink.record_receipt(&receipt);
                    }
                    return Err(KurungError::MemoryLimitExceeded {
                        max_bytes: self.policy.max_memory_bytes,
                        receipt: Box::new(receipt),
                    });
                }
                return Err(KurungError::RuntimeTrap(e.to_string()));
            }
        };

        if let Some(mem) = instance.get_memory(&store, "memory") {
            let initial_bytes = mem.data_size(&store);
            let ctx = store.data_mut();
            ctx.peak_memory_bytes = ctx.peak_memory_bytes.max(initial_bytes);
        }

        let func = instance
            .get_typed_func::<(), i64>(&store, entrypoint)
            .map_err(|e| {
                KurungError::RuntimeTrap(format!("entrypoint '{entrypoint}' error: {e}"))
            })?;

        let call_result = func.call(&mut store, ());
        let duration_us = start_time.elapsed().as_micros() as u64;
        let remaining_fuel = store.get_fuel().unwrap_or(0);
        let fuel_consumed = self.policy.max_fuel.saturating_sub(remaining_fuel);

        match call_result {
            Ok(return_value) => {
                if let Some(mem) = instance.get_memory(&store, "memory") {
                    let final_bytes = mem.data_size(&store);
                    let ctx = store.data_mut();
                    ctx.peak_memory_bytes = ctx.peak_memory_bytes.max(final_bytes);
                }

                let ctx = store.data();
                let stdout = ctx.output.as_bytes().to_vec();
                let output_sha256 = sha256_hex(&stdout);
                let receipt = ExecutionReceipt {
                    module_sha256,
                    input_sha256,
                    output_sha256,
                    fuel_limit: self.policy.max_fuel,
                    fuel_consumed,
                    peak_memory_bytes: ctx.peak_memory_bytes,
                    output_truncated: ctx.output.is_truncated(),
                    cache_hit,
                    duration_us,
                    status: "OK".to_string(),
                };

                if let Some(sink) = &self.sink {
                    sink.record_receipt(&receipt);
                }

                Ok(ExecutionOutcome {
                    return_value,
                    stdout,
                    receipt,
                })
            }
            Err(err) => {
                let ctx = store.data();
                let stdout = ctx.output.as_bytes().to_vec();
                let is_mem_exceeded = ctx.memory_limit_exceeded;
                let is_out_of_fuel = err.as_trap_code() == Some(TrapCode::OutOfFuel);

                let status_label = if is_out_of_fuel {
                    "FUEL_EXHAUSTED"
                } else if is_mem_exceeded {
                    "MEMORY_LIMIT_EXCEEDED"
                } else {
                    "TRAP"
                };

                let receipt = ExecutionReceipt {
                    module_sha256,
                    input_sha256,
                    output_sha256: sha256_hex(&stdout),
                    fuel_limit: self.policy.max_fuel,
                    fuel_consumed,
                    peak_memory_bytes: ctx.peak_memory_bytes,
                    output_truncated: ctx.output.is_truncated(),
                    cache_hit,
                    duration_us,
                    status: status_label.to_string(),
                };

                if let Some(sink) = &self.sink {
                    sink.record_receipt(&receipt);
                }

                if is_out_of_fuel {
                    Err(KurungError::FuelExhausted {
                        limit: self.policy.max_fuel,
                        consumed: fuel_consumed,
                        receipt: Box::new(receipt),
                    })
                } else if is_mem_exceeded {
                    Err(KurungError::MemoryLimitExceeded {
                        max_bytes: self.policy.max_memory_bytes,
                        receipt: Box::new(receipt),
                    })
                } else {
                    Err(KurungError::RuntimeTrap(err.to_string()))
                }
            }
        }
    }

    fn register_host_functions(linker: &mut Linker<HostContext>) -> Result<(), KurungError> {
        linker
            .func_wrap(
                "kurung",
                "input_len",
                |caller: Caller<'_, HostContext>| -> i32 { caller.data().input.len() as i32 },
            )
            .map_err(|e| KurungError::Compilation(e.to_string()))?;

        linker
            .func_wrap(
                "kurung",
                "read_input",
                |mut caller: Caller<'_, HostContext>, dest_ptr: i32, max_len: i32| -> i32 {
                    if dest_ptr < 0 || max_len < 0 {
                        return -1;
                    }
                    let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory())
                    else {
                        return -1;
                    };
                    let mem_size = mem.data_size(&caller);
                    let to_copy = {
                        let input_ref = &caller.data().input;
                        input_ref.len().min(max_len as usize).min(MAX_HOST_IO_CHUNK)
                    };
                    if (dest_ptr as usize).saturating_add(to_copy) > mem_size {
                        return -1;
                    }
                    let chunk = caller.data().input[..to_copy].to_vec();
                    if mem.write(&mut caller, dest_ptr as usize, &chunk).is_err() {
                        return -1;
                    }
                    to_copy as i32
                },
            )
            .map_err(|e| KurungError::Compilation(e.to_string()))?;

        linker
            .func_wrap(
                "kurung",
                "write_output",
                |mut caller: Caller<'_, HostContext>, src_ptr: i32, len: i32| -> i32 {
                    if src_ptr < 0 || len < 0 || len as usize > MAX_HOST_IO_CHUNK {
                        return -1;
                    }
                    let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory())
                    else {
                        return -1;
                    };
                    let mem_size = mem.data_size(&caller);
                    if (src_ptr as usize).saturating_add(len as usize) > mem_size {
                        return -1;
                    }
                    let mut buf = vec![0u8; len as usize];
                    if mem.read(&caller, src_ptr as usize, &mut buf).is_err() {
                        return -1;
                    }
                    caller.data_mut().output.write_chunk(&buf) as i32
                },
            )
            .map_err(|e| KurungError::Compilation(e.to_string()))?;

        linker
            .func_wrap(
                "kurung",
                "vfs_write",
                |mut caller: Caller<'_, HostContext>,
                 path_ptr: i32,
                 path_len: i32,
                 src_ptr: i32,
                 src_len: i32|
                 -> i32 {
                    if path_ptr < 0
                        || path_len <= 0
                        || path_len as usize > MAX_VFS_PATH_LEN
                        || src_ptr < 0
                        || src_len < 0
                        || src_len as usize > MAX_HOST_IO_CHUNK
                    {
                        return -1;
                    }
                    let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory())
                    else {
                        return -1;
                    };
                    let mem_size = mem.data_size(&caller);
                    if (path_ptr as usize).saturating_add(path_len as usize) > mem_size
                        || (src_ptr as usize).saturating_add(src_len as usize) > mem_size
                    {
                        return -1;
                    }
                    let mut path_buf = vec![0u8; path_len as usize];
                    if mem.read(&caller, path_ptr as usize, &mut path_buf).is_err() {
                        return -1;
                    }
                    let Ok(path_str) = std::str::from_utf8(&path_buf) else {
                        return -1;
                    };
                    let mut content_buf = vec![0u8; src_len as usize];
                    if mem
                        .read(&caller, src_ptr as usize, &mut content_buf)
                        .is_err()
                    {
                        return -1;
                    }
                    match caller.data_mut().vfs.write_file(path_str, &content_buf) {
                        Ok(()) => src_len,
                        Err(_) => -2,
                    }
                },
            )
            .map_err(|e| KurungError::Compilation(e.to_string()))?;

        linker
            .func_wrap(
                "kurung",
                "vfs_read",
                |mut caller: Caller<'_, HostContext>,
                 path_ptr: i32,
                 path_len: i32,
                 dest_ptr: i32,
                 dest_cap: i32|
                 -> i32 {
                    if path_ptr < 0
                        || path_len <= 0
                        || path_len as usize > MAX_VFS_PATH_LEN
                        || dest_ptr < 0
                        || dest_cap < 0
                    {
                        return -1;
                    }
                    let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory())
                    else {
                        return -1;
                    };
                    let mem_size = mem.data_size(&caller);
                    if (path_ptr as usize).saturating_add(path_len as usize) > mem_size {
                        return -1;
                    }
                    let mut path_buf = vec![0u8; path_len as usize];
                    if mem.read(&caller, path_ptr as usize, &mut path_buf).is_err() {
                        return -1;
                    }
                    let Ok(path_str) = std::str::from_utf8(&path_buf) else {
                        return -1;
                    };
                    let content = match caller.data().vfs.read_file(path_str) {
                        Ok(bytes) => bytes.to_vec(),
                        Err(_) => return -2,
                    };
                    let to_copy = content.len().min(dest_cap as usize).min(MAX_HOST_IO_CHUNK);
                    if (dest_ptr as usize).saturating_add(to_copy) > mem_size {
                        return -1;
                    }
                    if mem
                        .write(&mut caller, dest_ptr as usize, &content[..to_copy])
                        .is_err()
                    {
                        return -1;
                    }
                    to_copy as i32
                },
            )
            .map_err(|e| KurungError::Compilation(e.to_string()))?;

        Ok(())
    }
}

pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}
