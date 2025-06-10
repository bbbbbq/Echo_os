use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use elf_ext;
use filesystem::path::Path;
use lazy_static::lazy_static;
use mem::memset::MemSet;
use spin::Mutex;

lazy_static! {
    pub static ref TASK_CACHE_MAP: Mutex<alloc::collections::BTreeMap<Path, Arc<TaskCache>>> =
        Mutex::new(alloc::collections::BTreeMap::new());
}

pub struct TaskCache {
    pub elf_name: Path,
    pub entry: usize,
    pub base: usize,
    pub heap_bottom: usize,
    pub ph_addr: usize,
    pub ph_size: usize,
    pub mem_set: MemSet,
}

impl TaskCache {
    pub fn new(elf_name: Path) -> Self {
        let elf_info = elf_ext::load_elf_frame(elf_name.clone());
        Self {
            elf_name,
            entry: elf_info.entry_point,
            base: 0,
            heap_bottom: 0,
            ph_addr: elf_info.ph_addr,
            ph_size: elf_info.ph_size,
            mem_set: MemSet::new(),
        }
    }
}
