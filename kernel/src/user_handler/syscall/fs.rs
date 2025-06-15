use crate::executor::error::TaskError;
use crate::test_ls;
use crate::user_handler::handler::UserHandler;
use crate::user_handler::userbuf::UserBuf;
use alloc::string::{String, ToString};
use alloc::vec::{self, Vec};
use filesystem::vfs::{DirEntry, FileType};
use filesystem::file::{File, Stat};
use filesystem::path::Path;
use filesystem::vfs::OpenFlags;
use log::debug;
use memory_addr::VirtAddr;

const AT_FDCWD: isize = -100;

impl UserHandler {
    pub async fn sys_write(
        &self,
        fd: usize,
        buf_ptr: VirtAddr,
        count: usize,
    ) -> Result<usize, TaskError> {
        debug!(
            "sys_write @ fd: {}, buf_ptr: {:?}, count: {}",
            fd, buf_ptr, count
        );
        let buffer = unsafe { core::slice::from_raw_parts(buf_ptr.as_ptr(), count) };
        let mut file = self.task.get_fd(fd).expect("invalid fd");
        let result = file.write(buffer)?;
        debug!("sys_write result: {}", result);
        Ok(result)
    }

    pub async fn sys_mkdirat(
        &mut self,
        dirfd: isize,
        path_str: &str,
        _mode: usize,
    ) -> Result<usize, TaskError> {
        debug!(
            "sys_mkdirat @ dirfd: {}, path: {}, mode: {}",
            dirfd, path_str, _mode
        );

        let task = self.task.clone();
        let cwd;
        if dirfd == AT_FDCWD {
            let path = task.pcb.lock().curr_dir.to_string();
            cwd = File::open(&path, OpenFlags::O_DIRECTORY | OpenFlags::O_RDWR)?;
        } else {
            cwd = self.task.get_fd(dirfd as usize).expect("invalid dirfd");
        }
        cwd.mkdir_at(path_str)?;
        test_ls();
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
        let mode = mode as u32;
        debug!("sys_openat @ dirfd: {}, filename: {}, flags: {:?}, mode: {}", dirfd, filename, flags, mode);
        let cwd = if dirfd as isize == -100 {
            File::open(
                &self.task.pcb.lock().curr_dir.to_string(),
                OpenFlags::O_DIRECTORY | OpenFlags::O_RDWR,
            )?
        } else {
            self.task.get_fd(dirfd).ok_or(TaskError::EBADF)?
        };
        let file = cwd.open_at(&filename, flags)?;
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

    pub async fn sys_getdents64(
        &self,
        fd: usize,
        buf_ptr: UserBuf<u8>,
        len: usize, // Max length of user-space buffer
    ) -> Result<usize, TaskError> {
        debug!(
            "sys_getdents64 @ fd: {}, user_buf_ptr: {:?}, user_buf_len: {}",
            fd, buf_ptr, len
        );

        let file = self.task.get_fd(fd).ok_or(TaskError::EBADF)?;

        let mut dir_entries: Vec<DirEntry> = Vec::new();
        // file.getdents now populates dir_entries and returns the count of entries read.
        // We don't strictly need the count here as we'll iterate over dir_entries.
        let _num_entries = file.getdents(&mut dir_entries)?;

        let mut user_output_bytes: Vec<u8> = Vec::with_capacity(len);
        let mut current_total_bytes_in_user_output = 0;

        for entry in dir_entries {
            let name_bytes = entry.filename.as_bytes(); // Changed: d_name -> filename
            let name_len = name_bytes.len();

            // struct linux_dirent64 {
            //   d_ino (u64): 8 bytes
            //   d_off (i64): 8 bytes
            //   d_reclen (u16): 2 bytes
            //   d_type (u8): 1 byte
            //   d_name[]: name_len + 1 (for null terminator)
            // }
            let fixed_part_size = 8 + 8 + 2 + 1; // 19 bytes
            let d_reclen_unaligned = fixed_part_size + name_len + 1; // +1 for null terminator
            let d_reclen_aligned = (d_reclen_unaligned + 7) & !7; // Align to 8 bytes

            if current_total_bytes_in_user_output + d_reclen_aligned > len {
                // Not enough space in user buffer for this entry
                break;
            }

            // d_ino - Placeholder, as DirEntry doesn't store inode number directly
            let d_ino_val: u64 = 1; // Placeholder for inode number, as DirEntry doesn't store it.
            user_output_bytes.extend_from_slice(&d_ino_val.to_ne_bytes());

            // d_off: Offset of the next dirent structure. Here, it's the offset after this one.
            let d_off = (current_total_bytes_in_user_output + d_reclen_aligned) as i64;
            user_output_bytes.extend_from_slice(&d_off.to_ne_bytes());

            // d_reclen
            user_output_bytes.extend_from_slice(&(d_reclen_aligned as u16).to_ne_bytes());

            // d_type
            let dt_type: u8 = match entry.file_type { // Changed: d_type -> file_type
                FileType::File => 8,      // DT_REG
                FileType::Directory => 4, // DT_DIR
                _ => 0,                   // DT_UNKNOWN
            };
            user_output_bytes.push(dt_type);

            // d_name (with null terminator)
            user_output_bytes.extend_from_slice(name_bytes);
            user_output_bytes.push(0); // Null terminator

            // Padding to d_reclen_aligned
            let current_entry_len = fixed_part_size + name_len + 1;
            if d_reclen_aligned > current_entry_len {
                user_output_bytes.resize(current_total_bytes_in_user_output + d_reclen_aligned, 0);
            }
            
            current_total_bytes_in_user_output += d_reclen_aligned;
        }

        if !user_output_bytes.is_empty() {
            buf_ptr.write_slice(&user_output_bytes); // Changed: Removed ? operator
        }
        
        Ok(current_total_bytes_in_user_output)
    }
}
