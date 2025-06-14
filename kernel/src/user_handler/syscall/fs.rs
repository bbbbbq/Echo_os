use crate::user_handler::handler::UserHandler;
use crate::user_handler::userbuf::UserBuf;
use memory_addr::VirtAddr;

use crate::executor::error::TaskError;
use crate::alloc::string::{String, ToString};
use filesystem::path::Path;
use log::{debug, info};
use filesystem::vfs::OpenFlags;
use filesystem::file::{File, Stat};
use alloc::vec;

impl UserHandler
{
    pub async fn sys_write(&self, fd: usize, buf_ptr: VirtAddr, count: usize) -> Result<usize, TaskError> {
        debug!("sys_write @ fd: {}, buf_ptr: {:?}, count: {}", fd, buf_ptr, count);
        let buffer = unsafe { core::slice::from_raw_parts(buf_ptr.as_ptr(), count) };
        let mut file = self.task.get_fd(fd).expect("invalid fd");
        let result = file.write(buffer)?;
        debug!("sys_write result: {}", result);
        Ok(result)
    }

    pub async fn sys_mkdirat(&mut self, dirfd: isize, path: &str, mode: usize) -> Result<usize, TaskError> {
        debug!(
            "sys_mkdirat @ dirfd: {}, path: {}, mode: {}",
            dirfd, path, mode
        );
        
        let dir = self.task.get_fd(dirfd as usize).ok_or(TaskError::EPERM)?;
        
        dir.mkdir_at(path)?;
        debug!("sys_mkdirat success");
        Ok(0)
    }

    pub async fn sys_close(&mut self, fd: usize) -> Result<usize, TaskError> {
        debug!("sys_close @ fd: {}", fd);
        self.task.pcb.lock().fd_table.close(fd);
        debug!("sys_close success");
        Ok(0)
    }

    pub async fn sys_chdir(&mut self, path: &str) -> Result<usize, TaskError> {
        debug!("sys_chdir @ path: {}", path);
        self.task.pcb.lock().curr_dir = Path::new(path.to_string()).into();
        debug!("sys_chdir success");
        Ok(0)
    }

    pub async fn sys_getcwd(&mut self, buf_ptr: VirtAddr, size: usize) -> Result<usize, TaskError> {
        debug!("sys_getcwd @ buf_ptr: {:?}, size: {}", buf_ptr, size);
        let buffer = unsafe { core::slice::from_raw_parts_mut(buf_ptr.as_mut_ptr(), size) };
        let cwd_path = self.task.pcb.lock().curr_dir.to_string();
        let cwd_bytes = cwd_path.as_bytes();

        debug!("sys_getcwd: path={}", cwd_path);

        if cwd_bytes.len() + 1 > size {
            // Not enough space in user buffer, including null terminator.
            debug!("sys_getcwd failed: buffer too small");
            return Err(TaskError::EINVAL);
        }

        let copy_len = cwd_bytes.len();
        buffer[..copy_len].copy_from_slice(cwd_bytes);
        buffer[copy_len] = 0; // Null terminate the string.

        debug!("sys_getcwd success: copied {} bytes", copy_len + 1);
        Ok(copy_len + 1)
    }

    pub async fn sys_openat(
        &self,
        dirfd: usize,
        filename_ptr: UserBuf<u8>,
        flags: usize,
        mode: usize,
    ) -> Result<isize, TaskError> {
        let filename = filename_ptr.read_string();
        let flags = OpenFlags::from_bits_truncate(flags as u32);
        debug!(
            "sys_openat @ dirfd: {}, filename: {}, flags: {:?}, mode: {}",
            dirfd, filename, flags, mode
        );

        let open_path = if filename.starts_with('/') {
            filename
        } else {
            // TODO: handle dirfd properly, for now assume AT_FDCWD
            let pcb = self.task.pcb.lock();
            let cwd = pcb.curr_dir.to_string();

            // Handle "." and "./" to avoid paths like "/foo/."
            if filename == "." || filename == "./" {
                cwd
            } else {
                let mut full_path = if cwd == "/" {
                    String::from("/")
                } else {
                    cwd + "/"
                };
                let relative_path = filename.strip_prefix("./").unwrap_or(&filename);
                full_path.push_str(relative_path);
                full_path
            }
        };

        debug!("sys_openat: final path: {}", open_path);

        let file = File::open(&open_path, flags)?;
        let fd = self.task.pcb.lock().fd_table.alloc(file);
        Ok(fd as isize)
    }


    pub async fn sys_dup3(&self, fd_src: usize, fd_dst: usize) -> Result<usize, TaskError> {
        debug!("sys_dup3 @ fd_src: {}, fd_dst: {}", fd_src, fd_dst);
        let file = self.task.get_fd(fd_src).ok_or(TaskError::EBADF)?;
        self.task.pcb.lock().fd_table.set(fd_dst, file);
        Ok(fd_dst)
    }

    pub async fn sys_dup(&self, fd: usize) -> Result<usize, TaskError> {
        debug!("sys_dup @ fd: {}", fd);
        let file = self.task.get_fd(fd).ok_or(TaskError::EBADF)?;
        let fd_dst = self.task.pcb.lock().fd_table.alloc(file);
        Ok(fd_dst)
    }

    pub async fn sys_fstat(&self, fd: usize, stat_ptr: UserBuf<Stat>) -> Result<isize, TaskError> {
        debug!("sys_fstat @ fd: {} stat_ptr: {:?}", fd, stat_ptr);

        let file = self.task.get_fd(fd).ok_or(TaskError::EBADF)?;
        let mut stat = Stat::new();
        file.stat(&mut stat)?;
        stat_ptr.write(stat);
        Ok(0)
    }

    pub async fn sys_getdents64(&self, fd: usize, buf_ptr: UserBuf<u8>, len: usize) -> Result<usize, TaskError> {
        debug!("sys_getdents64 @ fd: {} buf_ptr: {:?} len: {}", fd, buf_ptr, len);
        let file = self.task.get_fd(fd).ok_or(TaskError::EBADF)?;
        let mut buf = vec![0; len];
        let result = file.getdents(&mut buf)?;
        buf_ptr.write_slice(&buf[..result]);
        Ok(result)
    }
}