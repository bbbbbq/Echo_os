#[derive(Debug)]
pub enum TaskError {
    InvalidPath,
    ElfLoadError,
    NotFound,
    InvalidArgument,
    PermissionDenied,
    MemoryError,
    ExecutionError,
    ResourceBusy,
    IoError,
    Timeout,
    Interrupted,
    NotSupported,
}
