use crate::mount::get_mount_node;
use crate::vfs::{DirEntry, FileAttr, FileType, Inode, OpenFlags, VfsError, VfsResult};
use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::fmt::Debug;

#[derive(Debug, Clone)]
pub struct File {
    pub inner: Arc<dyn Inode>,
    pub openflags: OpenFlags,
    pub offset: usize,
}

impl File {

    pub fn open_relative(&self, file_name: &str,open_flags:OpenFlags) -> VfsResult<Self> {
        let current_inode = self.inner.clone();
        let inode = current_inode.lookup(file_name)?;
        Ok(Self {
            inner: inode,
            openflags: open_flags,
            offset: 0,
        })
    }

    pub fn open_at(&self, path: &str, open_flags: OpenFlags) -> VfsResult<Self> {
        let components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        if components.is_empty() {
            return Err(VfsError::InvalidArgument);
        }

        let (file_name, dir_components) = components.split_last().unwrap();

        let mut current_inode = self.inner.clone();
        for component in dir_components {
            current_inode = current_inode.lookup(component)?;
        }

        let dir_inode = current_inode;

        match dir_inode.lookup(file_name) {
            Ok(inode) => {
                // File exists
                if open_flags.contains(OpenFlags::O_CREAT) && open_flags.contains(OpenFlags::O_EXCL) {
                    return Err(VfsError::AlreadyExists);
                }

                let attr = inode.getattr()?;
                if open_flags.contains(OpenFlags::O_DIRECTORY)
                    && attr.file_type != FileType::Directory
                {
                    return Err(VfsError::NotDirectory);
                }

                if attr.file_type == FileType::Directory && open_flags.is_writable() {
                    return Err(VfsError::IsDirectory);
                }

                let mut file = Self {
                    inner: inode,
                    openflags: open_flags,
                    offset: 0,
                };

                if open_flags.contains(OpenFlags::O_TRUNC) {
                    if !open_flags.is_writable() {
                        return Err(VfsError::InvalidArgument);
                    }
                    file.inner.truncate(0)?;
                }

                Ok(file)
            }
            Err(VfsError::NotFound) => {
                // File does not exist
                if open_flags.contains(OpenFlags::O_CREAT) {
                    if open_flags.contains(OpenFlags::O_DIRECTORY) {
                        dir_inode.mkdir_at(file_name)?;
                    } else {
                        dir_inode.create_file(file_name)?;
                    }
                    let inode = dir_inode.lookup(file_name)?;
                    Ok(Self {
                        inner: inode,
                        openflags: open_flags,
                        offset: 0,
                    })
                } else {
                    Err(VfsError::NotFound)
                }
            }
            Err(e) => Err(e),
        }
    }

    pub fn open(path: &str, open_flags: OpenFlags) -> VfsResult<Self> {
        let (resolved_mount_path, mount_node) = match get_mount_node(path.into()) {
            Some((p, node)) => (p, node),
            None => return Err(VfsError::NotFound),
        };

        let root_inode = mount_node.get_inode();

        let full_path_str = path.to_string();
        let mount_point_str = resolved_mount_path.to_string();

        let relative_path_str_intermediate = if mount_point_str == "/" {
            if full_path_str.starts_with('/') {
                &full_path_str[1..]
            } else {
                &full_path_str
            }
        } else {
            full_path_str
                .strip_prefix(&mount_point_str)
                .unwrap_or(&full_path_str)
        };

        let final_relative_path_str = if relative_path_str_intermediate.starts_with('/')
            && relative_path_str_intermediate.len() > 1
        {
            &relative_path_str_intermediate[1..]
        } else if relative_path_str_intermediate == "/" {
            ""
        } else {
            relative_path_str_intermediate
        };

        let components: Vec<&str> = final_relative_path_str
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        if components.is_empty() {
            // Opening the root directory of the mount point.
            return Ok(Self {
                inner: root_inode,
                openflags: open_flags,
                offset: 0,
            });
        }

        let (file_name, dir_components) = components.split_last().unwrap();
        let mut current_inode = root_inode;
        for component in dir_components {
            current_inode = current_inode.lookup(component)?;
        }

        let dir_inode = current_inode;

        match dir_inode.lookup(file_name) {
            Ok(inode) => {
                // File exists
                if open_flags.contains(OpenFlags::O_CREAT) && open_flags.contains(OpenFlags::O_EXCL) {
                    return Err(VfsError::AlreadyExists);
                }

                let attr = inode.getattr()?;
                if open_flags.contains(OpenFlags::O_DIRECTORY)
                    && attr.file_type != FileType::Directory
                {
                    return Err(VfsError::NotDirectory);
                }

                if attr.file_type == FileType::Directory && open_flags.is_writable() {
                    return Err(VfsError::IsDirectory);
                }

                let mut file = Self {
                    inner: inode,
                    openflags: open_flags,
                    offset: 0,
                };

                if open_flags.contains(OpenFlags::O_TRUNC) {
                    if !open_flags.is_writable() {
                        return Err(VfsError::InvalidArgument);
                    }
                    file.inner.truncate(0)?;
                }

                Ok(file)
            }
            Err(VfsError::NotFound) => {
                // File does not exist
                if open_flags.contains(OpenFlags::O_CREAT) {
                    if open_flags.contains(OpenFlags::O_DIRECTORY) {
                        dir_inode.mkdir_at(file_name)?;
                    } else {
                        dir_inode.create_file(file_name)?;
                    }
                    let inode = dir_inode.lookup(file_name)?;
                    Ok(Self {
                        inner: inode,
                        openflags: open_flags,
                        offset: 0,
                    })
                } else {
                    Err(VfsError::NotFound)
                }
            }
            Err(e) => Err(e),
        }
    }

    pub fn new(inner: Arc<dyn Inode>, openflags: OpenFlags) -> Self {
        Self {
            inner,
            openflags,
            offset: 0,
        }
    }

    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> VfsResult<usize> {
        if !self.openflags.is_readable() {
            return Err(VfsError::PermissionDenied);
        }
        self.inner.read_at(offset, buf)
    }

    pub fn read(&mut self, buf: &mut [u8]) -> VfsResult<usize> {
        if !self.openflags.is_readable() {
            return Err(VfsError::PermissionDenied);
        }
        let len = self.inner.read_at(self.offset, buf)?;
        self.offset += len;
        Ok(len)
    }

    pub fn write_at(&self, offset: usize, buf: &[u8]) -> VfsResult<usize> {
        if !self.openflags.is_writable() {
            return Err(VfsError::PermissionDenied);
        }
        self.inner.write_at(offset, buf)
    }

    pub fn write(&mut self, buf: &[u8]) -> VfsResult<usize> {
        if !self.openflags.is_writable() {
            return Err(VfsError::PermissionDenied);
        }
        let len = self.inner.write_at(self.offset, buf)?;
        self.offset += len;
        Ok(len)
    }

    pub fn flush(&self) -> VfsResult<()> {
        if !self.openflags.is_writable() {
            return Err(VfsError::PermissionDenied);
        }
        self.inner.flush()
    }

    pub fn mkdir_at(&self, path: &str) -> VfsResult<()> {
        if !self.openflags.is_writable() {
            return Err(VfsError::PermissionDenied);
        }
        self.inner.mkdir_at(path)
    }

    pub fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        if !self.openflags.is_readable() {
            return Err(VfsError::PermissionDenied);
        }
        self.inner.read_dir()
    }

    pub fn get_file_size(&self) -> VfsResult<usize> {
        let attr = self.inner.getattr()?;
        Ok(attr.size)
    }
}
