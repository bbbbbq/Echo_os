use super::id_alloc::TaskId;
use crate::alloc::string::ToString;
use crate::executor::executor::get_cur_usr_task;
use crate::executor::executor::{GLOBLE_EXECUTOR, release_task};
use crate::executor::id_alloc::alloc_tid;
use crate::executor::task::{AsyncTask, AsyncTaskItem};
use crate::user_handler::entry::user_entry;
use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use core::mem::size_of;
use core::time::Duration;
use elf_ext::{LoadElfReturn, load_elf_frame};
use filesystem::fd_table::FdTable;
use filesystem::file::File;
use filesystem::path::Path;
use heap::HeapUser;
use log::info;
use mem::memset::MemSet;
use mem::pagetable::PageTable;
use memory_addr::{VirtAddr, VirtAddrRange};
use spin::{Mutex, MutexGuard, RwLock};
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
    pub cx: TrapFrame,
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

    pub fn init_cx(load_elf_return: LoadElfReturn) -> TrapFrame {
        let base = load_elf_return.base;
        let entry_point = load_elf_return.entry_point;
        let sp = load_elf_return.stack_top;
        let mut cx = TrapFrame::new();
        cx.set_sepc(base + entry_point);
        cx.set_sp(sp);
        cx
    }

    // elf or other file
    pub async fn new_frome_file(parent: Option<Weak<UserTask>>, path: Path) -> Option<Arc<Self>> {
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
                    VirtAddr::from_usize(load_elf_return.heap_bottom + load_elf_return.heap_size),
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

    fn push_array(&self, tcb: &mut ThreadControlBlock, array: &[usize]) -> usize {
        let byte_len = array.len() * size_of::<usize>();
        let sp = tcb.cx[TrapFrameArgs::SP] - byte_len;
        let ptr = VirtAddr::from(sp).as_mut_ptr();
        unsafe {
            let slice = core::slice::from_raw_parts_mut(ptr, byte_len);
            let src_bytes = core::slice::from_raw_parts(array.as_ptr() as *const u8, byte_len);
            slice.copy_from_slice(src_bytes);
        }
        tcb.cx[TrapFrameArgs::SP] = sp;
        sp
    }

    fn push_str(&self, tcb: &mut ThreadControlBlock, s: &str) -> usize {
        const ULEN: usize = size_of::<usize>();
        let sp = tcb.cx[TrapFrameArgs::SP] - (s.len() + 1); // +1 for null terminator
        let aligned_sp = sp & !(ULEN - 1);
        let ptr = VirtAddr::from(aligned_sp).as_mut_ptr();
        unsafe {
            let slice = core::slice::from_raw_parts_mut(ptr, s.len() + 1);
            slice[..s.len()].copy_from_slice(s.as_bytes());
            slice[s.len()] = 0;
        }
        tcb.cx[TrapFrameArgs::SP] = aligned_sp;
        aligned_sp
    }

    pub fn init_task_stack(&self, args: Vec<String>, envp: Vec<String>) {
        let mut tcb = self.tcb.write();

        // Push environment variables and get pointers
        let envp_ptrs: Vec<usize> = envp
            .iter()
            .map(|env| self.push_str(&mut tcb, env))
            .collect();

        // Push arguments and get pointers
        let argv_ptrs: Vec<usize> = args
            .iter()
            .map(|arg| self.push_str(&mut tcb, arg))
            .collect();

        // Align stack before pushing pointers
        let sp = tcb.cx[TrapFrameArgs::SP];
        let aligned_sp = sp & !(size_of::<usize>() - 1);
        tcb.cx[TrapFrameArgs::SP] = aligned_sp;

        // Push auxv (null terminator)
        self.push_array(&mut tcb, &[0, 0]);

        // Push envp pointers (with null terminator)
        self.push_array(&mut tcb, &[0]);
        self.push_array(&mut tcb, &envp_ptrs);

        // Push argv pointers (with null terminator)
        self.push_array(&mut tcb, &[0]);
        self.push_array(&mut tcb, &argv_ptrs);

        // Push argc
        self.push_array(&mut tcb, &[args.len()]);

        // Set a0 to argc and a1 to argv pointer
        tcb.cx.x[10] = args.len();
        tcb.cx.x[11] = tcb.cx[TrapFrameArgs::SP];
    }

    pub fn force_cx_ref(&self) -> &'static mut TrapFrame {
        unsafe { &mut self.tcb.as_mut_ptr().as_mut().unwrap().cx }
    }

    pub fn get_fd(&self, fd: usize) -> Option<File> {
        self.pcb.lock().fd_table.get(fd).cloned()
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
            cx: parent.cx.clone(),
            thread_exit_code: None,
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
            cx: parent_tcb.cx.clone(),
            thread_exit_code: None,
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

pub async fn add_user_task(filename: &str, args: Vec<&str>, envp: Vec<&str>) -> TaskId {
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
    let task = UserTask::new_frome_file(None, Path::new(filename.to_owned()))
        .await
        .expect("Failed to create task from file");

    info!(
        "Initializing task stack with {} args and {} env vars",
        args.len(),
        envp.len()
    );
    task.init_task_stack(args, envp);
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
    task_id
}

