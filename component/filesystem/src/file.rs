use crate::mount::get_mount_node;
use crate::path::Path;
use crate::vfs::{DirEntry, Dirent64, FileType, Inode, SeekFrom, VfsError, VfsResult};
use alloc::{string::ToString, sync::Arc, vec::Vec};
use bitflags::bitflags;
use core::fmt::Debug;
use core::mem::size_of;
use core::sync::atomic::{AtomicUsize, Ordering};
use log::{debug, error};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OpenFlags: usize {
        const O_RDONLY    = 0o0;
        const O_WRONLY    = 0o1;
        const O_RDWR      = 0o2;
        const O_CREAT     = 0o100;
        const O_EXCL      = 0o200;
        const O_NOCTTY    = 0o400;
        const O_TRUNC     = 0o1000;
        const O_APPEND    = 0o2000;
        const O_NONBLOCK  = 0o4000;
        const O_DSYNC     = 0o10000;
        const O_SYNC      = 0o4010000;
        const O_RSYNC     = 0o4010000;
        const O_DIRECTORY = 0o200000;
        const O_NOFOLLOW  = 0o400000;
        const O_CLOEXEC   = 0o2000000;
        const O_DIRECT    = 0o40000;
        const O_NOATIME   = 0o1000000;
        const O_PATH      = 0o10000000;
        const O_TMPFILE   = 0o20200000;
    }
}

impl OpenFlags {
    pub fn is_readable(&self) -> bool {
        let mode = self.bits() & 0x3;
        mode == Self::O_RDONLY.bits() || mode == Self::O_RDWR.bits()
    }

    pub fn is_writable(&self) -> bool {
        let mode = self.bits() & 0x3;
        mode == Self::O_WRONLY.bits() || mode == Self::O_RDWR.bits()
    }

    pub fn new_read_write() -> Self {
        Self::O_RDWR
    }
}

#[derive(Debug, Clone)]
pub struct File {
    pub inner: Arc<dyn Inode>,
    pub openflags: OpenFlags,
    // 通过 `Arc<AtomicUsize>` 实现偏移在克隆后的共享，保证多次 `get_fd` 调用看到相同偏移。
    pub offset: Arc<AtomicUsize>,
    pub path: Path,
}

