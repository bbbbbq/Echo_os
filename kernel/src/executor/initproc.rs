use arch::os_shut_down;

use crate::executor::error;
use crate::executor::ops::yield_now;
use crate::executor::executor::{release_task, TASK_MAP, tid2task};
use crate::executor::thread::add_user_task;
use alloc::vec::Vec;
use filesystem::file::OpenFlags;
use filesystem::file::File;
use console::println;
use log::info;
use log::debug;
use crate::executor::task::TaskType;
use log::error;
use alloc::vec;

//!
//! initproc 模块：内核初始化进程与命令调度。
//!
//! 提供内核启动后第一个用户进程的创建与命令执行流程。

/// 执行指定命令字符串。
///
/// # 参数
/// - `cmd`: 命令字符串。
async fn command(cmd: &str) {
    let mut args: Vec<&str> = cmd.split(" ").filter(|x| *x != "").collect();
    debug!("cmd: {}  args: {:?}", cmd, args);
    let filename = args.drain(..1).last().unwrap();
    match File::open(filename.into(), OpenFlags::O_RDONLY) {
        Ok(_) => {
            info!("exec: {}", filename);
            let mut args_extend = vec![filename];
            args_extend.extend(args.into_iter());
            info!("Final arguments: {:?}", args_extend);
            let task_id = add_user_task(&filename, args_extend, Vec::new());
            info!("Task created with ID: {:?}", task_id);
            let task = tid2task(task_id).unwrap();

            loop {
                if task.exit_code().is_some() {
                    release_task(task_id);
                    break;
                }
                yield_now().await;
            }
        }
        Err(_) => {
            error!("unknown command: {}", cmd);
        }
    }
}

/// 内核初始化进程主入口。
///
/// 负责启动第一个用户进程（如 shell），并在无用户任务时关闭系统。
pub async fn initproc() {
    println!("start kernel tasks");
    command("busybox sh").await;
    //command("bin/ls").await;
    // command("basic/brk").await;
    // command("basic/chdir").await;
    // command("basic/clone").await;
    // command("basic/close").await;
    // command("basic/dup").await;
    // command("basic/dup2").await;
    // command("basic/execve").await;
    // command("basic/exit").await;
    // command("basic/fork").await;
    // command("basic/fstat").await;
    // command("basic/getcwd").await;
    // command("basic/getdents").await;
    // command("basic/getpid").await;
    // command("basic/getppid").await;
    // command("basic/gettimeofday").await;
    // command("basic/mkdir").await;
    // command("basic/mmap").await;
    // command("basic/mount").await;
    // command("basic/munmap").await;
    // command("basic/open").await;
    // command("basic/openat").await;
    // command("basic/pipe").await;
    // command("basic/read").await;
    // command("basic/sleep").await;
    // command("basic/times").await;
    // command("basic/umount").await;
    // command("basic/uname").await;
    // command("basic/unlink").await;
    // command("basic/wait").await;
    // command("basic/waitpid").await;
    // command("basic/yield").await;

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
