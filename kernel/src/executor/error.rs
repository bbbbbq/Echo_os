#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskError {
    NotFound,
    Invalid, // Represents other, more general errors
}

impl TaskError {
    pub fn into_raw(self) -> isize {
        match self {
            TaskError::NotFound => 1,  // EPERM
            TaskError::Invalid => 22, // EINVAL
        }
    }
}
