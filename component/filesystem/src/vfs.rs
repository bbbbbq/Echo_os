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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct OpenFlags(pub u32);

impl OpenFlags {
    // Access mode flags
    pub const O_RDONLY: Self = Self(0o0);
    pub const O_WRONLY: Self = Self(0o1);
    pub const O_RDWR: Self = Self(0o2);

    // Creation and file status flags
    pub const O_CREAT: Self = Self(0o100);
    pub const O_EXCL: Self = Self(0o200);
    pub const O_NOCTTY: Self = Self(0o400);
    pub const O_TRUNC: Self = Self(0o1000);
    pub const O_APPEND: Self = Self(0o2000);
    pub const O_NONBLOCK: Self = Self(0o4000);
    pub const O_DSYNC: Self = Self(0o10000);
    pub const O_SYNC: Self = Self(0o4010000);
    pub const O_RSYNC: Self = Self(0o4010000);
    pub const O_DIRECTORY: Self = Self(0o200000);
    pub const O_NOFOLLOW: Self = Self(0o400000);
    pub const O_CLOEXEC: Self = Self(0o2000000);
    pub const O_DIRECT: Self = Self(0o40000);
    pub const O_NOATIME: Self = Self(0o1000000);
    pub const O_PATH: Self = Self(0o10000000);
    pub const O_TMPFILE: Self = Self(0o20200000);
}

impl core::ops::BitOr for OpenFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl OpenFlags {
    pub fn new_read_write() -> Self
    {
        let inner: u32 = Self::O_RDONLY.0 | Self::O_WRONLY.0;
        Self(inner)
    }

    pub fn is_readable(&self) -> bool {
        let mode = self.0 & 0o3;
        mode == Self::O_RDONLY.0 || mode == Self::O_RDWR.0
    }

    pub fn is_writable(&self) -> bool {
        let mode = self.0 & 0o3;
        mode == Self::O_WRONLY.0 || mode == Self::O_RDWR.0
    }

    pub fn contains(&self, flags: OpenFlags) -> bool {
        (self.0 & flags.0) == flags.0
    }

    pub fn from_bits_truncate(bits: u32) -> Self {
        Self(bits)
    }
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

    fn poll(&self, event: PollEvent) -> VfsResult<PollEvent> {
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
