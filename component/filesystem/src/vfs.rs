use crate::path::Path;
use core::marker::Send;
use core::marker::Sync;
use core::result::Result;
use struct_define::poll_event::PollEvent;
use downcast_rs::{DowncastSync, impl_downcast};
extern crate alloc;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FileType {
    File,
    Directory,
    CharDevice,
    BlockDevice,
    Pipe,
    SymLink,
    Socket,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsError {
    InvalidPath,
    NotFound,
    AlreadyExists,
    InvalidArgument,
    PermissionDenied,
    IoError,
    NotDirectory,
    NotFile,
    NotSupported,
    OutOfMemory,
    OutOfSpace,
    IsDirectory,
    NotEmpty,
    Busy,
    BadFileDescriptor,
    InvalidOperation,
}



pub type VfsResult<T> = Result<T, VfsError>;

pub struct FileAttr {
    pub size: usize,
    pub file_type: FileType,
    pub nlinks: u32,
    pub uid: u32,
    pub gid: u32,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub blk_size: u32,
    pub blocks: u32,
}

pub struct DirEntry {
    pub filename: String,
    pub len: usize,
    pub file_type: FileType,
}

pub trait Inode: DowncastSync + Send + Sync + core::fmt::Debug {
    fn read_at(&self, _offset: usize, _buf: &mut [u8]) -> VfsResult<usize> {
        unimplemented!()
    }
    fn write_at(&self, _offset: usize, _buf: &[u8]) -> VfsResult<usize> {
        unimplemented!()
    }
    fn mkdir_at(&self, _name: &str) -> VfsResult<()> {
        unimplemented!()
    }
    fn rm_dir(&self, _name: &str) -> VfsResult<()> {
        unimplemented!()
    }
    fn rm_file(&self, _name: &str) -> VfsResult<()> {
        unimplemented!()
    }
    fn lookup(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> {
        unimplemented!()
    }
    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        unimplemented!()
    }
    fn create_file(&self, _name: &str) -> VfsResult<()> {
        unimplemented!()
    }
    fn truncate(&self, _size: usize) -> VfsResult<()> {
        unimplemented!()
    }
    fn flush(&self) -> VfsResult<()> {
        Ok(())
    }
    fn rename(&self, _name: &str) -> VfsResult<()> {
        unimplemented!()
    }
    fn mount(&self, _fs: Arc<dyn FileSystem>, _path: Path) -> VfsResult<()> {
        unimplemented!()
    }
    fn umount(&self) -> VfsResult<()> {
        unimplemented!()
    }
    fn getattr(&self) -> VfsResult<FileAttr> {
        Err(VfsError::NotSupported)
    }

    fn get_type(&self) -> VfsResult<FileType> {
        Err(VfsError::NotSupported)
    }

    fn poll(&self, _event: PollEvent) -> VfsResult<PollEvent> {
        Err(VfsError::NotSupported)
    }
}

impl_downcast!(Inode);

pub enum FsType {
    Ext4fs,
    Tmpfs,
    DevFs,
}

pub trait FileSystem: Send + Sync {
    fn root_inode(&self) -> Option<Arc<dyn Inode>>;
    fn get_type(&self) -> FsType;
}
