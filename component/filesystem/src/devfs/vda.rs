use crate::vfs::{FileType, Inode, VfsResult};

#[derive(Debug)]
pub struct VdaDev {
    file_type: FileType,
}

impl VdaDev {
    pub fn new() -> Self {
        Self {
            file_type: FileType::BlockDevice,
        }
    }
}

impl Inode for VdaDev {
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