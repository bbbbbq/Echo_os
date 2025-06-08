use crate::{ExitCode, TaskType, id::TaskId};

use super::task_def::TaskTrait;

pub struct KernelTask {
    kernel_task_id: usize,
}

impl TaskTrait for KernelTask {
    fn get_task_id(&self) -> TaskId {
        TaskId(self.kernel_task_id)
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

