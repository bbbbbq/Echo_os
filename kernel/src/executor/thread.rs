use super::id_alloc::TaskId;
use crate::alloc::string::ToString;
use crate::executor::executor::Executor;
use crate::user_handler::entry::user_entry;
use super::executor::{add_ready_task, get_cur_usr_task};
use crate::executor::id_alloc::alloc_tid;
use alloc::sync;
use alloc::sync::Arc;
use alloc::sync::Weak;
use alloc::vec::Vec;
use config::target::plat::PAGE_SIZE;
use config::target::plat::USER_DYN_ADDR;
use elf_ext::ElfExt;
use elf_ext::LoadElfReturn;
use filesystem::file;
use filesystem::file::File;
use filesystem::path::Path;
use filesystem::{fd_table::FdTable, vfs::{OpenFlags, VfsError}};
use frame::{alloc_continues, alloc_frame};
use heap::HeapUser;
use log::debug;
use log::info;
use mem::memregion::MemRegion;
use mem::memregion::MemRegionType;
use mem::memset::MemSet;
use mem::pagetable;
use mem::pagetable::get_boot_page_table;
use mem::pagetable::PageTable;
use memory_addr::MemoryAddr;
use memory_addr::{PhysAddr, PhysAddrRange, VirtAddr, VirtAddrRange};
use page_table_multiarch::MappingFlags;
use riscv::register::sstatus;
use spin::Mutex;
use spin::rwlock::RwLock;
use trap::trap::TrapFrame;
use elf_ext::load_elf_frame;
use alloc::string::String;
use crate::executor::task::{AsyncTask, AsyncTaskItem};
use crate::executor::executor::GLOBLE_EXECUTOR;
use trap::trapframe::TrapFrameArgs;
use core::mem::size_of;
use super::error::TaskError;
use alloc::borrow::ToOwned;
use alloc::boxed::Box;



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


    pub fn init_cx(load_elf_return:LoadElfReturn) -> TrapFrame
    {
        let base = load_elf_return.base;
        let entry_point = load_elf_return.entry_point;
        let sp = load_elf_return.stack_top;
        let mut cx = TrapFrame::new();
        cx.set_sepc(base+entry_point);
        cx.set_sp(sp);
        cx
    }

    // elf or other file
    pub async fn new_frome_file(parent: Option<Weak<UserTask>>, path: Path) -> Option<Arc<Self>> {
        let mut load_elf_return = load_elf_frame(path.clone());
        if load_elf_return.entry_point == 0 {
            // Not a valid ELF file
            return None;
        }
        let cx = UserTask::init_cx(load_elf_return.clone());
        info!("load_elf_return: {:?}", load_elf_return);
        let mut pagetable = PageTable::new();
        
        for region in load_elf_return.memset.regions.iter_mut() {
            info!("region: {:?}", region);
            pagetable.map_region_user(region);
        }
        
        // Initialize task based on load_elf_return information
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
                cx: cx,
                thread_exit_code: None,
            }),
        });
        
        Some(task)
    }


    

    pub fn push_num(&self, num: usize) -> usize {
        let mut tcb = self.tcb.write();

        const ULEN: usize = size_of::<usize>();
        let sp = tcb.cx[TrapFrameArgs::SP] - ULEN;

        unsafe {
            *VirtAddr::from(sp).as_mut_ptr_of::<usize>() = num;
        }
        tcb.cx[TrapFrameArgs::SP] = sp;
        sp
    }

    pub fn push_bytes(&self, bytes: &[u8]) -> usize {
        let mut tcb = self.tcb.write();

        let sp = tcb.cx[TrapFrameArgs::SP] - bytes.len();
        let ptr = VirtAddr::from(sp).as_mut_ptr();
        unsafe {
            let slice = core::slice::from_raw_parts_mut(ptr, bytes.len());
            slice.copy_from_slice(bytes);
        }
        tcb.cx[TrapFrameArgs::SP] = sp;
        sp
    }

    pub fn init_task_stack(&self, args: Vec<String>, envp: Vec<String>) {
        let mut tcb = self.tcb.write();

        // Push environment variables
        let mut envp_ptrs: Vec<usize> = Vec::new();
        for env in envp.iter().rev() {
            self.push_num(0); // Null terminator
            let sp = self.push_bytes(env.as_bytes());
            envp_ptrs.push(sp);
        }

        // Push arguments
        let mut argv_ptrs: Vec<usize> = Vec::new();
        for arg in args.iter().rev() {
            self.push_num(0); // Null terminator
            let sp = self.push_bytes(arg.as_bytes());
            argv_ptrs.push(sp);
        }

        // Push auxv
        self.push_num(0); // AT_NULL
        self.push_num(0);

        // Push envp pointers
        self.push_num(0); // Null terminator
        for ptr in envp_ptrs.iter() {
            self.push_num(*ptr);
        }

        // Push argv pointers
        self.push_num(0); // Null terminator
        for ptr in argv_ptrs.iter() {
            self.push_num(*ptr);
        }

        // Push argc
        self.push_num(args.len());

        // Set a0 to argc and a1 to argv, though this is often done by the entry code
        tcb.cx.x[10] = args.len();
        tcb.cx.x[11] = tcb.cx[TrapFrameArgs::SP];
    }


    pub fn force_cx_ref(&self) -> &'static mut TrapFrame {
        unsafe { &mut self.tcb.as_mut_ptr().as_mut().unwrap().cx }
    }
}

impl AsyncTask for UserTask {
    fn before_run(&self) {
        self.page_table.change_pagetable();
    }
    fn get_task_id(&self) -> TaskId {
        self.task_id
    }
    
    fn get_task_type(&self) -> super::task::TaskType {
        super::task::TaskType::User
    }
    
    fn exit(&self, _exit_code: usize) {
        unimplemented!()
    }
    
    fn exit_code(&self) -> Option<usize> {
        unimplemented!()
    }
}

pub async fn add_user_task(filename: &str, args: Vec<&str>, envp: Vec<&str>) -> TaskId
{
    let parent = get_cur_usr_task();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let envp: Vec<String> = envp.iter().map(|s| s.to_string()).collect();

    let task = UserTask::new_frome_file(Some(Arc::downgrade(&parent)), Path::new(filename.to_owned()))
        .await.expect("Failed to create task from file");

    task.init_task_stack(args, envp);
    parent.before_run();
    let task_id = task.get_task_id();
    let future = user_entry();
    let task_tmp = AsyncTaskItem { task, future };
    GLOBLE_EXECUTOR.lock().spawn(task_tmp);
    task_id
}