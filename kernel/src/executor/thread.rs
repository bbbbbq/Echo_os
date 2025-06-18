use super::id_alloc::TaskId;
use crate::alloc::string::ToString;
use crate::executor::error::{self, TaskError};
use crate::executor::executor::{get_cur_usr_task, release_task, spawn};
use crate::executor::id_alloc::alloc_tid;
use crate::executor::task::{AsyncTask, AsyncTaskItem};
use crate::executor::{self, thread};
use crate::user_handler::entry::user_entry;
use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use async_recursion::async_recursion;
use config::target::plat::{PAGE_SIZE, STACK_SIZE, USER_DYN_ADDR, USER_STACK_INIT_SIZE, USER_STACK_TOP};
use core::mem::size_of;
use core::ops::Mul;
use core::ptr::slice_from_raw_parts_mut;
use core::time::Duration;
use elf_ext::ElfExt;
use filesystem::fd_table::FdTable;
use filesystem::file::{File, OpenFlags};
use filesystem::path::Path;
use frame::{FRAME_ALLOCATOR, alloc_continues, alloc_frame, dealloc_continues};
use heap::HeapUser;
use log::{debug, info};
use mem::memregion::{MemRegion, MemRegionType};
use mem::memset::MemSet;
use mem::pagetable::PageTable;
use mem::{PhysAddrExt, VirtAddrExt};
use memory_addr::{MemoryAddr, PhysAddr, VirtAddr, VirtAddrRange, align_up};
use page_table_multiarch::MappingFlags;
use spin::{Mutex, MutexGuard, RwLock};
use struct_define::elf::elf;
use trap::trapframe::TrapFrame;
use trap::trapframe::TrapFrameArgs;
use xmas_elf::ElfFile;
use xmas_elf::program::{SegmentData, Type};

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
    pub heap: usize,
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
            heap: 0,
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
            clear_child_tid: None,
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

    pub fn push_str(&self, str: &str) -> usize {
        self.push_arr(str.as_bytes())
    }

    pub fn push_arr(&self, buffer: &[u8]) -> usize {
        let mut tcb = self.tcb.write();
        const ULEN: usize = size_of::<usize>();
        let len = buffer.len();
        let sp = tcb.cx[TrapFrameArgs::SP] - align_up(len + 1, ULEN);
        info!("push_arr sp: {:#x}", sp);
        VirtAddr::from(sp)
            .slice_mut_as_len(len)
            .copy_from_slice(buffer);
        tcb.cx[TrapFrameArgs::SP] = sp;
        sp
    }

    pub fn push_num(&self, num: usize) -> usize {
        let mut tcb = self.tcb.write();

        const ULEN: usize = size_of::<usize>();
        let sp = tcb.cx[TrapFrameArgs::SP] - ULEN;

        *VirtAddr::from(sp).get_mut() = num;
        tcb.cx[TrapFrameArgs::SP] = sp;
        sp
    }

    pub fn get_sp(&self) -> usize {
        self.tcb.read().cx[TrapFrameArgs::SP]
    }

    pub fn alloc_map_frame(&self, vaddr: VirtAddr, flags: MappingFlags) {
        self.page_table.lock().map(vaddr, flags);
    }
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
    // let curr_task = get_cur_usr_task();
    let task = UserTask::new(Weak::new(), "/".into());
    task.page_table.lock().restore().unwrap();
    task.before_run();
    info!(
        "task page_table: {:?}",
        task.page_table.lock().page_table.root_paddr()
    );
    exec_with_process(
        task.clone(),
        Path::new_empty(),
        String::from(filename),
        args.into_iter().map(String::from).collect(),
        envp.into_iter().map(String::from).collect(),
    )
    .await
    .expect("can't add task to excutor");
    spawn(task.clone(), user_entry());
    task.get_task_id()
}

