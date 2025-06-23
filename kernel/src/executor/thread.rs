use super::id_alloc::TaskId;
use crate::alloc::string::ToString;
use crate::executor::error::TaskError;
use crate::executor::executor::{release_task, spawn};
use crate::executor::id_alloc::alloc_tid;
use crate::executor::task::AsyncTask;
use crate::signal::SigProcMask;
use crate::signal::flages::{SigAction, SignalFlags};
use crate::signal::list::{REAL_TIME_SIGNAL_NUM, SignalList};
use crate::user_handler::entry::user_entry;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use arch::flush_tlb;
use config::target::plat::{PAGE_SIZE, USER_DYN_ADDR, USER_STACK_INIT_SIZE, USER_STACK_TOP};
use core::mem::size_of;
use core::ops::Mul;
use core::time::Duration;
use elf_ext::ElfExt;
use filesystem::fd_table::FdTable;
use filesystem::file::{File, OpenFlags};
use filesystem::path::Path;
use frame::alloc_continues;
use log::{debug, error, info};
use mem::memregion::MemRegion;
use mem::memset::MemSet;
use mem::pagetable::PageTable;
use mem::{PhysAddrExt, VirtAddrExt};
use memory_addr::{MemoryAddr, PhysAddr, PhysAddrRange, VirtAddr, VirtAddrRange, align_up};
use page_table_multiarch::MappingFlags;
use spin::{Mutex, MutexGuard, RwLock};
use struct_define::elf::elf;
use trap::trapframe::TrapFrame;
use trap::trapframe::TrapFrameArgs;
use xmas_elf::ElfFile;
use xmas_elf::program::Type;

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
    pub sigaction: [SigAction; 65],
    pub children: Vec<Arc<UserTask>>,
    pub shms: BTreeMap<usize, Arc<Shm>>,
    pub exit_code: Option<usize>,
    pub time: Option<Duration>,
}

