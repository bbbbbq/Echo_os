use crate::vfs::{FileType, VfsError, Inode, DirEntry, FileSystem, VfsResult, FileAttr};
use crate::path::Path; // Alias if Path is ambiguous with vfs::Path
use alloc::sync::Arc;
use alloc::vec::Vec;

pub struct NullDev;

impl NullDev {
    pub fn new() -> Self {
        Self
    }
}

impl Inode for NullDev {
    fn get_type(&self) -> VfsResult<FileType> {
        Ok(FileType::CharDevice)
    }

    fn read_at(&self, _offset: usize, _buf: &mut [u8]) -> VfsResult<usize> {
        Ok(0) // Reading from /dev/null always returns EOF (0 bytes read)
    }

    fn write_at(&self, _offset: usize, buf: &[u8]) -> VfsResult<usize> {
        Ok(buf.len()) // Writing to /dev/null succeeds but discards data
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
            size: 0,
            file_type: FileType::CharDevice,
        })
    }
}
