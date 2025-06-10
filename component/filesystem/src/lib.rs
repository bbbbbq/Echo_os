#![no_std]

extern crate alloc;

pub mod devfs;
pub mod fd_table;
pub mod file;
pub mod mount;
pub mod path;
pub mod plug;
pub mod vfs;

use crate::alloc::string::ToString;
use crate::devfs::DevFs;
use crate::mount::mount_fs;
use crate::path::Path;
use crate::plug::lwext4::Ext4FileSystemWrapper;
use alloc::sync::Arc;
use lazy_static::lazy_static;
use spin::Mutex;
lazy_static! {
    pub static ref ROOT_FS: Mutex<Option<Arc<Ext4FileSystemWrapper>>> = Mutex::new(None);
}

pub fn init_fs() {
    log::info!("Starting filesystem initialization");
    mount_ext4();
    mount_devfs();
}

pub fn mount_ext4() {
    match crate::plug::lwext4::Ext4FileSystemWrapper::new(0) {
        Ok(ext4_fs) => {
            *ROOT_FS.lock() = Some(Arc::clone(&ext4_fs));
            let mount_path = Path::new("/".to_string());
            mount_fs(ext4_fs, mount_path);
            log::info!("Filesystem mounted successfully and ROOT_FS initialized");
        }
        Err(e) => {
            log::warn!("Failed to initialize filesystem: error code={}", e);
        }
    }
}

pub fn mount_devfs() {
    log::info!("Attempting to mount DevFs at /dev...");
    let dev_filesystem = Arc::new(DevFs::new());
    let mount_path = Path::new("/dev".to_string());
    mount_fs(dev_filesystem, mount_path);
    log::info!("dev init success");
}
