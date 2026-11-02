#![forbid(unsafe_code)]

use std::sync::Arc;
use kurung_policy::{PolicyError, SandboxPolicy};
use kurung_vfs::VfsError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KurungError {
    #[error("policy validation failed: {0}")]
    Policy(#[from] PolicyError),
    #[error("vfs error: {0}")]
    Vfs(#[from] VfsError),
    #[error("wasm compilation error: {0}")]
    Compilation(String),
    #[error("deterministic fuel exhausted: consumed {consumed} of {limit} allowed instructions")]
    FuelExhausted { limit: u64, consumed: u64 },
    #[error("linear memory ceiling exceeded: max allowed {max_bytes} bytes")]
    MemoryLimitExceeded { max_bytes: usize },
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

pub struct KurungEngine {
    policy: SandboxPolicy,
    sink: Option<Arc<dyn AttestationSink>>,
}

impl KurungEngine {
    pub fn new(policy: SandboxPolicy) -> Result<Self, KurungError> {
        policy.validate()?;
        Ok(Self { policy, sink: None })
    }

    pub fn with_attestation_sink(mut self, sink: Arc<dyn AttestationSink>) -> Self {
        self.sink = Some(sink);
        self
    }

    pub fn execute_wat(
        &self,
        _wat_source: &str,
        _entrypoint: &str,
        _input: &[u8],
    ) -> Result<ExecutionOutcome, KurungError> {
        // RED stub
        Err(KurungError::RuntimeTrap("not implemented".to_string()))
    }

    pub fn execute_wasm(
        &self,
        _wasm_bytes: &[u8],
        _entrypoint: &str,
        _input: &[u8],
    ) -> Result<ExecutionOutcome, KurungError> {
        // RED stub
        Err(KurungError::RuntimeTrap("not implemented".to_string()))
    }
}

