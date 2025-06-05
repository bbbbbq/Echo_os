use crate::vfs::{Inode, OpenFlags};
use crate::path::Path;
use alloc::sync::Arc;
use crate::vfs::VfsResult;
use crate::vfs::DirEntry;
use alloc::vec::Vec;
use crate::mount::get_mount_node;


pub struct File {
    pub inner: Arc<dyn Inode>,
    pub openflags: OpenFlags,
    pub offset: usize,
}

impl File {
    pub fn open(path: Path, flags: OpenFlags) -> VfsResult<Self> {
        let (resolved_mount_path, mount_node) = match get_mount_node(path.clone()) {
            Some((p, node)) => (p, node),
            None => return Err(crate::vfs::VfsError::NotFound),
        };
        
        let root_inode = mount_node.get_inode();
        
        let full_path_str = path.to_string();
        let mount_point_str = resolved_mount_path.to_string();

        // Calculate the relative path string within the mounted filesystem
        let relative_path_str_intermediate = if mount_point_str == "/" {
            if full_path_str.starts_with('/') {
                &full_path_str[1..] // e.g., "foo/bar" from "/foo/bar"
            } else {
                &full_path_str // Should ideally not happen if paths are always absolute-like
            }
        } else {
            // e.g., for full_path_str="/dev/null" and mount_point_str="/dev", this yields "/null"
            // for full_path_str="/dev" and mount_point_str="/dev", this yields ""
            full_path_str.strip_prefix(&mount_point_str).unwrap_or(&full_path_str)
        };

        // Clean up: remove leading slash from relative_path_str_intermediate if it's not the only character
        let final_relative_path_str = if relative_path_str_intermediate.starts_with('/') && relative_path_str_intermediate.len() > 1 {
            &relative_path_str_intermediate[1..] // e.g., "null" from "/null"
        } else if relative_path_str_intermediate == "/" { // Handles case where relative path itself is root of mounted fs
            "" // An empty relative path means the root of the mounted filesystem
        } else {
            relative_path_str_intermediate
        };

        let components: Vec<&str> = final_relative_path_str.split('/').filter(|s| !s.is_empty()).collect();
        // Example: path="/dev/null", mount_node.path="/dev" -> final_relative_path_str="null", components=["null"]
        // Example: path="/hello.txt", mount_node.path="/" -> final_relative_path_str="hello.txt", components=["hello.txt"]
        // Example: path="/dev", mount_node.path="/dev" -> final_relative_path_str="", components=[]

        let mut current_inode = root_inode; // This is the root inode of the MOUNTED filesystem (e.g., DevFsDirInode)

        if !components.is_empty() {
            for component in components {
                // component is now relative to the mount_node's root_inode
                // e.g., for /dev/null, component will be "null"
                match current_inode.lookup(component) {
                    Ok(inode) => current_inode = inode,
                    Err(e) => return Err(e),
                }
            }
        }
        // If components is empty, current_inode remains the root_inode of the mounted_fs, which is correct.
        
        Ok(Self {
            inner: current_inode,
            openflags: flags,
            offset: 0
        })
    }

    pub fn new(inner: Arc<dyn Inode>, openflags: OpenFlags) -> Self {
        Self { inner, openflags, offset: 0 }
    }

    pub fn read_at(&self, buf: &mut [u8]) -> VfsResult<usize> {
        if !self.openflags.contains(OpenFlags::O_RDONLY) && !self.openflags.contains(OpenFlags::O_RDWR) {
            return Err(crate::vfs::VfsError::PermissionDenied);
        }
        self.inner.read_at(self.offset, buf)
    }

    pub fn write_at(&self, buf: &[u8]) -> VfsResult<usize> {
        if !self.openflags.contains(OpenFlags::O_WRONLY) && !self.openflags.contains(OpenFlags::O_RDWR) {
            return Err(crate::vfs::VfsError::PermissionDenied);
        }
        self.inner.write_at(self.offset, buf)
    }

    pub fn mkdir_at(&self, name: &str) -> VfsResult<()> {
        if !self.openflags.contains(OpenFlags::O_WRONLY) && !self.openflags.contains(OpenFlags::O_RDWR) {
            return Err(crate::vfs::VfsError::PermissionDenied);
        }
        self.inner.mkdir_at(name)
    }

    pub fn rm_dir(&self, name: &str) -> VfsResult<()> {
        if !self.openflags.contains(OpenFlags::O_WRONLY) && !self.openflags.contains(OpenFlags::O_RDWR) {
            return Err(crate::vfs::VfsError::PermissionDenied);
        }
        self.inner.rm_dir(name)
    }

    pub fn rm_file(&self, name: &str) -> VfsResult<()> {
        if !self.openflags.contains(OpenFlags::O_WRONLY) && !self.openflags.contains(OpenFlags::O_RDWR) {
            return Err(crate::vfs::VfsError::PermissionDenied);
        }
        self.inner.rm_file(name)
    }

    pub fn lookup(&self, name: &str) -> VfsResult<Arc<dyn Inode>> {
        if !self.openflags.contains(OpenFlags::O_RDONLY) && !self.openflags.contains(OpenFlags::O_RDWR) {
            return Err(crate::vfs::VfsError::PermissionDenied);
        }
        self.inner.lookup(name)
    }

    pub fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        if !self.openflags.contains(OpenFlags::O_RDONLY) && !self.openflags.contains(OpenFlags::O_RDWR) {
            return Err(crate::vfs::VfsError::PermissionDenied);
        }
        self.inner.read_dir()
    }

    pub fn create_file(&self, name: &str) -> VfsResult<()> {
        if !self.openflags.contains(OpenFlags::O_WRONLY) && !self.openflags.contains(OpenFlags::O_RDWR) {
            return Err(crate::vfs::VfsError::PermissionDenied);
        }
        self.inner.create_file(name)
    }

    pub fn truncate(&self, size: usize) -> VfsResult<()> {
        if !self.openflags.contains(OpenFlags::O_WRONLY) && !self.openflags.contains(OpenFlags::O_RDWR) {
            return Err(crate::vfs::VfsError::PermissionDenied);
        }
        self.inner.truncate(size)
    }

    pub fn flush(&self) -> VfsResult<()> {
        if !self.openflags.contains(OpenFlags::O_WRONLY) && !self.openflags.contains(OpenFlags::O_RDWR) {
            return Err(crate::vfs::VfsError::PermissionDenied);
        }
        self.inner.flush()
    }

    pub fn get_file_size(&self) -> VfsResult<usize> {
        // Assuming self.inner (Inode) has a getattr() method returning VfsResult<FileAttr>
        // and FileAttr has a 'size' field.
        let attr = self.inner.getattr()?;
        Ok(attr.size)
    }
}