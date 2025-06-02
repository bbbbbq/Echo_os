#![no_std]
#![no_main]
use console::println;
use core::panic::PanicInfo;
use core::ptr::NonNull;
use device::init_dt;
use flat_device_tree;
use flat_device_tree::{node::FdtNode, standard_nodes::Compatible, Fdt};
use heap;
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
use boot;
use device::device_set;
use frame;

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
    
    let blk_device = device_set::get_device(0);
    if let Some(ref device) = blk_device {
        let device_type = device.get_type();
        let type_str = match device_type {
            device::define::DeviceType::Block => "Block",
            device::define::DeviceType::Network => "Network",
            device::define::DeviceType::Console => "Console",
            device::define::DeviceType::Unknown => "Unknown",
        };
        info!("device_type: {}", type_str);
    } else {
        info!("No device found at index 0");
    }

    if let Some(block_device) = blk_device {
        let device_type = block_device.get_type();
        info!("Detected device of type: {:?}", match device_type {
            device::define::DeviceType::Block => "Block",
            device::define::DeviceType::Network => "Network",
            device::define::DeviceType::Console => "Console",
            _ => "Unknown",
        });
        
        if let Some(block_driver) = block_device.as_block_driver() {
            let mut buffer = [0u8; 512];
            match block_driver.read(2, &mut buffer) {
                Ok(_) => {
                    info!("Block device read test successful");
                    info!("First bytes: {:?}", &buffer[..512]);
                }
                Err(e) => {
                    error!("Block device read failed: {}", e);
                }
            }
        } else {
            warn!("Device does not implement BlockDriver trait");
        }
    }

    info!("kernel_end");
    loop {}
}
