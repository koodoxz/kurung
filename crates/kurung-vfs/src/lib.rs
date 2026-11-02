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
            data: Vec::new(),
            max_bytes,
            truncated: false,
        }
    }

    pub fn write_chunk(&mut self, _chunk: &[u8]) -> usize {
        // RED stub
        0
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

    pub fn mount_readonly_file(&mut self, _path: &str, _content: Vec<u8>) -> Result<(), VfsError> {
        Ok(())
    }

    pub fn read_file(&self, path: &str) -> Result<&[u8], VfsError> {
        Err(VfsError::FileNotFound(path.to_string()))
    }

    pub fn write_file(&mut self, path: &str, _content: &[u8]) -> Result<(), VfsError> {
        Err(VfsError::FileNotFound(path.to_string()))
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
        let written1 = buf.write_chunk(b"123456");
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
    }
}

