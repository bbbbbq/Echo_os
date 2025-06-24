use crate::path::Path;
use crate::vfs::{DirEntry, FileAttr, FileSystem, FileType, Inode, VfsError, VfsResult};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

#[derive(Debug)]
pub struct SelfDir;

impl SelfDir {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug)]
pub struct SelfExeSymlink;

impl SelfExeSymlink {
    pub fn new() -> Self {
        Self
    }
}

impl Inode for SelfDir {
    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        Ok(vec![
            DirEntry {
                filename: String::from("exe"),
                len: 0,
                file_type: FileType::SymLink,
            },
            // Add more entries as needed
        ])
    }

    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn Inode>> {
        match name {
            "exe" => Ok(Arc::new(SelfExeSymlink::new())),
            _ => Err(VfsError::NotFound),
        }
    }

    fn read_at(&self, _offset: usize, _buf: &mut [u8]) -> VfsResult<usize> {
        Err(VfsError::IsDirectory)
    }

    fn write_at(&self, _offset: usize, _buf: &[u8]) -> VfsResult<usize> {
        Err(VfsError::IsDirectory)
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

    fn create_file(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn truncate(&self, _size: usize) -> VfsResult<()> {
        Err(VfsError::NotSupported)
    }

    fn getattr(&self) -> VfsResult<FileAttr> {
        Ok(FileAttr {
            size: 0,
            file_type: FileType::Directory,
            nlinks: 1,
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            blk_size: 512,
            blocks: 0,
        })
    }
}

// Implementation for the symlink to the executable
impl Inode for SelfExeSymlink {
    fn get_type(&self) -> VfsResult<FileType> {
        Ok(FileType::SymLink)
    }

    fn read_at(&self, offset: usize, buf: &mut [u8]) -> VfsResult<usize> {
        // This returns the path to the current executable
        // In a real implementation, you would look up the actual executable path
        let path = "/bin/current_executable";
        let path_bytes = path.as_bytes();
        
        if offset >= path_bytes.len() {
            return Ok(0);
        }
        
        let available_bytes = path_bytes.len() - offset;
        let bytes_to_copy = core::cmp::min(available_bytes, buf.len());
        
        buf[..bytes_to_copy].copy_from_slice(&path_bytes[offset..offset + bytes_to_copy]);
        
        Ok(bytes_to_copy)
    }

    fn write_at(&self, _offset: usize, _buf: &[u8]) -> VfsResult<usize> {
        Err(VfsError::NotSupported)
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

    fn getattr(&self) -> VfsResult<FileAttr> {
        let path = "/bin/current_executable";
        Ok(FileAttr {
            size: path.len(),
            file_type: FileType::SymLink,
            nlinks: 1,
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            blk_size: 512,
            blocks: ((path.len() + 511) / 512) as u32,
        })
    }
} 