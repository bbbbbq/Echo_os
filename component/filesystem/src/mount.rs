use alloc::sync::Arc;
use alloc::vec::Vec;
use crate::path::Path;
use crate::vfs::{Inode, FileSystem};
use lazy_static::*;
use spin::Mutex;
use log::trace;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref MOUNT_LIST: Mutex<Vec<(Path, MountNode)>> = Mutex::new(Vec::new());
}
#[derive(Clone)]
pub struct MountNode {
    pub root_inner: Arc<dyn Inode>,
    pub fs: Arc<dyn FileSystem>
}

impl MountNode {
    pub fn new(fs: Arc<dyn FileSystem>, root: Arc<dyn Inode>) -> Self {
        MountNode {
            root_inner: root,
            fs: fs
        }
    }

    pub fn get_inode(&self) -> Arc<dyn Inode>
    {
        self.root_inner.clone()
    }
}

pub fn mount_fs(fs: Arc<dyn FileSystem>, path: Path) 
{
    trace!("Mounting filesystem at path: {:?}", path);
    if let Some(root) = fs.root_inode() {
        let mount_node = MountNode::new(fs, root);
        MOUNT_LIST.lock().push((path, mount_node));
        trace!("Filesystem mounted successfully");
    } else {
        trace!("Failed to get root inode for filesystem");
    }
}

pub fn umount_fs(path: Path) -> bool {
    trace!("Unmounting filesystem at path: {:?}", path);
    let mut mount_list = MOUNT_LIST.lock();
    if let Some(index) = mount_list.iter().position(|(p, _)| p == &path) {
        mount_list.remove(index);
        trace!("Filesystem unmounted successfully");
        true
    } else {
        trace!("No filesystem mounted at path: {:?}", path);
        false
    }
}

pub fn get_mount_node(path: Path) -> Option<MountNode> {
    let mount_list = MOUNT_LIST.lock();
    for (mount_path, mount_node) in mount_list.iter() {
        if path == *mount_path || path.get_inner().starts_with(&mount_path.get_inner()) {
            return Some(mount_node.clone());
        }
    }
    None
}