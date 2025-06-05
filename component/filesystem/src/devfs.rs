use alloc::string::String; // Import String
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::vec; // Import the vec! macro
use super::vfs::{DirEntry, FileSystem, FsType, Inode, VfsResult, FileType};
use super::path::Path; // For Inode::mount method signature
use spin::Mutex; // Import Mutex
use lazy_static::lazy_static;

lazy_static!
{
    pub static ref DEV_ROOT_ID: Mutex<Arc<DevFsDirInode>> = Mutex::new(Arc::new(DevFsDirInode::new()));
}

#[derive(Debug)]
pub struct DevFs {
    // For now, DevFs might not need to store files directly here if it dynamically lists devices.
    // If you plan to have static entries like /dev/null, /dev/zero, they could be represented here
    // or handled entirely within DevFsDirInode and DevFsDeviceInode.
    // For simplicity, let's assume it's stateless for now, or its state is implicitly managed by device drivers.
}

impl DevFs {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Debug)]
pub struct DevFsDirInode; // Represents the /dev directory

impl DevFsDirInode {
    pub fn new() -> Self {
        DevFsDirInode
    }
}

impl Inode for DevFsDirInode {
    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        // TODO: List actual devices. For now, return a fixed list or empty.
        Ok(vec![
            DirEntry {
                filename: String::from("null"),
                len: 0, // Device files often have 0 size in directory listings
                file_type: FileType::CharDevice,
            },
            DirEntry {
                filename: String::from("zero"),
                len: 0,
                file_type: FileType::CharDevice,
            },
        ])
    }

    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn Inode>> {
        match name {
            "null" => Ok(Arc::new(DevFsDeviceInode::new(DeviceNodeType::Null))),
            "zero" => Ok(Arc::new(DevFsDeviceInode::new(DeviceNodeType::Zero))),
            _ => Err(super::vfs::VfsError::NotFound),
        }
    }

    fn read_at(&self, _offset: usize, _buf: &mut [u8]) -> VfsResult<usize> {
        Err(super::vfs::VfsError::IsDirectory)
    }

    fn write_at(&self, _offset: usize, _buf: &[u8]) -> VfsResult<usize> {
        Err(super::vfs::VfsError::IsDirectory)
    }

    fn mkdir_at(&self, _name: &str) -> VfsResult<()> {
        Err(super::vfs::VfsError::NotSupported)
    }

    fn rm_dir(&self, _name: &str) -> VfsResult<()> {
        Err(super::vfs::VfsError::NotSupported)
    }

    fn rm_file(&self, _name: &str) -> VfsResult<()> {
        Err(super::vfs::VfsError::NotSupported)
    }

    fn create_file(&self, _name: &str) -> VfsResult<()> {
        Err(super::vfs::VfsError::NotSupported)
    }

    fn truncate(&self, _size: usize) -> VfsResult<()> {
        Err(super::vfs::VfsError::NotSupported)
    }
}

impl FileSystem for DevFs {
    fn root_inode(&self) -> Option<Arc<dyn Inode>> {
        Some(Arc::new(DevFsDirInode))
    }

    fn get_type(&self) -> FsType {
        FsType::DevFs
    }
}


#[derive(Debug, Clone, Copy)]
enum DeviceNodeType {
    Null,
    Zero,
}

#[derive(Debug)]
struct DevFsDeviceInode {
    device_type: DeviceNodeType,
}

impl DevFsDeviceInode {
    fn new(device_type: DeviceNodeType) -> Self {
        Self { device_type }
    }
}

impl Inode for DevFsDeviceInode {
    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> VfsResult<usize> {
        match self.device_type {
            DeviceNodeType::Null => Ok(0), // EOF
            DeviceNodeType::Zero => {
                for byte in buf.iter_mut() {
                    *byte = 0;
                }
                Ok(buf.len())
            }
        }
    }

    fn write_at(&self, _offset: usize, buf: &[u8]) -> VfsResult<usize> {
        match self.device_type {
            DeviceNodeType::Null | DeviceNodeType::Zero => {
                // Data is discarded, but report bytes "written"
                Ok(buf.len())
            }
        }
    }

    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        Err(super::vfs::VfsError::NotDirectory)
    }

    fn lookup(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> {
        Err(super::vfs::VfsError::NotDirectory)
    }

    fn mkdir_at(&self, _name: &str) -> VfsResult<()> {
        Err(super::vfs::VfsError::NotDirectory)
    }

    fn create_file(&self, _name: &str) -> VfsResult<()> {
        Err(super::vfs::VfsError::NotSupported) // Cannot create files "in" a device node
    }

    fn rm_dir(&self, _name: &str) -> VfsResult<()> {
        Err(super::vfs::VfsError::NotDirectory)
    }

    fn rm_file(&self, _name: &str) -> VfsResult<()> {
        Err(super::vfs::VfsError::NotSupported) // Device nodes are not removed like regular files
    }

    fn truncate(&self, _size: usize) -> VfsResult<()> {
        Err(super::vfs::VfsError::NotSupported)
    }

    fn flush(&self) -> VfsResult<()> {
        Ok(()) // For /dev/null and /dev/zero, flush is a no-op
    }

    fn rename(&self, _new_name: &str) -> VfsResult<()> {
        Err(super::vfs::VfsError::NotSupported)
    }

    fn mount(&self, _fs: Arc<dyn FileSystem>, _path: Path) -> VfsResult<()> {
        Err(super::vfs::VfsError::NotSupported)
    }

    fn umount(&self) -> VfsResult<()> {
        Err(super::vfs::VfsError::NotSupported)
    }
}
