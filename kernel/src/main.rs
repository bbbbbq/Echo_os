#![no_std]
#![no_main]
use console::println;
use core::panic::PanicInfo;
use device::init_dt;
use filesystem::file::File;
use filesystem::init_fs;
use filesystem::path::Path;
use filesystem::vfs::OpenFlags;
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
use arch::os_shut_down;
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
    unreachable!()
}

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

    info!("\n\n\\n\n\n\n\n");
    // test_ls();
    spawn_blank(initproc());
    info_task_queue();
    GLOBLE_EXECUTOR.run();
    info!("kernel_end");
    arch::os_shut_down();
    loop {}
}

pub fn test_ls() {
    let file = File::open(&"/".to_string(), OpenFlags::O_DIRECTORY | OpenFlags::O_RDWR).unwrap();
    let mut buffer = Vec::<DirEntry>::new();
    file.getdents(&mut buffer).unwrap();
    for entry in buffer {
        println!("{}", entry.filename);
    }
    // os_shut_down();
}
