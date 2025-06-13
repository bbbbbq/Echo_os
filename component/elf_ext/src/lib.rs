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
use xmas_elf::sections::SectionData;
use xmas_elf::symbol_table::DynEntry64;
use xmas_elf::symbol_table::Entry;
use config::riscv64_qemu::plat::USER_DYN_ADDR;
use log::info;


pub trait ElfExt {
    fn relocate(&self, base: usize) -> Result<usize, &str>;
    fn dynsym(&self) -> Result<&[DynEntry64], &'static str>;
}

impl ElfExt for ElfFile<'_> {
    fn dynsym(&self) -> Result<&[DynEntry64], &'static str> {
        match self
            .find_section_by_name(".dynsym")
            .ok_or(".dynsym not found")?
            .get_data(self)
            .map_err(|_| "corrupted .dynsym")?
        {
            SectionData::DynSymbolTable64(dsym) => Ok(dsym),
            _ => Err("bad .dynsym"),
        }
    }

    fn relocate(&self, base: usize) -> Result<usize, &str> {
        let section = self.find_section_by_name(".rela.dyn")
            .ok_or(".rela.dyn not found")?;
        
        let data = section.get_data(self)
            .map_err(|_| "corrupted .rela.dyn")?;
        
        let entries = match data {
            SectionData::Rela64(entries) => entries,
            _ => return Err("bad .rela.dyn"),
        };
        
        info!("Relocating ELF with {} entries at base 0x{:x}", entries.len(), base);
        
        let dynsym = self.dynsym()?;
        for entry in entries.iter() {
            const REL_GOT: u32 = 6;
            const REL_PLT: u32 = 7;
            const REL_RELATIVE: u32 = 8;
            const R_RISCV_64: u32 = 2;
            const R_RISCV_RELATIVE: u32 = 3;
            const R_AARCH64_RELATIVE: u32 = 0x403;
            const R_AARCH64_GLOBAL_DATA: u32 = 0x401;

            let entry_type = entry.get_type();
            info!("Processing relocation entry type: {}", entry_type);
            
            match entry_type {
                REL_GOT | REL_PLT | R_RISCV_64 | R_AARCH64_GLOBAL_DATA => {
                    let sym_idx = entry.get_symbol_table_index() as usize;
                    let dynsym_entry = &dynsym[sym_idx];
                    
                    if dynsym_entry.shndx() == 0 {
                        let name = dynsym_entry.get_name(self)?;
                        info!("Symbol needs resolution: {}", name);
                        panic!("need to find symbol: {:?}", name);
                    } else {
                        let resolved_addr = base + dynsym_entry.value() as usize;
                        info!("Symbol resolved to address: 0x{:x}", resolved_addr);
                    };
                }
                REL_RELATIVE | R_RISCV_RELATIVE | R_AARCH64_RELATIVE => {
                    info!("Processing relative relocation");
                }
                t => {
                    info!("Unknown relocation type: {}", t);
                    unimplemented!("unknown type: {}", t);
                }
            }
        }
        
        info!("Relocation completed successfully");
        Ok(base)
    }
}

#[derive(Clone)]
pub struct LoadElfReturn {
    pub frame_addr: usize,
    pub file_size: usize,
    pub ph_count: usize,
    pub ph_addr: usize,
    pub ph_entry_size: usize,
    pub entry_point: usize,
    pub memset: MemSet,
    pub stack_top: usize,
    pub stack_size: usize,
    pub heap_bottom: usize,
    pub heap_size: usize,
    pub base:usize,
}

impl core::fmt::Debug for LoadElfReturn {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LoadElfReturn")
            .field("frame_addr", &format_args!("0x{:x}", self.frame_addr))
            .field("file_size", &self.file_size)
            .field("ph_addr", &format_args!("0x{:x}", self.ph_addr))
            .field("ph_count", &self.ph_count)
            .field("ph_entry_size", &self.ph_entry_size)
            .field("entry_point", &format_args!("0x{:x}", self.entry_point))
            .field("memset", &self.memset)
            .field("stack_top", &format_args!("0x{:x}", self.stack_top))
            .field("stack_size", &self.stack_size)
            .field("heap_start", &format_args!("0x{:x}", self.heap_bottom))
            .field("heap_size", &self.heap_size)
            .finish()
    }
}

