//! /dev/uart 设备节点实现
//!
//! 提供串口设备的写入输出，兼容Unix语义。

use crate::path::Path;
use crate::vfs::VfsResult;
use crate::vfs::{DirEntry, FileAttr, FileSystem, FileType, Inode, VfsError};
use alloc::sync::Arc;
use alloc::vec::Vec;
use console::print;
<<<<<<< HEAD

/// /dev/uart 设备结构体。
=======
use console::riscv64::getch;
use struct_define::poll_event::PollEvent;
use spin::Mutex;

>>>>>>> 73599fce51808454c7e446d9fc82074df6e31d3d
#[derive(Debug)]
pub struct UartDev {
    file_type: FileType,
    buffer: Mutex<Vec<u8>>,
}

impl UartDev {
    /// 创建新的 UartDev 实例。
    pub fn new() -> Self {
        Self {
            file_type: FileType::CharDevice,
            buffer: Mutex::new(Vec::new()),
        }
    }
}

impl Inode for UartDev {
<<<<<<< HEAD
    /// 获取设备类型。
=======
    fn ioctl(&self, _command: usize, _arg: usize) -> VfsResult<usize> {
        Ok(0)
    }

>>>>>>> 73599fce51808454c7e446d9fc82074df6e31d3d
    fn get_type(&self) -> VfsResult<FileType> {
        Ok(self.file_type)
    }

<<<<<<< HEAD
    /// 读取未实现。
    fn read_at(&self, _offset: usize, _buf: &mut [u8]) -> VfsResult<usize> {
        // TODO: Implement actual UART read logic.
        // This would typically involve calling the UART driver.
        unimplemented!("UART read_at is not yet implemented")
    }
    /// 写入数据到串口。
=======
    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> VfsResult<usize> {
        // 首先检查缓冲区中是否有数据
        {
            let mut buffer = self.buffer.lock();
            if !buffer.is_empty() {
                let to_read = core::cmp::min(buffer.len(), buf.len());
                buf[..to_read].copy_from_slice(&buffer[..to_read]);
                
                // 从缓冲区移除已读数据
                if to_read < buffer.len() {
                    buffer.drain(0..to_read);
                } else {
                    buffer.clear();
                }
                
                return Ok(to_read);
            }
        }
        
        // 如果缓冲区为空，尝试从 SBI 控制台读取
        if let Some(ch) = getch() {
            buf[0] = ch;
            return Ok(1);
        }
        
        // 没有数据可读，返回 EAGAIN
        Err(VfsError::Again)
    }

>>>>>>> 73599fce51808454c7e446d9fc82074df6e31d3d
    fn write_at(&self, _offset: usize, buf: &[u8]) -> VfsResult<usize> {
        // 写入数据到控制台
        for &byte in buf {
            print!("{}", byte as char);
        }
        
        // 返回写入的字节数
        Ok(buf.len())
    }

    fn mkdir_at(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::NotSupported) // UART is not a directory
    }

    fn rm_dir(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::NotSupported) // UART is not a directory
    }

    fn rm_file(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::NotSupported) // UART is a device node, not a regular file to be removed this way
    }

    fn lookup(&self, _name: &str) -> VfsResult<Arc<dyn Inode>> {
        Err(VfsError::NotDirectory) // UART is not a directory
    }

    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        Err(VfsError::NotDirectory) // UART is not a directory
    }

    fn create_file(&self, _name: &str) -> VfsResult<()> {
        Err(VfsError::NotSupported) // Cannot create files "inside" a UART device node
    }

    fn truncate(&self, _size: usize) -> VfsResult<()> {
        Err(VfsError::NotSupported) // Truncation is not applicable to UART
    }

    /// 刷新操作为无操作。
    fn flush(&self) -> VfsResult<()> {
        // If the UART has output buffers, they could be flushed here.
        // For a simple model, this can be a no-op.
        Ok(())
    }

    fn rename(&self, _new_name: &str) -> VfsResult<()> {
        Err(VfsError::NotSupported) // Renaming a device node like this is not typical
    }

    fn mount(&self, _fs: Arc<dyn FileSystem>, _path: Path) -> VfsResult<()> {
        Err(VfsError::NotSupported) // Cannot mount a filesystem on a UART device
    }

    fn umount(&self) -> VfsResult<()> {
        Err(VfsError::NotSupported) // UART is not a mount point
    }

    /// 获取文件属性。
    fn getattr(&self) -> VfsResult<FileAttr> {
        Ok(FileAttr {
            size: 0, // UART device size is typically 0
            file_type: self.file_type,
            nlinks: 1,
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            blk_size: 512,
            blocks: 0,
        })
    }

    fn poll(&self, events: PollEvent) -> VfsResult<PollEvent> {
        let mut revents = PollEvent::NONE;
        
        // 如果用户关心是否可写，标记为可写
        if events.contains(PollEvent::OUT) || events.contains(PollEvent::WRNORM) {
            revents.insert(PollEvent::OUT);
        }
        
        // 检查是否有输入数据可读
        if events.contains(PollEvent::IN) || events.contains(PollEvent::RDNORM) {
            // 首先检查缓冲区是否有数据
            let has_buffered_data = !self.buffer.lock().is_empty();
            
            if has_buffered_data {
                revents.insert(PollEvent::IN);
            } else {
                // 如果缓冲区没有数据，使用更积极的 SBI 输入检测
                // 先检查一次
                if let Some(ch) = getch() {
                    // 如果有输入，放入缓冲区而不是直接返回
                    // 这样可以确保随后的 read_at 调用能够正确读取到数据
                    self.buffer.lock().push(ch);
                    revents.insert(PollEvent::IN);
                } else {
                    // 如果第一次检测没有输入，尝试更积极的检测
                    // 用少量的自旋等待检测按键
                    for _ in 0..3 {
                        // 等待一小段时间再检测
                        for _ in 0..500 {
                            core::hint::spin_loop();
                        }
                        
                        if let Some(ch) = getch() {
                            self.buffer.lock().push(ch);
                            revents.insert(PollEvent::IN);
                            break;
                        }
                    }
                }
            }
        }
        
        Ok(revents)
    }


}
