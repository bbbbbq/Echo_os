use lwext4_rust;
use lwext4_rust::bindings::{
    O_CREAT, O_RDWR, O_TRUNC, O_WRONLY, SEEK_SET, ext4_inode, ext4_raw_inode_fill,
};

use alloc::format;
use device::device_set::get_device;
use device::{BlockDriver, DeviceType as EchoDeviceType}; // Removed Driver, define module removed
use virtio::blk::VirtioBlkDriver;
use virtio_drivers::transport::mmio::MmioTransport;

use lwext4_rust::{Ext4BlockWrapper, Ext4File, InodeTypes, KernelDevOp};
// device::define::BlockDriver is already imported above and used by try_get_block_driver
use crate::vfs::{
    DirEntry, FileAttr, FileSystem, FileType, FsType, Inode, OpenFlags, VfsError, VfsResult,
};
use alloc::ffi::CString;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::{string::String, vec::Vec};
use core::iter::zip;
use log::{debug, error, info, warn};
use spin::Mutex;

const BLOCK_SIZE: usize = 512;

fn try_get_block_driver(dev_id: usize) -> Result<Arc<dyn BlockDriver>, String> {
    let device_option = get_device(dev_id);
    match device_option {
        Some(device_arc) => {
            if device_arc.get_type() == EchoDeviceType::Block {
                match device_arc.downcast_arc::<VirtioBlkDriver<MmioTransport>>() {
                    Ok(concrete_driver) => Ok(concrete_driver as Arc<dyn BlockDriver>),
                    Err(_) => Err(String::from("Failed to downcast to VirtioBlkDriver")),
                }
            } else {
                Err(format!(
                    "Device {} is not a block device. Type: {:?}",
                    dev_id,
                    device_arc.get_type()
                ))
            }
        }
        None => Err(format!("Failed to get device {}", dev_id)),
    }
}

pub struct Ext4DiskWrapper {
    id: usize,
    offset: usize,
    block_id: usize,
    dev_id: usize,
}

impl Ext4DiskWrapper {
    pub fn new(id: usize) -> Self {
        Self {
            id: id,
            offset: 0,
            block_id: 0,
            dev_id: id,
        }
    }

    pub fn size(&self) -> u64 {
        match try_get_block_driver(self.dev_id) {
            Ok(dev) => dev.capacity() * BLOCK_SIZE as u64,
            Err(e) => {
                error!(
                    "Ext4DiskWrapper::size failed to get block device {}: {}",
                    self.dev_id, e
                );
                0 // Default or error value for size
            }
        }
    }

    pub fn position(&self) -> u64 {
        (self.block_id * BLOCK_SIZE + self.offset) as u64
    }

    pub fn set_position(&mut self, pos: u64) {
        self.block_id = pos as usize / BLOCK_SIZE;
        self.offset = pos as usize % BLOCK_SIZE;
    }

    pub fn read_one(&mut self, buf: &mut [u8]) -> Result<usize, i32> {
        // info!("block id: {}", self.block_id);
        let read_size = if self.offset == 0 && buf.len() >= BLOCK_SIZE {
            // whole block
            let dev = match try_get_block_driver(self.dev_id) {
                Ok(d) => d,
                Err(e) => {
                    error!(
                        "Ext4DevOp failed to get block device {}: {}. Returning EIO.",
                        self.dev_id, e
                    );
                    return Err(-5);
                }
            };
            let _ = dev.read(self.block_id, &mut buf[0..BLOCK_SIZE]);
            self.block_id += 1;
            BLOCK_SIZE
        } else {
            // partial block
            let mut data = [0u8; BLOCK_SIZE];
            let start = self.offset;
            let count = buf.len().min(BLOCK_SIZE - self.offset);
            if start > BLOCK_SIZE {
                info!("block size: {} start {}", BLOCK_SIZE, start);
            }

            let dev = match try_get_block_driver(self.dev_id) {
                Ok(d) => d,
                Err(e) => {
                    error!(
                        "Ext4DevOp failed to get block device {}: {}. Returning EIO.",
                        self.dev_id, e
                    );
                    return Err(-5);
                }
            };
            let _ = dev.read(self.block_id, &mut data);
            buf[..count].copy_from_slice(&data[start..start + count]);

            self.offset += count;
            if self.offset >= BLOCK_SIZE {
                self.block_id += 1;
                self.offset -= BLOCK_SIZE;
            }
            count
        };
        Ok(read_size)
    }

