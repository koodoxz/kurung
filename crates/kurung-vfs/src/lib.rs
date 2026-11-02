#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use kurung_policy::{PolicyError, SandboxPolicy};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VfsError {
    #[error("policy error: {0}")]
    Policy(#[from] PolicyError),
    #[error("file not found in virtual filesystem: '{0}'")]
    FileNotFound(String),
}

#[derive(Debug, Clone)]
pub struct BoundedOutputBuffer {
    data: Vec<u8>,
    max_bytes: usize,
    truncated: bool,
}

impl BoundedOutputBuffer {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            data: Vec::with_capacity(max_bytes.min(4096)),
            max_bytes,
            truncated: false,
        }
    }

    pub fn write_chunk(&mut self, chunk: &[u8]) -> usize {
        let remaining = self.max_bytes.saturating_sub(self.data.len());
        if chunk.len() > remaining {
            self.data.extend_from_slice(&chunk[..remaining]);
            self.truncated = true;
            remaining
        } else {
            self.data.extend_from_slice(chunk);
            chunk.len()
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn is_truncated(&self) -> bool {
        self.truncated
    }
}

#[derive(Debug, Clone)]
pub struct VirtualFileSystem {
    files: BTreeMap<String, Vec<u8>>,
    total_bytes: usize,
    policy: SandboxPolicy,
}

impl VirtualFileSystem {
    pub fn new(policy: SandboxPolicy) -> Self {
        Self {
            files: BTreeMap::new(),
            total_bytes: 0,
            policy,
        }
    }

    /// Mounts a trusted host-supplied file into the virtual file system prior to guest execution.
    pub fn mount_readonly_file(&mut self, path: &str, content: Vec<u8>) -> Result<(), VfsError> {
        let mut mount_policy = self.policy.clone();
        mount_policy.allow_vfs_read = true;
        if mount_policy.allowed_prefixes.is_empty() {
            mount_policy.allowed_prefixes.push("/".to_string());
        }
        let normalized = mount_policy.sanitize_virtual_path(path, false)?;
        let prev_len = self.files.get(&normalized).map_or(0, Vec::len);
        self.total_bytes = self.total_bytes.saturating_sub(prev_len) + content.len();
        self.files.insert(normalized, content);
        Ok(())
    }

    pub fn read_file(&self, path: &str) -> Result<&[u8], VfsError> {
        let normalized = self.policy.sanitize_virtual_path(path, false)?;
        self.files
            .get(&normalized)
            .map(Vec::as_slice)
            .ok_or(VfsError::FileNotFound(normalized))
    }

    pub fn write_file(&mut self, path: &str, content: &[u8]) -> Result<(), VfsError> {
        let normalized = self.policy.sanitize_virtual_path(path, true)?;
        let prev_len = self.files.get(&normalized).map_or(0, Vec::len);
        let new_total = self.total_bytes.saturating_sub(prev_len) + content.len();

        if new_total > self.policy.max_vfs_bytes {
            return Err(VfsError::Policy(PolicyError::WriteQuotaExceeded {
                requested: new_total,
                max_allowed: self.policy.max_vfs_bytes,
            }));
        }

        self.total_bytes = new_total;
        self.files.insert(normalized, content.to_vec());
        Ok(())
    }

    pub fn total_bytes_used(&self) -> usize {
        self.total_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounded_output_buffer_truncates_log_bomb() {
        let mut buf = BoundedOutputBuffer::new(10);
        assert!(!buf.is_truncated());
        let written1 = buf.write_chunk(b"123456");
        assert!(!buf.is_truncated());
        let written2 = buf.write_chunk(b"7890ABCDEF");
        assert_eq!(written1, 6);
        assert_eq!(written2, 4);
        assert_eq!(buf.as_bytes(), b"1234567890");
        assert!(buf.is_truncated());
    }

    #[test]
    fn test_vfs_enforces_quota_and_default_deny() {
        let policy = SandboxPolicy::strict_default();
        let mut vfs = VirtualFileSystem::new(policy);
        assert!(matches!(
            vfs.write_file("/workspace/out.txt", b"hello"),
            Err(VfsError::Policy(PolicyError::VfsAccessDenied(_)))
        ));

        let mut rw_policy = SandboxPolicy::strict_default();
        rw_policy.allow_vfs_read = true;
        rw_policy.allow_vfs_write = true;
        rw_policy.max_vfs_bytes = 8;
        rw_policy.allowed_prefixes = vec!["/workspace".to_string()];

        let mut rw_vfs = VirtualFileSystem::new(rw_policy);
        rw_vfs.write_file("/workspace/a.txt", b"12345").unwrap();
        assert_eq!(rw_vfs.total_bytes_used(), 5);
        assert_eq!(rw_vfs.read_file("/workspace/a.txt").unwrap(), b"12345");

        // Overwrite existing file within quota
        rw_vfs.write_file("/workspace/a.txt", b"12345678").unwrap();
        assert_eq!(rw_vfs.total_bytes_used(), 8);

        // Exceed quota
        assert!(matches!(
            rw_vfs.write_file("/workspace/b.txt", b"9"),
            Err(VfsError::Policy(PolicyError::WriteQuotaExceeded { .. }))
        ));
    }
}
