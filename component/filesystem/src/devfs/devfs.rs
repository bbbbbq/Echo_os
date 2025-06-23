//! DevFs 设备文件系统实现
//!
//! 提供 /dev 目录的虚拟设备节点支持。

use crate::vfs::{DirEntry, FileAttr, FileSystem, FileType, FsType, Inode, VfsError, VfsResult};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

use super::null::NullDev;
use super::uart::UartDev;
use super::zero::ZeroDev;

/// 设备文件系统主结构体。
#[derive(Debug)]
pub struct DevFs {}

impl DevFs {
    /// 创建新的DevFs实例。
    pub fn new() -> Self {
        Self {}
    }
}

/// 支持的设备类型。
#[derive(Debug, Clone, Copy)]
pub enum DevType {
    Null,
    Zero,
    Uart,
    Block,
}

/// DevFs 目录Inode实现。
#[derive(Debug)]
pub struct DevFsDirInode
{
    dev_type:DevType,
}

impl DevFsDirInode {
    /// 创建新的目录Inode。
    pub fn new() -> Self {
        DevFsDirInode{
            dev_type:DevType::Null,
        }
    }

    /// 获取设备类型。
    pub fn get_dev_type(&self) -> DevType {
        self.dev_type
    }

    /// 设置设备类型。
    pub fn set_dev_type(&mut self, dev_type: DevType) {
        self.dev_type = dev_type;
    }
}

impl Inode for DevFsDirInode {
    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        Ok(vec![
            DirEntry {
                filename: String::from("null"),
                len: 0,
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

impl FileSystem for DevFs {
    fn root_inode(&self) -> Option<Arc<dyn Inode>> {
        Some(Arc::new(DevFsDirInode { dev_type: DevType::Null }))
    }

    fn get_type(&self) -> FsType {
        FsType::DevFs
    }
}
