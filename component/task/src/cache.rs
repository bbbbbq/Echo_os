
use elf_ext;
use filesystem::path::Path;
use lazy_static::lazy_static;
use spin::Mutex;
use elf_ext::LoadElfReturn;

lazy_static! {
    pub static ref TASK_CACHE_MAP: Mutex<alloc::collections::BTreeMap<Path, LoadElfReturn>> =
        Mutex::new(alloc::collections::BTreeMap::new());
}

pub fn load_elf_cache(path: Path) -> LoadElfReturn {
    let mut map = TASK_CACHE_MAP.lock();
    if map.contains_key(&path) {
        return map.get(&path).unwrap().clone();
    }
    let elf_info = elf_ext::load_elf_frame(path.clone());
    map.insert(path, elf_info.clone());
    elf_info
}

pub fn find_task_cache(path: Path) -> Option<LoadElfReturn> {
    let map = TASK_CACHE_MAP.lock();
    map.get(&path).cloned()
}


pub fn remove_task_cache(path: Path) {
    let mut map = TASK_CACHE_MAP.lock();
    map.remove(&path);
}