use filesystem::vfs::VfsError;

//!
//! 任务错误类型定义模块。
//!
//! 提供 TaskError 枚举及与 VfsError 的转换、错误码与字符串描述。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 任务相关错误类型。
pub enum TaskError {
    /// 操作不被允许（EPERM）
    EPERM,
    /// 无效操作
    Invalid,
    /// 参数无效（EINVAL）
    EINVAL,
    /// 错误的文件描述符（EBADF）
    EBADF,
    /// 打开文件过多（EMFILE）
    EMFILE,
    /// 错误地址（EFAULT）
    EFAULT,
    /// 无效的 clone 标志
    InvalidCloneFlags,
    /// 无子进程（ECHILD）
    ECHILD,
    /// 内存不足（ENOMEM）
    ENOMEM,
    /// 文件表已满（ENFILE）
    ENFILE,
    /// 无此进程（ESRCH）
    ESRCH,
    /// 不是终端设备（ENOTTY）
    ENOTTY,
    /// VFS 层错误
    Vfs(VfsError),
}

impl From<VfsError> for TaskError {
    /// VfsError 到 TaskError 的转换。
    fn from(e: VfsError) -> Self {
        Self::Vfs(e)
    }
}

impl TaskError {
    /// 获取错误类型的字符串描述。
    pub fn as_str(&self) -> &str {
        match self {
            TaskError::EBADF => "Bad file descriptor",
            TaskError::EPERM => "Operation not permitted",
            TaskError::Invalid => "Invalid",
            TaskError::EINVAL => "Invalid argument",
            TaskError::EMFILE => "Too many open files",
            TaskError::InvalidCloneFlags => "Invalid clone flags",
            TaskError::ECHILD => "No child process",
            TaskError::ENOMEM => "Out of memory",
            TaskError::ESRCH => "No such process",
            TaskError::ENOTTY => "Not a tty",
            TaskError::Vfs(_) => "VfsError",
            TaskError::EFAULT => "Bad address",
            TaskError::ENFILE => "Too many open files",
        }
    }

    /// 转换为原始错误码（isize）。
    pub fn into_raw(self) -> isize {
        match self {
            TaskError::EMFILE => 24, // EMFILE
            TaskError::EPERM => 1,  // EPERM
            TaskError::Invalid => 22, // EINVAL
            TaskError::ENFILE => 25,
            TaskError::EINVAL => 22, // EINVAL
            TaskError::EBADF => 9, // EBADF
            TaskError::InvalidCloneFlags => 22, // EINVAL
            TaskError::ECHILD => 10, // ECHILD
            TaskError::ENOMEM => 12, // ENOMEM
            TaskError::ESRCH => 3, // ESRCH
            TaskError::ENOTTY => 25, // ENOTTY
            TaskError::Vfs(_) => 2, // ENOENT
            TaskError::EFAULT => 14, // EFAULT
        }
    }
}
