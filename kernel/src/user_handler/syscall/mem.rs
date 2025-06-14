use crate::executor::error::TaskError;
use crate::user_handler::handler::UserHandler;
use memory_addr::VirtAddr;
impl UserHandler {
    pub async fn sys_brk(&mut self, addr: usize) -> Result<usize, TaskError> {
        // if addr != 0 {
        //     get_boot_page_table().change_pagetable();
        //     let mut heap = self.task.get_heap();
        //     let old_end = heap.get_end();
        //     debug!("sys_brk @ new: {:#x} old: {:#x}", old_end + addr, old_end);
        //     let new_end = heap.sbrk(addr, &mut self.task.page_table.lock());
        //     self.task.set_heap(heap);
        //     self.task.page_table.lock().change_pagetable();
        //     Ok(new_end)
        // } 
        // else {
        //     Ok(self.task.get_heap().get_end())
        // }
        let mut heap = self.task.get_heap();
        heap.virt_range.end = VirtAddr::from_usize(heap.virt_range.end.as_usize() + addr);
        self.task.set_heap(heap);
        Ok(heap.virt_range.end.as_usize())
    }
}
