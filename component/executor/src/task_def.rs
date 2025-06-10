use core::pin::Pin;

use alloc::{boxed::Box, sync::Arc};
use downcast_rs::{DowncastSync, impl_downcast};

use super::id::TaskId;
use crate::{ExitCode, TaskType};
use core::fmt::Debug;
pub trait TaskTrait: Send + Sync + DowncastSync + Debug {
    fn get_task_id(&self) -> TaskId;

    fn get_task_type(&self) -> TaskType;

    fn before_run(&self);

    fn get_exit_code(&self) -> ExitCode;

    fn exit(&self);
}

impl_downcast!(sync TaskTrait);

pub type PinedFuture = Pin<Box<dyn Future<Output = ()> + Send + Sync + 'static>>;

pub struct Task {
    pub task_inner: Arc<dyn TaskTrait>,
    pub task_future: PinedFuture,
}
