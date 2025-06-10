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
    pub mem_set: MemSet,
}

impl TaskCache {
    pub fn new(elf_name: Path) -> Self {
        let elf_info = elf_ext::load_elf_frame(elf_name.clone());
        Self {
            elf_name,
            entry: elf_info.entry_point,
            mem_set: elf_info.memset,
        }
    }
}
