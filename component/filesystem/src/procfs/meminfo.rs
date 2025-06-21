use crate::path::Path;
use crate::vfs::{DirEntry, FileAttr, FileSystem, FileType, Inode, VfsError, VfsResult};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

#[derive(Debug)]
pub struct MemInfo;

impl MemInfo {
    pub fn new() -> Self {
        Self
    }

    fn generate_meminfo_content() -> String {
        // 返回一个非常简单的内存信息，避免生成过长的字符串
        String::from(
            "MemTotal:        1024 kB\n\
             MemFree:          512 kB\n\
             Buffers:           32 kB\n\
             Cached:            64 kB\n"
        )
    }
}

impl Inode for MemInfo {
    fn get_type(&self) -> VfsResult<FileType> {
        Ok(FileType::File)
    }

    fn read_at(&self, offset: usize, buf: &mut [u8]) -> VfsResult<usize> {
        let content = Self::generate_meminfo_content();
        let content_bytes = content.as_bytes();
        
        // Handle the case where offset is beyond the content length
        if offset >= content_bytes.len() {
            return Ok(0); // End of file
        }
        
        // Calculate how many bytes we can actually copy
        let available_bytes = content_bytes.len() - offset;
        let bytes_to_copy = core::cmp::min(available_bytes, buf.len());
        
        // Copy the bytes to the output buffer
        buf[..bytes_to_copy].copy_from_slice(&content_bytes[offset..offset + bytes_to_copy]);
        
        Ok(bytes_to_copy)
    }

    fn write_at(&self, _offset: usize, _buf: &[u8]) -> VfsResult<usize> {
        Err(VfsError::NotSupported) // meminfo is read-only
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

    fn getattr(&self) -> VfsResult<FileAttr> {
        let content = Self::generate_meminfo_content();
        Ok(FileAttr {
            size: content.len(),
            file_type: FileType::File,
            nlinks: 1,
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            blk_size: 512,
            blocks: ((content.len() + 511) / 512) as u32,
        })
    }
} 