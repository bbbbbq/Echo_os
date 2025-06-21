use crate::vfs::{DirEntry, FileAttr, FileSystem, FileType, FsType, Inode, VfsError, VfsResult};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

use super::meminfo::MemInfo;
use super::stat::StatInfo;
use super::self_dir::SelfDir;

#[derive(Debug)]
pub struct ProcFs {}

impl ProcFs {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ProcFileType {
    MemInfo,
    Stat,
    SelfDir,
}

#[derive(Debug)]
pub struct ProcFsDirInode {
    proc_type: ProcFileType,
}

impl ProcFsDirInode {
    pub fn new() -> Self {
        ProcFsDirInode {
            proc_type: ProcFileType::MemInfo,
        }
    }

    pub fn get_proc_type(&self) -> ProcFileType {
        self.proc_type
    }

    pub fn set_proc_type(&mut self, proc_type: ProcFileType) {
        self.proc_type = proc_type;
    }
}

impl Inode for ProcFsDirInode {
    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        Ok(vec![
            DirEntry {
                filename: String::from("meminfo"),
                len: 0,
                file_type: FileType::File,
            },
            DirEntry {
                filename: String::from("stat"),
                len: 0,
                file_type: FileType::File,
            },
            DirEntry {
                filename: String::from("self"),
                len: 0,
                file_type: FileType::Directory,
            },
        ])
    }

    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn Inode>> {
        match name {
            "meminfo" => Ok(Arc::new(MemInfo::new())),
            "stat" => Ok(Arc::new(StatInfo::new())),
            "self" => Ok(Arc::new(SelfDir::new())),
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

impl FileSystem for ProcFs {
    fn root_inode(&self) -> Option<Arc<dyn Inode>> {
        Some(Arc::new(ProcFsDirInode { proc_type: ProcFileType::MemInfo }))
    }

    fn get_type(&self) -> FsType {
        FsType::ProcFs
    }
} 