// 把elf文件存储到frame内存中，返回elf文件的地址，ph地址，entry_point，memset
pub fn load_elf_frame(path: Path) -> LoadElfReturn {
    debug!("Loading ELF file from path: {:?}", path);
    let file = File::open(&path.to_string(), OpenFlags::O_RDONLY).expect("Failed to open ELF file");
    let file_size = file.get_file_size().expect("Failed to get file size");
    debug!("ELF file size: {} bytes", file_size);

    let frame_addr = alloc_continues(file_size.div_ceil(PAGE_SIZE))[0]
        .paddr
        .as_usize();
    debug!("Allocated frame at address: 0x{:x}", frame_addr);

    let buffer = unsafe { core::slice::from_raw_parts_mut(frame_addr as *mut u8, file_size) };
    let read_size = file.read_at(0,buffer).expect("Failed to read ELF file");
    assert_eq!(read_size, file_size);

    debug!(
        "Successfully loaded ELF file, size: {}, address: 0x{:x}",
        file_size, frame_addr
    );
    let elf = ElfFile::new(buffer).expect("Failed to parse ELF file");
    let ph_addr = frame_addr + elf.header.pt2.ph_offset() as usize;
    let _ph_count = elf.header.pt2.ph_count() as usize;
    let ph_entry_size = elf.header.pt2.ph_entry_size() as usize;
    let _entry_point = elf.header.pt2.entry_point() as usize;

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

        let paddr_start = PhysAddr::from_usize((frame_addr as u64 + ph.offset()) as usize)
            .align_down(PAGE_SIZE);
        let paddr_end = PhysAddr::from(
            paddr_start.as_usize() + (end_va.as_usize() - start_va.as_usize()),
        );

        let region = MemRegion::new_mapped(
            start_va,
            end_va,
            paddr_start,
            paddr_end,
            flags,
            "elf_segment".to_string(),
            region_type,
        );
        memset.push_region(region);
    }

    // 添加用户栈区域
    let stack_top = VirtAddr::from(config::target::plat::USER_STACK_TOP);
    let stack_bottom = VirtAddr::from(
        config::target::plat::USER_STACK_TOP - config::target::plat::USER_STACK_INIT_SIZE,
    );
    let stack_size = config::target::plat::USER_STACK_INIT_SIZE;
    let stack_pages = stack_size.div_ceil(PAGE_SIZE);
    let stack_paddr_start = alloc_continues(stack_pages)[0].paddr;
    let stack_paddr_end = PhysAddr::from(stack_paddr_start.as_usize() + stack_size);

    let stack_region = MemRegion::new_mapped(
        stack_bottom,
        stack_top,
        stack_paddr_start,
        stack_paddr_end,
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
    let heap_size = PAGE_SIZE; // Default heap size
    let heap_pages = heap_size.div_ceil(PAGE_SIZE);
    let heap_paddr_start = alloc_continues(heap_pages)[0].paddr;
    let heap_paddr_end = PhysAddr::from(heap_paddr_start.as_usize() + heap_size);

    let heap_region = MemRegion::new_mapped(
        heap_start_addr,
        heap_end_addr,
        heap_paddr_start,
        heap_paddr_end,
        MappingFlags::USER | MappingFlags::READ | MappingFlags::WRITE,
        "user_heap".to_string(),
        MemRegionType::HEAP,
    );
    memset.push_region(heap_region);
    debug!("Added user heap: {:?} - {:?}", heap_start_addr, heap_end_addr);

    let base = elf.relocate(USER_DYN_ADDR).unwrap_or(0);

    info!("debug base: {}", base);

    LoadElfReturn {
        frame_addr,
        file_size,
        ph_addr,
        ph_count: ph_count.into(),
        ph_entry_size,
        entry_point: elf.header.pt2.entry_point() as usize,
        memset,
        stack_top: stack_top.as_usize(),
        stack_size: config::target::plat::USER_STACK_INIT_SIZE,
        heap_bottom: heap_start_addr.as_usize(),
        heap_size: config::target::plat::HEAP_SIZE,
        base,
    }
}