    pub fn write_one(&mut self, buf: &[u8]) -> Result<usize, i32> {
        let write_size = if self.offset == 0 && buf.len() >= BLOCK_SIZE {
            // whole block
            let dev = match try_get_block_driver(self.dev_id) {
                Ok(d) => d,
                Err(e) => {
                    error!(
                        "Ext4DevOp failed to get block device {}: {}. Returning EIO.",
                        self.dev_id, e
                    );
                    return Err(-5);
                }
            };
            let _ = dev.write(self.block_id, &buf[0..BLOCK_SIZE]);
            self.block_id += 1;
            BLOCK_SIZE
        } else {
            // partial block
            let mut data = [0u8; BLOCK_SIZE];
            let start = self.offset;
            let count = buf.len().min(BLOCK_SIZE - self.offset);

            let dev = match try_get_block_driver(self.dev_id) {
                Ok(d) => d,
                Err(e) => {
                    error!(
                        "Ext4DevOp failed to get block device {}: {}. Returning EIO.",
                        self.dev_id, e
                    );
                    return Err(-5);
                }
            };
            let _ = dev.read(self.block_id, &mut data);
            data[start..start + count].copy_from_slice(&buf[..count]);
            let dev = match try_get_block_driver(self.dev_id) {
                Ok(d) => d,
                Err(e) => {
                    error!(
                        "Ext4DevOp failed to get block device {}: {}. Returning EIO.",
                        self.dev_id, e
                    );
                    return Err(-5);
                }
            };
            let _ = dev.write(self.block_id, &data);

            self.offset += count;
            if self.offset >= BLOCK_SIZE {
                self.block_id += 1;
                self.offset -= BLOCK_SIZE;
            }
            count
        };
        Ok(write_size)
    }
}

impl KernelDevOp for Ext4DiskWrapper {
    type DevType = Self;

    fn read(dev: &mut Self, mut buf: &mut [u8]) -> Result<usize, i32> {
        debug!("READ block device buf={}", buf.len());
        let mut read_len = 0;
        while !buf.is_empty() {
            match dev.read_one(buf) {
                Ok(0) => break,
                Ok(n) => {
                    let tmp = buf;
                    buf = &mut tmp[n..];
                    read_len += n;
                }
                Err(_e) => return Err(-1),
            }
        }
        debug!("READ rt len={}", read_len);
        Ok(read_len)
    }

    fn write(dev: &mut Self, mut buf: &[u8]) -> Result<usize, i32> {
        debug!("WRITE block device buf={}", buf.len());
        let mut write_len = 0;
        while !buf.is_empty() {
            match dev.write_one(buf) {
                Ok(0) => break,
                Ok(n) => {
                    buf = &buf[n..];
                    write_len += n;
                }
                Err(_e) => return Err(-1),
            }
        }
        debug!("WRITE rt len={}", write_len);
        Ok(write_len)
    }

    fn seek(dev: &mut Self, off: i64, whence: i32) -> Result<i64, i32> {
        let size = dev.size();
        debug!(
            "SEEK block device size:{}, pos:{}, offset={}, whence={}",
            size,
            &dev.position(),
            off,
            whence
        );
        let new_pos = match whence as u32 {
            lwext4_rust::bindings::SEEK_SET => Some(off),
            lwext4_rust::bindings::SEEK_CUR => {
                dev.position().checked_add_signed(off).map(|v| v as i64)
            }
            lwext4_rust::bindings::SEEK_END => size.checked_add_signed(off).map(|v| v as i64),
            _ => {
                error!("invalid seek() whence: {}", whence);
                Some(off)
            }
        }
        .ok_or(-1)?;

        if new_pos as u64 > size {
            warn!("Seek beyond the end of the block device");
        }
        dev.set_position(new_pos as u64);
        Ok(new_pos)
    }

    fn flush(dev: &mut Self::DevType) -> Result<usize, i32> {
        unimplemented!();
    }
}

pub struct Ext4FileSystemWrapper {
    inner: Ext4BlockWrapper<Ext4DiskWrapper>,
    root: Arc<dyn Inode>,
}

unsafe impl Send for Ext4FileSystemWrapper {}
unsafe impl Sync for Ext4FileSystemWrapper {}

impl Ext4FileSystemWrapper {
    pub fn new(blk_id: usize) -> Result<Arc<Self>, i32> {
        let disk_wrapper = Ext4DiskWrapper::new(blk_id);
        let inner = match Ext4BlockWrapper::<Ext4DiskWrapper>::new(disk_wrapper) {
            Ok(wrapper) => wrapper,
            Err(e) => return Err(e),
        };
        let root = Arc::new(Ext4FileWrapper::new("/", InodeTypes::EXT4_DE_DIR));
        Ok(Arc::new(Self { inner, root }))
    }
}

impl FileSystem for Ext4FileSystemWrapper {
    fn root_inode(&self) -> Option<Arc<dyn Inode>> {
        Some(self.root.clone())
    }

