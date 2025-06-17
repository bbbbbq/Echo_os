use super::id_alloc::TaskId;
use crate::alloc::string::ToString;
use crate::executor::executor::get_cur_usr_task;
use crate::executor::executor::{GLOBLE_EXECUTOR, release_task};
use crate::executor::id_alloc::alloc_tid;
use crate::executor::task::{AsyncTask, AsyncTaskItem};
use crate::user_handler::entry::user_entry;
use alloc::borrow::ToOwned;
use alloc::collections::btree_map::BTreeMap;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use config::target::plat::{PAGE_SIZE, USER_STACK_TOP};
use core::mem::size_of;
use core::task;
use core::time::Duration;
use elf_ext::{LoadElfReturn, load_elf_frame};
use filesystem::fd_table::FdTable;
use filesystem::file::{self, File};
use filesystem::path::{self, Path};
use heap::HeapUser;
use log::{debug, error, info};
use mem::memset::MemSet;
use mem::pagetable::PageTable;
use mem::stack::StackRegion;
use memory_addr::{PhysAddr, PhysAddrRange, VirtAddr, VirtAddrRange, align_up, align_up_4k};
use spin::{Mutex, MutexGuard, RwLock};
use struct_define::aux::aux_type;
use trap::trapframe::TrapFrame;
use trap::trapframe::TrapFrameArgs;

#[derive(Debug, Clone)]
pub struct Shm {
    pub shm_id: usize,
    pub shm_addr: VirtAddr,
    pub shm_size: usize,
}

#[derive(Debug, Clone)]
pub struct ProcessControlBlock {
    pub fd_table: FdTable,
    pub mem_set: MemSet,
    pub curr_dir: Arc<Path>,
    pub heap: HeapUser,
    pub entry: usize,
    pub threads: Vec<Weak<UserTask>>,
    pub children: Vec<Arc<UserTask>>,
    pub shms: BTreeMap<usize, Arc<Shm>>,
    pub exit_code: Option<usize>,
    pub time: Option<Duration>,
}

#[derive(Clone)]
pub struct ThreadControlBlock {
    pub stack_region: StackRegion,
    pub cx: TrapFrame,
    pub clear_child_tid: Option<usize>,
    pub thread_exit_code: Option<usize>,
}

