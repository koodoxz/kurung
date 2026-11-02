#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PolicyError {
    #[error("invalid fuel limit: {0} (must be > 0)")]
    InvalidFuelLimit(u64),
    #[error("invalid memory limit: {0} bytes (must be >= 65536 and <= 1073741824)")]
    InvalidMemoryLimit(usize),
    #[error("path traversal or unauthorized path detected: '{0}'")]
    PathTraversalDetected(String),
    #[error("vfs access denied by default-deny policy for path: '{0}'")]
    VfsAccessDenied(String),
    #[error("write quota exceeded: requested {requested} bytes, max allowed {max_allowed} bytes")]
    WriteQuotaExceeded { requested: usize, max_allowed: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SandboxPolicy {
    pub max_fuel: u64,
    pub max_memory_bytes: usize,
    pub max_output_bytes: usize,
    pub max_vfs_bytes: usize,
    pub allow_vfs_read: bool,
    pub allow_vfs_write: bool,
    pub allowed_prefixes: Vec<String>,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self::strict_default()
    }
}

impl SandboxPolicy {
    pub fn strict_default() -> Self {
        Self {
            max_fuel: 500_000,
            max_memory_bytes: 16 * 1024 * 1024, // 16 MB
            max_output_bytes: 64 * 1024,        // 64 KB
            max_vfs_bytes: 0,
            allow_vfs_read: false,
            allow_vfs_write: false,
            allowed_prefixes: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), PolicyError> {
        // RED stub: does not validate yet
        Ok(())
    }

    pub fn sanitize_virtual_path(&self, _raw_path: &str, _is_write: bool) -> Result<String, PolicyError> {
        // RED stub: unimplemented behavior
        Ok(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rejects_zero_fuel_and_invalid_memory() {
        let mut p = SandboxPolicy::strict_default();
        p.max_fuel = 0;
        assert_eq!(p.validate(), Err(PolicyError::InvalidFuelLimit(0)));

        let mut p2 = SandboxPolicy::strict_default();
        p2.max_memory_bytes = 1024; // < 64KB WASM page
        assert_eq!(p2.validate(), Err(PolicyError::InvalidMemoryLimit(1024)));
    }

    #[test]
    fn test_blocks_path_traversal_and_absolute_paths() {
        let mut p = SandboxPolicy::strict_default();
        p.allow_vfs_read = true;
        p.allowed_prefixes = vec!["/workspace".to_string()];

        assert!(matches!(
            p.sanitize_virtual_path("../../Windows/System32/cmd.exe", false),
            Err(PolicyError::PathTraversalDetected(_))
        ));
        assert!(matches!(
            p.sanitize_virtual_path("/workspace/../etc/passwd", false),
            Err(PolicyError::PathTraversalDetected(_))
        ));
        assert!(matches!(
            p.sanitize_virtual_path("C:\\Windows\\System32", false),
            Err(PolicyError::PathTraversalDetected(_))
        ));
        assert_eq!(
            p.sanitize_virtual_path("/workspace/data.json", false).unwrap(),
            "/workspace/data.json"
        );
    }
}