    fn get_type(&self) -> FsType {
        FsType::Ext4fs
    }
}

pub struct Ext4FileWrapper {
    inner: Mutex<Ext4File>,
    file_type: FileType,
}

unsafe impl Send for Ext4FileWrapper {}
unsafe impl Sync for Ext4FileWrapper {}

pub fn inode_types_2_file_type(inodetype: InodeTypes) -> FileType {
    match inodetype {
        InodeTypes::EXT4_DE_DIR => FileType::Directory,
        InodeTypes::EXT4_DE_UNKNOWN => FileType::File,
        InodeTypes::EXT4_DE_REG_FILE => FileType::File,
        InodeTypes::EXT4_DE_CHRDEV => FileType::CharDevice,
        InodeTypes::EXT4_DE_BLKDEV => FileType::BlockDevice,
        InodeTypes::EXT4_DE_FIFO => FileType::Pipe,
        InodeTypes::EXT4_DE_SOCK => FileType::Socket,
        InodeTypes::EXT4_DE_SYMLINK => FileType::SymLink,
        _ => FileType::Unknown,
    }
}

impl Ext4FileWrapper {
    pub fn new(path: &str, types: InodeTypes) -> Self {
        info!("FileWrapper new {:?} {}", types, path);
        let file = Ext4File::new(path, types.clone());
        let file_type = inode_types_2_file_type(types);
        Self {
            inner: Mutex::new(file),
            file_type: file_type,
        }
    }

    fn path_deal_with(&self, path: &str) -> String {
        if path.starts_with('/') {
            warn!("path_deal_with: {}", path);
        }
        let p = path.trim_matches('/'); // 首尾去除
        if p.is_empty() || p == "." {
            return String::new();
        }

        if let Some(rest) = p.strip_prefix("./") {
            return self.path_deal_with(rest);
        }
        let rest_p = p.replace("//", "/");
        if p != rest_p {
            return self.path_deal_with(&rest_p);
        }

        let file = self.inner.lock();
        let path = file.get_path();
        let fpath = String::from(path.to_str().unwrap().trim_end_matches('/')) + "/" + p;
        info!("dealt with full path: {}", fpath.as_str());
        fpath
    }

    fn remove(&self, path: &str) -> Result<usize, i32> {
        info!("remove ext4fs: {}", path);
        let fpath = self.path_deal_with(path);
        let fpath = fpath.as_str();

        assert!(!fpath.is_empty());

        let mut file = self.inner.lock();
        if file.check_inode_exist(fpath, InodeTypes::EXT4_DE_DIR) {
            file.dir_rm(fpath)
        } else {
            file.file_remove(fpath)
        }
    }

    fn create(&self, path: &str, ty: InodeTypes) -> Result<usize, i32> {
        info!("create {:?} on Ext4fs: {}", ty, path);
        let fpath = self.path_deal_with(path);
        let fpath = fpath.as_str();
        if fpath.is_empty() {
            return Ok(0);
        }

        let types = ty;

        let mut file = self.inner.lock();
        if file.check_inode_exist(fpath, types.clone()) {
            Ok(0)
        } else {
            if types == InodeTypes::EXT4_DE_DIR {
                file.dir_mk(fpath)
            } else {
                file.file_open(fpath, O_WRONLY | O_CREAT | O_TRUNC)
                    .expect("create file failed");
                file.file_close()
            }
        }
    }
}

impl Inode for Ext4FileWrapper {
    fn get_type(&self) -> VfsResult<FileType> {
        Ok(self.file_type)
    }

    fn read_at(&self, offset: usize, buf: &mut [u8]) -> VfsResult<usize> {
        debug!("To read_at {}, buf len={}", offset, buf.len());
        let mut file = self.inner.lock();
        let path = file.get_path();
        let path = path.to_str().unwrap();

        match file.file_open(path, OpenFlags::O_RDONLY.0) {
            Ok(_) => {}
            Err(_) => return Err(crate::vfs::VfsError::IoError),
        }

        match file.file_seek(offset as i64, 0) {
            Ok(_) => {}
            Err(_) => return Err(crate::vfs::VfsError::IoError),
        }

        let result = match file.file_read(buf) {
            Ok(n) => Ok(n),
            Err(_) => Err(crate::vfs::VfsError::IoError),
        };

        let _ = file.file_close();

        result
    }

    fn write_at(&self, offset: usize, buf: &[u8]) -> VfsResult<usize> {
        debug!("To write_at {}, buf len={}", offset, buf.len());
        let mut file = self.inner.lock();
        let path = file.get_path();
        let path = path.to_str().unwrap();
        match file.file_open(path, O_RDWR) {
            Ok(_) => {}
            Err(_) => return Err(VfsError::IoError),
        }

        match file.file_seek(offset as i64, SEEK_SET) {
            Ok(_) => {}
            Err(_) => return Err(VfsError::IoError),
        }

        let result = match file.file_write(buf) {
            Ok(n) => Ok(n),
            Err(_) => Err(VfsError::IoError),
        };

        let _ = file.file_close();
        result
    }

