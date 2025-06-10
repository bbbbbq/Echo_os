use config::target::plat::VIRT_ADDR_START;
use core::{arch::global_asm, arch::naked_asm, ptr::addr_of_mut};
use riscv::register::satp;
// Define PTE flags as a simple bitflags enum
use bitflags::bitflags;
use console::println;
// Helper function to create page table entries
fn create_pte(addr: usize, flags: u64) -> u64 {
    ((addr >> 12) << 10) as u64 | flags
}

bitflags! {
    #[derive(Clone, Copy)]
    pub struct PTEFlags: u64 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

#[unsafe(link_section = ".data.boot_page_table")]
static mut BOOT_PT: [u64; 512] = [0; 512];

unsafe extern "C" fn init_boot_page_table() {
    let boot_pt = unsafe { addr_of_mut!(BOOT_PT).as_mut().unwrap() };
    let flags = PTEFlags::A | PTEFlags::D | PTEFlags::R | PTEFlags::V | PTEFlags::W | PTEFlags::X;

    for i in 0..0x100 {
        let target_addr = i * 0x4000_0000;
        // 0x00000000_00000000 -> 0x00000000_00000000 (256G, 1G PerPage)
        boot_pt[i] = create_pte(target_addr, flags.bits());
        // 0xffffffc0_00000000 -> 0x00000000_00000000 (256G, 1G PerPage)
        boot_pt[i + 0x100] = create_pte(target_addr, (flags | PTEFlags::G).bits());
    }
}

unsafe extern "C" fn init_mmu() {
    let ptr = (&raw mut BOOT_PT) as *mut _ as usize;
    unsafe { satp::set(satp::Mode::Sv39, 0, ptr >> 12) };
    riscv::asm::sfence_vma_all();
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        "   mv      s0, a0
            mv      s1, a1
            la      sp, bstack_top
            li      t0, {virt_addr_start}
            not     t0, t0
            and     sp, sp, t0

            call    {init_boot_page_table}
            call    {init_mmu}

            li      t0, {virt_addr_start}   // add virtual address
            or      sp, sp, t0

            la      a2, {entry}
            or      a2, a2, t0
            mv      a0, s0
            mv      a1, s1
            jalr    a2                      // call rust_main
        ",
        init_boot_page_table = sym init_boot_page_table,
        init_mmu = sym init_mmu,
        entry = sym rust_entry,
        virt_addr_start = const VIRT_ADDR_START,
    )
}

unsafe extern "C" {
    fn kernel_main(hartid: usize, dtb: usize);
}

global_asm!(
    "
        .section .bss.bstack
        .global bstack
        .global bstack_top
        bstack:
        .fill 0x80000
        .size bstack, . - bstack
        bstack_top:
    "
);

pub fn rust_entry(hartid: usize, dtb: usize) {
    if dtb != 0xbfe00000 {
        println!("dtb : {:x}", dtb);
        loop {}
    }

    unsafe {
        kernel_main(hartid, dtb);
    }
}
