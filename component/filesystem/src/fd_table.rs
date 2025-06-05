use alloc::vec::Vec;
use alloc::sync::Arc;
use crate::vfs::Inode;
use spin::Mutex;

/// File descriptor table that manages open file descriptors
pub struct FdTable {
    // Store tuples of (fd, inode) in a Vec
    pub table: Mutex<Vec<(usize, Arc<dyn Inode>)>>
}

impl FdTable {
    pub fn new() -> Self {
        Self {
            table: Mutex::new(Vec::new())
        }
    }

    pub fn push(&mut self, inode: Arc<dyn Inode>) -> usize {
        let mut table = self.table.lock();
        let fd = table.len();
        table.push((fd, inode));
        fd
    }
    
    pub fn get(&self, fd: usize) -> Option<Arc<dyn Inode>> {
        let table = self.table.lock();
        table.iter()
            .find(|(id, _)| *id == fd)
            .map(|(_, inode)| inode.clone())
    }

    pub fn remove(&mut self, fd: usize) -> bool {
        let mut table = self.table.lock();
        if let Some(pos) = table.iter().position(|(id, _)| *id == fd) {
            table.remove(pos);
            true
        } else {
            false
        }
    }
}