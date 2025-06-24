use super::id_alloc::TaskId;
use crate::alloc::string::ToString;
use crate::executor::error::{self, TaskError};
use crate::executor::executor::{get_cur_usr_task, release_task, spawn};
use crate::executor::id_alloc::alloc_tid;
use crate::executor::task::{AsyncTask, AsyncTaskItem};
use crate::executor::{self, thread};
use crate::signal::flages::SigAction;
use crate::signal::list::{REAL_TIME_SIGNAL_NUM, SignalList};
use crate::signal::{self, SigProcMask};
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
use elf_ext::ElfExt;
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

//!
//! 用户任务与线程管理模块。
//!
//! 提供用户进程/线程的创建、内存管理、堆栈初始化、克隆、退出、文件描述符管理等。

/// 共享内存描述结构。
#[derive(Debug, Clone)]
pub struct Shm {
    /// 共享内存 ID
    pub shm_id: usize,
    /// 共享内存虚拟地址
    pub shm_addr: VirtAddr,
    /// 共享内存大小
    pub shm_size: usize,
}

/// 进程控制块。
#[derive(Debug, Clone)]
pub struct ProcessControlBlock {
    /// 文件描述符表
    pub fd_table: FdTable,
    /// 内存区域集合
    pub mem_set: MemSet,
    /// 当前工作目录
    pub curr_dir: Arc<Path>,
    /// 堆顶指针
    pub heap: usize,
    /// 程序入口地址
    pub entry: usize,
    /// 线程列表
    pub threads: Vec<Weak<UserTask>>,
    /// 信号处理表
    pub sigaction: [SigAction; 65],
    /// 子进程列表
    pub children: Vec<Arc<UserTask>>,
    /// 共享内存映射
    pub shms: BTreeMap<usize, Arc<Shm>>,
    /// 进程退出码
    pub exit_code: Option<usize>,
    /// 进程运行时间
    pub time: Option<Duration>,
}

/// 线程控制块。
#[derive(Clone)]
pub struct ThreadControlBlock {
    /// 用户栈区域
    pub stack_region: StackRegion,
    /// TrapFrame 上下文
    pub cx: TrapFrame,
    /// 信号掩码
    pub sigmask: SigProcMask,
    /// 信号列表
    pub signal: SignalList,
    /// 实时信号队列
    pub signal_queue: [usize; REAL_TIME_SIGNAL_NUM],
    /// 线程退出信号
    pub exit_signal: u8,
    /// 线程退出时清理的地址
    pub clear_child_tid: Option<usize>,
    /// 线程退出码
    pub thread_exit_code: Option<usize>,
}

/// 用户任务（进程/线程）结构。
#[allow(dead_code)]
pub struct UserTask {
    /// 任务 ID
    pub task_id: TaskId,
    /// 进程 ID
    pub process_id: TaskId,
    /// 页表
    pub page_table: Arc<Mutex<PageTable>>,
    /// 进程控制块
    pub pcb: Arc<Mutex<ProcessControlBlock>>,
    /// 父任务
    pub parent: RwLock<Weak<UserTask>>,
    /// 线程控制块
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
    /// 创建新的用户任务。
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

    /// 获取当前栈指针。
    pub fn get_sp(&self) -> usize {
        self.tcb.read().cx[TrapFrameArgs::SP]
    }

    /// 初始化 TrapFrame 上下文。
    pub fn init_cx(load_elf_return: LoadElfReturn) -> TrapFrame {
        let base = load_elf_return.base;
        let entry_point = load_elf_return.entry_point;
        let sp = load_elf_return.stack_region.get_top();
        let mut cx = TrapFrame::new();
        cx.set_sepc(base + entry_point);
        cx.set_sp(sp);
        cx
    }

    /// 向栈中压入字节数组。
    pub fn push_arr(&self, buffer: &[u8]) -> usize {
        self.tcb.write().stack_region.push_bytes(buffer)
    }

    /// 进程/线程组退出。
    pub fn exit_group(&self, exit_code: usize) {
        self.tcb.write().thread_exit_code = Some(exit_code);
        self.pcb.lock().exit_code = Some(exit_code);
    }

    /// 从 ELF 文件创建新任务。
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

    /// 向栈中压入一个 usize。
    pub fn push_num(&self, num: usize) -> usize {
        self.tcb.write().stack_region.push_num(num);
        let sp = self.tcb.read().stack_region.get_sp();
        self.tcb.write().cx.set_sp(sp);
        sp
    }

