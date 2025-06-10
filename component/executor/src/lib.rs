#![no_std]

extern crate alloc;

pub mod executor;
pub mod id;
pub mod kernel_task;
pub mod ops;
pub mod select;
pub mod task_def;
pub mod waker;

pub enum TaskType {
    Kernel,
    Thread,
}

pub enum ExitCode {
    Normal(i32),
    Killed(i32),
}
