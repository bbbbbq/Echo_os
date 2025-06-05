use super::vfs::{OpenFlags,Inode};
use alloc::sync::Arc;
use crate::vfs::VfsResult;
use crate::vfs::DirEntry;
use alloc::vec::Vec;

pub struct File {
    pub inner: Arc<dyn Inode>,
    pub openflages: OpenFlags,
    pub offset: usize,
}

impl File {
    pub fn new(inner: Arc<dyn Inode>, openflages: OpenFlags) -> Self {
        Self { inner, openflages, offset: 0 }
    }

    pub fn read_at(&self, buf: &mut [u8]) -> VfsResult<usize> {
        if !self.openflages.contains(OpenFlags::O_RDONLY) && !self.openflages.contains(OpenFlags::O_RDWR) {
            return Err(crate::vfs::VfsError::PermissionDenied);
        }
        self.inner.read_at(self.offset, buf)
    }

    pub fn write_at(&self, buf: &[u8]) -> VfsResult<usize> {
        if !self.openflages.contains(OpenFlags::O_WRONLY) && !self.openflages.contains(OpenFlags::O_RDWR) {
            return Err(crate::vfs::VfsError::PermissionDenied);
        }
        self.inner.write_at(self.offset, buf)
    }

    pub fn mkdir_at(&self, name: &str) -> VfsResult<()> {
        if !self.openflages.contains(OpenFlags::O_WRONLY) && !self.openflages.contains(OpenFlags::O_RDWR) {
            return Err(crate::vfs::VfsError::PermissionDenied);
        }
        self.inner.mkdir_at(name)
    }

    pub fn rm_dir(&self, name: &str) -> VfsResult<()> {
        if !self.openflages.contains(OpenFlags::O_WRONLY) && !self.openflages.contains(OpenFlags::O_RDWR) {
            return Err(crate::vfs::VfsError::PermissionDenied);
        }
        self.inner.rm_dir(name)
    }

    pub fn rm_file(&self, name: &str) -> VfsResult<()> {
        if !self.openflages.contains(OpenFlags::O_WRONLY) && !self.openflages.contains(OpenFlags::O_RDWR) {
            return Err(crate::vfs::VfsError::PermissionDenied);
        }
        self.inner.rm_file(name)
    }

    pub fn lookup(&self, name: &str) -> VfsResult<Arc<dyn Inode>> {
        if !self.openflages.contains(OpenFlags::O_RDONLY) && !self.openflages.contains(OpenFlags::O_RDWR) {
            return Err(crate::vfs::VfsError::PermissionDenied);
        }
        self.inner.lookup(name)
    }

    pub fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        if !self.openflages.contains(OpenFlags::O_RDONLY) && !self.openflages.contains(OpenFlags::O_RDWR) {
            return Err(crate::vfs::VfsError::PermissionDenied);
        }
        self.inner.read_dir()
    }

    pub fn create_file(&self, name: &str) -> VfsResult<()> {
        if !self.openflages.contains(OpenFlags::O_WRONLY) && !self.openflages.contains(OpenFlags::O_RDWR) {
            return Err(crate::vfs::VfsError::PermissionDenied);
        }
        self.inner.create_file(name)
    }

    pub fn truncate(&self, size: usize) -> VfsResult<()> {
        if !self.openflages.contains(OpenFlags::O_WRONLY) && !self.openflages.contains(OpenFlags::O_RDWR) {
            return Err(crate::vfs::VfsError::PermissionDenied);
        }
        self.inner.truncate(size)
    }

    pub fn flush(&self) -> VfsResult<()> {
        if !self.openflages.contains(OpenFlags::O_WRONLY) && !self.openflages.contains(OpenFlags::O_RDWR) {
            return Err(crate::vfs::VfsError::PermissionDenied);
        }
        self.inner.flush()
    }
}