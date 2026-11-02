#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const WASM_PAGE_SIZE: usize = 65_536;
pub const MAX_ALLOWED_MEMORY_BYTES: usize = 1024 * 1024 * 1024; // 1 GiB hard ceiling

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PolicyError {
    #[error("invalid fuel limit: {0} (must be > 0)")]
    InvalidFuelLimit(u64),
    #[error("invalid memory limit: {0} bytes (must be >= 65536 and <= 1073741824)")]
    InvalidMemoryLimit(usize),
    #[error("invalid output limit: {0} bytes (must be > 0 and <= 67108864)")]
    InvalidOutputLimit(usize),
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
            max_memory_bytes: 16 * 1024 * 1024, // 16 MiB
            max_output_bytes: 64 * 1024,        // 64 KiB
            max_vfs_bytes: 0,
            allow_vfs_read: false,
            allow_vfs_write: false,
            allowed_prefixes: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), PolicyError> {
        if self.max_fuel == 0 {
            return Err(PolicyError::InvalidFuelLimit(0));
        }
        if self.max_memory_bytes < WASM_PAGE_SIZE || self.max_memory_bytes > MAX_ALLOWED_MEMORY_BYTES {
            return Err(PolicyError::InvalidMemoryLimit(self.max_memory_bytes));
        }
        if self.max_output_bytes == 0 || self.max_output_bytes > 64 * 1024 * 1024 {
            return Err(PolicyError::InvalidOutputLimit(self.max_output_bytes));
        }
        Ok(())
    }

    pub fn sanitize_virtual_path(&self, raw_path: &str, is_write: bool) -> Result<String, PolicyError> {
        if raw_path.is_empty()
            || raw_path.contains('\0')
            || raw_path.contains('\\')
            || raw_path.contains(':')
            || !raw_path.starts_with('/')
        {
            return Err(PolicyError::PathTraversalDetected(raw_path.to_string()));
        }

        let mut segments = Vec::new();
        for part in raw_path.split('/') {
            if part.is_empty() {
                continue;
            }
            if part == "." || part == ".." {
                return Err(PolicyError::PathTraversalDetected(raw_path.to_string()));
            }
            segments.push(part);
        }

        if segments.is_empty() {
            return Err(PolicyError::PathTraversalDetected(raw_path.to_string()));
        }

        let normalized = format!("/{}", segments.join("/"));

        if is_write && !self.allow_vfs_write {
            return Err(PolicyError::VfsAccessDenied(normalized));
        }
        if !is_write && !self.allow_vfs_read {
            return Err(PolicyError::VfsAccessDenied(normalized));
        }

        let prefix_allowed = self.allowed_prefixes.iter().any(|prefix| {
            let clean_prefix = prefix.trim_end_matches('/');
            if clean_prefix.is_empty() {
                return false;
            }
            normalized == clean_prefix
                || normalized
                    .strip_prefix(clean_prefix)
                    .is_some_and(|rest| rest.starts_with('/'))
        });

        if !prefix_allowed {
            return Err(PolicyError::VfsAccessDenied(normalized));
        }

        Ok(normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rejects_zero_fuel_and_invalid_memory_and_output() {
        let mut p = SandboxPolicy::strict_default();
        p.max_fuel = 0;
        assert_eq!(p.validate(), Err(PolicyError::InvalidFuelLimit(0)));

        let mut p2 = SandboxPolicy::strict_default();
        p2.max_memory_bytes = 1024;
        assert_eq!(p2.validate(), Err(PolicyError::InvalidMemoryLimit(1024)));

        let mut p3 = SandboxPolicy::strict_default();
        p3.max_output_bytes = 0;
        assert_eq!(p3.validate(), Err(PolicyError::InvalidOutputLimit(0)));

        assert_eq!(SandboxPolicy::default().validate(), Ok(()));
    }

    #[test]
    fn test_blocks_path_traversal_and_absolute_paths() {
        let mut p = SandboxPolicy::strict_default();
        p.allow_vfs_read = true;
        p.allow_vfs_write = false;
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
        assert!(matches!(
            p.sanitize_virtual_path("/", false),
            Err(PolicyError::PathTraversalDetected(_))
        ));
        assert!(matches!(
            p.sanitize_virtual_path("/workspace_evil/file.txt", false),
            Err(PolicyError::VfsAccessDenied(_))
        ));
        assert!(matches!(
            p.sanitize_virtual_path("/workspace/data.json", true),
            Err(PolicyError::VfsAccessDenied(_))
        ));
        assert_eq!(
            p.sanitize_virtual_path("/workspace/data.json", false).unwrap(),
            "/workspace/data.json"
        );
    }
}
