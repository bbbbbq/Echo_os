#!/bin/bash

# 修复entry.rs文件
sed -i 's/let cx: &mut SignalUserContext = UserBuf::<SignalUserContext>::from(sp).get_mut();/let cx_buf = UserBuf::<SignalUserContext>::new(sp as *mut SignalUserContext);\n        let cx = unsafe { \&mut *(sp as *mut SignalUserContext) };/g' kernel/src/user_handler/entry.rs
sed -i 's/self.task.tcb.write().sigmask = sigaction.mask;/let mask_value = sigaction.mask.mask;\n        self.task.tcb.write().sigmask = crate::signal::flages::SigProcMask { mask: mask_value };/g' kernel/src/user_handler/entry.rs

# 在entry.rs中添加size_of导入
sed -i '/use crate::user_handler::handler::UserTaskControlFlow;/a use core::mem::size_of;' kernel/src/user_handler/entry.rs

# 修复thread.rs中的SigProcMask导入
sed -i 's/use crate::signal::{self, SigProcMask};/use crate::signal::flages::SigProcMask;/g' kernel/src/executor/thread.rs

# 修复mod.rs中的SigProcMask导入
sed -i 's/use crate::signal::SigProcMask;/use crate::signal::flages::SigProcMask;/g' kernel/src/user_handler/syscall/mod.rs

# 修复signal.rs中的SigProcMask导入
sed -i 's/use crate::signal::{SigMaskHow, SigProcMask};/use crate::signal::{SigMaskHow, flages::SigProcMask};/g' kernel/src/user_handler/syscall/signal.rs

# 修复list.rs中的SigProcMask导入
sed -i 's/use crate::signal::{self, SigProcMask};/use crate::signal::flages::{SigProcMask, SignalFlags};/g' kernel/src/signal/list.rs
sed -i '/use crate::signal::flages::SignalFlags;/d' kernel/src/signal/list.rs

echo "修复完成，尝试编译..."
cd /home/cgbc/os_dev_env/code/Echo_os && cargo build --target riscv64gc-unknown-none-elf 