#![no_std]
#![no_main]
use console::println;
use core::panic::PanicInfo;
use flat_device_tree;
use virtio::halimpl::HalImpl;
use heap;
use flat_device_tree::{node::FdtNode, standard_nodes::Compatible, Fdt};
use virtio_drivers::{
    device::{
        blk::VirtIOBlk,
    },
    transport::{
        mmio::{MmioTransport, VirtIOHeader},
        DeviceType, Transport,
    },
};
use core::ptr::NonNull;
use log::{info, warn, error, debug};
extern crate alloc;
use alloc::vec;
use frame;
use boot;

/// This is the entry point of the kernel.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "[panic] Panicked at {}:{} \n\t{}",
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        println!("[panic] Panicked: {}", info.message());
    }
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(hartid: usize, dtb: usize) -> ! {
    console::init();
    println!("hart_id : {:x} dtb: {:x}", hartid, dtb);
    heap::init();
    
    info!("Running memory frame alignment tests...");
    let (success, failure) = frame::test_frame_allocation();
    if failure > 0 {
        error!("Frame alignment tests failed: {} failures, {} successes", failure, success);
    } else {
        info!("All frame alignment tests passed: {} successes", success);
    }
    
    // Test specifically the size needed for VirtIO (usually 2 pages)
    let virtio_frames_aligned = frame::test_frame_alignment(2);
    if virtio_frames_aligned {
        info!("VirtIO frame alignment test passed");
    } else {
        error!("VirtIO frame alignment test failed - frames not properly aligned to 4K boundaries");
    }
    
    // Continue with device tree initialization
    init_dt(dtb);
    loop {}
}




fn init_dt(dtb: usize) {
    info!("device tree @ {:#x}", dtb);
    // Safe because the pointer is a valid pointer to unaliased memory.
    let fdt = unsafe { Fdt::from_ptr(dtb as *const u8).unwrap() };
    walk_dt(fdt);
}

fn walk_dt(fdt: Fdt) {
    for node in fdt.all_nodes() {
        if let Some(compatible) = node.compatible() {
            if compatible.all().any(|s| s == "virtio,mmio") {
                virtio_probe(node);
            }
        }
    }
}

fn virtio_probe(node: FdtNode) {
    if let Some(reg) = node.reg().next() {
        let paddr = reg.starting_address as usize;
        let size = reg.size.unwrap();
        let vaddr = paddr;
        info!("walk dt addr={:#x}, size={:#x}", paddr, size);
        info!(
            "Device tree node {}: {:?}",
            node.name,
            node.compatible().map(Compatible::first),
        );
        let header = NonNull::new(vaddr as *mut VirtIOHeader).unwrap();
        match unsafe { MmioTransport::new(header) } {
            Err(e) => warn!("Error creating VirtIO MMIO transport: {}", e),
            Ok(transport) => {
                info!(
                    "Detected virtio MMIO device with vendor id {:#X}, device type {:?}, version {:?}",
                    transport.vendor_id(),
                    transport.device_type(),
                    transport.version(),
                );
                virtio_device(transport);
            }
        }
    }
}

fn virtio_device(transport: impl Transport) {
    match transport.device_type() {
        DeviceType::Block => virtio_blk(transport),
        t => warn!("Unrecognized virtio device: {:?}", t),
    }
}


fn virtio_blk<T: Transport>(transport: T) {
    info!("virtio-blk test start");
    let mut blk = VirtIOBlk::<HalImpl, T>::new(transport).expect("failed to create blk driver");
    let mut input = vec![0xffu8; 512];
    let mut output = vec![0; 512];
    for i in 0..32 {
        for x in input.iter_mut() {
            *x = i as u8;
        }
        blk.write_blocks(i, &input).expect("failed to write");
        blk.read_blocks(i, &mut output).expect("failed to read");
        assert_eq!(input, output);
    }
    info!("virtio-blk test finished");
}