    /// 向栈中压入字节数组（私有）。
    fn push_array(&self, array: &[u8]) -> usize {
        self.tcb.write().stack_region.push_bytes(array);
        let sp = self.tcb.read().stack_region.get_sp();
        self.tcb.write().cx.set_sp(sp);
        sp
    }

    /// 向栈中压入字符串。
    pub fn push_str(&self, str: &str) -> usize {
        self.push_arr(str.as_bytes());
        let sp = self.tcb.read().stack_region.get_sp();
        self.tcb.write().cx.set_sp(sp);
        sp
    }

    /// 初始化任务栈。
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

    /// 获取 TrapFrame 的可变引用。
    pub fn force_cx_ref(&self) -> &'static mut TrapFrame {
        unsafe { &mut self.tcb.as_mut_ptr().as_mut().unwrap().cx }
    }

    /// 获取指定文件描述符的文件。
    pub fn get_fd(&self, fd: usize) -> Option<File> {
        self.pcb.lock().fd_table.get(fd)
    }

    /// 获取当前工作目录文件。
    pub fn get_cwd(&self) -> File {
        let path = self.pcb.lock().curr_dir.clone();
        File::open(&path.to_string(), filesystem::file::OpenFlags::O_DIRECTORY)
            .unwrap()
            .into()
    }

    /// 释放任务资源。
    pub fn release(&self) {
        // Ensure that the task was exited successfully.
        assert!(self.exit_code().is_some() || self.tcb.read().thread_exit_code.is_some());
        release_task(self.task_id);
    }

    /// 线程退出。
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

    /// 克隆线程。
    pub fn thread_clone(&self) -> Arc<Self> {
        let parent = self.tcb.read();

        let task_id = alloc_tid();
        let cur_tcb = RwLock::new(ThreadControlBlock {
            stack_region: self.tcb.read().stack_region.clone(),
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

    /// 克隆进程。
    pub fn process_clone(self: Arc<Self>) -> Arc<Self> {
        info!("process_clone");
        info!("task mem_set {:?}", self.pcb.lock().mem_set);

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
        drop(pcb);
        for region in parent_mem_set.regions.iter() {
            let mut new_region = region.clone();
            let _ = new_task.page_table.lock().map_region_user(&mut new_region);
            new_task.page_table.lock().protect_region(&mut new_region, MappingFlags::USER | MappingFlags::READ | MappingFlags::EXECUTE);
            new_pcb.mem_set.push_region(new_region.clone());
            parent_task.page_table.lock().protect_region(&mut new_region, MappingFlags::USER | MappingFlags::READ | MappingFlags::EXECUTE);
        }
        drop(new_pcb);
        drop(parent_task);
        new_task
    }

    /// sbrk 系统调用实现，扩展堆空间。
    pub fn sbrk(&self, addr: usize) -> usize {
        let curr_page = self.pcb.lock().heap.div_ceil(PAGE_SIZE);
        let after_page = addr.div_ceil(PAGE_SIZE);
        let frames = alloc_continues(after_page - curr_page);
        let start_paddr = PhysAddr::from(frames[0].paddr.as_usize());
        let size = (after_page - curr_page) * PAGE_SIZE;
        let end_paddr = start_paddr + size;
        let mut mem_region = MemRegion::new_mapped(
            VirtAddr::from(curr_page * PAGE_SIZE),
            VirtAddr::from(after_page * PAGE_SIZE),
            start_paddr,
            end_paddr,
            MappingFlags::USER | MappingFlags::WRITE | MappingFlags::READ,
            "heap_segment".to_string(),
            MemRegionType::HEAP,
        );
        self.pcb.lock().mem_set.push_region(mem_region.clone());
        let _ = self.page_table.lock().map_region_user(&mut mem_region);
        self.pcb.lock().heap = addr;
        addr
    }

    /// 查询虚拟地址对应的物理地址和映射标志。
    pub fn query_va(&self, va: VirtAddr) -> Option<(PhysAddr, MappingFlags)> {
        self.page_table.lock().translate(va)
    }

    /// 对 PCB 进行闭包操作。
    pub fn inner_map<T>(&self, mut f: impl FnMut(&mut MutexGuard<ProcessControlBlock>) -> T) -> T {
        f(&mut self.pcb.lock())
    }

    /// 向栈中压入字符串。
    pub fn push_str(&self, str: &str) -> usize {
        self.push_arr(str.as_bytes())
    }

    /// 向栈中压入字节数组。
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

    /// 向栈中压入一个 usize。
    pub fn push_num(&self, num: usize) -> usize {
        let mut tcb = self.tcb.write();

        const ULEN: usize = size_of::<usize>();
        let sp = tcb.cx[TrapFrameArgs::SP] - ULEN;

        *VirtAddr::from(sp).get_mut() = num;
        tcb.cx[TrapFrameArgs::SP] = sp;
        sp
    }

    /// 获取当前堆顶。
    pub fn get_heap(&self) -> usize {
        self.pcb.lock().heap
    }

    /// 获取下一个可用虚拟地址。
    pub fn get_last_free_addr(&self, size: usize) -> VirtAddr {
        static mut MMAP_BASE: usize = 0x20000000;
        unsafe {
            MMAP_BASE = MMAP_BASE + size;
        }
        unsafe { VirtAddr::from_usize(MMAP_BASE) }
    }
}

impl AsyncTask for UserTask {
    /// 运行前切换页表。
    fn before_run(&self) {
        self.page_table.lock().change_pagetable();
    }

    /// 获取任务 ID。
    fn get_task_id(&self) -> TaskId {
        self.task_id
    }

    /// 获取任务类型。
    fn get_task_type(&self) -> super::task::TaskType {
        super::task::TaskType::User
    }

    /// 退出任务。
    fn exit(&self, _exit_code: usize) {
        unimplemented!()
    }

    /// 获取退出码。
    fn exit_code(&self) -> Option<usize> {
        self.tcb.read().thread_exit_code
    }
}

/// 添加用户任务并返回任务 ID。
///
/// # 参数
/// - `filename`: 可执行文件名
/// - `args`: 参数列表
/// - `envp`: 环境变量列表
///
/// # 返回值
/// 返回新建任务的 TaskId。
pub async fn add_user_task(filename: &str, args: Vec<&str>, envp: Vec<&str>) -> TaskId {
    info!("Adding user task: {}", filename);
    let parent = get_cur_usr_task();
    if let Some(p) = &parent {
        info!("Parent task ID: {:?}", p.get_task_id());
    } else {
        info!("No parent user task found.");
    }
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

/// 以指定进程上下文执行 ELF 文件。
///
/// # 参数
/// - `task`: 目标用户任务
/// - `curr_dir`: 当前目录
/// - `path`: ELF 路径
/// - `args`: 参数
/// - `envp`: 环境变量
///
/// # 返回值
/// 返回新建的用户任务。
pub async fn exec_with_process(
    task: Arc<UserTask>,
    curr_dir: Path,
    path: String,
    args: Vec<String>,
    envp: Vec<String>,
) -> Result<Arc<UserTask>, TaskError> {
    info!("exec_with_process: {}", path);
    let path = curr_dir.join(&path);

    let mut file = File::open(&path.to_string(), filesystem::file::OpenFlags::O_RDONLY).unwrap();
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
            mem_region.is_mapped = true;
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

    info!("Creating task from file: {}", filename);
    let (task, load_elf_return) = UserTask::new_from_file(None, Path::new(filename.to_owned()))
        .expect("Failed to create task from file");

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

/// 初始化用户任务栈。
///
/// # 参数
/// - `user_task`: 目标用户任务
/// - `args`: 参数
/// - `base`: 程序基址
/// - `path`: 路径
/// - `entry_point`: 入口地址
/// - `ph_count`: 程序头数量
/// - `ph_entry_size`: 程序头大小
/// - `ph_addr`: 程序头地址
/// - `heap_bottom`: 堆底
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

    let pages = USER_STACK_INIT_SIZE.div_ceil(PAGE_SIZE);
    let frames = alloc_continues(pages);
    let start_paddr = PhysAddr::from(frames[0].paddr.as_usize());
    let end_paddr = start_paddr + USER_STACK_INIT_SIZE;
    let mem_region = MemRegion::new_mapped(
        VirtAddr::from(stack_start),
        VirtAddr::from(stack_end),
        start_paddr,
        end_paddr,
        flags,
        "stack".to_string(),
        MemRegionType::STACK,
    );
    user_task.pcb.lock().mem_set.push_region(mem_region.clone());
    let _ = user_task.page_table.lock().map_region_user(&mut mem_region.clone());

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
