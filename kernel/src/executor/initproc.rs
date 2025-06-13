use arch::os_shut_down;
use crate::executor::ops::yield_now;
use crate::executor::executor::{release_task, TASK_MAP, tid2task};
use alloc::vec::Vec;
use filesystem::vfs::OpenFlags;
use filesystem::file::File;
use crate::executor::id_alloc::alloc_tid;
use alloc::boxed::Box;
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
            // yield_now().await;
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
    // command("./runtest.exe -w entry-dynamic.exe argv").await;
    // command("./entry-dynamic.exe argv").await;
    // command("busybox echo run time-test").awaait;
    // command("time-test").await;

    // command("busybox sh basic/run-all.sh").await;

    // command("busybox echo run netperf_testcode.sh").await;
    // command("busybox sh netperf_testcode.sh").await;

    // command("busybox echo run busybox_testcode.sh").await;
    // command("busybox sh busybox_testcode.sh").awit;

    // command("busybox echo run libctest_testcode.sh").await;
    // command("busybox sh libctest_testcode.sh").await;
    // command("runtest.exe -w entry-static.exe utime").await;
    // command("busybox ln -s /busybox /bin/cat").await;
    // command("./bin/cat libctest_testcode.sh").await;
    // command("busybox ls -l /bin").await;
    // command("busybox ln -s /busybox /bin/ln").await;
    // command("busybox ln -s /busybox /bin/wget").await;
    // command("busybox ln -s /busybox /bin/xz").await;
    // command("busybox ls -l /bin").await;
    // command("busybox sh init.sh").await;
    // command("busybox ls -l /bin").await;

    command("chdir").await;
    // command("busybox echo run lua_testcode.sh").await;
    // command("busybox sh lua_testcode.sh").await;

    // command("busybox init").await;
    // command("busybox sh").await;
    // command("busybox sh init.sh").await;

    // command("busybox echo run cyclic_testcode.sh").await;
    // command("busybox sh cyclictest_testcode.sh").await;
    // kill_all_tasks().await;

    // command("libc-bench").await;

    // command("busybox echo run iperf_testcode.sh").await;
    // command("busybox sh iperf_testcode.sh").await;
    // kill_all_tasks().await;

    // command("busybox echo run iozone_testcode.sh").await;
    // command("busybox sh iozone_testcode.sh ").await;

    // command("busybox echo run lmbench_testcode.sh").await;
    // command("busybox sh lmbench_testcode.sh").await;

    // command("busybox echo run unixbench_testcode.sh").await;
    // command("busybox sh unixbench_testcode.sh").await;

    // command("copy-file-range-test-1").await;
    // command("copy-file-range-test-2").await;
    // command("copy-file-range-test-3").await;
    // command("copy-file-range-test-4").await;
    // command("interrupts-test-1").await;
    // command("interrupts-test-2").await;

    // switch_to_kernel_page_table();
    println!("!TEST FINISH!");

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
