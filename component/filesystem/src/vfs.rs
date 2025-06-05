use core::marker::Send;
use core::marker::Sync;
use core::result::Result;
use downcast_rs::{impl_downcast, DowncastSync};
use crate::path::Path;
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

#[derive(Debug)]
pub enum VfsError {
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
}

pub type VfsResult<T> = Result<T, VfsError>;

pub struct DirEntry {
    pub filename: String,
    pub len: usize,
    pub file_type: FileType,
}

pub trait Inode: DowncastSync + Send + Sync {
    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> VfsResult<usize> {
        unimplemented!()
    }
    fn write_at(&self, _offset: usize, buf: &[u8]) -> VfsResult<usize> {
        unimplemented!()
    }
    fn mkdir_at(&self, name: &str) -> VfsResult<()> {
        unimplemented!()
    }
    fn rm_dir(&self, name: &str) -> VfsResult<()> {
        unimplemented!()
    }
    fn rm_file(&self, name: &str) -> VfsResult<()> {
        unimplemented!()
    }
    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn Inode>> {
        unimplemented!()
    }
    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        unimplemented!()
    }
    fn create_file(&self, name: &str) -> VfsResult<()> {
        unimplemented!()
    }
    fn truncate(&self, _size: usize) -> VfsResult<()> {
        unimplemented!()
    }
    fn flush(&self) -> VfsResult<()> {
        unimplemented!()
    }
    fn rename(&self, name: &str) -> VfsResult<()> {
        unimplemented!()
    }
    fn mount(&self, fs: Arc<dyn FileSystem>, path: Path) -> VfsResult<()> {
        unimplemented!()
    }
    fn umount(&self) -> VfsResult<()> {
        unimplemented!()
    }
    fn get_type(&self) -> VfsResult<FileType> {
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
