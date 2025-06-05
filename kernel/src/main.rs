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
        }
        Err(e) => {
            error!("Failed to open /hello.txt: {:?}", e);
        }
    }

    info!("--- Starting devfs tests ---");

    // Test /dev/null
    info!("Testing /dev/null");
    let null_path = Path::new("/dev/null".to_string());
    match File::open(null_path.clone(), OpenFlags::O_RDWR) {
        Ok(null_file) => {
            info!("Successfully opened /dev/null");
            let mut read_buf = [0u8; 10];
            match null_file.read_at(&mut read_buf) {
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        info!("/dev/null read returned 0 bytes as expected.");
                    } else {
                        error!("/dev/null read returned {} bytes, expected 0.", bytes_read);
                    }
                }
                Err(e) => error!("Failed to read from /dev/null: {:?}", e),
            }

            let write_buf = [1, 2, 3, 4, 5];
            match null_file.write_at(&write_buf) {
                Ok(bytes_written) => {
                    if bytes_written == write_buf.len() {
                        info!("/dev/null write reported {} bytes written as expected.", bytes_written);
                    } else {
                        error!("/dev/null write reported {} bytes, expected {}.", bytes_written, write_buf.len());
                    }
                }
                Err(e) => error!("Failed to write to /dev/null: {:?}", e),
            }
        }
        Err(e) => error!("Failed to open /dev/null: {:?}", e),
    }

    // Test /dev/zero
    info!("Testing /dev/zero");
    let zero_path = Path::new("/dev/zero".to_string());
    match File::open(zero_path.clone(), OpenFlags::O_RDWR) {
        Ok(zero_file) => {
            info!("Successfully opened /dev/zero");
            let mut read_buf = [1u8; 10]; // Pre-fill with non-zero to check
            match zero_file.read_at(&mut read_buf) {
                Ok(bytes_read) => {
                    if bytes_read == read_buf.len() {
                        let mut all_zeros = true;
                        for &byte in read_buf.iter() {
                            if byte != 0 {
                                all_zeros = false;
                                break;
                            }
                        }
                        if all_zeros {
                            info!("/dev/zero read returned 10 zero bytes as expected.");
                        } else {
                            error!("/dev/zero read did not fill buffer with zeros. Buffer: {:?}", read_buf);
                        }
                    } else {
                        error!("/dev/zero read returned {} bytes, expected {}.", bytes_read, read_buf.len());
                    }
                }
                Err(e) => error!("Failed to read from /dev/zero: {:?}", e),
            }

            let write_buf = [1, 2, 3, 4, 5];
            match zero_file.write_at(&write_buf) {
                Ok(bytes_written) => {
                    if bytes_written == write_buf.len() {
                        info!("/dev/zero write reported {} bytes written as expected.", bytes_written);
                    } else {
                        error!("/dev/zero write reported {} bytes, expected {}.", bytes_written, write_buf.len());
                    }
                }
                Err(e) => error!("Failed to write to /dev/zero: {:?}", e),
            }
        }
        Err(e) => error!("Failed to open /dev/zero: {:?}", e),
    }

    // Test /dev/uart
    info!("Testing /dev/uart");
    let uart_path = Path::new("/dev/uart".to_string());
    match File::open(uart_path.clone(), OpenFlags::O_WRONLY) { // Or O_RDWR if read is implemented
        Ok(uart_file) => {
            info!("Successfully opened /dev/uart");
            let uart_msg = "Hello UART from devfs test!\n";
            match uart_file.write_at(uart_msg.as_bytes()) {
                Ok(bytes_written) => {
                    if bytes_written == uart_msg.len() {
                        info!("/dev/uart write reported {} bytes written. Check console for: '{}'", bytes_written, uart_msg.trim_end());
                    } else {
                        error!("/dev/uart write reported {} bytes, expected {}.", bytes_written, uart_msg.len());
                    }
                }
                Err(e) => error!("Failed to write to /dev/uart: {:?}", e),
            }
        }
        Err(e) => error!("Failed to open /dev/uart: {:?}", e),
    }
    info!("--- Finished devfs tests ---");
}