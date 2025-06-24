//! 虚拟文件系统（VFS）核心接口与类型定义
//!
//! 提供文件类型、错误类型、Inode与文件系统trait等抽象。

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

<<<<<<< HEAD
/// 文件类型枚举。
=======
// 添加 SeekFrom 枚举定义
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SeekFrom {
    SET(usize),
    CURRENT(isize),
    END(isize),
}

// 修改 Dirent64 结构体定义，对应 Linux 的 struct linux_dirent64
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Dirent64<'a> {
    pub d_ino: u64,     // 64 位 inode 编号
    pub d_off: i64,     // 下一个条目的偏移量
    pub d_reclen: u16,  // 当前条目的总长度
    pub d_type: u8,     // 文件类型（DT_REG=普通文件，DT_DIR=目录等）
    pub d_name: &'a [u8], // 文件名（以 null 结尾，动态长度）
}

impl Default for Dirent64<'_> {
    fn default() -> Self {
        Self {
            d_ino: 0,
            d_off: 0,
            d_reclen: 0,
            d_type: 0,
            d_name: &[0; 0],
        }
    }
}

>>>>>>> 73599fce51808454c7e446d9fc82074df6e31d3d
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

/// 虚拟文件系统错误类型。
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
    Again,
}

<<<<<<< HEAD
/// VFS操作结果类型。
=======
>>>>>>> 73599fce51808454c7e446d9fc82074df6e31d3d
pub type VfsResult<T> = Result<T, VfsError>;

/// 文件属性结构体。
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
<<<<<<< HEAD

/// 目录项结构体。
=======
#[derive(Debug, Clone)]
>>>>>>> 73599fce51808454c7e446d9fc82074df6e31d3d
pub struct DirEntry {
    pub filename: String,
    pub len: usize,
    pub file_type: FileType,
}

<<<<<<< HEAD
/// Inode trait，所有文件/目录/设备节点需实现。
=======
impl DirEntry {
    pub fn new(filename: String, len: usize, file_type: FileType) -> Self {
        Self {
            filename,
            len,
            file_type,
        }
    }
    
    pub fn get_filename(&self) -> &str {
        &self.filename
    }
    
    pub fn get_len(&self) -> usize {
        self.len
    }
    
    pub fn get_file_type(&self) -> FileType {
        self.file_type
    }

    pub fn convert_to_dirent64(&'_ self) -> Dirent64<'_> {
        let d_type = match self.file_type {
            FileType::File => 1,
            FileType::Directory => 2,
            FileType::CharDevice => 3,
            FileType::BlockDevice => 4,
            FileType::Pipe => 5,
            FileType::Socket => 6,
            FileType::SymLink => 7,
            FileType::Unknown => 0,
        };

        // 文件名字节切片（不包含结尾的 0 字节）。
        let name_bytes = self.filename.as_bytes();
        // +1 用于尾部的 0 字节，对齐 Linux 的 dirent64 定义
        let reclen = (core::mem::size_of::<Dirent64>() + name_bytes.len() + 1) as u16;

        Dirent64 {
            d_ino: 1,   // TODO: 使用真实 inode 编号
            d_off: 0,
            d_reclen: reclen,
            d_type,
            d_name: name_bytes,
        }
    }
}

>>>>>>> 73599fce51808454c7e446d9fc82074df6e31d3d
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

    fn ioctl(&self, _command: usize, _arg: usize) -> VfsResult<usize> {
        Err(VfsError::NotSupported)
    }
}

impl_downcast!(Inode);

/// 文件系统类型枚举。
pub enum FsType {
    Ext4fs,
    Tmpfs,
    DevFs,
}

/// 文件系统trait，所有文件系统需实现。
pub trait FileSystem: Send + Sync {
    fn root_inode(&self) -> Option<Arc<dyn Inode>>;
    fn get_type(&self) -> FsType;
}
