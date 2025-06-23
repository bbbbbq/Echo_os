//! 文件描述符表(FdTable)模块
//!
//! 提供进程/线程的文件描述符管理。

use super::file::{File, OpenFlags};
use alloc::collections::BTreeMap;
use log::warn;
use alloc::sync::Arc;

/// 文件描述符表结构体。
#[derive(Debug, Default, Clone)]
pub struct FdTable {
    pub table: BTreeMap<usize, File>,
}

impl FdTable {
    /// 创建新的文件描述符表，初始化标准输入输出错误流。
    pub fn new() -> Self {
        let mut table = BTreeMap::new();
        // 前三个文件描述符是输入流、输出流、错误流
        table.insert(
            0,
            File::new(
                Arc::new(crate::devfs::uart::UartDev::new()),
                OpenFlags::O_RDONLY,
            ),
        ); // stdin
        table.insert(
            1,
            File::new(
                Arc::new(crate::devfs::uart::UartDev::new()),
                OpenFlags::O_WRONLY,
            ),
        ); // stdout
        table.insert(
            2,
            File::new(
                Arc::new(crate::devfs::uart::UartDev::new()),
                OpenFlags::O_WRONLY,
            ),
        ); // stderr
        Self { table }
    }

    /// 获取指定fd的文件对象。
    pub fn get(&self, fd: usize) -> Option<&File> {
        self.table.get(&fd)
    }

    /// 插入或替换fd对应的文件对象。
    pub fn insert(&mut self, fd: usize, file: File) {
        self.table.insert(fd, file);
    }

    /// 移除指定fd。
    pub fn remove(&mut self, fd: usize) {
        self.table.remove(&fd);
    }

    /// 关闭指定fd并尝试flush。
    pub fn close(&mut self, fd: usize) {
        if let Some(file) = self.table.remove(&fd) {
            if let Err(e) = file.flush() {
                warn!("Failed to flush file on close (fd={}): {:?}", fd, e);
            }
        }
    }

    /// 分配新的fd。
    pub fn alloc(&mut self, file: File) -> usize {
        let fd = (3..).find(|fd| self.table.get(fd).is_none()).unwrap();
        self.table.insert(fd, file);
        fd
    }

    /// 设置指定fd的文件对象。
    pub fn set(&mut self, fd: usize, file: File) {
        self.table.insert(fd, file);
    }

    /// 分配新的fd但不插入。
    pub fn alloc_id(&mut self) -> usize {
        let fd = (3..).find(|fd| self.table.get(fd).is_none()).unwrap();
        fd
    }
}
