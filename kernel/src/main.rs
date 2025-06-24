#![no_std]
#![no_main]
use arch::os_shut_down;
use console::println;
use mem::pagetable::PageTable;
use memory_addr::VirtAddr;
use page_table_multiarch::MappingFlags;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};
use device::init_dt;
use filesystem::file::File;
use filesystem::init_fs;
use filesystem::file::OpenFlags;
use heap;
// Changed to get_block_device
// define module removed
// define module removed
use log::{error, info};
extern crate alloc;
use alloc::vec::Vec;
use filesystem::vfs::DirEntry;
pub mod executor;
use crate::alloc::string::ToString;
use boot;
pub mod user_handler;
use crate::executor::executor::{GLOBLE_EXECUTOR, info_task_queue, spawn_blank};
use crate::executor::initproc::initproc;
use boot::boot_page_table;
<<<<<<< HEAD
pub mod backtrace;
use backtrace::backtrace;
=======
pub mod signal;
>>>>>>> 73599fce51808454c7e446d9fc82074df6e31d3d

//! Echo_os 内核主入口模块
//!
//! 负责内核初始化、主循环、异常处理等核心功能。

/// 内核 panic 时的处理函数。
///
/// # 参数
/// * `info` - panic 信息。
///
/// 该函数会打印 panic 信息并关闭系统。
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        error!(
            "[panic] Panicked at {}:{} \n\t{}",
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        error!("[panic] Panicked: {}", info.message());
    }
    os_shut_down();
}

/// 内核主入口函数。
///
/// # 参数
/// * `hartid` - 当前 CPU 的硬件线程编号。
/// * `dtb` - 设备树地址。
///
/// # 安全
/// 该函数为裸机入口，需保证调用环境正确。
///
/// # 行为
/// 初始化控制台、内存、异常、设备树、文件系统等，
/// 并启动第一个用户进程，进入任务调度主循环。
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(hartid: usize, dtb: usize) -> ! {
    console::init();
    unsafe {
        info!("boot_page_table: {:x}", boot_page_table());
    }
    println!("hart_id : {:x} dtb: {:x}", hartid, dtb);
    heap::init();
    trap::trap::init();
    init_dt(dtb);
    init_fs();
<<<<<<< HEAD

    info!("\n\n\n\n\n\n");
    // test_ls();
=======
    // test_pagetable();
    // loop{}
    info!("\n\n\n\n\n\n");
    //test_ls();
>>>>>>> 73599fce51808454c7e446d9fc82074df6e31d3d
    spawn_blank(initproc());
    info_task_queue();
    GLOBLE_EXECUTOR.run();
    info!("kernel_end");
    arch::os_shut_down();
}

/// 测试文件系统根目录下的文件列表。
///
/// # 示例
/// ```
/// test_ls();
/// ```
pub fn test_ls() {
<<<<<<< HEAD
    let file = File::open(&"/".to_string(), OpenFlags::O_DIRECTORY | OpenFlags::O_RDWR).unwrap();
    let mut buffer = Vec::<DirEntry>::new();
    file.getdents(&mut buffer).unwrap();
    for entry in buffer {
        println!("{}", entry.filename);
    }
    // os_shut_down();
=======
    let file = File::open(&"/.".to_string(), OpenFlags::O_DIRECTORY | OpenFlags::O_RDWR).unwrap();
    info!("file: {:?}", file);
    os_shut_down();
>>>>>>> 73599fce51808454c7e446d9fc82074df6e31d3d
}