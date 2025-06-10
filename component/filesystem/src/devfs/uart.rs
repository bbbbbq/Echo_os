use crate::path::Path;
use crate::vfs::VfsResult;
use crate::vfs::{DirEntry, FileAttr, FileSystem, FileType, Inode, VfsError};
use alloc::sync::Arc;
use alloc::vec::Vec;
use console::print;
pub struct UartDev {
    file_type: FileType,
}

impl UartDev {
    pub fn new() -> Self {
        Self {
            file_type: FileType::CharDevice,
        }
    }
}

impl Inode for UartDev {
    fn get_type(&self) -> VfsResult<FileType> {
        Ok(self.file_type)
    }

    fn read_at(&self, _offset: usize, _buf: &mut [u8]) -> VfsResult<usize> {
        // TODO: Implement actual UART read logic.
        // This would typically involve calling the UART driver.
        unimplemented!("UART read_at is not yet implemented")
    }
    fn write_at(&self, _offset: usize, buf: &[u8]) -> VfsResult<usize> {
        // Write data to UART by printing each byte
        for &byte in buf {
            print!("{}", byte as char);
        }
        // Return the number of bytes written
        Ok(buf.len())
    }

    fn mkdir_at(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::NotSupported) // UART is not a directory
    }

    fn rm_dir(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::NotSupported) // UART is not a directory
    }

    fn rm_file(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::NotSupported) // UART is a device node, not a regular file to be removed this way
    }

    fn lookup(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> {
        Err(VfsError::NotDirectory) // UART is not a directory
    }

    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        Err(VfsError::NotDirectory) // UART is not a directory
    }

    fn create_file(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::NotSupported) // Cannot create files "inside" a UART device node
    }

    fn truncate(&self, _size: usize) -> VfsResult<()> {
        Err(VfsError::NotSupported) // Truncation is not applicable to UART
    }

    fn flush(&self) -> VfsResult<()> {
        // If the UART has output buffers, they could be flushed here.
        // For a simple model, this can be a no-op.
        Ok(())
    }

    fn rename(&self, _new_name: &str) -> VfsResult<()> {
        Err(VfsError::NotSupported) // Renaming a device node like this is not typical
    }

    fn mount(&self, _fs: Arc<dyn FileSystem>, _path: Path) -> VfsResult<()> {
        Err(VfsError::NotSupported) // Cannot mount a filesystem on a UART device
    }

    fn umount(&self) -> VfsResult<()> {
        Err(VfsError::NotSupported) // UART is not a mount point
    }

    fn getattr(&self) -> VfsResult<FileAttr> {
        Ok(FileAttr {
            size: 0, // UART device size is typically 0
            file_type: self.file_type,
        })
    }
}
