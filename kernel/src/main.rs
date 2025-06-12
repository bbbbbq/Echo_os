#![no_std]
#![no_main]
use alloc::vec;
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
pub mod executor;
use crate::alloc::string::ToString;
use boot;
use executor::thread::UserTask;
pub mod user_handler;
use crate::executor::executor::{GLOBLE_EXECUTOR, TASK_QUEUE, info_task_queue, spawn_blank};
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
    loop {}
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
    spawn_blank(initproc());
    info_task_queue();
    GLOBLE_EXECUTOR.lock().run_ready_task();
    info!("kernel_end");
    os_shut_down();
    loop {}
}

pub fn test_file() {
    info!("Attempting to open /hello.txt");
    let path = Path::new("/hello.txt".to_string());
    let flags = OpenFlags::O_RDONLY;

    match File::open(path, flags) {
        Ok(_file) => {
            info!("Successfully opened /hello.txt");
            let mut buffer = [0u8; 64]; // Buffer to read file content
            match _file.read_at(&mut buffer) {
                Ok(bytes_read) => {
                    if bytes_read > 0 {
                        // Attempt to convert the read bytes to a UTF-8 string
                        match core::str::from_utf8(&buffer[..bytes_read]) {
                            Ok(s) => {
                                info!("Content of /hello.txt: \"{}\"", s.trim_end_matches('\0'))
                            }
                            Err(_) => error!("Content of /hello.txt is not valid UTF-8"),
                        }
                    } else {
                        info!("/hello.txt is empty or read 0 bytes.");
                    }
                }
                Err(e) => error!("Failed to read /hello.txt: {:?}", e),
            }

            // Test get_file_size for /hello.txt
            match _file.get_file_size() {
                Ok(size) => {
                    if size == 48 {
                        // Expected size for hello.txt
                        info!("/hello.txt get_file_size returned {} as expected.", size);
                    } else {
                        error!("/hello.txt get_file_size returned {}, expected 48.", size);
                    }
                }
                Err(e) => error!("Failed to get_file_size for /hello.txt: {:?}", e),
            }
        }
        Err(e) => {
            error!("Failed to open /hello.txt: {:?}", e);
        }
    }
}

// pub fn test_thread() {
//     let path = Path::new("/test/hello".to_string());
//     let file = File::open(path, OpenFlags::O_RDONLY).unwrap();
//     let _thread = proc::thread::Thread::new_thread(file, vec![], "hello".to_string(), vec![]);
// }
