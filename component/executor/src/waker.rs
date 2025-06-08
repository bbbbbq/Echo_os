use alloc::{sync::Arc, task::Wake};

use crate::id::TaskId;

pub struct Waker {
    pub task_id: TaskId,
}

impl Wake for Waker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {}
}