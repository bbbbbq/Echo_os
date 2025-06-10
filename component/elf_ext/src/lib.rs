#![no_std]

extern crate alloc;

use alloc::string::ToString;
use config::target::plat::PAGE_SIZE;
use filesystem::{file::File, path::Path, vfs::OpenFlags};
use frame::alloc_continues;
use log::debug;
use mem::memregion::MemRegionType;
use mem::{memregion::MemRegion, memset::MemSet};
use memory_addr::{MemoryAddr, PhysAddr, VirtAddr};
use page_table_multiarch::MappingFlags;
use xmas_elf::ElfFile;
use core::ops::Mul;


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
    pub memset: MemSet,
    pub stack_top: usize,
    pub stack_size: usize,
    pub heap_start: usize,
    pub heap_size: usize,
}

impl core::fmt::Debug for LoadElfReturn {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LoadElfReturn")
            .field("frame_addr", &format_args!("0x{:x}", self.frame_addr))
            .field("file_size", &self.file_size)
            .field("ph_addr", &format_args!("0x{:x}", self.ph_addr))
            .field("ph_size", &self.ph_size)
            .field("entry_point", &format_args!("0x{:x}", self.entry_point))
            .field("memset", &self.memset)
            .field("stack_top", &format_args!("0x{:x}", self.stack_top))
            .field("stack_size", &self.stack_size)
            .field("heap_start", &format_args!("0x{:x}", self.heap_start))
            .field("heap_size", &self.heap_size)
            .finish()
    }
}

// 把elf文件存储到frame内存中，返回elf文件的地址，ph地址，entry_point，memset
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

    // 获取要映射的内存区域
    let mut memset = MemSet::new();
    let ph_count = elf.header.pt2.ph_count();
    for i in 0..ph_count {
        let ph = elf.program_header(i).unwrap();
        if ph.get_type() != Ok(xmas_elf::program::Type::Load) {
            continue;
        }

        let va = VirtAddr::from(ph.virtual_addr() as usize);
        let mem_size = ph.mem_size() as usize;
        let mut flags = MappingFlags::USER;
        if ph.flags().is_read() {
            flags |= MappingFlags::READ;
        }
        if ph.flags().is_write() {
            flags |= MappingFlags::WRITE;
        }
        if ph.flags().is_execute() {
            flags |= MappingFlags::EXECUTE;
        }

        let start_va = va.align_down(PAGE_SIZE);
        let end_va = VirtAddr::from(va.as_usize() + mem_size).align_up(PAGE_SIZE);

                let region_type = if ph.flags().is_execute() {
            MemRegionType::Text
        } else if ph.flags().is_write() {
            MemRegionType::DATA
        } else {
            MemRegionType::RODATA
        };
        let region = MemRegion::new_anonymous(start_va, end_va, flags, "elf_segment".to_string(), region_type);
        memset.push_region(region);
    }

    // 添加用户栈区域
    let stack_top = VirtAddr::from(config::target::plat::USER_STACK_TOP);
    let stack_bottom = VirtAddr::from(
        config::target::plat::USER_STACK_TOP - config::target::plat::USER_STACK_INIT_SIZE,
    );
        let stack_region = MemRegion::new_anonymous(
        stack_bottom,
        stack_top,
        MappingFlags::USER | MappingFlags::READ | MappingFlags::WRITE,
        "user_stack".to_string(),
        MemRegionType::STACK,
    );
    memset.push_region(stack_region);
    debug!("Added user stack: {:?} - {:?}", stack_bottom, stack_top);

    // 获取程序所有段之后的内存，4K 对齐后作为堆底,预先一页大小
    let heap_bottom = elf
        .program_iter()
        .map(|x| (x.virtual_addr() + x.mem_size()) as usize)
        .max()
        .unwrap()
        .div_ceil(PAGE_SIZE)
        .mul(PAGE_SIZE);

    let heap_start_addr: VirtAddr = heap_bottom.into();
    let heap_end_addr = heap_start_addr.add(PAGE_SIZE);
        let heap_region = MemRegion::new_anonymous(
        heap_start_addr,
        heap_end_addr,
        MappingFlags::USER | MappingFlags::READ | MappingFlags::WRITE,
        "user_heap".to_string(),
        MemRegionType::HEAP,
    );
    memset.push_region(heap_region);
    debug!("Added user heap: {:?} - {:?}", heap_start_addr, heap_end_addr);

    LoadElfReturn {
        frame_addr,
        file_size,
        ph_addr,
        ph_size,
        entry_point: elf.header.pt2.entry_point() as usize,
        memset,
        stack_top: stack_top.as_usize(),
        stack_size: config::target::plat::USER_STACK_INIT_SIZE,
        heap_start: heap_bottom,
        heap_size: PAGE_SIZE,
    }
}
