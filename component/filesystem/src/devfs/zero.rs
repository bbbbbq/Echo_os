use crate::path::Path; // Alias if Path is ambiguous with vfs::Path
use crate::vfs::{DirEntry, FileAttr, FileSystem, FileType, Inode, VfsError, VfsResult};
use alloc::sync::Arc;
use alloc::vec::Vec;

pub struct ZeroDev;

impl ZeroDev {
    pub fn new() -> Self {
        Self
    }
}

impl Inode for ZeroDev {
    fn get_type(&self) -> VfsResult<FileType> {
        Ok(FileType::CharDevice)
    }

    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> VfsResult<usize> {
        for byte in buf.iter_mut() {
            *byte = 0;
        }
        Ok(buf.len()) // Reading from /dev/zero provides an infinite stream of null bytes
    }

    fn write_at(&self, _offset: usize, buf: &[u8]) -> VfsResult<usize> {
        Ok(buf.len()) // Writing to /dev/zero succeeds but discards data (like /dev/null)
    }

    fn mkdir_at(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn rm_dir(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn rm_file(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn lookup(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> {
        Err(VfsError::NotDirectory)
    }

    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        Err(VfsError::NotDirectory)
    }

    fn create_file(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn truncate(&self, _size: usize) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn flush(&self) -> VfsResult<()> {
        Ok(())
    }

    fn rename(&self, _new_name: &str) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn mount(&self, _fs: Arc<dyn FileSystem>, _path: Path) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn umount(&self) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn getattr(&self) -> VfsResult<FileAttr> {
        Ok(FileAttr {
            size: 0, // /dev/zero is infinite, but getattr usually shows 0 for devices
            file_type: FileType::CharDevice,
        })
    }
}
