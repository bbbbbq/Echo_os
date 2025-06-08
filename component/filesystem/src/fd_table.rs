use alloc::sync::Arc;
use crate::file::File;
use alloc::collections::BTreeMap;


pub struct FdTable {
    pub table: BTreeMap<usize, File>,
}

impl FdTable {
    pub fn new() -> Self {
        let mut table = BTreeMap::new();
        // 前三个文件描述符是输入流、输出流、错误流
        table.insert(0, File::new(Arc::new(crate::devfs::uart::UartDev::new()), crate::vfs::OpenFlags::O_RDONLY)); // stdin
        table.insert(1, File::new(Arc::new(crate::devfs::uart::UartDev::new()), crate::vfs::OpenFlags::O_WRONLY)); // stdout
        table.insert(2, File::new(Arc::new(crate::devfs::uart::UartDev::new()), crate::vfs::OpenFlags::O_WRONLY)); // stderr
        Self { table }
    }

    pub fn get(&self, fd: usize) -> Option<&File> {
        self.table.get(&fd)
    }

    pub fn insert(&mut self, fd: usize, file: File) {
        self.table.insert(fd, file);
    }

    pub fn remove(&mut self, fd: usize) {
        self.table.remove(&fd);
    }

    pub fn close(&mut self, fd: usize) {
        if let Some(file) = self.table.remove(&fd) {
            file.flush().unwrap();
        }
    }
}