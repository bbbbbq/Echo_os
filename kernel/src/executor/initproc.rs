use alloc::boxed::Box;
use arch::os_shut_down;

use log::error;
use crate::executor::ops::yield_now;
use crate::executor::executor::{release_task, tid2task, TASK_MAP, TASK_QUEUE};
use crate::executor::thread::add_user_task;
use alloc::vec::Vec;
use filesystem::file::OpenFlags;
use filesystem::file::File;
use console::println;
use log::info;
use log::debug;
use crate::executor::task::{AsyncTaskItem, TaskType};
use alloc::vec;

async fn command(cmd: &str) {
    let mut args: Vec<&str> = cmd.split(" ").filter(|x| *x != "").collect();
    debug!("cmd: {}  args: {:?}", cmd, args);
    let filename = args.drain(..1).last().unwrap();
    match File::open(filename.into(), OpenFlags::O_RDONLY) {
        Ok(_) => {
            info!("exec: {}", filename);
            let mut args_extend = vec![filename];
            args_extend.extend(args.into_iter());
            let task_id = add_user_task(&filename, args_extend, Vec::new()).await;
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


pub async fn initproc() {
    println!("start kernel tasks");
    command("busybox sh").await;
    command("busybox sh").await;
    //command("basic/brk").await;
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
    // command("getdents").await;
    // command("getpid").await;
    // command("getppid").await;
    // command("gettimeofday").await;
    // command("mkdir").await;
    // command("mmap").await;
    // command("mount").await;
    // command("munmap").await;
    // command("open").await;
    // command("openat").await;
    // command("pipe").await;
    // command("read").await;
    // command("sleep").await;
    // command("times").await;
    // command("umount").await;
    // command("uname").await;
    // command("unlink").await;
    // command("wait").await;
    // command("waitpid").await;
    // command("yield").await;

    // Shutdown if there just have blankkernel task.
    if let Some(task) = TASK_MAP
        .lock()
        .values()
        .find(|x| x.get_task_type() != TaskType::Kernel)
    {
        TASK_QUEUE.lock().push_back(AsyncTaskItem {
            task: task.clone(),
            future: Box::pin(async {}),
        });
    }
}
