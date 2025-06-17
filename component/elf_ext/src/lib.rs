#![no_std]

extern crate alloc;

use alloc::format;
use alloc::string::ToString;
use config::riscv64_qemu::plat::USER_DYN_ADDR;
use config::target::plat::{PAGE_SIZE, USER_STACK_INIT_SIZE, USER_STACK_TOP};
use core::ops::Mul;
use filesystem::{
    file::{File, OpenFlags},
    path::Path,
};
use frame::alloc_continues;
use log::{debug, error};
use log::info;
use mem::{memregion::MemRegion, memset::MemSet};
use mem::{memregion::MemRegionType, stack::StackRegion};
use memory_addr::{MemoryAddr, PhysAddr, PhysAddrRange, VirtAddr, VirtAddrRange};
use page_table_multiarch::MappingFlags;
use xmas_elf::sections::SectionData;
use xmas_elf::symbol_table::DynEntry64;
use xmas_elf::symbol_table::Entry;
use xmas_elf::{ElfFile, header};

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
        let section = self
            .find_section_by_name(".rela.dyn")
            .ok_or(".rela.dyn not found")?;

        let data = section.get_data(self).map_err(|_| "corrupted .rela.dyn")?;

        let entries = match data {
            SectionData::Rela64(entries) => entries,
            _ => return Err("bad .rela.dyn"),
        };

        info!(
            "Relocating ELF with {} entries at base 0x{:x}",
            entries.len(),
            base
        );

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
    pub stack_region: StackRegion,
    pub heap_bottom: usize,
    pub base: usize,
    pub sbss_start:usize,
    pub sbss_size:usize,
    pub bss_start:usize,
    pub bss_size:usize,
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
            .field("heap_start", &format_args!("0x{:x}", self.heap_bottom))
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
    let read_size = file.read_at(0, buffer).expect("Failed to read ELF file");
    assert_eq!(read_size, file_size);
    debug!(
        "Successfully loaded ELF file, size: {}, address: 0x{:x}",
        file_size, frame_addr
    );
    let elf = ElfFile::new(buffer).expect("Failed to parse ELF file");
    let ph_entry_size = elf.header.pt2.ph_entry_size() as usize;

    // 获取要映射的内存区域
    let mut memset = MemSet::new();
    let mut elf_region_start_vaddr = 0xffffffffffffffffusize;
    let ph_count = elf.header.pt2.ph_count();
    for i in 0..ph_count {
        let ph = elf.program_header(i).unwrap();
        if ph.get_type() != Ok(xmas_elf::program::Type::Load) {
            continue;
        }

        let va = VirtAddr::from(ph.virtual_addr() as usize);
        if va.as_usize() < elf_region_start_vaddr {
            elf_region_start_vaddr = va.as_usize();
        }
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

        let paddr_start =
            PhysAddr::from_usize((frame_addr as u64 + ph.offset()) as usize).align_down(PAGE_SIZE);
        let paddr_end =
            PhysAddr::from(paddr_start.as_usize() + (end_va.as_usize() - start_va.as_usize()));

        let region = MemRegion::new_mapped(
            start_va,
            end_va,
            paddr_start,
            paddr_end,
            flags | MappingFlags::USER,
            format!("elf_segment_{}", i).to_string(),
            region_type,
        );
        memset.push_region(region);
    }

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

    // let heap_region = MemRegion::new_mapped(
    //     heap_start_addr,
    //     heap_end_addr,
    //     heap_paddr_start,
    //     heap_paddr_end,
    //     MappingFlags::USER | MappingFlags::READ | MappingFlags::WRITE,
    //     "user_heap".to_string(),
    //     MemRegionType::HEAP,
    // );
    // memset.push_region(heap_region);
    // debug!(
    //     "Added user heap: {:?} - {:?}",
    //     heap_start_addr, heap_end_addr
    // );

    let map_base = memset.get_base();
    let base = if elf.header.pt2.type_().as_type() == header::Type::Executable {
        0
    } else {
        map_base
    };

    info!("ELF info: map_base=0x{:x}, base=0x{:x}", map_base, base);
    let vaddr_end = VirtAddr::from(USER_STACK_TOP);
    let vaddr_start = VirtAddr::from_usize(vaddr_end.as_usize() - USER_STACK_INIT_SIZE);
    let frame_traces = alloc_continues(USER_STACK_INIT_SIZE.div_ceil(PAGE_SIZE));
    let paddr_start = frame_traces[0].paddr;
    let paddr_end = PhysAddr::from(paddr_start.as_usize() + USER_STACK_INIT_SIZE);
    let stack_region = StackRegion::new(
        PhysAddrRange::new(paddr_start, paddr_end),
        VirtAddrRange::new(vaddr_start, vaddr_end),
    );

    let mut bss_start = 0;
    let mut bss_end = 0;
    let mut sbss_start = 0;
    let mut sbss_end = 0;
    info!("elf_region_start_vaddr: 0x{:x}", elf_region_start_vaddr);
    for section in elf.section_iter() {
        if let Ok(name) = section.get_name(&elf) {
            if name == ".bss" {
                bss_start = section.address() as usize;
                bss_end = bss_start + section.size() as usize;
                debug!("Found .bss section: 0x{:x} - 0x{:x}", bss_start, bss_end);
            } else if name == ".sbss" {
                sbss_start = section.address() as usize;
                sbss_end = sbss_start + section.size() as usize;
                debug!("Found .sbss section: 0x{:x} - 0x{:x}", sbss_start, sbss_end);
            }
        }
    }
    let sbss_size = sbss_end - sbss_start;
    let bss_size = bss_end - bss_start;

    // // Clear .bss and .sbss sections by setting memory to zero
    // if bss_end > bss_start {
    //     let bss_start_paddr = frame_addr + bss_start - elf_region_start_vaddr;
    //     let bss_size = bss_end - bss_start;
    //     unsafe {
    //         core::ptr::write_bytes(bss_start_paddr as *mut u8, 0, bss_size);
    //     }
    //     debug!("Zeroed .bss section: 0x{:x} - 0x{:x}", bss_start_paddr, bss_start_paddr + bss_size);
    // }
    
    // if sbss_end > sbss_start {
    //     let sbss_start_paddr = frame_addr + sbss_start - elf_region_start_vaddr;
    //     let sbss_size = sbss_end - sbss_start;
    //     unsafe {
    //         core::ptr::write_bytes(sbss_start_paddr as *mut u8, 0, sbss_size);
    //     }
    //     debug!("Zeroed .sbss section: 0x{:x} - 0x{:x}", sbss_start_paddr, sbss_start_paddr + sbss_size);
    // }
    


    LoadElfReturn {
        frame_addr,
        file_size,
        ph_addr: map_base + elf.header.pt2.ph_offset() as usize,
        ph_count: ph_count.into(),
        ph_entry_size,
        entry_point: elf.header.pt2.entry_point() as usize,
        memset,
        stack_region,
        heap_bottom: heap_start_addr.as_usize(),
        base,
        sbss_start,
        sbss_size,
        bss_start,
        bss_size,
    }
}
