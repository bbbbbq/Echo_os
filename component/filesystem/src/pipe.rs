//! 管道(pipe)实现模块
//!
//! 提供进程间通信的无名管道读写端。

use alloc::{
    collections::VecDeque,
    sync::{Arc, Weak},
};

use struct_define::poll_event::PollEvent;
use crate::vfs::VfsResult;
use spin::Mutex;
use crate::vfs::Inode;
use crate::vfs::VfsError;

/// 管道写端。
#[derive(Debug)]
pub struct PipeSender(Arc<Mutex<VecDeque<u8>>>);

impl Inode for PipeSender {
    /// 读操作未实现。
    fn read_at(&self, _offset: usize, _buf: &mut [u8]) -> VfsResult<usize> {
        core::unimplemented!()
    }
    /// 写入数据到管道。
    fn write_at(&self, _offset: usize, buf: &[u8]) -> VfsResult<usize> {
        log::warn!("write pipe:");
        let mut queue = self.0.lock();
        if queue.len() > 0x50000 {
            Err(VfsError::IoError)
        } else {
            let wlen = buf.len();
            queue.extend(buf.iter());
            Ok(wlen)
        }
    }
    // 其余方法均未实现或无操作。
    fn mkdir_at(&self, _name: &str) -> VfsResult<()> {
        core::unimplemented!()
    }
    fn rm_dir(&self, _name: &str) -> VfsResult<()> {
        core::unimplemented!()
    }
    fn rm_file(&self, _name: &str) -> VfsResult<()> {
        core::unimplemented!()
    }
    fn lookup(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> {
        core::unimplemented!()
    }
    fn read_dir(&self) -> VfsResult<alloc::vec::Vec<crate::vfs::DirEntry>> {
        core::unimplemented!()
    }
    fn create_file(&self, _name: &str) -> VfsResult<()> {
        core::unimplemented!()
    }
    fn truncate(&self, _size: usize) -> VfsResult<()> {
        core::unimplemented!()
    }
    /// 刷新操作为无操作。
    fn flush(&self) -> VfsResult<()> {
        Ok(())
    }
    fn rename(&self, _name: &str) -> VfsResult<()> {
        core::unimplemented!()
    }
    fn mount(&self, _fs: Arc<dyn crate::vfs::FileSystem>, _path: crate::path::Path) -> VfsResult<()> {
        core::unimplemented!()
    }
    fn umount(&self) -> VfsResult<()> {
        core::unimplemented!()
    }
    fn getattr(&self) -> VfsResult<crate::vfs::FileAttr> {
        Err(crate::vfs::VfsError::NotSupported)
    }
    fn get_type(&self) -> VfsResult<crate::vfs::FileType> {
        Err(crate::vfs::VfsError::NotSupported)
    }
    /// poll事件支持。
    fn poll(&self, events: PollEvent) -> VfsResult<PollEvent> {
        let mut res = PollEvent::NONE;
        if events.contains(PollEvent::OUT) && self.0.lock().len() <= 0x50000 {
            res |= PollEvent::OUT;
        }
        Ok(res)
    }
}

/// 管道读端。
#[derive(Debug)]
pub struct PipeReceiver {
    queue: Arc<Mutex<VecDeque<u8>>>,
    sender: Weak<PipeSender>,
}

impl Inode for PipeReceiver {
    /// 从管道读取数据。
    fn read_at(&self, _offset: usize, _buf: &mut [u8]) -> VfsResult<usize> {
        let mut queue = self.queue.lock();
        if queue.is_empty() {
            if Weak::strong_count(&self.sender) > 0 {
                // 写入端还存在，但管道是空的，返回 Again
                return Err(VfsError::Again);
            } else {
                // 写入端已关闭，管道也是空的，返回 EOF
                return Ok(0);
            }
        }
        // 管道中有数据，正常读取
        let mut i = 0;
        while i < _buf.len() && !queue.is_empty() {
            _buf[i] = queue.pop_front().unwrap();
            i += 1;
        }
        Ok(i)
    }
    /// poll事件支持。
    fn poll(&self, events: PollEvent) -> VfsResult<PollEvent> {
        let mut res = PollEvent::NONE;
        if events.contains(PollEvent::IN) {
            if !self.queue.lock().is_empty() {
                res |= PollEvent::IN;
            } else if Weak::strong_count(&self.sender) == 0 {
                res |= PollEvent::ERR;
            }
        }
        if events.contains(PollEvent::ERR)
            && self.queue.lock().is_empty()
            && Weak::strong_count(&self.sender) == 0
        {
            res |= PollEvent::ERR;
        }
        Ok(res)
    }
}

/// 创建一对管道读写端。
///
/// # 返回
/// (读端, 写端)
pub fn create_pipe() -> (Arc<PipeReceiver>, Arc<PipeSender>) {
    let queue = Arc::new(Mutex::new(VecDeque::new()));
    let sender = Arc::new(PipeSender(queue.clone()));
    (
        Arc::new(PipeReceiver {
            queue: queue.clone(),
            sender: Arc::downgrade(&sender),
        }),
        sender,
    )
}
