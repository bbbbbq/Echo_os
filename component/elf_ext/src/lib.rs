#![no_std]

use xmas_elf::ElfFile;
use filesystem::{file::File, path::Path, vfs::OpenFlags};
use frame::alloc_continues;
use config::target::plat::PAGE_SIZE;

pub trait ElfExt {
    fn relocated(&self) -> usize;
}

impl ElfExt for ElfFile<'_> {
    fn relocated(&self) -> usize {
        todo!()
    }
}

fn load_elf_frame(path: Path) -> (usize,usize) {
    let file = File::open(path, OpenFlags::O_RDONLY).expect("Failed to open ELF file");
    let file_size = file.get_file_size().expect("123");
    let frame_addr = alloc_continues(file_size.div_ceil(PAGE_SIZE))[0].paddr.as_usize();
    let buffer = unsafe { core::slice::from_raw_parts_mut(frame_addr as *mut u8, file_size) };
    let addr = frame_addr;
    (addr,file_size)
}
