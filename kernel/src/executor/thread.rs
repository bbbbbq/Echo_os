use super::id_alloc::TaskId;
use crate::alloc::string::ToString;
use crate::executor::id_alloc::alloc_tid;
use alloc::sync::Arc;
use alloc::sync::Weak;
use alloc::vec::Vec;
use config::target::plat::PAGE_SIZE;
use config::target::plat::USER_DYN_ADDR;
use elf_ext::ElfExt;
use filesystem::file::File;
use filesystem::path::Path;
use filesystem::{fd_table::FdTable, vfs::OpenFlags};
use frame::{alloc_continues, alloc_frame};
use heap::HeapUser;
use log::debug;
use log::info;
use mem::memregion::MemRegion;
use mem::memregion::MemRegionType;
use mem::memset::MemSet;
use mem::pagetable;
use mem::pagetable::PageTable;
use memory_addr::MemoryAddr;
use memory_addr::{PhysAddr, PhysAddrRange, VirtAddr, VirtAddrRange};
use page_table_multiarch::MappingFlags;
use spin::Mutex;
use spin::rwlock::RwLock;
use trap::trap::TrapFrame;
use elf_ext::load_elf_frame;

pub struct ProcessControlBlock {
    pub fd_table: FdTable,
    pub mem_set: MemSet,
    pub curr_dir: Arc<Path>,
    pub heap: HeapUser,
    pub entry: usize,
    pub threads: Vec<UserTask>,
    pub exit_code: Option<usize>,
}

pub struct ThreadControlBlock {
    pub cx: TrapFrame,
    pub thread_exit_code: Option<usize>,
}

#[allow(dead_code)]
pub struct UserTask {
    pub task_id: TaskId,
    pub process_id: TaskId,
    pub page_table: Arc<PageTable>,
    pub pcb: Arc<Mutex<ProcessControlBlock>>,
    pub parent: RwLock<Weak<UserTask>>,
    pub tcb: RwLock<ThreadControlBlock>,
}

impl core::fmt::Debug for UserTask {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let pcb = self.pcb.lock();
        let tcb = self.tcb.read();
        f.debug_struct("UserTask")
            .field("task_id", &self.task_id)
            .field("process_id", &self.process_id)
            .field("entry", &pcb.entry)
            .field("exit_code", &pcb.exit_code)
            .field("thread_exit_code", &tcb.thread_exit_code)
            .field("current_dir", &pcb.curr_dir)
            .field("thread_count", &pcb.threads.len())
            .field("fd_table", &pcb.fd_table)
            .field("memory_set", &pcb.mem_set)
            .field("heap", &pcb.heap)
            .finish()
    }
}

impl UserTask {
    pub fn new(parent: Weak<UserTask>, work_dir: Path) -> Arc<Self> {
        let task_id = TaskId(0);
        let process_id = TaskId(0);
        let page_table = Arc::new(PageTable::new());
        let pcb = Arc::new(Mutex::new(ProcessControlBlock {
            fd_table: FdTable::new(),
            mem_set: MemSet::new(),
            curr_dir: Arc::new(work_dir),
            heap: HeapUser::new(VirtAddrRange::new(
                VirtAddr::from_usize(0),
                VirtAddr::from_usize(0),
            )),
            entry: 0,
            threads: Vec::new(),
            exit_code: None,
        }));
        let parent = RwLock::new(parent);
        let tcb = RwLock::new(ThreadControlBlock {
            cx: TrapFrame::new(),
            thread_exit_code: None,
        });
        Arc::new(UserTask {
            task_id,
            process_id,
            page_table,
            pcb,
            parent,
            tcb,
        })
    }

    // elf or other file
    pub fn new_frome_file(parent: Option<Weak<UserTask>>, path: Path) -> Arc<Self> {
        let mut load_elf_return = load_elf_frame(path.clone());
        info!("load_elf_return: {:?}", load_elf_return);
        let mut pagetable = PageTable::new();
        // pagetable.restore(); // This might not be needed here or could be an old pattern.
        for region in load_elf_return.memset.regions.iter_mut() {
            info!("region: {:?}", region);
            pagetable.map_region_user(region);
        }
        // 根据load_elf_return的信息来初始化
        let task = Arc::new(Self {
            task_id: alloc_tid(),
            process_id: alloc_tid(),
            page_table: Arc::new(pagetable),
            pcb: Arc::new(Mutex::new(ProcessControlBlock {
                fd_table: FdTable::new(),
                mem_set: load_elf_return.memset,
                curr_dir: Arc::new(path.clone()),
                heap: HeapUser::new(VirtAddrRange::new(
                    VirtAddr::from_usize(load_elf_return.heap_bottom),
                    VirtAddr::from_usize(load_elf_return.heap_bottom + load_elf_return.heap_size),
                )),
                entry: load_elf_return.entry_point,
                threads: Vec::new(),
                exit_code: None,
            })),
            parent: RwLock::new(parent.unwrap_or_else(|| Weak::new())),
            tcb: RwLock::new(ThreadControlBlock {
                cx: TrapFrame::new(),
                thread_exit_code: None,
            }),
        });
        
        task
    }
}
