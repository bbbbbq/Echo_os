#![no_std]

//! 驱动通用API模块
//!
//! 定义设备类型、驱动trait、块设备trait等通用接口。

use core::any::Any;
use downcast_rs::{impl_downcast, DowncastSync};

/// 设备类型枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    /// 块设备
    Block,
    /// 网络设备
    Network,
    /// 图形设备
    Gpu,
    /// 输入设备
    Input,
    /// 实时时钟
    Rtc,
    /// 串口设备
    Serial,
    /// 定时器
    Timer,
    /// 其他类型
    Misc,
}

extern crate alloc;

/// 驱动trait，所有驱动需实现。
pub trait Driver: DowncastSync + Send + Sync {
    /// 获取驱动ID。
    fn get_id(&self) -> usize;
    /// 获取设备类型。
    fn get_type(&self) -> DeviceType;
    /// 以Any类型返回自身引用。
    fn as_any(&self) -> &dyn Any;
    /// 尝试将驱动对象转换为块设备驱动。
    ///
    /// # 返回
    /// 若为块设备驱动，返回Some(Arc<dyn BlockDriver>)，否则返回None。
    fn try_get_block_driver(
        self: alloc::sync::Arc<Self>,
    ) -> Option<alloc::sync::Arc<dyn BlockDriver>>;
}
impl_downcast!(sync Driver);

/// 块设备驱动trait。
pub trait BlockDriver: DowncastSync + Driver {
    /// 读取指定块到缓冲区。
    fn read(&self, block_id: usize, buf: &mut [u8]) -> Result<(), &'static str>;
    /// 写入缓冲区到指定块。
    fn write(&self, block_id: usize, buf: &[u8]) -> Result<(), &'static str>;
    /// 获取块设备容量。
    fn capacity(&self) -> u64;
}
