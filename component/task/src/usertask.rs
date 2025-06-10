
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

use executor::id::{ProcId, TaskId, alloc_task_id};
use executor::task_def::TaskTrait;
use executor::{ExitCode, TaskType};
use filesystem::fd_table::FdTable;
use crate::error::TaskError;
use mem::memset::MemSet;
use mem::pagetable::PageTable;
use spin::RwLock;
use trap::trapframe::TrapFrame;

use alloc::string::String;
use async_recursion::async_recursion;
use alloc::boxed::Box;
use crate::cache::{find_task_cache, load_elf_cache};
use filesystem::path::Path;

pub struct ProcessControlBlock {
    pub pagetable: PageTable,
    pub mem_set: MemSet,
    pub fd_table: FdTable,
    pub cwd: Path,
    pub entry: usize,
    pub children: Vec<Arc<UserTask>>,
    pub threads: Vec<Weak<UserTask>>,
    pub exit_code: usize,
    pub pro_id: ProcId,
    pub heap_bottom: usize,
}

pub struct ThreadControlBlock {
    pub context: TrapFrame,
    pub stack_top: usize,
    pub thread_id: TaskId,
    pub thread_exit_code: u64,
}

pub struct UserTask {
    pub pcb: RwLock<ProcessControlBlock>,
    pub tcb: RwLock<ThreadControlBlock>,
    pub parent: Arc<Weak<UserTask>>,
}

impl TaskTrait for UserTask {
    fn get_task_id(&self) -> TaskId {
        self.tcb.read().thread_id
    }

    fn get_task_type(&self) -> TaskType {
        TaskType::Thread
    }

    fn before_run(&self) {
        self.pcb.read().pagetable.change_pagetable();
    }

    fn get_exit_code(&self) -> ExitCode {
        let exit_val = self.tcb.read().thread_exit_code as i32;
        ExitCode::Normal(exit_val)
    }

    fn exit(&self) {
        // 实际的退出逻辑会很复杂，包括资源清理、通知父进程、调度等。
        // log::info!("Task {} received exit signal.", self.get_task_id().0);
        todo!(
            "UserTask exit logic needs to be implemented, including resource cleanup, 
               notifying parent, setting exit codes in pcb/tcb, 
               and potentially process termination if it's the last thread."
        );
    }
}

impl UserTask {
    pub fn new(parent: Weak<UserTask>, cur_dir: Path) -> Self {
        let thread_id = alloc_task_id();
        let process_id = ProcId::new();

        Self {
            pcb: RwLock::new(ProcessControlBlock {
                pro_id: process_id,
                pagetable: PageTable::new(),
                mem_set: MemSet::new(),
                fd_table: FdTable::new(),
                cwd: cur_dir,
                entry: 0,
                children: Vec::new(),
                threads: Vec::new(),
                exit_code: 0,
                heap_bottom: 0,
            }),
            tcb: RwLock::new(ThreadControlBlock {
                thread_id,
                thread_exit_code: 0,
                stack_top: 0,
                context: TrapFrame::new(),
            }),
            parent: Arc::new(parent),
        }
    }

    pub fn push_stack(&self, data: u64) {
        let mut tcb = self.tcb.write();
        let sp = tcb.context.get_sp();
        let new_sp = sp - core::mem::size_of::<u64>();
        let sp_ptr = new_sp as *mut u64;
        unsafe {
            *sp_ptr = data;
        }
        tcb.context.set_sp(new_sp);
    }
}

#[async_recursion(Sync)]
pub async fn exec_with_process(
    path: Path,
    cur_dir: Path,
    args: Vec<String>,
    _envp: Vec<String>,
) -> Result<Arc<UserTask>, TaskError>
{
    let mut elf_info = find_task_cache(path.clone());
    if elf_info.is_none()
    {
        elf_info = Some(load_elf_cache(path));
    }
    let elf_info = elf_info.unwrap();

    let task = Arc::new(UserTask::new(Weak::new(), cur_dir.clone()));

    task.pcb.write().entry = elf_info.entry_point;
    task.pcb.write().pagetable.map_mem_set_user(elf_info.memset.clone());
    task.pcb.write().mem_set = elf_info.memset;
    task.pcb.write().heap_bottom = elf_info.heap_bottom;
    task.pcb.write().cwd = cur_dir;
    task.tcb.write().stack_top = elf_info.stack_top;
    task.tcb.write().context.set_sp(elf_info.stack_top);
    task.tcb.write().context.set_sepc(elf_info.base + elf_info.entry_point);
    


    Ok(task)
}