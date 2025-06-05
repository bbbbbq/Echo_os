use alloc::string::String; // Import String
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::vec; // Import the vec! macro
use crate::vfs::{DirEntry, FileSystem, FsType, Inode, VfsResult, FileType, VfsError};
use crate::path::Path; // For Inode::mount method signature
use spin::Mutex; // Import Mutex
use lazy_static::lazy_static;
use super::null::NullDev;
use super::zero::ZeroDev;
use super::uart::UartDev;



#[derive(Debug)]
pub struct DevFs {}

impl DevFs {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Debug)]
pub struct DevFsDirInode; // Represents the /dev directory

impl DevFsDirInode {
    pub fn new() -> Self {
        DevFsDirInode
    }
}

impl Inode for DevFsDirInode {
    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        // TODO: List actual devices. For now, return a fixed list or empty.
        Ok(vec![
            DirEntry {
                filename: String::from("null"),
                len: 0, // Device files often have 0 size in directory listings
                file_type: FileType::CharDevice,
            },
            DirEntry {
                filename: String::from("zero"),
                len: 0,
                file_type: FileType::CharDevice,
            },
            DirEntry {
                filename: String::from("uart"),
                len: 0,
                file_type: FileType::CharDevice,
            },
        ])
    }

    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn Inode>> {
        match name {
            "null" => Ok(Arc::new(NullDev::new())),
            "zero" => Ok(Arc::new(ZeroDev::new())),
            "uart" => Ok(Arc::new(UartDev::new())),
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
}

impl FileSystem for DevFs {
    fn root_inode(&self) -> Option<Arc<dyn Inode>> {
        Some(Arc::new(DevFsDirInode))
    }

    fn get_type(&self) -> FsType {
        FsType::DevFs
    }
}
