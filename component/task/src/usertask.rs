use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use executor::task_def::TaskTrait;
use mem::pagetable::PageTable;     // 保留这个路径，假设 'pagetable' 是正确的模块名
use executor::id::{alloc_task_id, ProcId, TaskId};
use mem::memset::MemSet;
use filesystem::fd_table::FdTable;
use filesystem::path::Path;
use trap::trapframe::TrapFrame;
use spin::RwLock;
use executor::{ExitCode, TaskType};


pub struct ProcessControlBlock
{
    pub pagetable:PageTable,
    pub mem_set: MemSet,
    pub fd_table: FdTable,
    pub cwd: Path,
    pub entry: usize,
    pub children: Vec<Arc<UserTask>>,
    pub threads: Vec<Weak<UserTask>>,
    pub exit_code: usize,
    pub pro_id:ProcId,
}

pub struct ThreadControlBlock
{
    pub context: TrapFrame,
    pub thread_id:TaskId,
    pub thread_exit_code:u64,
}

pub struct UserTask
{
    pub pcb:RwLock<ProcessControlBlock>,
    pub tcb:RwLock<ThreadControlBlock>,
    pub parent: Arc<Weak<UserTask>>
}


impl TaskTrait for UserTask
{
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
        todo!("UserTask exit logic needs to be implemented, including resource cleanup, 
               notifying parent, setting exit codes in pcb/tcb, 
               and potentially process termination if it's the last thread.");
    }
}


impl UserTask
{
    pub fn new(parent:Weak<UserTask>,cur_dir:Path) -> Self
    {
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
            }),
            tcb: RwLock::new(ThreadControlBlock {
                thread_id,
                thread_exit_code: 0,
                context: TrapFrame::new(),
            }),
            parent: Arc::new(parent),
        }
    }

    // pub fn initwithelf(parent:Weak<UserTask>,file_path:Path,cur_dir:Path,args: Vec<String>,envp: Vec<String>) -> Self
    // {
    //     let task = UserTask::new(parent, cur_dir);

    //     []
    // }
}