pub async fn exec_with_process(
    task: Arc<UserTask>,
    curr_dir: Path,
    path: String,
    args: Vec<String>,
    envp: Vec<String>,
) -> Result<Arc<UserTask>, TaskError> {
    info!("exec_with_process: {}", path);
    let path = curr_dir.join(&path);

    let mut file = File::open(&path.to_string(), OpenFlags::O_RDONLY).unwrap();
    let file_size = file.get_file_size()?;
    let pages = file_size.div_ceil(PAGE_SIZE);
    let frames = alloc_continues(pages);
    let buffer = frames[0].paddr.slice_mut_as_len(file_size);
    let risze = file.read(buffer)?;
    assert!(risze == file_size);

    let elf = ElfFile::new(&buffer).unwrap();
    let elf_header = elf.header;

    let entry_point = elf_header.pt2.entry_point() as usize;
    info!("entry_point: {:#x}", entry_point);
    // this assert ensures that the file is elf file.
    assert_eq!(
        elf_header.pt1.magic,
        [0x7f, 0x45, 0x4c, 0x46],
        "invalid elf!"
    );

    // WARRNING: this convert async task to user task.
    let user_task = task.clone();

    let header = elf
        .program_iter()
        .find(|ph| ph.get_type() == Ok(Type::Interp));

    // if let Some(header) = header {
    //     if let Ok(SegmentData::Undefined(_data)) = header.get_data(&elf) {
    //         dealloc_continues(frames[0], pages);
    //         let mut new_args = vec![String::from("libc.so")];
    //         new_args.extend(args);
    //         return Box::pin(exec_with_process(
    //             task,
    //             curr_dir,
    //             new_args[0].clone(),
    //             new_args,
    //             envp,
    //         ))
    //         .await;
    //     }
    // }

    // 获取程序所有段之后的内存，4K 对齐后作为堆底
    let heap_bottom = elf
        .program_iter()
        .map(|x| (x.virtual_addr() + x.mem_size()) as usize)
        .max()
        .unwrap()
        .div_ceil(PAGE_SIZE)
        .mul(PAGE_SIZE);

    let base = elf.relocate(USER_DYN_ADDR).unwrap_or(0);

    init_task_stack(
        user_task.clone(),
        args,
        base,
        &path.to_string(),
        entry_point,
        elf_header.pt2.ph_count() as usize,
        elf_header.pt2.ph_entry_size() as usize,
        elf.get_ph_addr().unwrap_or(0) as usize,
        heap_bottom,
    );

    elf.program_iter()
        .filter(|x| x.get_type().unwrap() == xmas_elf::program::Type::Load)
        .for_each(|ph| {
            let mem_size = ph.mem_size() as usize;
            let virt_addr = VirtAddr::from(base + ph.virtual_addr() as usize);
            let virt_addr_end = virt_addr + mem_size;
            let aligned_start = virt_addr.align_down(PAGE_SIZE);
            let aligned_end = virt_addr_end.align_up(PAGE_SIZE);

            let flages = ph.flags();
            let mut mapflages = MappingFlags::USER | MappingFlags::READ | MappingFlags::empty();
            if flages.is_read() {
                mapflages = mapflages | MappingFlags::READ;
            }
            if flages.is_write() {
                mapflages = mapflages | MappingFlags::WRITE;
            }
            if flages.is_execute() {
                mapflages = mapflages | MappingFlags::EXECUTE;
            }

            debug!(
                "task map {:?} -> {:?}, flags: {:?}",
                virt_addr..virt_addr_end,
                aligned_start..aligned_end,
                mapflages
            );

            // This direct mapping approach assumes that the physical memory backing the ELF file
            // is large enough for the entire memory size of the segment, including BSS sections.
            // This might not hold true if mem_size > file_size, which could lead to issues.
            let virt_offset = virt_addr.as_usize() - aligned_start.as_usize();
            let paddr_start =
                PhysAddr::from(frames[0].paddr.as_usize() + ph.offset() as usize - virt_offset);
            let paddr_end = paddr_start + (aligned_end.as_usize() - aligned_start.as_usize());
            let mut mem_region = MemRegion::new_mapped(
                aligned_start,
                aligned_end,
                paddr_start,
                paddr_end,
                mapflages,
                "elf_segment".to_owned(),
                MemRegionType::ELF,
            );
            let _ = user_task.page_table.lock().map_region_user(&mut mem_region);
        });
    dealloc_continues(frames[0], pages);
    Ok(user_task)
}

pub fn init_task_stack(
    user_task: Arc<UserTask>,
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

    let stack_start = USER_STACK_TOP - USER_STACK_INIT_SIZE;
    let stack_end = USER_STACK_TOP;
    let flags = MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER;
    for addr in (stack_start..stack_end).step_by(PAGE_SIZE) {
        user_task.alloc_map_frame(VirtAddr::from(addr), flags);
    }

    log::debug!(
        "[task {:?}] entry: {:#x}",
        user_task.get_task_id(),
        base + entry_point
    );
    user_task.inner_map(|inner| {
        inner.heap = heap_bottom;
        inner.entry = base + entry_point;
    });

    let mut tcb = user_task.tcb.write();

    tcb.cx = TrapFrame::new();
    tcb.cx[TrapFrameArgs::SP] = USER_STACK_TOP; // stack top;
    tcb.cx[TrapFrameArgs::SEPC] = base + entry_point;

    drop(tcb);

    // push stack
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
            ptr
        })
        .collect();

    let args: Vec<usize> = args
        .into_iter()
        .rev()
        .map(|x| {
            let ptr = user_task.push_str(&x);
            ptr
        })
        .collect();

    let random_ptr = user_task.push_arr(&[0u8; 16]);

    let mut auxv = BTreeMap::new();
    auxv.insert(elf::AT_PLATFORM, user_task.push_str("riscv"));
    auxv.insert(elf::AT_EXECFN, user_task.push_str(path));
    auxv.insert(elf::AT_PHNUM, ph_count);
    auxv.insert(elf::AT_PAGESZ, PAGE_SIZE);
    auxv.insert(elf::AT_ENTRY, base + entry_point);
    auxv.insert(elf::AT_PHENT, ph_entry_size);
    auxv.insert(elf::AT_PHDR, base + ph_addr);
    auxv.insert(elf::AT_GID, 0);
    auxv.insert(elf::AT_EGID, 0);
    auxv.insert(elf::AT_UID, 0);
    auxv.insert(elf::AT_EUID, 0);
    auxv.insert(elf::AT_SECURE, 0);
    auxv.insert(elf::AT_RANDOM, random_ptr);

    // auxv top
    user_task.push_num(0);

    // Push auxv
    auxv.iter().for_each(|(key, v)| {
        user_task.push_num(*v);
        user_task.push_num(*key);
    });

    user_task.push_num(0);

    envp.iter().for_each(|x| {
        user_task.push_num(*x);
    });

    user_task.push_num(0);

    args.iter().for_each(|x| {
        user_task.push_num(*x);
    });

    let argc = args.len();
    user_task.push_num(argc);

    let dump_end = USER_STACK_TOP - 0x1D1;
    let _vaddr = USER_STACK_TOP;

    log::info!(
        "Dumping stack from vaddr {:#x} down to {:#x}",
        USER_STACK_TOP,
        dump_end
    );
}