    fn mkdir_at(&self, name: &str) -> VfsResult<()> {
        let types = InodeTypes::EXT4_DE_DIR;
        match self.create(name, types) {
            Ok(_) => Ok(()),
            Err(_) => Err(VfsError::IoError),
        }
    }

    fn rm_dir(&self, name: &str) -> VfsResult<()> {
        match self.remove(name) {
            Ok(_) => Ok(()),
            Err(_) => Err(VfsError::IoError),
        }
    }

    fn rm_file(&self, name: &str) -> VfsResult<()> {
        match self.remove(name) {
            Ok(_) => Ok(()),
            Err(_) => Err(VfsError::IoError),
        }
    }

    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn Inode>> {
        let path = self.path_deal_with(name);
        let path_str = path.as_str();

        if path_str.is_empty() {
            return Err(crate::vfs::VfsError::InvalidArgument);
        }

        let mut file = self.inner.lock();
        if file.check_inode_exist(path_str, InodeTypes::EXT4_DE_REG_FILE)
            || file.check_inode_exist(path_str, InodeTypes::EXT4_DE_DIR)
        {
            Ok(Arc::new(Ext4FileWrapper::new(
                path_str,
                InodeTypes::EXT4_DE_REG_FILE,
            )))
        } else {
            Err(crate::vfs::VfsError::NotFound)
        }
    }

    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        let iters = match self.inner.lock().lwext4_dir_entries() {
            Ok(entries) => entries,
            Err(_) => return Err(VfsError::IoError),
        };

        let mut ans = Vec::new();
        for (name, file_type) in zip(iters.0, iters.1) {
            let filename = match CString::from_vec_with_nul(name) {
                Ok(cstr) => match cstr.to_str() {
                    Ok(s) => s.to_string(),
                    Err(_) => return Err(VfsError::InvalidArgument),
                },
                Err(_) => return Err(VfsError::InvalidArgument),
            };

            ans.push(DirEntry {
                filename,
                len: 0,
                file_type: inode_types_2_file_type(file_type),
            })
        }
        Ok(ans)
    }

    fn create_file(&self, name: &str) -> VfsResult<()> {
        let types = InodeTypes::EXT4_DE_REG_FILE;
        match self.create(name, types) {
            Ok(_) => Ok(()),
            Err(_) => Err(VfsError::IoError),
        }
    }

    fn truncate(&self, size: usize) -> VfsResult<()> {
        info!("truncate file to size={}", size);
        let mut file = self.inner.lock();
        let path = file.get_path();
        let path = path.to_str().unwrap();

        match file.file_open(path, O_RDWR | O_CREAT | O_TRUNC) {
            Ok(_) => {}
            Err(_) => return Err(VfsError::IoError),
        }

        let result = match file.file_truncate(size as u64) {
            Ok(_) => Ok(()),
            Err(_) => Err(VfsError::IoError),
        };

        let _ = file.file_close();
        result
    }

    fn flush(&self) -> VfsResult<()> {
        // Nothing to do for now, just return success
        Ok(())
    }

    fn getattr(&self) -> VfsResult<FileAttr> {
        let file_guard = self.inner.lock();
        let path_cstr = file_guard.get_path();

        let mut inode_info: ext4_inode = unsafe { core::mem::zeroed() };
        let mut inode_num: u32 = 0;
        let ret =
            unsafe { ext4_raw_inode_fill(path_cstr.as_ptr(), &mut inode_num, &mut inode_info) };

        if ret == 0 {
            let size_val = (inode_info.size_hi as u64) << 32 | (inode_info.size_lo as u64);

            Ok(FileAttr {
                size: size_val as usize,
                file_type: self.file_type,
            })
        } else {
            error!(
                "ext4_raw_inode_fill failed for path: {:?}, ret: {}",
                path_cstr, ret
            );
            Err(VfsError::IoError) // Or map 'ret' to a more specific VfsError
        }
    }

    fn rename(&self, new_name: &str) -> VfsResult<()> {
        let mut file = self.inner.lock();
        let path = file.get_path();
        let old_path = path.to_str().unwrap();
        let new_path = self.path_deal_with(new_name).as_str().to_string();

        info!("rename from {} to {}", old_path, new_path);

        match file.file_rename(old_path, &new_path) {
            Ok(_) => Ok(()),
            Err(_) => Err(VfsError::IoError),
        }
    }
}
