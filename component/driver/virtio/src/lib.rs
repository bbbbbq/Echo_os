#![no_std]

//! Virtio 驱动适配层
//!
//! 提供 Virtio 块设备、HAL 实现等。

extern crate alloc;

/// 块设备驱动实现。
pub mod blk;
/// HAL 层实现。
pub mod halimpl;

