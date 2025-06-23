//! /dev/vda 设备节点实现
//!
//! 提供块设备节点的基本接口。

use crate::vfs::{FileType, Inode, VfsResult};

/// /dev/vda 设备结构体。
#[derive(Debug)]
pub struct VdaDev {
    file_type: FileType,
}

impl VdaDev {
    /// 创建新的 VdaDev 实例。
    pub fn new() -> Self {
        Self {
            file_type: FileType::BlockDevice,
        }
    }
}

impl Inode for VdaDev {
    /// 获取设备类型。
    fn get_type(&self) -> VfsResult<FileType> {
        Ok(self.file_type)
    }
    
    fn read_at(&self, _offset: usize, _buf: &mut [u8]) -> VfsResult<usize> {
        core::unimplemented!()
    }
    
    fn write_at(&self, _offset: usize, _buf: &[u8]) -> VfsResult<usize> {
        core::unimplemented!()
    }
    
    fn mkdir_at(&self, _name: &str) -> VfsResult<()> {
        core::unimplemented!()
    }
    
    fn rm_dir(&self, _name: &str) -> VfsResult<()> {
        core::unimplemented!()
    }
    
    fn rm_file(&self, _name: &str) -> VfsResult<()> {
        core::unimplemented!()
    }
    
    fn lookup(&self, _name: &str) -> VfsResult<alloc::sync::Arc<dyn Inode>> {
        core::unimplemented!()
    }
    
    fn read_dir(&self) -> VfsResult<alloc::vec::Vec<crate::vfs::DirEntry>> {
        core::unimplemented!()
    }
    
    fn create_file(&self, _name: &str) -> VfsResult<()> {
        core::unimplemented!()
    }
    
    fn truncate(&self, _size: usize) -> VfsResult<()> {
        core::unimplemented!()
    }
    
    /// 刷新操作为无操作。
    fn flush(&self) -> VfsResult<()> {
        Ok(())
    }
    
    fn rename(&self, _name: &str) -> VfsResult<()> {
        core::unimplemented!()
    }
    
    fn mount(&self, _fs: alloc::sync::Arc<dyn crate::vfs::FileSystem>, _path: crate::path::Path) -> VfsResult<()> {
        core::unimplemented!()
    }
    
    fn umount(&self) -> VfsResult<()> {
        core::unimplemented!()
    }
    
    fn getattr(&self) -> VfsResult<crate::vfs::FileAttr> {
        Err(crate::vfs::VfsError::NotSupported)
    }
    
    fn poll(&self, _event: struct_define::poll_event::PollEvent) -> VfsResult<struct_define::poll_event::PollEvent> {
        Err(crate::vfs::VfsError::NotSupported)
    }
}