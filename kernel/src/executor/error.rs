use filesystem::vfs::VfsError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskError {
    NotFound,
    Invalid, 
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
            TaskError::NotFound => "NotFound",
            TaskError::Invalid => "Invalid",
            TaskError::Vfs(_) => "VfsError", 
        }
    }

    pub fn into_raw(self) -> isize {
        match self {
            TaskError::NotFound => 1,  // EPERM
            TaskError::Invalid => 22, // EINVAL
            TaskError::Vfs(_) => 2, // ENOENT
        }
    }
}
