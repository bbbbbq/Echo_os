#![no_std]

use config::target::plat::PAGE_SIZE;
use filesystem::{file::File, path::Path, vfs::OpenFlags};
use frame::alloc_continues;
use log::debug;
use xmas_elf::ElfFile;

pub trait ElfExt {
    fn relocated(&self) -> usize;
}

impl ElfExt for ElfFile<'_> {
    fn relocated(&self) -> usize {
        todo!()
    }
}

pub struct LoadElfReturn {
    pub frame_addr: usize,
    pub file_size: usize,
    pub ph_addr: usize,
    pub ph_size: usize,
    pub entry_point: usize,
}

// 把elf文件存储到frame内存中
pub fn load_elf_frame(path: Path) -> LoadElfReturn {
    debug!("Loading ELF file from path: {:?}", path);
    let file = File::open(path, OpenFlags::O_RDONLY).expect("Failed to open ELF file");
    let file_size = file.get_file_size().expect("Failed to get file size");
    debug!("ELF file size: {} bytes", file_size);

    let frame_addr = alloc_continues(file_size.div_ceil(PAGE_SIZE))[0]
        .paddr
        .as_usize();
    debug!("Allocated frame at address: 0x{:x}", frame_addr);

    let buffer = unsafe { core::slice::from_raw_parts_mut(frame_addr as *mut u8, file_size) };
    let read_size = file.read_at(buffer).expect("Failed to read ELF file");
    assert_eq!(read_size, file_size);

    debug!(
        "Successfully loaded ELF file, size: {}, address: 0x{:x}",
        file_size, frame_addr
    );
    let elf = ElfFile::new(buffer).expect("Failed to parse ELF file");
    let ph_addr = frame_addr + elf.header.pt2.ph_offset() as usize;
    let ph_size = elf.header.pt2.ph_entry_size() as usize * elf.header.pt2.ph_count() as usize;
    let entry_point = elf.header.pt2.entry_point() as usize;
    LoadElfReturn {
        frame_addr,
        file_size,
        ph_addr,
        ph_size,
        entry_point,
    }
}
