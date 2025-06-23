//! 挂载点管理模块
//!
//! 提供文件系统的挂载、卸载、查找等功能。

use crate::path::{self, Path};
use crate::vfs::{FileSystem, Inode};
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use log::trace;
use spin::Mutex;

lazy_static! {
    /// 全局挂载点列表。
    pub static ref MOUNT_LIST: Mutex<Vec<(Path, MountNode)>> = Mutex::new(Vec::new());
}

/// 挂载节点，包含根inode和文件系统引用。
#[derive(Clone)]
pub struct MountNode {
    pub root_inner: Arc<dyn Inode>,
    pub fs: Option<Arc<dyn FileSystem>>,
}

impl MountNode {
    /// 创建新的挂载节点。
    pub fn new(fs: Option<Arc<dyn FileSystem>>, root: Arc<dyn Inode>) -> Self {
        MountNode {
            root_inner: root,
            fs: fs,
        }
    }

    /// 获取根inode。
    pub fn get_inode(&self) -> Arc<dyn Inode> {
        self.root_inner.clone()
    }
}

/// 挂载文件系统到指定路径。
///
/// # 参数
/// * `fs` - 文件系统对象。
/// * `path` - 挂载路径。
pub fn mount_fs(fs: Arc<dyn FileSystem>, path: Path) {
    trace!("Mounting filesystem at path: {:?}", path);
    if let Some(root) = fs.root_inode() {
        let mount_node = MountNode::new(Some(fs), root);
        MOUNT_LIST.lock().push((path, mount_node));
        trace!("Filesystem mounted successfully");
    } else {
        trace!("Failed to get root inode for filesystem");
    }
}

/// 卸载指定路径的文件系统。
///
/// # 参数
/// * `path` - 挂载路径。
/// # 返回
/// 卸载成功返回true，否则false。
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

/// 查找与指定路径匹配的挂载节点。
///
/// # 参数
/// * `path` - 查询路径。
/// # 返回
/// 匹配的(挂载点路径, 挂载节点)。
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
                input_path_str
                    .as_bytes()
                    .get(current_prefix_len)
                    .map_or(false, |&c| c == b'/')
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

/// 直接挂载inode为挂载点。
///
/// # 参数
/// * `inode` - 根inode。
/// * `path` - 挂载路径。
pub fn mount_inode(inode:Arc<dyn Inode>,path:Path)
{
    let mount_node = MountNode::new(None,inode);
    MOUNT_LIST.lock().push((path, mount_node));
}