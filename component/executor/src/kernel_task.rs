use crate::{
    ExitCode, TaskType,
    id::{TaskId, alloc_task_id},
};

use super::task_def::TaskTrait;

pub struct KernelTask {
    kernel_task_id: TaskId,
}

impl TaskTrait for KernelTask {
    fn get_task_id(&self) -> TaskId {
        self.kernel_task_id
    }

    fn get_task_type(&self) -> TaskType {
        TaskType::Kernel
    }

    fn before_run(&self) {}

    fn get_exit_code(&self) -> ExitCode {
        unimplemented!();
    }

    fn exit(&self) {
        unimplemented!();
    }
}

impl KernelTask {
    pub fn new() -> Self {
        let id = alloc_task_id();
        Self { kernel_task_id: id }
    }
}