#[derive(Clone)]
pub struct ThreadControlBlock {
    pub cx: TrapFrame,
    pub sigmask: SigProcMask,
    pub signal: SignalList,
    pub signal_queue: [usize; REAL_TIME_SIGNAL_NUM], // a queue for real time signals
    pub exit_signal: u8,
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
        let task_id = alloc_tid();
        let process_id = task_id;
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
            sigaction: [SigAction::new(); 65],
        }));
        let parent = RwLock::new(parent);
        let tcb = RwLock::new(ThreadControlBlock {
            cx: TrapFrame::new(),
            clear_child_tid: None,
            thread_exit_code: None,
            sigmask: SigProcMask::new(),
            signal: SignalList::new(),
            signal_queue: [0; REAL_TIME_SIGNAL_NUM],
            exit_signal: 0,
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
        self.pcb.lock().fd_table.get(fd)
    }

    pub fn get_cwd(&self) -> File {
        let path = self.pcb.lock().curr_dir.clone();
        File::open(&path.to_string(), filesystem::file::OpenFlags::O_DIRECTORY)
            .unwrap()
            .into()
    }

    pub fn release(&self) {
        // Ensure that the task was exited successfully.
        assert!(self.exit_code().is_some() || self.tcb.read().thread_exit_code.is_some());
        release_task(self.task_id);
    }

    pub fn exit_with_signal(&self, signal: usize) {
        self.exit(128 + signal);
    }

    pub fn thread_exit(&self, exit_code: usize) {
        // 如果是进程的主线程，执行完整的进程退出逻辑，保证向父进程发送 SIGCHLD
        if self.task_id == self.process_id {
            self.exit(exit_code);
            // 进程退出后即可释放资源
            self.release();
            return;
        }

        // 否则仅标记当前线程退出并从线程列表中移除
        self.tcb.write().thread_exit_code = Some(exit_code);
        self.pcb
            .lock()
            .threads
            .retain(|x| x.upgrade().map_or(false, |x| x.task_id != self.task_id));
        self.release();
    }

    pub fn thread_clone(&self) -> Arc<Self> {
        let parent = self.tcb.read();

        let task_id = alloc_tid();
        let cur_tcb = RwLock::new(ThreadControlBlock {
            cx: parent.cx.clone(),
            thread_exit_code: None,
            clear_child_tid: None,
            sigmask: SigProcMask::new(),
            signal: SignalList::new(),
            signal_queue: [0; REAL_TIME_SIGNAL_NUM],
            exit_signal: 0,
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

    pub fn sbrk(&self, addr: usize) -> usize {
        let curr_page = self.pcb.lock().heap.div_ceil(PAGE_SIZE);
        let after_page = addr.div_ceil(PAGE_SIZE);
        let frames = alloc_continues(after_page - curr_page);
        let start_paddr = PhysAddr::from(frames[0].paddr.as_usize());
        let size = (after_page - curr_page) * PAGE_SIZE;
        let end_paddr = start_paddr + size;
        let mut mem_region = MemRegion::new_mapped(
            VirtAddrRange::new(
                VirtAddr::from(curr_page * PAGE_SIZE),
                VirtAddr::from(after_page * PAGE_SIZE),
            ),
            PhysAddrRange::new(start_paddr, end_paddr),
            MappingFlags::USER | MappingFlags::WRITE | MappingFlags::READ,
        );
        self.pcb.lock().mem_set.push_region(mem_region.clone());
        let _ = self.page_table.lock().map_region_user(&mut mem_region);
        self.pcb.lock().heap = addr;
        addr
    }

    pub fn process_clone(self: Arc<Self>) -> Arc<Self> {
        info!("process_clone");
        // info!("task mem_set {:?}", self.pcb.lock().mem_set);

        let parent_task: Arc<UserTask> = self.clone();
        let work_dir = parent_task.clone().pcb.lock().curr_dir.clone();
        let new_task = Self::new(Arc::downgrade(&parent_task), work_dir.as_ref().clone());
        let _ = new_task.page_table.lock().restore();
        let mut new_tcb_writer = new_task.tcb.write();
        // clone fd_table and clone heap
        let mut new_pcb = new_task.pcb.lock();
        let mut pcb = self.pcb.lock();
        new_pcb.fd_table = pcb.fd_table.clone();
        new_pcb.heap = pcb.heap;
        new_tcb_writer.cx = self.tcb.read().cx.clone();
        new_tcb_writer.cx[TrapFrameArgs::RET] = 0;
        new_pcb.curr_dir = pcb.curr_dir.clone();
        pcb.children.push(new_task.clone());
        new_pcb.shms = pcb.shms.clone();
        // 显式结束对 new_task 可变借用的生命周期，避免后续移动冲突
        drop(new_tcb_writer);
        let parent_mem_set = pcb.mem_set.clone();
        let parent_fd_table_len = pcb.fd_table.table.len();
        let parent_heap = pcb.heap;
        let parent_curr_dir = pcb.curr_dir.clone();
        drop(pcb);
        for region in parent_mem_set.regions.iter() {
            let mut new_region = region.clone();
            let _ = new_task.page_table.lock().map_region_user(&mut new_region);
            new_task.page_table.lock().protect_region(
                &mut new_region,
                MappingFlags::USER | MappingFlags::READ | MappingFlags::EXECUTE,
            );
            new_pcb.mem_set.push_region(new_region.clone());
            parent_task.page_table.lock().protect_region(
                &mut new_region,
                MappingFlags::USER | MappingFlags::READ | MappingFlags::EXECUTE,
            );
        }

        assert_eq!(
            new_pcb.mem_set.regions.len(),
            parent_mem_set.regions.len(),
            "Parent and child mem_set region count mismatch"
        );
        for (i, (child_region, parent_region)) in new_pcb
            .mem_set
            .regions
            .iter()
            .zip(parent_mem_set.regions.iter())
            .enumerate()
        {
            assert_eq!(
                child_region.range, parent_region.range,
                "Region #{} vaddr_range mismatch",
                i
            );
            assert_eq!(
                child_region.map_flags, parent_region.map_flags,
                "Region #{} flags mismatch",
                i
            );
        }
        assert_eq!(new_pcb.heap, parent_heap, "Heap address mismatch");
        assert_eq!(
            new_pcb.fd_table.table.len(),
            parent_fd_table_len,
            "fd_table length mismatch"
        );
        assert_eq!(
            new_pcb.curr_dir.to_string(),
            parent_curr_dir.to_string(),
            "Current directory mismatch"
        );
        //比较sp是否一致
        let parent_sp = parent_task.tcb.read().cx[TrapFrameArgs::SP];
        let child_sp = new_task.tcb.read().cx[TrapFrameArgs::SP];
        assert_eq!(parent_sp, child_sp, "Stack pointer mismatch");

        //比较两个pagetable的虚拟地址和物理地址的映射是否一致
        //打印每一个mem_map_trace的引用计数
        drop(new_pcb);
        drop(parent_task);
        new_task
    }

    // pub fn fork(&self) -> Arc<Self> {
    //     let new_task = self.clone();
    //     let new_task_arc = Arc::new(new_task);
    //     self.pcb.lock().children.push(new_task_arc.clone());
    // }

    pub fn query_va(&self, va: VirtAddr) -> Option<(PhysAddr, MappingFlags)> {
        self.page_table.lock().translate(va)
    }

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

    pub fn get_heap(&self) -> usize {
        self.pcb.lock().heap
    }

    pub fn get_last_free_addr(&self, size: usize) -> VirtAddr {
        static mut MMAP_BASE: usize = 0x20000000;
        unsafe {
            MMAP_BASE = MMAP_BASE + size;
        }
        unsafe { VirtAddr::from_usize(MMAP_BASE) }
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

    fn exit(&self, exit_code: usize) {
        // Clear child TID address if specified
        let exit_signal;
        {
            let mut tcb = self.tcb.write();
            if let Some(uaddr) = tcb.clear_child_tid {
                if uaddr != 0 {
                    debug!("write addr: {:#x}", uaddr);
                    let phys_addr = self
                        .page_table
                        .lock()
                        .translate(VirtAddr::from(uaddr))
                        .expect("can't find a valid addr")
                        .0;
                    unsafe {
                        *phys_addr.get_mut() = 0usize;
                    }
                }
            }
            exit_signal = tcb.exit_signal;
        }

        // Set thread exit code
        self.pcb.lock().exit_code = Some(exit_code);

        // recycle memory resources if the pcb just used by this thread
        if Arc::strong_count(&self.pcb) == 1 {
            let mut pcb = self.pcb.lock();
            pcb.mem_set.clear();
            pcb.fd_table.table.clear();
            pcb.children.clear();
        }

        if let Some(parent) = self.parent.read().upgrade() {
            if exit_signal != 0 {
                parent
                    .tcb
                    .write()
                    .signal
                    .add_signal(SignalFlags::from_num(exit_signal as usize));
            } else {
                parent.tcb.write().signal.add_signal(SignalFlags::SIGCHLD);
            }
        } else {
            self.pcb.lock().children.clear();
        }
    }

    fn exit_code(&self) -> Option<usize> {
        // Prefer process-level exit_code if set; otherwise fall back to thread_exit_code
        let pcb_code = self.pcb.lock().exit_code;
        pcb_code.or(self.tcb.read().thread_exit_code)
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
    _envp: Vec<String>,
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

    let _header = elf
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

            let virt_offset = virt_addr.as_usize() - aligned_start.as_usize();
            let paddr_start =
                PhysAddr::from(frames[0].paddr.as_usize() + ph.offset() as usize - virt_offset);
            let paddr_end = paddr_start + (aligned_end.as_usize() - aligned_start.as_usize());
            let mut mem_region = MemRegion::new_mapped(
                VirtAddrRange::new(aligned_start, aligned_end),
                PhysAddrRange::new(paddr_start, paddr_end),
                mapflages,
            );
            user_task.pcb.lock().mem_set.push_region(mem_region.clone());
            let _ = user_task.page_table.lock().map_region_user(&mut mem_region);
        });

    let mut sbss_range = None;
    let mut bss_range = None;
    let mut sdata_range = None;
    let mut data_range = None;

    for sh in elf.section_iter() {
        if let Ok(name) = sh.get_name(&elf) {
            let range = (sh.address(), sh.address() + sh.size());
            match name {
                ".sbss" => sbss_range = Some(range),
                ".bss" => bss_range = Some(range),
                ".sdata" => sdata_range = Some(range),
                ".data" => data_range = Some(range),
                _ => {}
            }
        }
    }

    if let Some((sbss_start, sbss_end)) = sbss_range {
        info!("sbss_start: {:#x}, sbss_end: {:#x}", sbss_start, sbss_end);
    }
    if let Some((bss_start, bss_end)) = bss_range {
        info!("bss_start: {:#x}, bss_end: {:#x}", bss_start, bss_end);
    }
    if let Some((sdata_start, sdata_end)) = sdata_range {
        info!(
            "sdata_start: {:#x}, sdata_end: {:#x}",
            sdata_start, sdata_end
        );
    }
    if let Some((data_start, data_end)) = data_range {
        info!("data_start: {:#x}, data_end: {:#x}", data_start, data_end);
    }

    // 清零 .sbss 和 .bss section
    // Zero out the .sbss and .bss sections
    if let Some((sbss_start, sbss_end)) = sbss_range {
        if sbss_start < sbss_end {
            unsafe {
                core::slice::from_raw_parts_mut(
                    sbss_start as *mut u8,
                    (sbss_end - sbss_start) as usize,
                )
                .fill(0);
            }
        }
    }

    if let Some((bss_start, bss_end)) = bss_range {
        if bss_start < bss_end {
            unsafe {
                core::slice::from_raw_parts_mut(
                    bss_start as *mut u8,
                    (bss_end - bss_start) as usize,
                )
                .fill(0);
            }
        }
    }

    let paddr = user_task.query_va(VirtAddr::from(0x7ffffda0)).unwrap();
    info!(
        "test_stack_translate vaddr: {:#x}, paddr: {:#x} flages: {:?}",
        0x7ffffda0, paddr.0, paddr.1
    );
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
    // 允许栈页可执行，使信号 tramp 链接可被取指执行
    let flags = MappingFlags::READ | MappingFlags::WRITE | MappingFlags::EXECUTE | MappingFlags::USER;

    let pages = USER_STACK_INIT_SIZE.div_ceil(PAGE_SIZE);
    let frames = alloc_continues(pages);
    let start_paddr = PhysAddr::from(frames[0].paddr.as_usize());
    let end_paddr = start_paddr + USER_STACK_INIT_SIZE;
    let mem_region = MemRegion::new_mapped(
        VirtAddrRange::new(VirtAddr::from(stack_start), VirtAddr::from(stack_end)),
        PhysAddrRange::new(start_paddr, end_paddr),
        flags,
    );
    user_task.pcb.lock().mem_set.push_region(mem_region.clone());
    let _ = user_task
        .page_table
        .lock()
        .map_region_user(&mut mem_region.clone());

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

    flush_tlb();
}
