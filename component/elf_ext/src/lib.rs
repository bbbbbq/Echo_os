#![no_std]

extern crate alloc;

use alloc::format;
use alloc::string::ToString;
use config::riscv64_qemu::plat::USER_DYN_ADDR;
use config::target::plat::PAGE_SIZE;
use core::ops::Mul;
use filesystem::{
    file::{File, OpenFlags},
    path::Path,
};
use log::{debug, warn};
use log::info;
use mem::{memregion::MemRegion, memset::MemSet};
use memory_addr::{MemoryAddr, PhysAddr, VirtAddr};
use page_table_multiarch::MappingFlags;
use xmas_elf::sections::SectionData;
use xmas_elf::symbol_table::DynEntry64;
use xmas_elf::symbol_table::Entry;
use xmas_elf::{ElfFile, program::Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Errno {
    InvalidElf,
    SectionNotFound,
    CorruptedSection,
    BadSectionFormat,
    SymbolResolutionFailed,
    RelocationFailed,
}

impl core::fmt::Display for Errno {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Errno::InvalidElf => write!(f, "Invalid ELF file"),
            Errno::SectionNotFound => write!(f, "Section not found"),
            Errno::CorruptedSection => write!(f, "Corrupted section"),
            Errno::BadSectionFormat => write!(f, "Bad section format"),
            Errno::SymbolResolutionFailed => write!(f, "Symbol resolution failed"),
            Errno::RelocationFailed => write!(f, "Relocation failed"),
        }
    }
}



pub trait ElfExt {
    fn relocate(&self, base: usize) -> Result<usize, &str>;
    fn dynsym(&self) -> Result<&[DynEntry64], &'static str>;
    fn get_ph_addr(&self) -> Result<u64, Errno>;
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

    // 获取elf加载需要的内存大小
    fn get_ph_addr(&self) -> Result<u64, Errno> {
        if let Some(phdr) = self
            .program_iter()
            .find(|ph| ph.get_type() == Ok(Type::Phdr))
        {
            // if phdr exists in program header, use it
            Ok(phdr.virtual_addr())
        } else if let Some(elf_addr) = self
            .program_iter()
            .find(|ph| ph.get_type() == Ok(Type::Load) && ph.offset() == 0)
        {
            // otherwise, check if elf is loaded from the beginning, then phdr can be inferred.
            Ok(elf_addr.virtual_addr() + self.header.pt2.ph_offset())
        } else {
            warn!("elf: no phdr found, tls might not work");
            Err(Errno::SectionNotFound)
        }
    }
}
