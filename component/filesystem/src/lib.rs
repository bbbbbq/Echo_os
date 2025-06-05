#![no_std]

extern crate alloc;

pub mod vfs;
pub mod file;
pub mod path;
pub mod plug;
pub mod mount;
pub mod fd_table;
pub mod io;
pub mod devfs;

use alloc::sync::Arc;
use alloc::string::ToString;
use crate::plug::lwext4::Ext4FileSystemWrapper;
use crate::mount::mount_fs;
use crate::path::Path;

pub fn init_fs()
{
    log::info!("Starting filesystem initialization");
    
    // Check if block device 0 is available
    log::debug!("Attempting to create Ext4FileSystemWrapper for device 0");
    match crate::plug::lwext4::Ext4FileSystemWrapper::new(0) {
        Ok(ext4_fs) => {
            log::info!("Ext4FileSystemWrapper created successfully");
            let mount_path = Path::new("/".to_string());
            log::debug!("Mounting at path: /{:?}", mount_path);
            mount_fs(ext4_fs, mount_path);
            log::info!("Filesystem mount operation completed successfully");
        },
        Err(e) => {
            log::warn!("Failed to initialize filesystem: error code={}", e);
        }
    }
    log::info!("Filesystem initialization complete");
}