impl File {
    pub fn open_relative(&self, file_name: &str, open_flags: OpenFlags) -> VfsResult<Self> {
        let current_inode = self.inner.clone();
        let inode = current_inode.lookup(file_name)?;
        let new_path = self.path.join(file_name);
        Ok(Self {
            inner: inode,
            openflags: open_flags,
            offset: Arc::new(AtomicUsize::new(0)),
            path: new_path,
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
                if open_flags.contains(OpenFlags::O_CREAT) && open_flags.contains(OpenFlags::O_EXCL)
                {
                    return Err(VfsError::AlreadyExists);
                }

                let attr = inode.getattr()?;
                if open_flags.contains(OpenFlags::O_DIRECTORY)
                    && attr.file_type != FileType::Directory
                {
                    return Err(VfsError::NotDirectory);
                }

                if attr.file_type == FileType::Directory
                    && !open_flags.contains(OpenFlags::O_DIRECTORY)
                    && open_flags.is_writable()
                {
                    return Err(VfsError::IsDirectory);
                }

                let file = Self {
                    inner: inode,
                    openflags: open_flags,
                    offset: Arc::new(AtomicUsize::new(0)),
                    path: Path::from(path),
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
                        offset: Arc::new(AtomicUsize::new(0)),
                        path: Path::from(path),
                    })
                } else {
                    Err(VfsError::NotFound)
                }
            }
            Err(e) => Err(e),
        }
    }

    pub fn open(path: &str, open_flags: OpenFlags) -> VfsResult<Self> {
        let path = if path == "." || path == "/." {
            "/"
        } else {
            path
        };

        if path.is_empty() {
            "/"
        } else {
            path
        };
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
                offset: Arc::new(AtomicUsize::new(0)),
                path: Path::from(path),
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
                if open_flags.contains(OpenFlags::O_CREAT) && open_flags.contains(OpenFlags::O_EXCL)
                {
                    return Err(VfsError::AlreadyExists);
                }

                let attr = inode.getattr()?;
                if open_flags.contains(OpenFlags::O_DIRECTORY)
                    && attr.file_type != FileType::Directory
                {
                    return Err(VfsError::NotDirectory);
                }

                if attr.file_type == FileType::Directory
                    && !open_flags.contains(OpenFlags::O_DIRECTORY)
                    && open_flags.is_writable()
                {
                    return Err(VfsError::IsDirectory);
                }

                let file = Self {
                    inner: inode,
                    openflags: open_flags,
                    offset: Arc::new(AtomicUsize::new(0)),
                    path: Path::from(path),
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
                        offset: Arc::new(AtomicUsize::new(0)),
                        path: Path::from(path),
                    }
                    .into())
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
            offset: Arc::new(AtomicUsize::new(0)),
            path: Path::from(""), // TODO: new function should take a path
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
        let cur_off = self.offset.load(Ordering::Relaxed);
        let len = self.inner.read_at(cur_off, buf)?;
        self.offset.fetch_add(len, Ordering::Relaxed);
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
        let cur_off = self.offset.load(Ordering::Relaxed);
        let len = self.inner.write_at(cur_off, buf)?;
        self.offset.fetch_add(len, Ordering::Relaxed);
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

    pub fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        let attr = self.inner.getattr()?;
        stat.st_size = attr.size as u64;
        stat.st_mode = match attr.file_type {
            FileType::File => 0o100000,        // S_IFREG
            FileType::Directory => 0o040000,   // S_IFDIR
            FileType::SymLink => 0o120000,     // S_IFLNK
            FileType::CharDevice => 0o020000,  // S_IFCHR
            FileType::BlockDevice => 0o060000, // S_IFBLK
            FileType::Pipe => 0o010000,        // S_IFIFO
            FileType::Socket => 0o140000,      // S_IFSOCK
            FileType::Unknown => 0,
        };
        stat.st_nlink = attr.nlinks as u32;
        stat.st_uid = attr.uid as u32;
        stat.st_gid = attr.gid as u32;
        stat.st_atime_sec = attr.atime as u64;
        stat.st_mtime_sec = attr.mtime as u64;
        stat.st_ctime_sec = attr.ctime as u64;
        stat.st_blksize = attr.blk_size as u32;
        stat.st_blocks = attr.blocks as u32;
        stat.st_blksize = 512;
        stat.st_blocks = ((attr.size as u64 + 511) / 512) as u32;

        Ok(())
    }

    pub fn remove(&self, name: &str) -> VfsResult<()> {
        self.inner.rm_file(name)
    }

    pub fn rmdir(&self, name: &str) -> VfsResult<()> {
        self.inner.rm_dir(name)
    }
    pub fn getdents(&mut self, buffer: &mut [u8]) -> Result<usize, VfsError> {
        let dirents = self.read_dir()?;

        let mut buf_off = 0usize;
        let mut index = self.offset.load(Ordering::Relaxed);
        while index < dirents.len() {
            let mut dirent64 = dirents[index].convert_to_dirent64();
            dirent64.d_off = (index + 1) as i64;

            let name_len = dirent64.d_name.len();
            let reclen = dirent64.d_reclen as usize;
            if buf_off + reclen > buffer.len() {
                break;
            }

            buffer[buf_off..buf_off + 8].copy_from_slice(&dirent64.d_ino.to_le_bytes());
            buffer[buf_off + 8..buf_off + 16].copy_from_slice(&(dirent64.d_off as i64).to_le_bytes());
            buffer[buf_off + 16..buf_off + 18].copy_from_slice(&dirent64.d_reclen.to_le_bytes());
            buffer[buf_off + 18] = dirent64.d_type;
            buffer[buf_off + 19] = 0;

            let name_start = buf_off + 19;
            buffer[name_start..name_start + name_len].copy_from_slice(dirent64.d_name);
            buffer[name_start + name_len] = 0;

            buf_off += reclen;
            index += 1;
        }

        self.offset.store(index, Ordering::Relaxed);
        Ok(buf_off)
    }

    pub fn new_dev(inner: Arc<dyn Inode>) -> Self {
        Self {
            inner,
            openflags: OpenFlags::new_read_write(),
            offset: Arc::new(AtomicUsize::new(0)),
            path: Path::from(""), // Device files do not have a path in the same way
        }
    }

    pub fn mount(&self, _path: &str) -> Result<usize, VfsError> {
        unimplemented!()
    }

    pub fn remove_self(&self) -> VfsResult<()> {
        let dir = Self::open(&self.path.to_string(), OpenFlags::O_DIRECTORY)?;
        dir.remove(&self.path.get_name())
    }

    pub fn seek(&mut self, seek_from: SeekFrom) -> Result<usize, VfsError> {
        let offset = self.offset.load(Ordering::Relaxed);
        let mut stat = Stat::default();
        let attr = self.inner.getattr()?;
        stat.st_size = attr.size as u64;
        let mut new_off = match seek_from {
            SeekFrom::SET(off) => off as isize,
            SeekFrom::CURRENT(off) => offset as isize + off,
            SeekFrom::END(off) => stat.st_size as isize + off,
        };
        if new_off < 0 {
            new_off = 0;
        }
        // assert!(new_off >= 0);
        self.offset.store(new_off as _, Ordering::Relaxed);
        Ok(new_off as _)
    }

    
    pub fn ioctl(&self, command: usize, arg: usize) -> Result<usize, VfsError> {
        self.inner.ioctl(command, arg)
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct Stat {
    pub st_dev: u64,        // 文件所在设备的ID
    pub st_ino: u64,        // 文件的inode号
    pub st_mode: u32,       // 文件类型和权限
    pub st_nlink: u32,      // 硬链接数
    pub st_uid: u32,        // 所有者用户ID
    pub st_gid: u32,        // 所有者组ID
    pub st_rdev: u64,       // 特殊设备ID（仅设备文件有效）
    pub st_size: u64,       // 文件大小（字节数）
    pub st_atime_sec: u64,  // 最后访问时间（秒）
    pub st_atime_nsec: u64, // 最后访问时间（纳秒）
    pub st_mtime_sec: u64,  // 最后修改时间（秒）
    pub st_mtime_nsec: u64, // 最后修改时间（纳秒）
    pub st_ctime_sec: u64,  // 最后状态变更时间（秒）
    pub st_ctime_nsec: u64, // 最后状态变更时间（纳秒）
    pub st_blksize: u32,    // 文件I/O的块大小
    pub st_blocks: u32,     // 分配的磁盘块数
    pub st_padding: u32,    // 填充
}

impl Stat {
    pub fn new() -> Self {
        Self::default()
    }
}
