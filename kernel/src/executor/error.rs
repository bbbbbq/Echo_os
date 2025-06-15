use filesystem::vfs::VfsError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskError {
    EPERM,
    Invalid,
    EINVAL,
    EBADF,
    EMFILE,
    EFAULT,
    InvalidCloneFlags,
    ECHILD,
    ENOMEM,
    ENFILE,
    Vfs(VfsError),
}

impl From<VfsError> for TaskError {
    fn from(e: VfsError) -> Self {
        Self::Vfs(e)
    }
}

impl TaskError {
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
            TaskError::Vfs(_) => "VfsError",
            TaskError::EFAULT => "Bad address",
            TaskError::ENFILE => "Too many open files",
        }
    }

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
            TaskError::Vfs(_) => 2, // ENOENT
            TaskError::EFAULT => 14, // EFAULT
        }
    }
}