#[allow(dead_code)]
pub struct UserTask {
    pub task_id: TaskId,
    pub process_id: TaskId,
    pub page_table: Arc<Mutex<PageTable>>,
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
        let page_table = Arc::new(Mutex::new(PageTable::new()));
        let pcb = Arc::new(Mutex::new(ProcessControlBlock {
            fd_table: FdTable::new(),
            mem_set: MemSet::new(),
            curr_dir: Arc::new(work_dir),
            heap: HeapUser::new(VirtAddrRange::new(
                VirtAddr::from_usize(0),
                VirtAddr::from_usize(0),
            )),
            entry: 0,
            threads: vec![],
            children: vec![],
            shms: BTreeMap::new(),
            exit_code: None,
            time: None,
        }));
        let parent = RwLock::new(parent);
        let tcb = RwLock::new(ThreadControlBlock {
            stack_region: StackRegion::new_zero(),
            cx: TrapFrame::new(),
            thread_exit_code: None,
            clear_child_tid: None,
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

    pub fn get_sp(&self) -> usize {
        self.tcb.read().cx[TrapFrameArgs::SP]
    }

    pub fn init_cx(load_elf_return: LoadElfReturn) -> TrapFrame {
        let base = load_elf_return.base;
        let entry_point = load_elf_return.entry_point;
        let sp = load_elf_return.stack_region.get_top();
        let mut cx = TrapFrame::new();
        cx.set_sepc(base + entry_point);
        cx.set_sp(sp);
        cx
    }

    pub fn push_arr(&self, buffer: &[u8]) -> usize {
        self.tcb.write().stack_region.push_bytes(buffer)
    }

    pub fn exit_group(&self, exit_code: usize) {
        self.tcb.write().thread_exit_code = Some(exit_code);
        self.pcb.lock().exit_code = Some(exit_code);
    }

    pub fn new_from_file(
        parent: Option<Weak<UserTask>>,
        path: Path,
    ) -> Option<(Arc<Self>, LoadElfReturn)> {
        let curr_dir = if let Some(parent_weak) = &parent {
            if let Some(parent_arc) = parent_weak.upgrade() {
                parent_arc.pcb.lock().curr_dir.clone()
            } else {
                Arc::new(Path::new("/".to_owned()))
            }
        } else {
            Arc::new(Path::new("/".to_owned()))
        };

        let mut load_elf_return: LoadElfReturn = load_elf_frame(path.clone());
        if load_elf_return.entry_point == 0 {
            // Not a valid ELF file
            return None;
        }
        let cx = UserTask::init_cx(load_elf_return.clone());
        info!("load_elf_return: {:?}", load_elf_return);
        let mut pagetable = PageTable::new();
        let _ = pagetable.restore();
        for region in load_elf_return.memset.regions.iter_mut() {
            info!("region: {:?}", region);
            let _ = pagetable.map_region_user(region);
            region.is_mapped = true;
        }

        load_elf_return.stack_region.map(&mut pagetable);

        // Clone the return value before the original is partially moved.
        let return_elf_data = load_elf_return.clone();
        // Initialize task based on load_elf_return information
        let task = Arc::new(Self {
            task_id: alloc_tid(),
            process_id: alloc_tid(),
            page_table: Arc::new(Mutex::new(pagetable)),
            pcb: Arc::new(Mutex::new(ProcessControlBlock {
                fd_table: FdTable::new(),
                mem_set: load_elf_return.memset,
                curr_dir,
                heap: HeapUser::new(VirtAddrRange::new(
                    VirtAddr::from_usize(load_elf_return.heap_bottom),
                    VirtAddr::from_usize(load_elf_return.heap_bottom),
                )),
                entry: load_elf_return.entry_point,
                threads: vec![],
                children: vec![],
                shms: BTreeMap::new(),
                exit_code: None,
                time: None,
            })),
            parent: RwLock::new(parent.unwrap_or_else(|| Weak::new())),
            tcb: RwLock::new(ThreadControlBlock {
                stack_region: load_elf_return.stack_region,
                cx: cx,
                thread_exit_code: None,
                clear_child_tid: None,
            }),
        });

        let sbss_start = load_elf_return.sbss_start;
        let sbss_size = load_elf_return.sbss_size;
        let bss_start = load_elf_return.bss_start;
        let bss_size = load_elf_return.bss_size;

        let pagetable = task.page_table.lock().clone();

        for cur_vaddr in (sbss_start..sbss_start + sbss_size).step_by(4096) {
            let paddr = pagetable.translate(VirtAddr::from_usize(cur_vaddr)).expect("translate failed");
            let paddr_usize = paddr.as_usize();
            unsafe {
                core::ptr::write_bytes(paddr_usize as *mut u8, 0, PAGE_SIZE);
            }
        }

        for cur_vaddr in (bss_start..bss_start + bss_size).step_by(4096) {
            let paddr = pagetable.translate(VirtAddr::from_usize(cur_vaddr)).expect("translate failed");
            let paddr_usize = paddr.as_usize();
            unsafe {
                core::ptr::write_bytes(paddr_usize as *mut u8, 0, PAGE_SIZE);
            }
        }

        Some((task, return_elf_data))
    }

    pub fn push_num(&self, num: usize) -> usize {
        self.tcb.write().stack_region.push_num(num);
        let sp = self.tcb.read().stack_region.get_sp();
        self.tcb.write().cx.set_sp(sp);
        sp
    }

    fn push_array(&self, array: &[u8]) -> usize {
        self.tcb.write().stack_region.push_bytes(array);
        let sp = self.tcb.read().stack_region.get_sp();
        self.tcb.write().cx.set_sp(sp);
        sp
    }

    pub fn push_str(&self, str: &str) -> usize {
        self.push_arr(str.as_bytes());
        let sp = self.tcb.read().stack_region.get_sp();
        self.tcb.write().cx.set_sp(sp);
        sp
    }

    pub fn init_task_stack(
        user_task: &Arc<UserTask>,
        args: Vec<String>,
        base: usize,
        path: &str,
        entry_point: usize,
        ph_count: usize,
        ph_entry_size: usize,
        ph_addr: usize,
        heap_bottom: usize,
    ) {
        log::info!(
            "[init_task_stack] args: {:?}, base: {:#x}, path: {}, entry_point: {:#x}, ph_count: {}, ph_entry_size: {}, ph_addr: {:#x}, heap_bottom: {:#x}",
            args,
            base,
            path,
            entry_point,
            ph_count,
            ph_entry_size,
            ph_addr,
            heap_bottom
        );

        user_task.tcb.write().cx[TrapFrameArgs::SP] = USER_STACK_TOP;
        user_task.tcb.write().cx[TrapFrameArgs::SEPC] = base + entry_point;
        let envp = vec![
            "LD_LIBRARY_PATH=/",
            "PS1=\x1b[1m\x1b[32mByteOS\x1b[0m:\x1b[1m\x1b[34m\\w\x1b[0m\\$ \0",
            "PATH=/:/bin:/usr/bin",
            "UB_BINDIR=./",
        ];
        let envp: Vec<usize> = envp
            .into_iter()
            .rev()
            .map(|x| {
                let ptr = user_task.push_str(x);
                log::error!("ENV: {:#x} -> {} (SP: {:#x})", ptr, x, user_task.get_sp());
                ptr
            })
            .collect();
        let args: Vec<usize> = args
            .into_iter()
            .rev()
            .map(|x| {
                let ptr = user_task.push_str(&x);
                log::error!("ARG: {:#x} -> {} (SP: {:#x})", ptr, x, user_task.get_sp());
                ptr
            })
            .collect();
        let random_ptr = user_task.push_arr(&[0u8; 16]);
        log::error!("Random bytes at {:#x}", random_ptr);
        log::error!("Building auxiliary vector");
        let mut auxv = BTreeMap::new();
        auxv.insert(aux_type::AT_PLATFORM, user_task.push_str("riscv"));
        log::error!("SP after AT_PLATFORM: {:#x}", user_task.get_sp());
        auxv.insert(aux_type::AT_EXECFN, user_task.push_str(path));
        log::error!("SP after AT_EXECFN: {:#x}", user_task.get_sp());
        auxv.insert(aux_type::AT_PHNUM, ph_count);
        auxv.insert(aux_type::AT_PAGESZ, PAGE_SIZE);
        auxv.insert(aux_type::AT_ENTRY, base + entry_point);
        auxv.insert(aux_type::AT_PHENT, ph_entry_size);
        auxv.insert(aux_type::AT_PHDR, base + ph_addr);
        auxv.insert(aux_type::AT_GID, 0);
        auxv.insert(aux_type::AT_EGID, 0);
        auxv.insert(aux_type::AT_UID, 0);
        auxv.insert(aux_type::AT_EUID, 0);
        auxv.insert(aux_type::AT_SECURE, 0);
        auxv.insert(aux_type::AT_RANDOM, random_ptr);

        user_task.push_num(0);
        log::error!("SP after null terminator: {:#x}", user_task.get_sp());
        auxv.iter().for_each(|(key, v)| {
            log::error!("AUXV: {:#x} -> {:#x}", key, v);
            user_task.push_num(*v);
            log::error!("SP after value: {:#x}", user_task.get_sp());
            user_task.push_num(*key);
            log::error!("SP after key: {:#x}", user_task.get_sp());
        });
        user_task.push_num(0);
        log::error!("SP after env null terminator: {:#x}", user_task.get_sp());

        envp.iter().for_each(|x| {
            log::error!("ENV ptr: {:#x}", x);
            user_task.push_num(*x);
            log::error!("SP after env ptr: {:#x}", user_task.get_sp());
        });
        user_task.push_num(0);
        log::error!("SP after args null terminator: {:#x}", user_task.get_sp());
        args.iter().for_each(|x| {
            log::error!("ARG ptr: {:#x}", x);
            user_task.push_num(*x);
            log::error!("SP after arg ptr: {:#x}", user_task.get_sp());
        });

        let argc = args.len();
        log::error!("Argc: {}", argc);
        user_task.push_num(argc);
        log::error!("Final SP after argc: {:#x}", user_task.get_sp());

    // Print memory contents from stack top to USER_STACK_TOP - 0x1D1
    let dump_end = USER_STACK_TOP - 0x1D2;
    let mut vaddr = USER_STACK_TOP; // Align to 4 bytes

    log::info!("Dumping stack from vaddr {:#x} down to {:#x}", USER_STACK_TOP, dump_end);
    while vaddr >= dump_end {
        if let Some(paddr) = user_task.page_table.lock().translate(vaddr.into()) {
            let value = unsafe { *(paddr.as_usize() as *const u32) };
            log::info!("vaddr: {:#x} paddr: {:#x} => {:#010x}", vaddr, paddr, value);
        } else {
            log::warn!("Failed to translate vaddr {:#x}", vaddr);
        }
        if vaddr < 4 {
            break;
        }
        vaddr -= 4;
    }
    }

    pub fn force_cx_ref(&self) -> &'static mut TrapFrame {
        unsafe { &mut self.tcb.as_mut_ptr().as_mut().unwrap().cx }
    }

    pub fn get_fd(&self, fd: usize) -> Option<File> {
        self.pcb.lock().fd_table.get(fd).cloned()
    }

    pub fn get_cwd(&self) -> File {
        let path = self.pcb.lock().curr_dir.clone();
        File::open(&path.to_string(), filesystem::file::OpenFlags::O_DIRECTORY).unwrap()
    }

    pub fn get_heap(&self) -> HeapUser {
        self.pcb.lock().heap.clone()
    }

    pub fn set_heap(&self, heap: HeapUser) {
        self.pcb.lock().heap = heap;
    }

    pub fn release(&self) {
        // Ensure that the task was exited successfully.
        assert!(self.exit_code().is_some() || self.tcb.read().thread_exit_code.is_some());
        release_task(self.task_id);
    }

    pub fn thread_exit(&self, exit_code: usize) {
        self.tcb.write().thread_exit_code = Some(exit_code);
        if self.task_id != self.process_id {
            self.pcb
                .lock()
                .threads
                .retain(|x| x.upgrade().map_or(false, |x| x.task_id != self.task_id));
            self.release();
        }
    }

    pub fn thread_clone(&self) -> Arc<Self> {
        let parent = self.tcb.read();

        let task_id = alloc_tid();
        let cur_tcb = RwLock::new(ThreadControlBlock {
            stack_region: self.tcb.read().stack_region.clone(),
            cx: parent.cx.clone(),
            thread_exit_code: None,
            clear_child_tid: None,
        });
        cur_tcb.write().cx[TrapFrameArgs::RET] = 0;

        let new_task = Arc::new(Self {
            page_table: self.page_table.clone(),
            task_id,
            process_id: self.task_id,
            parent: RwLock::new(self.parent.read().clone()),
            pcb: self.pcb.clone(),
            tcb: cur_tcb,
        });
        self.pcb.lock().threads.push(Arc::downgrade(&new_task));
        new_task
    }

    pub fn process_clone(self: Arc<Self>) -> Arc<Self> {
        let parent_tcb = self.tcb.read();
        let mut parent_pcb = self.pcb.lock();

        let task_id = alloc_tid();

        // Clone the PCB. This requires ProcessControlBlock to be Clone.
        let mut new_pcb = (*parent_pcb).clone();

        // The new process has its own thread list, containing only itself.
        // And it has no children yet.
        new_pcb.threads = vec![];
        new_pcb.children = vec![];

        let new_tcb = RwLock::new(ThreadControlBlock {
            stack_region: self.tcb.read().stack_region.clone(),
            cx: parent_tcb.cx.clone(),
            thread_exit_code: None,
            clear_child_tid: None,
        });
        new_tcb.write().cx[TrapFrameArgs::RET] = 0; // Return 0 for child process

        let new_task = Arc::new(Self {
            // Each process has its own page table.
            // A real implementation would create a new page table and copy mappings (CoW).
            // For now, we create a new PageTable by cloning the inner part of the parent's.
            page_table: Arc::new(Mutex::new(self.page_table.lock().clone())),
            task_id,
            process_id: task_id, // For a new process, process_id is same as task_id
            parent: RwLock::new(Arc::downgrade(&self)), // The parent is the current task
            pcb: Arc::new(Mutex::new(new_pcb)),
            tcb: new_tcb,
        });

        // Add the new task to the parent's children list.
        parent_pcb.children.push(new_task.clone());
        // Add the main thread to its own thread list.
        new_task.pcb.lock().threads.push(Arc::downgrade(&new_task));

        new_task
    }

    // pub fn fork(&self) -> Arc<Self> {
    //     let new_task = self.clone();
    //     let new_task_arc = Arc::new(new_task);
    //     self.pcb.lock().children.push(new_task_arc.clone());
    // }

    pub fn inner_map<T>(&self, mut f: impl FnMut(&mut MutexGuard<ProcessControlBlock>) -> T) -> T {
        f(&mut self.pcb.lock())
    }

    // pub fn execve(&self, filename: &str, args: Vec<&str>, envp: Vec<&str>)
    // {
    //     add_user_task(filename, args, envp)
    // }
}

impl AsyncTask for UserTask {
    fn before_run(&self) {
        self.page_table.lock().change_pagetable();
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
        self.tcb.read().thread_exit_code
    }
}

pub fn add_user_task(filename: &str, args: Vec<&str>, envp: Vec<&str>) -> TaskId {
    info!("Adding user task: {}", filename);
    let parent = get_cur_usr_task();
    if let Some(p) = &parent {
        info!("Parent task ID: {:?}", p.get_task_id());
    } else {
        info!("No parent user task found.");
    }

    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let envp: Vec<String> = envp.iter().map(|s| s.to_string()).collect();

    info!("Creating task from file: {}", filename);
    let (task, load_elf_return) = UserTask::new_from_file(None, Path::new(filename.to_owned()))
        .expect("Failed to create task from file");

    info!(
        "Initializing task stack with {} args and {} env vars",
        args.len(),
        envp.len()
    );
    let mut filename = filename.to_owned();
    if !filename.starts_with('/') {
        let mut path = "/".to_owned();
        path.push_str(&filename);
        filename = path;
    }
    UserTask::init_task_stack(
        &task,
        args,
        load_elf_return.base,
        filename.as_str(),
        load_elf_return.entry_point,
        load_elf_return.ph_count,
        load_elf_return.ph_entry_size,
        load_elf_return.ph_addr,
        load_elf_return.heap_bottom,
    );
    if let Some(p) = &parent {
        p.before_run();
    }
    let task_id = task.get_task_id();
    info!("New task created with ID: {:?}", task_id);

    let future = user_entry();

    let task_tmp = AsyncTaskItem { task, future };
    info!("Spawning task into executor");
    GLOBLE_EXECUTOR.spawn(task_tmp);

    info!("User task {:?} added successfully", task_id);

    // test_addr_load
    task_id
}
