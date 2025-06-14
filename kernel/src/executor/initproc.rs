use arch::os_shut_down;
use crate::executor::ops::yield_now;
use crate::executor::executor::{release_task, TASK_MAP, tid2task};
use alloc::vec::Vec;
use filesystem::vfs::OpenFlags;
use filesystem::file::File;
use console::println;
use log::info;
use log::debug;
use crate::executor::task::TaskType;
use crate::executor::thread::add_user_task;
use alloc::vec;

async fn command(cmd: &str) {
    info!("Command started: {}", cmd);
    let mut args: Vec<&str> = cmd.split(" ").filter(|x| *x != "").collect();
    debug!("cmd: {}  args: {:?}", cmd, args);
    let filename = args.drain(..1).last().unwrap();
    info!("Attempting to execute file: {}", filename);
    match File::open(filename.into(), OpenFlags::O_RDONLY) {
        Ok(_) => {
            info!("File exists, preparing to execute: {}", filename);
            let mut args_extend = vec![filename];
            args_extend.extend(args.into_iter());
            info!("Final arguments: {:?}", args_extend);
            let task_id = add_user_task(&filename, args_extend, Vec::new()).await;
            info!("Task created with ID: {:?}", task_id);
            let task = tid2task(task_id).unwrap();
            loop {
                if task.exit_code().is_some() {
                    release_task(task_id);
                    break;
                }
                yield_now().await;
            }
            info!("Command completed: {}", cmd);
        }
        Err(e) => {
            info!("Failed to open file: {}, error: {:?}", filename, e);
            println!("unknown command: {}", cmd);
        }
    }
}

pub async fn initproc() {
    println!("start kernel tasks");
    // command("brk").await;
    // command("chdir").await;
    // command("clone").await;
    // command("close").await;
    // command("dup").await;
    // command("dup2").await;
    // command("execve").await;
    // command("exit").await;
    // command("fork").await;
    // command("fstat").await;
    // command("getcwd").await;
    command("getppid").await;
    // Shutdown if there just have blankkernel task.
    if TASK_MAP
        .lock()
        .values()
        .find(|x| x.get_task_type() != TaskType::Kernel)
        .is_none()
    {
        os_shut_down();
    }
}
