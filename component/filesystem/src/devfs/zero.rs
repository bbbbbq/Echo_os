//! /dev/zero 设备节点实现
//!
//! 提供无限零字节流的读写行为，兼容Unix语义。

use crate::path::Path; // Alias if Path is ambiguous with vfs::Path
use crate::vfs::{DirEntry, FileAttr, FileSystem, FileType, Inode, VfsError, VfsResult};
use alloc::sync::Arc;
use alloc::vec::Vec;

/// /dev/zero 设备结构体。
#[derive(Debug)]
pub struct ZeroDev;

impl ZeroDev {
    /// 创建新的 ZeroDev 实例。
    pub fn new() -> Self {
        Self
    }
}

impl Inode for ZeroDev {
    /// 获取设备类型。
    fn get_type(&self) -> VfsResult<FileType> {
        Ok(FileType::CharDevice)
    }

    /// 读取时填充缓冲区为0，返回读取字节数。
    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> VfsResult<usize> {
        for byte in buf.iter_mut() {
            *byte = 0;
        }
        Ok(buf.len()) // Reading from /dev/zero provides an infinite stream of null bytes
    }

    /// 写入总是成功但丢弃数据。
    fn write_at(&self, _offset: usize, buf: &[u8]) -> VfsResult<usize> {
        Ok(buf.len()) // Writing to /dev/zero succeeds but discards data (like /dev/null)
    }

    // 其余方法均返回不支持或非目录。
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

    /// 刷新操作为无操作。
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

    /// 获取文件属性。
    fn getattr(&self) -> VfsResult<FileAttr> {
        Ok(FileAttr {
            size: 0, // /dev/zero is infinite, but getattr usually shows 0 for devices
            file_type: FileType::CharDevice,
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
