/// Unique identifier for a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub usize);


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskType {
    Kernel,
    User,
}


pub trait AsyncTask
{
    /// Get the id of the task
    fn get_task_id(&self) -> TaskId;
    /// Run befire the kernel
    fn before_run(&self);
    /// Get task type.
    fn get_task_type(&self) -> TaskType;
    /// Exit a task with exit code.
    fn exit(&self, exit_code: usize);
    /// Check if the task was exited successfully
    fn exit_code(&self) -> Option<usize>;
}


pub struct KernelTask
{
    id:TaskId
}

impl AsyncTask for KernelTask {
    fn get_task_id(&self) -> TaskId {
        self.id
    }

    fn before_run(&self) {
        unimplemented!();
    }

    fn get_task_type(&self) -> TaskType {
        TaskType::Kernel
    }

    fn exit(&self, _exit_code: usize) {
        unimplemented!();
    }

    fn exit_code(&self) -> Option<usize> {
        Some(0)
    }
}
