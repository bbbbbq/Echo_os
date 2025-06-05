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

pub fn get_mount_node(path: Path) -> Option<(Path, MountNode)> {
    let mount_list = MOUNT_LIST.lock();
    let mut best_match_data: Option<(Path, MountNode)> = None;
    let mut max_prefix_len: usize = 0;

    let input_path_str = path.to_string();

    for (mount_point, mount_node) in mount_list.iter() {
        let mount_point_str = mount_point.to_string();

        if input_path_str.starts_with(&mount_point_str) {
            let current_prefix_len = mount_point_str.len();

            let is_exact_match = input_path_str.len() == current_prefix_len;
            let is_prefix_and_subdir = if !is_exact_match {
                input_path_str.as_bytes().get(current_prefix_len).map_or(false, |&c| c == b'/')
            } else {
                false 
            };
            let valid_match_as_prefix = if mount_point_str == "/" {
                true
            } else {
                is_exact_match || is_prefix_and_subdir
            };

            if valid_match_as_prefix && current_prefix_len > max_prefix_len {
                max_prefix_len = current_prefix_len;
                best_match_data = Some((mount_point.clone(), mount_node.clone()));
            }
        }
    }
    best_match_data
}