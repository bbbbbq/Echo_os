use crate::alloc::string::{String, ToString};
use crate::executor::error::TaskError;
use crate::executor::ops::{yield_now, sleep_for_duration, terminal_wait};
use filesystem::devfs::devfs::{DevFsDirInode, DevType};
use filesystem::file::{File, OpenFlags, Stat};
use filesystem::mount::{mount_inode, umount_fs};
use filesystem::path::Path;
use filesystem::pipe::create_pipe;
use filesystem::vfs::{DirEntry, FileAttr, FileType, VfsError};
use log::{debug, info};
use memory_addr::VirtAddr;
use struct_define::iov::IoVec;
use struct_define::poll_event::{PollEvent, PollFd};
use struct_define::termios::{Termios, Winsize, TCGETS, TCSETS, TCSETSW, TCSETSF, TIOCGWINSZ, TIOCSPGRP, TIOCGPGRP};
use struct_define::timespec::TimeSpec;
use crate::user_handler::handler::UserHandler;
use crate::user_handler::userbuf::UserBuf;
use timer::current_nsec;
use alloc::sync::Arc;
use alloc::vec::Vec;

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
            cwd = self.task.get_fd(dirfd as usize).expect("invalid dirfd").into();
        }
        cwd.mkdir_at(path_str)?;
        //test_ls();
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
        let filename = if filename.starts_with("./") {
            filename[2..].to_string()
        } else {
            filename
        };
        let flags = OpenFlags::from_bits_truncate(flags);
        let mode = mode as u32;
        debug!("sys_openat @ dirfd: {}, filename: {}, flags: {:?}, mode: {}", dirfd, filename, flags, mode);
        let dir_file = if dirfd as isize == -100 {
            self.task.get_cwd()
        } else {
            self.task.get_fd(dirfd).ok_or(TaskError::EBADF)?
        };
        let full_path = dir_file.path.join(&filename);
        let file = File::open(&full_path.to_string(), flags)?;
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
        //println!("sys_fstat @ fd: {} stat: {:?}", fd, stat);
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
            let dt_type: u8 = match entry.file_type {
                // Changed: d_type -> file_type
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

    pub async fn sys_read(
        &self,
        fd: usize,
        buf_ptr: UserBuf<u8>,
        count: usize,
    ) -> Result<usize, TaskError> {
        let mut file = self.task.get_fd(fd).ok_or(TaskError::EBADF)?;
        let mut buffer = unsafe { core::slice::from_raw_parts_mut(buf_ptr.ptr, count) };
        loop {
            match file.read(&mut buffer) {
                Ok(read_len) => return Ok(read_len),
                Err(VfsError::Again) => {
                    yield_now().await;
                    continue;
                }
                Err(err) => return Err(err.into()),
            }
        }
    }

    pub async fn sys_pipe2(
        &self,
        fds_ptr: UserBuf<u32>,
        _unknown: usize,
    ) -> Result<usize, TaskError> {
        debug!("sys_pipe2 @ fds_ptr: {}, _unknown: {}", fds_ptr, _unknown);
        let fds = fds_ptr.slice_mut_with_len(2);

        let (rx, tx) = create_pipe();
        let rx_file = File::new_dev(rx);
        let tx_file = File::new_dev(tx);
        let rx_fd = self.task.pcb.lock().fd_table.alloc(rx_file);
        let tx_fd = self.task.pcb.lock().fd_table.alloc(tx_file);
        fds[0] = rx_fd as u32;
        fds[1] = tx_fd as u32;
        // );

        // let dev_node = File::open(special, OpenFlags::RDONLY)?;
        // dev_node.mount(dir)?;
        Ok(0)
    }

    pub async fn sys_unlinkat(
        &self,
        dir_fd: isize,
        path: UserBuf<u8>,
        flags: usize,
    ) -> Result<usize, TaskError> {
        const AT_FDCWD: isize = -100;
        const AT_REMOVEDIR: usize = 0x200;

        let path_str = path.read_string();
        debug!(
            "sys_unlinkat @ dir_fd: {}, path: {}, flags: {:#x}",
            dir_fd, path_str, flags
        );

        let dir_file = if dir_fd == AT_FDCWD {
            self.task.get_cwd()
        } else {
            self.task.get_fd(dir_fd as usize).ok_or(TaskError::EBADF)?
        };

        if (flags & AT_REMOVEDIR) != 0 {
            // This is rmdir
            dir_file.rmdir(&path_str)?;
        } else {
            // This is unlink
            dir_file.remove(&path_str)?;
        }

        Ok(0)
    }

        pub async fn sys_mount(
        &self,
        source: UserBuf<u8>,
        target: UserBuf<u8>,
        fs_type: UserBuf<u8>,
        flags: usize,
        data: UserBuf<u8>,
    ) -> Result<usize, TaskError> {
        let source_str = source.read_string();
        let target_str = target.read_string();
        let fs_type_str = fs_type.read_string();
        let data_str = if data.ptr.is_null() {
            alloc::string::String::new()
        } else {
            data.read_string()
        };

        debug!(
            "sys_mount @ source: {}, target: {}, fs_type: {}, flags: {}, data: {}",
            source_str,
            target_str,
            fs_type_str,
            flags,
            data_str
        );
        let mut inode = DevFsDirInode::new();
        inode.set_dev_type(DevType::Null);
        let path = Path::from(target_str);
        mount_inode(Arc::new(inode), path);
        Ok(0)
    }

    pub async fn sys_umount(&self, target: UserBuf<u8>) -> Result<usize, TaskError> {
        let target_str = target.read_string();
        let path = Path::from(target_str);
        umount_fs(path);
        Ok(0)
    }

    pub async fn sys_ioctl(
        &self,
        fd: usize,
        request: usize,
        arg1: usize,
        arg2: usize,
        arg3: usize,
    ) -> Result<usize, TaskError> {
        debug!(
            "[task {:?}] ioctl: fd: {}, request: {:#x}, args: {:#x} {:#x} {:#x}",
            self.tid, fd, request, arg1, arg2, arg3
        );

        // 获取文件描述符对应的文件
        let file = self.task.get_fd(fd).ok_or(TaskError::EBADF)?;
        
        // 根据 request 处理不同的 ioctl 请求
        match request {
            // 处理终端属性获取请求
            TCGETS => {
                if arg1 != 0 {
                    // 创建默认的终端设置
                    let termios = Termios::default();
                    // 将设置写入用户提供的缓冲区
                    let termios_buf = UserBuf::<Termios>::new(arg1 as *mut Termios);
                    termios_buf.write(termios);
                }
                Ok(0)
            }
            // 处理终端属性设置请求
            TCSETS | TCSETSW | TCSETSF => {
                // 简单实现：接受请求但不实际改变设置
                Ok(0)
            }
            // 获取终端窗口大小
            TIOCGWINSZ => {
                if arg1 != 0 {
                    // 创建默认窗口大小（80列x24行）
                    let winsize = Winsize {
                        ws_row: 24,
                        ws_col: 80,
                        ws_xpixel: 0,
                        ws_ypixel: 0,
                    };
                    // 将窗口大小写入用户提供的缓冲区
                    let winsize_buf = UserBuf::<Winsize>::new(arg1 as *mut Winsize);
                    winsize_buf.write(winsize);
                }
                Ok(0)
            }
            // 获取前台进程组
            TIOCGPGRP => {
                if arg1 != 0 {
                    // 返回当前进程的进程组ID
                    let pgid_buf = UserBuf::<i32>::new(arg1 as *mut i32);
                    pgid_buf.write(self.task.process_id.0 as i32);
                }
                Ok(0)
            }
            // 设置前台进程组
            TIOCSPGRP => {
                // 简单实现：接受请求但不实际更改
                Ok(0)
            }
            // 其他 ioctl 请求 - 对于未知请求，返回成功但不执行任何操作
            _ => Ok(0),
        }
    }

    pub async fn sys_fstatat(
        &self,
        dir_fd: isize,
        path_ptr: UserBuf<u8>,
        stat_ptr: UserBuf<Stat>,
    ) -> Result<usize, TaskError> {
        let path_str = path_ptr.read_string();
        let path_str = if path_str.starts_with("./") {
            path_str[2..].to_string()
        } else {
            path_str
        };
        debug!("sys_fstatat @ dir_fd: {}, path: {}, stat_ptr: {:?}", dir_fd, path_str, stat_ptr);
        let dir_file = if dir_fd == AT_FDCWD {
            self.task.get_cwd()
        } else {
            self.task.get_fd(dir_fd as usize).ok_or(TaskError::EBADF)?
        };
        let full_path = dir_file.path.join(&path_str);
        debug!("full_path: {}", full_path.to_string());
        let file = File::open(full_path.to_string().as_str(), OpenFlags::O_RDONLY)?;
        let mut stat: Stat = Stat::default();
        file.stat(&mut stat)?;
        stat_ptr.write(stat);
        Ok(0)
    }

    pub async fn sys_fcntl(&self, fd: usize, cmd: usize, arg: usize) -> Result<usize, TaskError> {
        debug!("sys_fcntl @ fd: {}, cmd: {}, arg: {}", fd, cmd, arg);
        let mut file = self.task.get_fd(fd).ok_or(TaskError::EBADF)?;
        info!("file: {:?}", file);
        match cmd {
            // F_DUPFD: Duplicate file descriptor
            0 | 0x406 => self.sys_dup(fd).await,

            // F_GETFD: Get file descriptor flags
            1 => {
                if file.openflags.contains(OpenFlags::O_CLOEXEC) {
                    Ok(1) // FD_CLOEXEC is set
                } else {
                    Ok(0) // FD_CLOEXEC is not set
                }
            }

            // F_SETFD: Set file descriptor flags
            2 => {
                if arg & 1 != 0 {
                    file.openflags.insert(OpenFlags::O_CLOEXEC);
                } else {
                    file.openflags.remove(OpenFlags::O_CLOEXEC);
                }
                self.task.pcb.lock().fd_table.set(fd, file);
                Ok(0)
            }

            // F_GETFL: Get file status flags
            3 => Ok(file.openflags.bits()),

            // F_SETFL: Set file status flags
            4 => {
                file.openflags = OpenFlags::from_bits_truncate(arg);
                self.task.pcb.lock().fd_table.set(fd, file);
                Ok(0)
            }
            _ => Err(TaskError::EINVAL),
        }
    }

    
    pub async fn sys_writev(&self, fd: usize, iov: UserBuf<IoVec>, iocnt: usize) -> Result<usize, TaskError> {
        debug!("sys_writev @ fd: {}, iov: {}, iocnt: {}", fd, iov, iocnt);
        
        if !iov.is_valid() || iocnt == 0 {
            return Ok(0);
        }
        
        let mut wsize = 0;
        let iov = iov.slice_mut_with_len(iocnt);
        let mut file = self.task.get_fd(fd).ok_or(TaskError::EBADF)?;

        for io in iov {
            if io.base == 0 || io.len == 0 {
                continue;
            }
            let user_buf = UserBuf::<u8>::new(io.base as *mut u8);
            let buffer = user_buf.slice_mut_with_len(io.len);
            wsize += file.write(buffer)?;
        }

        Ok(wsize)
    }

     /// 优化后的 ppoll 实现，提高终端响应性
     pub async fn sys_ppoll(
        &self,
        poll_fds_ptr: UserBuf<PollFd>,
        nfds: usize,
        timeout_ptr: UserBuf<TimeSpec>,
        sigmask_ptr: usize,
    ) -> Result<usize, TaskError> {
        debug!(
            "sys_ppoll @ poll_fds_ptr: {}, nfds: {}, timeout_ptr: {}, sigmask_ptr: {:#X}",
            poll_fds_ptr, nfds, timeout_ptr, sigmask_ptr
        );
        
        // 检查参数有效性
        if nfds == 0 {
            return Ok(0);
        }
        
        let poll_fds = poll_fds_ptr.slice_mut_with_len(nfds);
        
        // 计算超时时间
        let (has_timeout, timeout_ns) = if timeout_ptr.is_valid() {
            let ts = timeout_ptr.get_ref();
            (true, ts.to_nsec())
        } else {
            (false, usize::MAX) // 无超时
        };
        
        // 设置结束时间
        let etime = if has_timeout {
            current_nsec() + timeout_ns
        } else {
            usize::MAX
        };
        
        // 对于终端的轮询，可以采取更主动的检测方式
        // 检查是否在监听终端输入
        let is_terminal_poll = nfds == 1 && 
            poll_fds[0].events.contains(PollEvent::IN) && 
            self.task.get_fd(poll_fds[0].fd as _)
                .map_or(false, |f| f.path.to_string().contains("tty") || f.path.to_string().contains("uart"));
        
        // 对终端输入采用更积极的轮询策略
        if is_terminal_poll {
            // 首先检查是否有输入
            poll_fds[0].revents = self.task.get_fd(poll_fds[0].fd as _)
                .map_or(PollEvent::NONE, |x| {
                    match x.inner.poll(poll_fds[0].events.clone()) {
                        Ok(events) => events,
                        Err(_) => PollEvent::ERR,
                    }
                });
                
            if poll_fds[0].revents != PollEvent::NONE {
                return Ok(1); // 有输入，立即返回
            }
            
            // 对于终端，使用特殊的等待函数
            // 这个函数使用自旋+短暂让出策略
            const TERMINAL_POLL_INTERVAL_MS: usize = 20;
            
            // 如果有超时，确保不会超过超时时间
            let wait_time = if has_timeout {
                let remaining_ns = etime.saturating_sub(current_nsec());
                let remaining_ms = remaining_ns / 1_000_000;
                if remaining_ms == 0 {
                    return Ok(0); // 已超时
                }
                remaining_ms.min(TERMINAL_POLL_INTERVAL_MS)
            } else {
                TERMINAL_POLL_INTERVAL_MS
            };
            
            // 使用特殊的终端等待函数
            terminal_wait(wait_time).await;
            
            // 再次检查是否有输入
            poll_fds[0].revents = self.task.get_fd(poll_fds[0].fd as _)
                .map_or(PollEvent::NONE, |x| {
                    match x.inner.poll(poll_fds[0].events.clone()) {
                        Ok(events) => events,
                        Err(_) => PollEvent::ERR,
                    }
                });
                
            return Ok(if poll_fds[0].revents != PollEvent::NONE { 1 } else { 0 });
        }
        
        // 对于非终端设备，使用原来的轮询逻辑
        let mut sleep_time_ms = 10; // 初始睡眠时间为10毫秒
        let max_sleep_time_ms = 200; // 缩短最大睡眠时间，提高响应性
        
        // 轮询循环
        loop {
            // 检查所有文件描述符
            let mut num = 0;
            for i in 0..nfds {
                poll_fds[i].revents = self.task.get_fd(poll_fds[i].fd as _)
                    .map_or(PollEvent::NONE, |x| {
                        match x.inner.poll(poll_fds[i].events.clone()) {
                            Ok(events) => events,
                            Err(_) => PollEvent::ERR, // 错误时返回ERR事件
                        }
                    });
                
                if poll_fds[i].revents != PollEvent::NONE {
                    num += 1;
                }
            }

            // 如果有事件发生或者超时，则返回
            if num > 0 || current_nsec() >= etime {
                return Ok(num);
            }
            
            // 检查超时
            if timeout_ptr.is_valid() && current_nsec() >= etime {
                debug!("ppoll 超时");
                return Ok(0); // 已超时
            }
            
            // 使用指数退避策略，但增长速度较慢，并设置较低的上限
            sleep_time_ms = (sleep_time_ms * 3 / 2).min(max_sleep_time_ms);
            
            // 计算实际睡眠时间
            let actual_sleep_time = if has_timeout {
                let remaining_ns = etime.saturating_sub(current_nsec());
                let remaining_ms = remaining_ns / 1_000_000;
                
                if remaining_ms == 0 {
                    return Ok(0); // 已超时
                }
                
                remaining_ms.min(sleep_time_ms as usize)
            } else {
                sleep_time_ms as usize
            };
            
            sleep_for_duration(actual_sleep_time).await;
        }
    }
}
