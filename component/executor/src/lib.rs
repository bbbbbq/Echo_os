#![no_std]

extern crate alloc;


pub mod task_def;
pub mod id;
pub mod select;
pub mod executor;
pub mod waker;
pub mod kernel_task;

pub enum TaskType {
    Kernel,
    Thread,
}

pub enum ExitCode {
    Normal(i32),
    Killed(i32),
}