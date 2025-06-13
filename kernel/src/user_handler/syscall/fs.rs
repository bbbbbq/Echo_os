use crate::user_handler::handler::UserHandler;
use memory_addr::VirtAddr;

use crate::executor::error::TaskError;
use crate::alloc::string::ToString;
use filesystem::path::Path;
use log::{debug, info};
use filesystem::vfs::OpenFlags;
use filesystem::file::File;
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
        &mut self,
        dir_fd: isize,
        filename: &str,
        flags: usize,
        _mode: usize,
    ) -> Result<usize, TaskError> {
        debug!("sys_openat @ dir_fd: {}, filename: {}, flags: {}, mode: {} curr_dir: {}", dir_fd, filename, flags, _mode, self.task.pcb.lock().curr_dir.to_string());
        let open_flags = OpenFlags::from_bits_truncate(flags as u32);

        let file = if filename.starts_with('/') {
            // Absolute path, dir_fd is ignored.
            File::open(filename, open_flags)?
        } else {
            // Relative path.
            let dir = if dir_fd == -100 { // AT_FDCWD
                let cwd = self.task.pcb.lock().curr_dir.clone();
                File::open(&cwd.to_string(), OpenFlags::O_RDONLY)?
            } else {
                self.task.get_fd(dir_fd as usize).ok_or(TaskError::EBADF)?
            };
            info!("filename: {:?}", filename);
            dir.open_at(filename, open_flags)?
        };

        let fd = self.task.pcb.lock().fd_table.alloc(file);
        Ok(fd)
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
}