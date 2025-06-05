#![no_std]
#![no_main]
use console::println;
use core::panic::PanicInfo;
use core::ptr::NonNull;
use device::init_dt;
use filesystem::init_fs;
use filesystem::file::File;
use filesystem::path::Path;
use filesystem::vfs::OpenFlags;
use flat_device_tree;
use flat_device_tree::{node::FdtNode, standard_nodes::Compatible, Fdt};
use heap;
use device::device_set::{get_device, get_block_device}; // Changed to get_block_device
use virtio::blk::VirtioBlkDriver;
use device::{Driver, BlockDriver}; // define module removed
use device::DeviceType as EchoDeviceType; // define module removed
use log::{debug, error, info, warn};
use virtio::halimpl::HalImpl;
use virtio_drivers::{
    device::blk::VirtIOBlk,
    transport::{
        mmio::{MmioTransport, VirtIOHeader},
        DeviceType, Transport,
    },
};
extern crate alloc;
use alloc::vec;
use crate::alloc::string::ToString;
use boot;
use device::device_set;
use frame;
use alloc::sync::Arc;

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
    loop {}
}



#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(hartid: usize, dtb: usize) -> ! {
    console::init();
    println!("hart_id : {:x} dtb: {:x}", hartid, dtb);
    heap::init();

    init_dt(dtb);
    init_fs();
    test_file(); // Call the test function
    
    info!("kernel_end");
    arch::os_shut_down();
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
                            Ok(s) => info!("Content of /hello.txt: \"{}\"", s.trim_end_matches('\0')),
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
                    if size == 48 { // Expected size for hello.txt
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