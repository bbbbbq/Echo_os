#![no_std]
extern crate alloc;

pub mod cache;
pub mod error;
pub mod usertask;
use log::info;
use executor::spawn_kernel_task;
use executor::run_task;

// pub enum TaskError {
//     NotFound,
//     InvalidArgument,
//     PermissionDenied,
//     MemoryError,
//     ExecutionError,
//     ResourceBusy,
//     IoError,
//     Timeout,
//     Interrupted,
//     NotSupported,
// }





pub fn init()
{
    spawn_kernel_task(initproc());
}

async fn initproc()
{
    info!("initproc");
    loop {
        
    }
}

pub fn run_tasks() {
    run_task();
}